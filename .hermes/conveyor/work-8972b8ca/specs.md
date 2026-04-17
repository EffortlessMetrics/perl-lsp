# Specs: Extend native_build_hints to Parse LIBS, DEFINE, OBJECT, and MYEXTLIB

**Work Item:** work-8972b8ca

---

## Feature Description

Extend the `NativeBuildHints` struct in `perl-lsp-config` to parse four additional build parameters from `Makefile.PL` only (not `Build.PL`):

1. **LIBS** — Library flags for native linking (e.g., `LIBS => '-L/path -lssl -lcrypto'`)
2. **DEFINE** — Preprocessor macros (e.g., `DEFINE => '-DFOO=1'`)
3. **OBJECT** — Object file list for XS linking
4. **MYEXTLIB** — External library list for XS modules

Also integrate `refresh_native_build_hints()` into workspace initialization in `handle_client_response()` using safe path handling (no `.unwrap()`).

---

## Behavior

### Extraction Rules

1. **Literal-only extraction**: Only extract literal values. Do not evaluate Perl expressions, variables, or `qw()` constructs.
2. **Quote handling**: Support both single-quoted and double-quoted string values.
3. **Array support**: Support Perl array syntax `['val1', 'val2']`.
4. **Comment skipping**: Ignore values inside comments (`#` to end-of-line).
5. **Dynamic skipping**: Ignore values that are not in quotes or arrays (e.g., `$libs`, `$(FLAGS)`).

### Makefile.PL Only

LIBS, DEFINE, OBJECT, and MYEXTLIB are ExtUtils::MakeMaker concepts. Build.PL (Module::Build) does not use these parameters. Extraction is only added to the Makefile.PL parsing path.

### LIBS Handling

LIBS values are typically single quoted strings containing multiple space-separated linker flags. The entire string is preserved as one entry (e.g., `'-L/lib -lfoo'`). The raw string is stored; the consumer is responsible for parsing individual flags if needed.

### Integration Point

In `crates/perl-lsp/src/runtime/workspace.rs` `handle_client_response()`, after building `effective_config` but before assigning to `folder.effective_workspace_config`, call `refresh_native_build_hints()` with safe path handling:

```rust
if let Some(path) = &folder.path {
    effective_config.refresh_native_build_hints(path);
}
```

Do NOT use `.unwrap()` — `folder.path` is `Option<PathBuf>` and can be `None`.

---

## Acceptance Criteria

### AC1: NativeBuildHints Struct Extended

The `NativeBuildHints` struct has four new fields:
- `libs_flags: Vec<String>`
- `define_flags: Vec<String>`
- `object_files: Vec<String>`
- `myextlib_files: Vec<String>`

### AC2: LIBS Extraction

Given a `Makefile.PL` containing `LIBS => '-L/usr/local/lib -lfoo -lbar'`, `detect_native_build_hints()` returns a `NativeBuildHints` where `libs_flags` contains `'-L/usr/local/lib -lfoo -lbar'` (the raw string value, not split).

### AC3: DEFINE Extraction

Given a `Makefile.PL` containing `DEFINE => '-DFOO=1 -DBAR=2'`, `detect_native_build_hints()` returns a `NativeBuildHints` where `define_flags` contains `'-DFOO=1 -DBAR=2'`.

### AC4: OBJECT Extraction

Given a `Makefile.PL` containing `OBJECT => 'foo.o bar.o'`, `detect_native_build_hints()` returns a `NativeBuildHints` where `object_files` contains `['foo.o', 'bar.o']` (space-separated values are split into individual entries).

### AC5: MYEXTLIB Extraction

Given a `Makefile.PL` containing `MYEXTLIB => 'someext.a anotherext.a'`, `detect_native_build_hints()` returns a `NativeBuildHints` where `myextlib_files` contains `['someext.a', 'anotherext.a']`.

### AC6: Comment and Dynamic Value Skipping

Given a `Makefile.PL` containing `# LIBS => '-lfoo'` or `LIBS => $dynamic`, the extraction correctly skips commented lines and dynamic values.

### AC7: Safe Integration Without Panic

When `folder.path` is `None`, the integration in `handle_client_response()` does not panic. The `refresh_native_build_hints()` call is skipped gracefully.

### AC8: Build.PL Not Modified for These Parameters

Verify that `detect_native_build_hints()` does not extract LIBS, DEFINE, OBJECT, or MYEXTLIB from Build.PL files, because these parameters are not used by Module::Build.

---

## Non-Goals

- **No code execution**: The module does not execute Makefile.PL, Build.PL, or any Perl code.
- **No dynamic evaluation**: Variable references (`$foo`), function calls, and `qw()` constructs are not evaluated.
- **No Build.PL extraction for these parameters**: Only Makefile.PL is parsed for LIBS/DEFINE/OBJECT/MYEXTLIB.
- **No runtime refresh**: Hints are detected once at workspace initialization, not continuously.
- **No consumer implementation**: The fields are added but not consumed by any downstream code path.

---

## Dependencies

- `perl-lsp-config` crate (this is the module being extended)
- No new external dependencies needed for the extraction logic
- Tests use the `tempfile` crate (already a dev dependency in `perl-lsp-config`)

---

## Files to Modify

| File | Change |
|------|--------|
| `crates/perl-lsp-config/src/native_build_hints.rs` | Add 4 new fields, add extraction functions for LIBS/DEFINE/OBJECT/MYEXTLIB from Makefile.PL |
| `crates/perl-lsp-config/tests/native_build_hints.rs` | Add tests for all 4 new extractions and the safe integration |
| `crates/perl-lsp/src/runtime/workspace.rs` | Call `refresh_native_build_hints()` with safe `if let Some(path)` guard |

---

## Test Cases

| Test | Input | Expected |
|------|-------|----------|
| `makefile_libs_single_string` | `LIBS => '-L/lib -lfoo'` | `libs_flags = ['-L/lib -lfoo']` |
| `makefile_define_single_string` | `DEFINE => '-DFOO=1'` | `define_flags = ['-DFOO=1']` |
| `makefile_object_split` | `OBJECT => 'foo.o bar.o'` | `object_files = ['foo.o', 'bar.o']` |
| `makefile_myextlib_split` | `MYEXTLIB => 'ext.a'` | `myextlib_files = ['ext.a']` |
| `makefile_all_combined` | All four parameters | All fields populated |
| `makefile_commented_skipped` | `# LIBS => '-lfoo'` | `libs_flags` empty |
| `makefile_dynamic_skipped` | `LIBS => $dynamic` | `libs_flags` empty |
| `build_pl_not_extracted` | Build.PL with LIBS | `libs_flags` empty |
| `safe_integration_no_panic` | `folder.path = None` | No panic, hints not refreshed |
