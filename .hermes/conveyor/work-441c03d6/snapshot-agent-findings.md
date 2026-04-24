# Snapshot Test Findings — work-441c03d6

## What This Change Does
The feature wires @INC search paths into `PullDiagnosticsProvider::get_document_diagnostics` via an optional `include_paths` parameter. When provided, PL701 (ModuleNotFound) diagnostics include the searched paths in their message (e.g., "Module 'X' not found. Searched @INC: /path1, /path2"). When not provided, a fallback message is shown ("Module 'X' not found in workspace or configured include paths").

## Snapshots Written

14 snapshot tests were written in `crates/perl-lsp-rs/tests/pull_diagnostics_snapshots.rs`:

### PL701 @INC Path Inclusion Snapshots (5 tests)
- **pl701_with_include_paths**: PL701 diagnostic when include_paths = Some(["/test/path1", "/test/path2"])
  - Input: `use Missing::Module::That::Does::Not::Exist;` with include_paths
  - Output: Message includes "Searched @INC: /test/path1, /test/path2"
  - Normalizes: result_id is not applicable (uses message content)

- **pl701_without_include_paths**: PL701 diagnostic when include_paths = None
  - Input: `use Missing::Module::That::Does::Not::Exist;` with include_paths = None
  - Output: Message shows fallback "not found in workspace or configured include paths"
  - Normalizes: result_id not applicable

- **pl701_with_empty_include_paths**: PL701 diagnostic when include_paths = Some(vec![])
  - Input: `use Missing::Module::That::Does::Not::Exist;` with include_paths = Some(vec![])
  - Output: Same fallback message as None (proves empty vec ≠ provided paths)
  - Normalizes: result_id not applicable

- **pl701_single_include_path**: Single path handling
  - Input: `use Missing::Module;` with include_paths = Some(["/custom/lib"])
  - Output: Message includes "/custom/lib"

- **pl701_include_path_with_spaces**: Path with spaces handling
  - Input: `use Missing::Module;` with include_paths = Some(["/path/with spaces", "/another/path"])
  - Output: Paths with spaces are correctly included in message

### Report Structure Snapshots (2 tests)
- **unchanged_report**: Unchanged document diagnostic report structure
  - Input: Same content requested twice with previous_result_id
  - Output: `kind: Unchanged, result_id: <md5hash>`
  - Normalizes: None (result_id is deterministic based on content)

- **full_report**: Full document diagnostic report for parse error
  - Input: Content with parse error: `my $x = ;`
  - Output: Full report with result_id and items array
  - Normalizes: result_id is deterministic based on content

- **full_report_multiple_diagnostics**: Multiple diagnostics in single report
  - Input: Content producing PL100, PL102, PL701
  - Output: All diagnostics sorted by code for stable ordering

### Parse Error Diagnostic Snapshots (3 tests)
- **parse_error_missing_semicolon**: Parse error diagnostic for missing semicolon
  - Input: `my $x = 1\nmy $y = 2;\n`
  - Output: PL001/PL002/PL003 diagnostic with suggestion
  - Normalizes: result_id not applicable

- **parse_error_unclosed_block**: Parse error for unclosed block
  - Input: `sub foo {\n    my $x = 1;\n`
  - Output: PL002 diagnostic with suggestion

- **parse_error_unclosed_string**: Parse error for unclosed string
  - Input: `my $x = "hello world;\n`
  - Output: PL001/PL002/PL003 diagnostic with suggestion

### Diagnostic Data JSON Snapshots (3 tests)
- **diagnostic_data_parse_error**: DiagnosticData JSON for PL001
  - Input: Parse error content
  - Output: JSON with code, category, fixable, tags

- **diagnostic_data_missing_strict**: DiagnosticData JSON for PL100
  - Input: Bare `print 'hello';`
  - Output: JSON with code, category, fixable, tags

- **diagnostic_data_pl701**: DiagnosticData JSON for PL701
  - Input: `use Missing::Module;`
  - Output: JSON with code, category, fixable, tags

## Edge Cases Covered
- **Single include path**: One path is correctly listed
- **Multiple include paths**: Multiple paths are comma-separated
- **Path with spaces**: Paths containing spaces are handled correctly
- **Empty vec**: `Some(vec![])` behaves same as `None` (fallback message)
- **None**: No include_paths shows fallback message
- **Deeply nested module**: `Missing::Module::That::Does::Not::Exist` in message

## Non-Deterministic Output Handling
- **result_id**: Uses MD5 hash of content, which is deterministic for the same input
- **message content**: Not normalized (the actual message content IS what we're snapshotting)
- **range positions**: Not normalized (deterministic for given input)
- **data JSON**: Deterministic JSON structure

## Summary
- Snapshot tests written: 14
- All passing: yes
- Coverage assessment: The @INC path inclusion feature (PL701) is fully covered. Report structures, parse error diagnostics, and diagnostic data JSON are all snapshot tested. The message format difference between "with include_paths" and "without include_paths" is clearly captured in the snapshots.

## Key Finding
The snapshot clearly shows the difference between with and without include_paths:
- **With**: `Module 'Missing::Module::That::Does::Not::Exist' not found. Searched @INC: /test/path1, /test/path2.`
- **Without**: `Module 'Missing::Module::That::Does::Not::Exist' not found in workspace or configured include paths`