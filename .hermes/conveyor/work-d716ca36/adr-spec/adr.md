# ADR-00XX: Thread @INC Paths Through CompletionProvider for Module Name Completion

## Status
Proposed

## Context

Module completion for `use` and `require` statements (GitHub #4314) only suggests modules from the workspace index, ignoring configured include paths (`.perl-lsp.toml` `includePaths`) and system @INC (when `useSystemInc: true`). Goto-definition already correctly searches all @INC sources, creating an inconsistency where developers see completion for modules that goto-definition cannot find, and cannot see completion for modules that goto-definition can find.

The root cause is architectural: `add_use_module_completions()` in `crates/perl-lsp-completion/src/completion/workspace.rs:198-251` only queries `WorkspaceIndex::find_symbols()` and `WorkspaceIndex::all_symbols()`. It never receives include paths.

The fix must enable completion to discover modules outside the workspace (e.g., `DBI`, `Moo`, `Moose` installed via cpan or system packages) without breaking existing behavior for workspace-only users.

## Decision

Thread `include_paths` and `system_inc_paths` through the `CompletionProvider` struct and extend `add_use_module_completions()` to scan include directories for `.pm` files matching the completion prefix.

**Option B** (Thread Through Provider) is chosen over:
- **Option A** (Add to WorkspaceIndex): Rejected because WorkspaceIndex is architecturally scoped to workspace files only (per ADR-0009 dual indexing strategy); expanding it to track external modules violates separation of concerns and bloats the index.
- **Option C** (Separate External Module Index): Rejected as unnecessary complexity — the same information (include paths, system paths) already exists in config and is already used by goto-definition.

### Architectural Pattern

The approach mirrors `resolve_module_to_path_with_doc()` in `module_resolution.rs:129-196`:
1. Get `config.include_paths.clone()` and `config.use_system_inc` from workspace config
2. If `use_system_inc`, call `config.get_system_inc().to_vec()` (requires re-locking for mutable access)
3. Pass paths to `CompletionProvider::new_with_index_and_source()`
4. Pass paths to `add_use_module_completions()` which scans directories for `.pm` files

### New Fields Added to `CompletionProvider`

```rust
// crates/perl-lsp-completion/src/completion.rs
pub struct CompletionProvider {
    // ... existing fields ...
    include_paths: Vec<PathBuf>,      // from .perl-lsp.toml includePaths
    system_inc_paths: Vec<PathBuf>,   // from system @INC when useSystemInc: true
}
```

### Extended `add_use_module_completions()` Signature

```rust
pub fn add_use_module_completions(
    completions: &mut Vec<CompletionItem>,
    context: &CompletionContext,
    workspace_index: &Option<Arc<WorkspaceIndex>>,
    include_paths: &[PathBuf],
    system_inc_paths: &[PathBuf],
    is_cancelled: &dyn Fn() -> bool,
)
```

### Performance Constraints

To prevent completion latency regression:
- **Timeout**: 30ms total budget for include path scanning (leaves headroom within 50ms total completion target)
- **Max depth**: 3 levels (Perl modules are rarely nested deeper)
- **Max entries**: 10,000 files total across all include directories
- **Symlinks**: `follow_links(false)` to prevent traversal loops
- **Cancellation**: Check `is_cancelled()` at each directory entry; return partial results if cancelled

### Detail Text Format

External modules are labeled in completion results:
- `"(include)"` — modules from `.perl-lsp.toml` `includePaths`
- `"(system)"` — modules from system @INC when `useSystemInc: true`

### WASM32 Handling

Filesystem scanning is gated behind `#[cfg(not(target_arch = "wasm32"))]` because:
- `get_system_inc()` already returns `Vec::new()` on wasm32
- Filesystem access APIs differ on wasm32
- On wasm32, completion degrades gracefully to workspace-only (same as today)

## Consequences

### Benefits
- **Consistency**: Module completion and goto-definition both respect @INC paths, ending the UX mismatch
- **No architectural debt**: Additive changes only; WorkspaceIndex remains focused on workspace files
- **Reusable pattern**: The "config → provider → search function" pattern mirrors resolution, making it the standard approach
- **Performance safeguards**: Timeout, depth limits, and cancellation are explicitly defined

### Tradeoffs / Risks
- **Filesystem I/O**: Scanning include directories on every completion request (no caching in v1). Acceptable because guards prevent abuse.
- **`use lib` not included**: Dynamic `use lib 'path'` extraction from document text is out of scope for this PR. Follow-up issue to be filed.
- **WASM32 degradation**: Users on wasm32 targets get workspace-only completion (same as today).

### What This Makes Easier
- Future `use lib` support (same pattern, just extract from document text)
- Shared `scan_modules_in_paths()` utility for diagnostics
- Performance infrastructure (timeout/cancellation for filesystem scans)

### What This Makes Harder
- Nothing significant. The change is additive and follows existing patterns.

## Alternatives Considered

### Option A: Expand WorkspaceIndex to Track External Modules
- **What**: Add `@INC` modules to the same index used for workspace files
- **Why rejected**: Violates ADR-0009 dual indexing strategy; WorkspaceIndex is architecturally scoped to workspace files; external modules would bloat the index and mix concerns

### Option C: Create a Separate External Module Index
- **What**: Build a second index specifically for @INC modules
- **Why rejected**: Unnecessary complexity — the include paths are already available in config, and a separate index would need the same threading logic without providing additional benefit

### Option B-Plus: Add Caching Layer
- **What**: Cache scanned module lists with TTL
- **Why deferred**: No caching infrastructure exists yet; issue description mentions it as future work; v0.12.x focuses on correctness before performance hardening

## Open Questions Resolved

| Question | Resolution |
|----------|------------|
| Timeout budget | 30ms hard cap (conservative, leaves headroom) |
| Detail text format | `"(include)"` and `"(system)"` (matches Perl convention) |
| Max depth | 3 levels |
| `use lib` support | Follow-up issue (filed after PR merges) |
| WASM32 | Graceful degradation (workspace-only) |
