# Async & Performance Improvements Scout Report

**Date**: 2026-03-20
**Status**: COMPLETE AUDIT
**Priority Ordering**: Below by impact & effort

---

## Executive Summary

Pearl LSP has a mixed async/perf story:
- **Debounce** (✅): Implemented, 250ms coalesce window, thread-based
- **Cancellation** (⚠️): Infrastructure exists but **NOT WIRED to parser calls**
- **Subprocesses** (❌): No timeouts on `perl -c`, `perlcritic`, `perltidy`
- **Incremental parsing** (❌): Advertises as incremental, does full reparse every time
- **Workspace indexing**: Synchronous, happens on main thread, blocks document updates
- **Request prioritization**: Completion dedup exists, but no priority queue or stale request cancellation
- **Memory management**: AST cache bounded (100 entries, 300s TTL), but no LRU sweep

---

## Detailed Findings

### 1. Parser Cancellation: Infrastructure Without Wiring ⚠️ CRITICAL

**Status**: PR #2268 merged, but **incomplete integration**

**Current State**:
- `perl_lsp_cancellation` crate provides `PerlLspCancellationToken` with atomic checks (<100μs latency)
- Token has 3 check methods: `is_cancelled()`, `is_cancelled_relaxed()`, `is_cancelled_hot_path()`
- Provider cleanup context exists for graceful cancellation

**The Problem**:
- `crates/perl-lsp/src/runtime/text_sync.rs:329` calls `Parser::new()` **with NO cancellation token**
- Same at line 89 (didOpen) and line 330 (didChange)
- The cancellation token is created but **never passed to the parser**
- Parser runs to completion regardless of `$/cancelRequest` from client

**Evidence**:
```rust
// Line 329 in text_sync.rs - NO CANCELLATION TOKEN
let mut parser = Parser::new(code_text);
match parser.parse() {
    Ok(ast) => { /* ... */ }
    Err(e) => { /* ... */ }
}
```

**Impact**: User cancels slow parse → 3-5s parse continues anyway → blocking response to other requests

**Fix Required**: Wire `cancellation_token` through `Parser::parse()` signature and add periodic checks in hot loops

---

### 2. Diagnostic Debounce: ✅ Working Well

**Status**: PR #2273 merged, **production ready**

**Implementation**:
- `crates/perl-lsp/src/runtime/diagnostic_debounce.rs`: Dedicated debouncer thread
- 250ms coalesce window (configurable via `DiagnosticDebouncer::with_interval()`)
- Per-URI debounce tracking with timeout management
- Fires pending on Drop (graceful shutdown)

**Integration**:
- Called at `text_sync.rs:432`: `self.publish_diagnostics_debounced(uri)`
- Rapid typing triggers schedule, waits 250ms, publishes once
- Tests verify: reset-on-repeat, multi-URI handling, pending-on-drop

**Performance**: ~50ms test debounce interval shows 0 publications during rapid fire, 1 after quiet period

**✅ No action needed**

---

### 3. Subprocess Timeouts: ❌ MISSING ON ALL SUBPROCESSES

**Critical Gaps**:

#### 3.1 Perltidy (formatting)
- **File**: `crates/perl-lsp-perltidy/src/lib.rs:199-237`
- **Issue**: Uses `SubprocessRuntime::run_command()` which has **NO timeout**
- **Risk**: Hangs on malformed input; blocks formatter indefinitely
- **Evidence**: `OsSubprocessRuntime::run_command()` at line 127 calls `child.wait_with_output()` — unbounded wait

#### 3.2 Perlcritic (linting)
- **File**: `crates/perl-lsp-tooling/src/perl_critic/mod.rs`
- **Issue**: Subprocess calls lack timeout wrapper
- **Risk**: perlcritic policy files can cause slowdowns; no protection

#### 3.3 Perl -c (syntax check)
- **Implicit in parser diagnostics**
- **Not yet researched**: Need to check if perl-lsp runs `perl -c` or parses directly

**SubprocessRuntime Trait**:
- Defined in `crates/perl-subprocess-runtime/src/lib.rs`
- Trait signature:
  ```rust
  fn run_command(&self, program: &str, args: &[&str], stdin: Option<&[u8]>)
    -> Result<SubprocessOutput, SubprocessError>
  ```
- **No timeout parameter**, no cancellation token, no signal handling

**Fix Required**: Add timeout wrapper to `OsSubprocessRuntime` + wire through trait. Suggest 10s default for perltidy, 5s for perlcritic.

---

### 4. Incremental Parsing: ❌ FALSE ADVERTISING

**The Claim**:
- Text sync advertises `TextDocumentSyncKind::Incremental` (line 8 comment in `text_sync.rs`)

**Reality**:
- Client sends range-based edits → `apply_changes()` merges them into `Rope`
- **Then entire document is reparsed** (line 330: full `Parser::new(code_text).parse()`)
- `crates/perl-incremental-parsing/` crate exists but is **NOT USED**

**Evidence**:
```rust
// Line 248-272: incremental TEXT SYNC (edits are merged)
// Line 327-339: FULL PARSE (entire AST rebuilt)
```

**Comment at line 9**: "Incremental *parsing* is future work"

**Impact**: Typing on large files (1000+ lines) causes full reparse stall every keystroke

**Fix**: Either:
- Change advertised sync kind to `Full` (2), or
- Implement real incremental parsing using `perl-incremental-parsing` crate (6-phase roadmap exists)

---

### 5. Workspace Indexing: ⚠️ SYNCHRONOUS BLOCKING

**Status**: Indexes on didChange + didOpen (before diagnostics publication)

**Current Flow**:
- `text_sync.rs:408-420`: Gets workspace index, calls `index_file()` synchronously
- Blocks main thread until indexing complete
- Happens BEFORE `notify_parse_complete()` coordinator notification

**Performance Risk**:
- Large files (>10K lines) with many symbols: indexing can take 100-200ms
- Blocks diagnostics publication and other LSP requests

**No Async Handling**:
- No `tokio::spawn()`, no `async fn`
- No cancellation support
- No progress reporting to client

**Fix**: Convert to async with `tokio::task::spawn_blocking()` or detach indexing to background task

---

### 6. Request Prioritization & Deduplication: ⚠️ PARTIAL

**What Exists**:
- Completion deduplication: `deduplicate_and_sort()` in `perl-lsp-completion` merges duplicates
- Completion sorting: Priority field on `CompletionItem`, sorted by scope distance
- File watcher registration: Dynamic registration for `.pl`, `.pm`, `.t`, `.psgi` files

**What's Missing**:
- No request queue with prioritization (hover > completion > references)
- No stale request cancellation when newer request arrives
- No deduplication across providers (hover vs completion on same position)
- No work-in-progress request tracking

**Evidence of manual priority**:
- `completion/scope_distance.rs:15`: "Variants are ordered from closest to farthest"
- But this is baked into algorithm, not a scheduler

**Fix**: Add request prioritization layer that:
- Prioritizes hovering over completion
- Cancels earlier request if new one arrives for same file+position
- Tracks in-flight requests to avoid duplicate work

---

### 7. Memory & Resource Management: ✅ AST CACHE BOUNDED

**AST Cache**:
- **File**: `crates/perl-lsp-performance/src/lib.rs:20-80`
- **Config**: 100 entries max, 300s TTL
- **Eviction**: FIFO when capacity exceeded
- **Integration**: Called in `text_sync.rs:83` (didOpen), line 323 (didChange)

**Limits Configuration**:
- `perl-lsp-limits/src/lib.rs:60,139`: `ast_cache_max_entries: 100`, `ast_cache_ttl_secs: 300`
- Mode override for embedded: reduced to 50 entries

**✅ No action needed** — appropriate bounds already in place

---

### 8. Startup Performance: ⚠️ UNKNOWN

**Initialize Flow**:
- Client sends `initialize` request
- Server calls `register_file_watchers_async()` at `lifecycle/watchers.rs:13`
- No workspace scan at startup; indexing happens on-demand

**Unknown**:
- Latency for first `initialized` notification (file watcher registration)
- Workspace indexing startup time with N Perl files (100? 1000?)
- Progress reporting to client during workspace scan

**What We Know**:
- File watcher registration is non-blocking (sends request, doesn't wait for response)
- No indexing until first `didOpen` or `didChange`
- No progress bar or `$/progress` notification

**Risk**: Large workspace (>1000 files) may appear hung during initial symbol index

---

## Priority-Ordered Improvement List

| Priority | Category | Title | Effort | Impact | Why |
|----------|----------|-------|--------|--------|-----|
| **P0** | Correctness | Wire parser cancellation token from text_sync | 2h | HIGH | User cancels → hangs for 3-5s, blocks all LSP requests |
| **P1** | Reliability | Add subprocess timeouts (perltidy, perlcritic) | 3h | HIGH | Hang risk on malformed input or slow policies |
| **P1** | Correctness | Fix incremental parsing false claim | 1h (docs) or 20h (impl) | HIGH | If implementing, huge perf win on large files |
| **P2** | Performance | Async workspace indexing | 4h | MEDIUM | Unblock diagnostics on large files |
| **P2** | Performance | Request prioritization + stale cancellation | 6h | MEDIUM | Reduces wasted work on rapid typing |
| **P3** | Observability | Workspace startup progress reporting | 2h | LOW | Better UX for large workspaces (100+ files) |
| **P4** | Performance | Async per-file watcher updates | 3h | LOW | Batch file watch events during rapid changes |

---

## Wiring Checklist for P0 (Parser Cancellation)

1. ✅ Cancellation token infrastructure exists (`perl_lsp_cancellation`)
2. ✅ Atomic check methods available (<100μs)
3. ❌ Token **not passed to** `Parser::new()` or `parser.parse()`
4. ❌ Parser loops **not checking** cancellation flag
5. ❌ No cancellation wiring in `dispatch/mod.rs` request handler

**Steps to complete**:
- Add optional `Option<CancellationToken>` parameter to `Parser::parse()`
- Insert periodic `if token.is_cancelled() return Err()` checks in hot loops (every 100K nodes?)
- Create cancellation token in `dispatch.rs` before calling text_sync
- Pass token through text_sync → didChange → parser.parse()
- Test: rapid typing + cancel → response within 100ms

---

## Files to Investigate Further

1. `crates/perl-parser/src/parser.rs` — where to add cancellation checks
2. `crates/perl-lsp/src/runtime/dispatch/mod.rs` — where to create tokens
3. `crates/perl-lsp/src/runtime/serving.rs` — request handling loop
4. `crates/perl-lsp-tooling/src/perl_critic/` — perlcritic subprocess calls
5. `benchmarks/` — establish baseline before perf work

---

## Questions for Clarification

1. Should `perl -c` be used for syntax validation, or is parser-only sufficient?
2. What's the target latency for cancellation (currently unbounded)?
3. Should incremental parsing be a v0.13 or v0.14 feature?
4. Are there known workspace sizes we should optimize for?
5. Should perltidy/perlcritic have user-configurable timeouts?

