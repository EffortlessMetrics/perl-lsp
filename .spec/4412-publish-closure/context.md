# Context: publish-closure gate implementation

**Issue**: #4412
**Parent epic**: #4410 (Microcrate collapse, v0.14.x)
**Design document**: PR #4413 (ADR-0041, microcrate collapse design)
**Scope**: `publish-closure` subcommand ONLY

---

## Why This Scope

The microcrate collapse (issue #4410) is a major refactor spanning 6+ months and multiple waves:
- **Wave A**: Collapse perl-workspace-index family (3 crates -> 1 module)
- **Wave B**: Collapse perl-lsp-* provider crates (merge into single lsp crate)
- **Wave C**: Cleanup and module reorg

During collapse, crates may become:
1. Absorbed as modules into a parent crate
2. Marked `publish = false` in workspace (no longer released to crates.io)
3. Transitively depended on by still-published crates

**The problem**: If a *published* crate still transitively depends on a *non-published* crate, `cargo publish` will fail when dependencies are resolved downstream. This is silent in the local workspace (internal refs work) but breaks crates.io users.

**This gate**: Pre-collapse safeguard. Before a Wave PR merges:
1. Identify the set of `publish = false` workspace crates
2. For each published crate, walk the transitive normal-dep closure
3. Flag if any non-published crate appears in that closure
4. BLOCK merge if violations exist

**When to trigger**: Every PR in the collapse wave.

**Cost**: ~100ms (single cargo metadata call + BFS).

---

## Scope Boundaries

### IN SCOPE (PR #1)
- `cargo xtask publish-closure` — full implementation
- `--crate-name` filter flag
- justfile recipe + pr-fast/ci-gate wiring
- Integration tests
- Closure verification only (not publish order)

### DEFERRED (separate issues, filed)
- `cargo xtask layer-check` (#4415) — verify only same-layer crates are public
- `cargo xtask published-crate-count` (#4416) — track shrinking count during collapse
- `layer-rules.toml` — formal layer configuration (needed for layer-check)

### OUT OF SCOPE
- Publish order (handled by `scripts/publish-topo.py`)
- Transitive dev/build-dep checking (only normal deps matter)
- External crate verification (only workspace members are violations)
- Pre-collapse warning generation

---

## Key Design Decisions

### 1. Metadata without --no-deps (CRITICAL)

**Decision**: Call `cargo metadata --format-version 1` WITHOUT `--no-deps`.

**Why**: The `--no-deps` flag strips the `resolve` section, which is essential for transitive closure walking. Without `resolve`, we can only see direct deps, not the full tree.

**Existing pattern**: `xtask/src/tasks/publish.rs` uses `--no-deps` (line 98) because it only needs the package list. We cannot reuse that code.

**Verification**: Confirmed against actual `cargo metadata` output on this workspace (April 2026).

### 2. Field name: `pkg` not `id`

**Decision**: In `resolve.nodes[].deps[]`, the dependency package ID field is named `pkg` in JSON.

**Why**: Cargo's JSON schema names the field `pkg` (not `id`). Confusing when packages also have an `id` field, but that's the spec.

**Evidence**: Verified against live `cargo metadata --format-version 1` output on perl-lsp workspace.

**Implication**: `#[serde(rename)]` is NOT needed — the field is already `pkg` in both struct and JSON.

### 3. Normal-dep filtering via DepKind

**Decision**: Follow only edges where `dep_kinds` contains `DepKind { kind: None, .. }`. Skip `kind == Some("dev")` and `kind == Some("build")`.

**Why**:
- Normal (non-dev, non-build) deps are what end up in users' dependency trees
- Dev deps (used by tests) and build deps (used by build.rs) can be non-published
- Published crate cannot expose a normal dep on unpublished code

**Edge case handling**:
- If `dep_kinds` is empty: treat as normal dep (conservatively)
- If `target` is specified (platform-specific): still counts (users on that platform get the dep)

### 4. Publish = false detection

**Decision**: A crate is `publish = false` if `publish: Some(vec![])` (empty JSON array).

**Pattern**: Matches `scripts/publish-topo.py` lines 85-89:
```python
def is_no_publish(pkg_name):
    return pkg_name in NO_PUBLISH_CRATES or (
        cargo_metadata.get(pkg_name, {}).get("publish") == []
    )
```

**Workspace members only**: Non-workspace crates (external, from crates.io) are never violations. They're trusted by virtue of being on the registry.

### 5. Allowlist source

**Decision**: Read published crate set from `[workspace.metadata.publish.allow]` in root Cargo.toml.

**Why**: Already authoritative source (used by `publish-topo.py`). Single source of truth.

**Location**: Root `Cargo.toml`, line 147+. 132 crates as of April 2026.

### 6. Workspace member identification

**Decision**: `workspace_members` in cargo metadata is a list of `cargo_manifest://` URIs. Extract crate name from the parent directory of each Cargo.toml path.

**Why**: Cargo doesn't directly expose member names; we derive them from paths.

**Edge case**: A path like `crates/perl-foo/Cargo.toml` -> crate name is `perl-foo` (directory name).

---

## Alternatives Rejected

### A. Reuse CargoMetadata from publish.rs
**Rejected**: That struct uses `--no-deps`, which drops the `resolve` section. Can't add `resolve` field without rerunning cargo, defeating the purpose of reuse.

### B. Parse Cargo.toml directly instead of cargo metadata
**Rejected**: Would need to handle workspace inheritance, path deps, conditional deps. Cargo metadata already does this correctly. Cargo metadata is the source of truth.

### C. Check publish closures at crate-publish time
**Rejected**: Would be too late (publish may silently fail downstream). Checking preemptively in the merge gate is better UX and faster feedback.

### D. Generate a `layer-rules.toml` as part of this PR
**Rejected**: Scope creep. layer-check needs this; publish-closure doesn't. Deferred to #4415.

### E. Filter violations to only direct violations
**Rejected**: Transitive violations are real problems. A published crate that depends on B, which depends on non-published C, still breaks users. Must report C.

---

## Implementation Pattern: BFS vs DFS

**Chosen**: BFS (breadth-first search) with explicit queue.

**Why**: 
- Workspace graph can have cycles (Rust allows cycles via dev deps)
- BFS with visited set avoids infinite loops
- Simpler to understand than DFS with back-edges
- Performance is identical for this graph size

**Implementation**:
```rust
fn check_transitive_closure(...) -> Vec<String> {
    let mut visited = HashSet::new();
    let mut queue = VecDeque::new();
    let mut bad_deps = Vec::new();
    
    queue.push_back(start_id.to_string());
    visited.insert(start_id.to_string());
    
    while let Some(current_id) = queue.pop_front() {
        // process node, check deps, enqueue unvisited
    }
    
    bad_deps
}
```

---

## Error Handling Philosophy

**Failed cargo metadata**: Bail with context.

**Crate not in allowlist**: Name it explicitly in error ("Crate 'foo' not found in publish allowlist").

**Multiple violations**: Report ALL before exiting (not fail-fast). Gives users a complete picture.

**Violations in closure**: Include both the published crate and the forbidden dep in the message:
```
ERROR: publish-closure violation
  Published crate `perl-foo` has transitive normal dep on `perl-ci-hygiene` (publish = false)
```

---

## Testing Strategy

### Unit tests
Three integration tests via `assert_cmd` (existing xtask pattern):
1. `publish_closure_passes_on_master` — full check, expect success
2. `publish_closure_single_crate_flag` — filter to one crate, expect success
3. `publish_closure_unknown_crate_exits_nonzero` — filter to nonexistent crate, expect failure

**Why assert_cmd, not unit tests**: We're testing the CLI contract, not internal functions. Real cargo metadata parsing matters.

### Why only 3 tests (not more)
- We can't easily mock a complex graph in tests
- Master is currently clean (no violations) — can't test violation detection without external setup
- Bug detection relies on the real workspace (real data)
- Regression detection works via CI gate on actual PRs

### No snapshot tests
Snapshot tests would need real graph fixtures (complex). Not worth the maintenance burden for a gate that's rarely wrong.

---

## Performance Characteristics

- **Typical run**: ~100-150ms (single cargo metadata call + BFS)
- **Scales with**: Transitive dep depth and number of published crates
- **Parallelizable**: No — single metadata call is sequential
- **Memory**: Negligible (~10MB metadata + small graph structures)

---

## Related Systems

### `scripts/publish-topo.py`
- **Computes**: Topological publish order (what to publish first)
- **Does NOT check**: Closure correctness
- **Relationship**: publish-closure is a complementary gate

### `xtask/src/tasks/publish.rs`
- **Does**: Publishes crates to crates.io
- **Uses**: `cargo metadata --no-deps` (only package list)
- **Relationship**: publish-closure runs BEFORE publish (blocks bad collapses)

### Future `layer-check` (#4415)
- **Will do**: Verify crate layers (only same-layer crates are public)
- **Relationship**: Stricter than publish-closure (closure correctness vs layer correctness)

---

## Maintenance Notes

### If the allowlist changes
The gate automatically adapts (reads from Cargo.toml each run).

### If workspace structure changes
The gate automatically adapts (cargo metadata always current).

### If the algorithm needs tweaking
Look in `xtask/src/tasks/publish_closure.rs`:
- `load_metadata()` — cargo metadata call
- `check_transitive_closure()` — BFS logic
- `run()` — main orchestration

### Known limitations
1. No distinction between "this crate is unpublishable" and "this crate was unpublished as part of collapse". Both are `publish = false`.
2. External crates never flagged as violations (correct behavior, but not detectable if there's a misunderstanding in external dependency).

---

## Roadmap Context

**When does this ship**:
- Implements PR #1 of the collapse (gates Wave A)
- Lands in 0.13.1 or 0.14.0 (post-0.13.0 public alpha)

**Future layers**:
- PR #2: Red-TDD tests for layer-check and published-crate-count
- PR #3: Implement layer-check + published-crate-count
- PR #4+: Execute collapse waves (A, B, C) with these gates active

---

## Why We Don't Publish This Separately

The gate is **xtask-only** (build tooling). It's not published to crates.io as a library. It's internal build automation, like `forbid-fatal-constructs` or `corpus-audit`.

**Users don't call this directly.** It runs in the CI gate when developers submit PRs during the collapse.

---

## References

- **Parent issue**: #4410 (Microcrate collapse)
- **Design ADR**: PR #4413 (ADR-0041)
- **Follow-ups**: 
  - #4415 (layer-check)
  - #4416 (published-crate-count)
- **Existing pattern**: `scripts/publish-topo.py` (Python, publish order)
- **Similar gates**:
  - `xtask forbid-fatal-constructs` (pattern for gated checks)
  - `xtask ci-hygiene` (pattern for linting)
