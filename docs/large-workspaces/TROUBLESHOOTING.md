# Large-Workspace Troubleshooting

Diagnosis steps and remediation strategies for performance and correctness
issues that only appear at scale (5 000+ files, 6+ hours of uptime).

For general setup problems (binary not found, server won't start), see
`docs/how-to/TROUBLESHOOTING.md` first.

---

## Quick Triage

Before digging into logs:

```bash
perllsp --version    # confirm the right binary is running
perllsp --health     # print index state, symbol count, SLO snapshot
RUST_LOG=perl_lsp=debug perllsp --stdio 2>perllsp-debug.log
```

The `--health` output includes:

- `index_state`: `Ready`, `Building`, or `Degraded`
- `file_count` / `symbol_count`: current index size
- `slo_compliance`: pass/fail per operation type

If `index_state` is `Degraded`, check the `reason` field — this is the
first branching point in every large-workspace investigation.

---

## Index Enters `Degraded` State

### Symptoms

- Completions become stale or empty
- Go-to-definition stops working for recently added files
- `perllsp --health` shows `index_state: Degraded`

### Causes and Remediation

| Cause | How to confirm | Fix |
|-------|----------------|-----|
| `max_files` limit hit | `reason: ResourceLimit(Files)` | Raise `maxIndexedFiles` or add `.lspignore` |
| `max_total_symbols` limit hit | `reason: ResourceLimit(Symbols)` | Raise `maxTotalSymbols` |
| Scan timeout | `reason: Timeout` | Raise `workspaceScanDeadlineMs` |
| IO error | `reason: IoError` | Check disk health; check NFS mount |
| Parse storm | `reason: ParseStorm` | Reduce concurrent editors; check for watch loops |

To trigger recovery after fixing the root cause, run
`workspace/executeCommand: perl.reindex` from the editor command palette,
or restart the LSP server.

---

## Slow Startup (>10s on a Large Workspace)

### Symptoms

- Editor shows "loading" spinner for an unusually long time
- `RUST_LOG=perl_lsp=debug` shows many `Indexing file…` messages spaced
  far apart

### Diagnosis

```bash
# Time the full index cycle end-to-end
time RUST_LOG=perl_lsp=info perllsp --stdio < /dev/null 2>&1 \
    | grep -E "index (started|completed|degraded)"
```

Compare against the expected baseline from `TESTING_GUIDE.md`.

### Common Causes

1. **Network filesystem**: NFS or SMB adds per-file `open()` latency.
   Migrate the workspace to a local disk or ramdisk for development.

2. **Deep directory tree with many non-Perl files**: The scanner visits
   every directory. Exclude heavy directories:

   ```json
   { "perl": { "workspace": { "excludePatterns": ["node_modules", ".git", "vendor"] } } }
   ```

3. **Very large individual files**: A single 50 000-line Perl script can
   take hundreds of milliseconds to parse. Confirm with:

   ```bash
   wc -l lib/**/*.pm | sort -rn | head -10
   ```

   Break up files larger than ~5 000 lines into modules, or add them to
   the exclude list.

---

## IDE Slowdowns After 4-6 Hours

This is the classic large-workspace degradation pattern. The index starts
fast but response times climb over a long session.

### Symptoms

- Hover and completion were fast at session start, now consistently >200ms
- Memory usage visible in the OS task manager has grown significantly
- No crash; the server is still running

### Diagnosis Steps

1. **Check the SLO snapshot**:

   ```bash
   perllsp --health 2>&1 | grep slo
   ```

   If `slo_compliance: fail` for `hover` or `completion`, the latency
   has crossed the hard limit.

2. **Check for cache bloat**:

   After 4-6 hours on a large workspace, the AST cache may be evicting and
   re-inserting entries frequently. Enable the cache-stats log:

   ```bash
   RUST_LOG=perl_workspace_index::workspace::cache=debug perllsp --stdio \
       2>&1 | grep "eviction\|hit_rate"
   ```

   A healthy hit rate is >90%. If you see `hit_rate: 0.4` or lower, the
   cache is too small for the working set.

3. **Check for write-lock contention**:

   ```bash
   RUST_LOG=perl_workspace_index=trace perllsp --stdio 2>&1 \
       | grep "write.lock\|lock.wait"
   ```

4. **Check for unbounded document store growth**:

   If the editor opens many files and never closes them, the `DocumentStore`
   grows. Count open documents:

   ```bash
   perllsp --health 2>&1 | grep "open_documents"
   ```

### Remediation

- **Short term**: Restart the LSP server. This flushes all caches and
  returns to baseline memory.
- **Medium term**: Raise `astCacheMaxEntries` if hit rate is low:

  ```json
  { "perl": { "limits": { "astCacheMaxEntries": 5000 } } }
  ```

- **Long term**: If restart frequency is high (more than once per day),
  file an issue with the `--health` output and the log snippet showing
  where latency grows. This is likely a caching or index-invalidation bug.

---

## Symbol Resolution Returns Wrong Results

### Symptoms

- Go-to-definition jumps to the wrong file
- `workspace/symbol` returns symbols that no longer exist
- Completions include renamed or deleted subs

### Diagnosis

The dual indexing strategy (PR #122) indexes symbols under both
`Package::name` and `name`. Stale entries in either table produce
ghost results.

```bash
# Ask the server to dump the index state (if the dump command is enabled)
RUST_LOG=perl_workspace_index::workspace::workspace_index=debug \
    perllsp --stdio 2>&1 | grep "stale\|evict\|remove"
```

Check whether the workspace received a `workspace/didDeleteFiles` or
`workspace/didChangeWatchedFiles` notification for the affected file.
If not, the editor's file-watcher may be misconfigured.

### Remediation

1. Reindex: `perl.reindex` command from the editor command palette.
2. If stale symbols persist after reindex, the index may not be removing
   old file entries before re-inserting new ones. Confirm by checking
   `WorkspaceIndex::index_file`'s remove-then-insert pattern.

---

## Workspace Index Corruption

### Symptoms

- Panic or internal error in the LSP server log
- `index_state` cannot transition out of `Error`
- Symbols from deleted packages still appear after full reindex

### Diagnosis

```bash
RUST_LOG=perl_workspace_index=error perllsp --stdio 2>&1 \
    | grep -E "ERROR|WARN|panic"
```

Corruption is rare because `WorkspaceIndex` uses `parking_lot::RwLock`
to prevent concurrent writes. If you see it, check:

- Whether the workspace was written to by another process while the
  server was indexing (race condition in file watchers).
- Whether a recent change modified `workspace_index.rs` without updating
  the remove-before-insert invariant.

### Remediation

Restart the LSP server. If corruption recurs after restart, reduce
`maxIndexedFiles` and report with a minimal reproduction.

---

## Collecting a Diagnostic Bundle

When filing a performance issue, include:

```bash
# Version and health
perllsp --version
perllsp --health

# Debug log (5-10 seconds of activity)
RUST_LOG=perl_lsp=debug,perl_workspace_index=debug \
    perllsp --stdio 2>perllsp-debug.log &
# ... trigger the slow operation in the editor ...
kill %1

# Workspace size
find . -name "*.pm" -o -name "*.pl" | wc -l
find . -name "*.pm" -o -name "*.pl" | xargs wc -l | tail -1
```

Attach `perllsp-debug.log` (trimmed to the relevant time window), the
version string, the health output, and the workspace size numbers.

---

## See Also

- `TESTING_GUIDE.md` — reproducing issues with synthetic workspaces
- `PROFILING_GUIDE.md` — capturing flamegraphs and heap profiles
- `MEMORY_PATTERNS.md` — understanding why memory grows
- `docs/how-to/TROUBLESHOOTING.md` — general (non-large-workspace) issues
- `docs/reference/PERFORMANCE_SLO.md` — SLO targets and degradation thresholds
