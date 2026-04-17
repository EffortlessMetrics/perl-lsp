# Specification: Include-Path Scanning for Module Completion

## Feature Description

When the user types `use` or `require` statements, the completion provider will now suggest module names from:
1. **Workspace index** (existing behavior)
2. **Configured `includePaths`** from `.perl-lsp.toml` (NEW)
3. **`PERL5LIB` environment variable** paths (NEW)
4. **System `@INC`** when `useSystemInc: true` is configured (NEW)

The completion items are sorted with external modules appearing after workspace modules but before generic symbols.

## Acceptance Criteria

### AC1: Modules from configured includePaths appear in completion
**Given** a `.perl-lsp.toml` with `includePaths = ["/path/to/lib"]`
**And** `/path/to/lib/DBI.pm` exists
**When** the user types `use DB`
**Then** `DBI` appears in the completion list

### AC2: Modules from PERL5LIB appear in completion
**Given** `PERL5LIB=/path/to/ext/lib` is set in the environment
**And** `/path/to/ext/lib/Moo.pm` exists
**When** the user types `use Mo`
**Then** `Moo` appears in the completion list

### AC3: Modules from system @INC appear when useSystemInc is true
**Given** `useSystemInc = true` is configured
**And** the system has `strict.pm` in `@INC`
**When** the user types `use st`
**Then** `strict` appears in the completion list

### AC4: External modules sort after workspace but before generic symbols
**Given** `My::Module` exists in workspace AND `/path/to/lib/My/Module.pm` exists
**When** the user types `use My`
**Then** `My::Module` from workspace appears before `My::Module` from include paths
**And** both appear before generic symbols (if any)

### AC5: Completion remains responsive with caching
**Given** include paths have been scanned once
**When** the user triggers completion
**Then** the response time is < 100ms for cached results
**And** the response time is < 500ms for initial scan (if needed)

### AC6: Cache is invalidated on config change
**Given** the completion cache has been populated
**When** the user modifies `includePaths` in `.perl-lsp.toml`
**Then** subsequent completions use the new paths (not stale cache)

### AC7: Prefix filtering works for external modules
**Given** `/path/to/lib/DBD/MySQL.pm` exists
**When** the user types `use DBD::My`
**Then** `DBD::MySQL` appears in the completion list

## Non-Goals (Out of Scope)

1. **Method completion** from external modules (e.g., completing `->dbh` after `use DBI`)
2. **Import symbol completion** (`use Module qw(...)`) from external modules
3. **Auto-loading** or lazy-loading of module metadata
4. **Goto-definition** changes (already works correctly)
5. **Diagnostics** changes (already uses include paths correctly)
6. **Lexical `use lib` paths** from source files (deferred to future work)

## Implementation Details

### File Changes

| File | Change |
|------|--------|
| `crates/perl-lsp/src/runtime/language/completion.rs` | Wire include paths from config to provider |
| `crates/perl-lsp-completion/src/completion.rs` | Add `include_paths`/`system_inc_paths` fields and new constructor |
| `crates/perl-lsp-completion/src/completion/workspace.rs` | Add `scan_directory_for_modules()` and `path_to_module_name()` |

### Path Retrieval (Step 4 Fix)

The initial plan used `include_paths_for_doc()` which does NOT merge PERL5LIB. The correct approach:

```rust
let perl5lib_paths = std::env::var("PERL5LIB")
    .map(|v| perl_lsp_config::WorkspaceConfig::parse_perl5lib(&v))
    .unwrap_or_default();
let config = self.config_for_doc(uri).unwrap_or_else(...);
let include_paths = config.effective_include_paths(&perl5lib_paths);
let system_inc_paths = if config.use_system_inc {
    config.get_system_inc().to_vec()
} else {
    Vec::new()
};
```

### Caching Strategy

- **Cache location**: On `CompletionProvider` instance (created fresh per completion request)
- **Cache key**: Include path directory (each path has its own cache entry)
- **Cache value**: Sorted list of module names (not full `CompletionItem`s to minimize memory)
- **Invalidation**: Config change creates new provider instance (provider is not shared)
- **TTL**: None needed — cache is per-provider-instance, invalidated by virtue of new instances

### Directory Scanning Interface

```rust
fn scan_directory_for_modules(
    dir: &Path,
    prefix: &str,
    seen: &mut HashSet<String>,
    completions: &mut Vec<CompletionItem>,
    is_cancelled: &dyn Fn() -> bool,
)
```

Behavior:
- Max recursion depth: 8 levels
- File extensions: only `.pm`
- Symlink handling: follow symlinks with cycle detection
- Error handling: skip unreadable directories silently
- Result limit: 20 per include path

### Path to Module Name Conversion

Converts filesystem paths to Perl module names:
- `lib/Foo/Bar.pm` → `Foo::Bar` (strip `lib/` prefix if present)
- `lib/perl5/vendor_perl/Foo/Bar.pm` → `Foo::Bar` (strip `perl5/vendor_perl` segment)
- `lib/DBI.pm` → `DBI` (module at root of include path)

Algorithm:
1. If path starts with `lib/`, strip that prefix
2. If path contains `perl5/vendor_perl/` or `perl5/site_perl/`, strip up to and including that segment
3. Replace `/` with `::`
4. Remove `.pm` extension
5. Validate result matches `/^[A-Z][a-zA-Z0-9_:]*(?:\::)?$/` (Perl module name pattern)

### Sort Tiering

| Tier | Sort Text Prefix | Source |
|------|------------------|--------|
| 0 | `"0_"` | Hardcoded common modules (strict, warnings, DBI, etc.) |
| 1 | `"1_"` | Workspace modules |
| 2 | `"2_"` | External modules from include paths |
| 9 | `"9_"` | Generic symbols |

## Dependencies

### Crate Dependencies
- `perl-lsp-completion` receives include paths as parameters (does not add new dependencies)
- `perl-lsp` passes include paths to completion provider

### Configuration Dependencies
- `includePaths` in `.perl-lsp.toml`
- `PERL5LIB` environment variable
- `perl5lib_precedence` (prepend/append)
- `useSystemInc`
- `resolution_timeout_ms` (for scan timeout)

## Edge Cases

1. **Empty @INC paths**: No-op, nothing to scan
2. **Non-existent include paths**: Skip silently, log at debug level
3. **Permission denied**: Skip silently, log at debug level
4. **Duplicate modules**: `seen: HashSet<String>` deduplicates within a completion request
5. **Case-insensitive filesystems**: Module names are case-sensitive; filesystem case doesn't matter
6. **Very deep hierarchies**: Max 8 levels prevents runaway recursion
7. **Cancellation**: `is_cancelled()` checked during directory traversal
