# ADR-0036: Include Path Scanning for Module Completion

## Status
Accepted

## Context

Issue #4314 reported that module completion for `use` and `require` statements only suggested modules from the workspace index, ignoring configured include paths (`.perl-lsp.toml` `includePaths`), `PERL5LIB`, and system `@INC`.

The `perl-lsp-completion` crate provides module name completion for `use` and `require` statements. Previously, it only consulted the workspace index (which contains symbols from files in the workspace). External modules from configured include paths were not being suggested.

This created an inconsistency: users with properly configured include paths would not see external dependencies like `DBI`, `JSON::XS`, or `Moo` in completion results.

### Why WalkDir Instead of perl-module-resolution

The codebase has a microcrate architecture for module resolution documented in ADR-0035:
- `perl-module-resolution` — facade crate for module resolution
- `perl-module-resolution-path` — filesystem path lookup
- `perl-module-resolution-uri` — URI-first search with timeout budgeting

However, `perl-module-resolution` is designed for "resolve a known module name to a path" (e.g., goto-definition), not "enumerate all modules under a directory." The completion feature requires directory enumeration to find all `.pm` files matching a prefix, which is a different operation than resolving a specific module path.

Additionally, `perl-module-resolution` has different timeout budgeting semantics (URI-based with `MAX_RESOLUTION_TIME_MS`) that are not suitable for bulk enumeration.

### Decision: Add Direct WalkDir Scanning

Rather than extending `perl-module-resolution` to support enumeration, we added direct WalkDir-based filesystem scanning in `perl-lsp-completion` with its own:
- `MAX_SCAN_DEPTH = 5` (should be 6 — see Consequences)
- `MAX_SCAN_ENTRIES = 10_000`
- `SCAN_TIMEOUT_MS = 30`

This approach was chosen for:
1. **Separation of concerns**: Module resolution (path lookup) is architecturally different from module enumeration (directory scanning)
2. **Independent evolution**: Completion scanning can evolve independently without coupling to resolution logic
3. **Faster implementation**: Avoided the need to modify perl-module-resolution's public API

## Decision

We extended `CompletionProvider` to accept `include_paths` and `system_inc_paths` vectors, and implemented `scan_modules_in_directory()` to recursively scan directories for `.pm` files.

### Implementation Details

1. **CompletionProvider struct** (`crates/perl-lsp-completion/src/completion.rs:168-180`):
   - Added `include_paths: Vec<PathBuf>` field
   - Added `system_inc_paths: Vec<PathBuf>` field

2. **New constructor** (`crates/perl-lsp-completion/src/completion.rs:327`):
   - `CompletionProvider::new_with_index_and_source_and_include_paths()` accepts include_paths and system_inc_paths

3. **add_use_module_completions()** (`crates/perl-lsp-completion/src/completion/workspace.rs:280-377`):
   - Accepts `include_paths`, `system_inc_paths`, and `is_cancelled` parameters
   - Scans include directories for `.pm` files after workspace index search
   - Respects cancellation callbacks and timeout budgets

4. **scan_modules_in_directory()** (`crates/perl-lsp-completion/src/completion/workspace.rs:390-461`):
   - Uses WalkDir with `MAX_SCAN_DEPTH`, `MAX_SCAN_ENTRIES`, `SCAN_TIMEOUT_MS`
   - Converts filesystem paths to module names via `path_to_module_name()`

5. **LSP handler wiring** (`crates/perl-lsp/src/runtime/language/completion.rs:576-595`):
   - Extracts `include_paths` and `system_inc_paths` from workspace config
   - Passes to `CompletionProvider::new_with_index_and_source_and_include_paths()`

### WASM32 Exclusion

Filesystem scanning is excluded on `wasm32` targets via `#[cfg(not(target_arch = "wasm32"))]` since WASM cannot perform arbitrary filesystem access.

## Consequences

### Benefits
- **Complete module suggestions**: Users now see external modules from include paths
- **Configurable**: `.perl-lsp.toml` `includePaths` and `useSystemInc` are respected
- **Responsive**: 30ms timeout prevents completion from blocking typing
- **Cancellable**: Users can cancel long-running scans

### Tradeoffs and Risks

1. **Off-by-one in MAX_SCAN_DEPTH**: The constant `MAX_SCAN_DEPTH = 5` with WalkDir's `max_depth(5)` causes files at the documented "depth 5" to not be found. WalkDir's `max_depth(N)` traverses depths 0 through N, so with `max_depth(5)`, a file at `A/B/C/D/E/Module.pm` (5 path segments beyond root, WalkDir depth 5) IS found, but `A/B/C/D/E/F/Module.pm` (WalkDir depth 6) is NOT found. The documentation claims depth 5 should be found, but with the test creating `A/B/C/D/E/Module.pm` (5 path segments), the actual WalkDir depth is 6. **FIX REQUIRED**: Change `MAX_SCAN_DEPTH` from `5` to `6`.

2. **Inconsistent depth documentation**: The documentation at `workspace.rs:30-34` uses 1-based counting (root = depth 1) but WalkDir uses 0-based (root = depth 0). This causes confusion.

3. **Microcrate drift risk**: A second WalkDir scanning implementation exists in `perl-lsp-completion` separate from any scanning in `perl-module-resolution`. Future changes to scanning policy may not propagate.

4. **No caching**: Directory scanning happens on every completion request. For large include directories, this may cause perceptible latency. Per ADR decision, caching is deferred to v0.13.0.

5. **Test bug**: `property_prefix_filtering_exact` has an incorrect assertion. It expects `Alphabet::Module` to NOT appear for prefix `Alpha`, but `Alphabet.starts_with("Alpha")` is `true`, so it correctly appears. The test uses a false negative case.

## Alternatives Considered

### 1. Extend perl-module-resolution with enumeration capability
- **What**: Add a `scan_modules_in_directory()` function to `perl-module-resolution`
- **Why rejected**: Different use case (enumeration vs resolution), different timeout semantics, would require API changes to a core dependency
- **Tradeoff**: Would reduce microcrate drift but introduces coupling and complexity

### 2. Use cached include path scanning
- **What**: Cache scanned modules between completion requests
- **Why rejected**: ADR decision to defer caching to v0.13.0; adding caching now would delay the fix
- **Tradeoff**: Simpler implementation now, but will need revisiting at scale

### 3. Only scan workspace index
- **What**: Do not add include path scanning; rely solely on workspace index
- **Why rejected**: Does not address the reported issue; users with external dependencies would not see them in completion
- **Tradeoff**: Simpler code but incomplete feature

## References

- Issue #4314: @INC paths not used in module completion
- ADR-0035: Microcrate decomposition for module resolution
- `MAX_SCAN_DEPTH` bug: empirical WalkDir testing confirms off-by-one
