# Specifications: @INC Paths in Module Completion

## Feature Description

When a user types `use DB` or `require Moo::` in a Perl file, the LSP provides module name completions from:
1. **Workspace index**: Modules defined in the workspace (existing behavior)
2. **Configured include paths**: Modules in directories listed in `.perl-lsp.toml` `includePaths`
3. **System @INC**: Modules in system Perl library paths when `useSystemInc: true`

This applies only to module name completion for `use` and `require` statements — not symbol completion inside modules.

## User-Facing Behavior

| Scenario | Before | After |
|----------|--------|-------|
| Type `use DBI<|>` in workspace without DBI | No completion | Shows `DBI` from include path with `"(include)"` detail |
| Type `use DB<|>` with `useSystemInc: true` | No completion | Shows `DBI` from system @INC with `"(system)"` detail |
| Type `use My::Module<|>` in workspace | Shows workspace modules | Unchanged (workspace still searched first) |
| Type `use DB<|>` — module in both workspace and include path | Shows once (workspace) | Shows once (workspace deduplication takes priority) |
| Type `use DB<|>` on wasm32 target | No completion | No completion (graceful degradation) |

## Acceptance Criteria

### AC1: Configured Include Paths Work
**Given** a `.perl-lsp.toml` with `includePaths: ["/path/to/libs"]`
**And** `/path/to/libs/DBI.pm` exists (containing `package DBI;`)
**When** the user types `use DB<|>`
**Then** the completion list includes `DBI` with `detail: "module (include)"`

### AC2: System @INC Works with useSystemInc
**Given** `useSystemInc: true` in `.perl-lsp.toml`
**And** the system Perl installation includes `Moo.pm`
**When** the user types `use Mo<|>`
**Then** the completion list includes `Moo` with `detail: "module (system)"`

### AC3: Deduplication Between Sources
**Given** a module `Foo.pm` exists in both the workspace and an include path
**When** the user types `use Fo<|>`
**Then** `Foo` appears only once in the completion list (workspace version takes priority)
**And** no duplicate entries are shown

### AC4: Prefix Filtering Works
**Given** an include path with `DBI.pm`, `DBD::SQLite.pm`, and `Moo.pm`
**When** the user types `use DB<|>`
**Then** the completion list shows `DBI` and `DBD::SQLite` but NOT `Moo`
**And** results are filtered to match the prefix `DB`

### AC5: Nested Module Paths Work
**Given** an include path with `File/Path/To/Module.pm`
**When** the user types `use File::Path::To::Mo<|>`
**Then** the completion list includes `File::Path::To::Module`
**And** directory separators `/` are converted to `::` in module names

### AC6: Empty Include Paths Handled Gracefully
**Given** `includePaths` is empty and `useSystemInc` is false
**When** the user types `use DB<|>`
**Then** completion behaves identically to before this change (workspace-only)

### AC7: Cancellation Stops Scanning
**Given** include paths containing thousands of `.pm` files
**And** the user presses Ctrl+C to cancel a slow completion request
**When** the cancellation is detected mid-scan
**Then** partial results collected so far are returned (not empty list)
**And** the scan stops immediately

### AC8: Performance Budget Maintained
**Given** an include path with up to 10,000 `.pm` files
**When** the user triggers completion
**Then** the include-path scanning portion completes within 30ms
**And** total completion response time remains under 50ms

### AC9: Permission Errors Don't Crash
**Given** an include path containing directories with no read permission
**When** the scanner encounters those directories
**Then** it skips them gracefully with a trace message
**And** completion continues with accessible directories

### AC10: WASM32 Graceful Degradation
**Given** the LSP running on a wasm32 target
**When** completion is requested
**Then** the behavior is identical to before this change (no filesystem scanning attempted)

## Non-Goals (Out of Scope)

1. **Symbol completion inside modules**: This spec covers only `use Module` / `require Module` name completion. Completion of package symbols (methods, variables) is out of scope.
2. **`use lib` dynamic extraction**: Extracting `use lib 'path'` statements from the current document text is not included. This will be a follow-up issue.
3. **Auto-import logic**: Resolving what modules export (like `use strict`) is not included.
4. **Goto-definition**: Already works correctly and is not modified.
5. **Caching infrastructure**: Module list caching with TTL is deferred to future work.
6. **Module dependency resolution**: Finding what modules a given module requires is not included.

## Dependencies

| Dependency | Purpose |
|------------|---------|
| `perl-lsp-config` crate | `WorkspaceConfig.include_paths`, `WorkspaceConfig.use_system_inc`, `WorkspaceConfig::get_system_inc()` |
| `perl-workspace-index` crate | `WorkspaceIndex::find_symbols()`, `WorkspaceIndex::all_symbols()` |
| `walkdir` crate | Directory traversal with depth limits |
| `LSP CancellationToken` | `is_cancelled` callback for interrupting slow scans |
| `prepend_use_lib_paths()` pattern | Future: will need same pattern for `use lib` extraction |

## Files to Modify

| File | Change |
|------|--------|
| `crates/perl-lsp-completion/src/completion.rs` | Add `include_paths` and `system_inc_paths` fields to `CompletionProvider`; update factory methods |
| `crates/perl-lsp-completion/src/completion/workspace.rs` | Extend `add_use_module_completions()` signature; implement `scan_directory_for_modules()` with timeout, depth limits, cancellation |
| `crates/perl-lsp/src/runtime/language/completion.rs` | Wire config through to `CompletionProvider::new_with_index_and_source()` |
| `crates/perl-lsp-completion/tests/completion_behavior_tests.rs` | Add tests for AC1-AC10 |

## Detail Text Format

| Source | Detail Text |
|--------|-------------|
| `.perl-lsp.toml` `includePaths` | `"module (include)"` |
| System @INC (`useSystemInc: true`) | `"module (system)"` |

## Performance Constraints

| Constraint | Value |
|------------|-------|
| Timeout budget | 30ms total for include path scanning |
| Max directory depth | 3 levels |
| Max files scanned | 10,000 total |
| Symlink handling | `follow_links(false)` |
| wasm32 | No filesystem scanning (graceful degradation) |

## Cancellation Behavior

- Check `is_cancelled()` at every directory entry during scanning
- On cancellation: return results collected so far (never return empty if any results found)
- On timeout: same as cancellation (return partial results)

## Test Coverage Required

At minimum, tests must cover:
1. Include path completion (AC1)
2. System @INC completion (AC2)
3. Deduplication (AC3)
4. Prefix filtering (AC4)
5. Nested module paths (AC5)
6. Empty paths (AC6)
7. Cancellation (AC7) — may use mock `is_cancelled` callback
8. Permission errors (AC9)
9. Performance boundary (AC8) — timing assertion within budget
