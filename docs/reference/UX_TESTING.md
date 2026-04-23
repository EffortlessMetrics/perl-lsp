# UX Testing Guide

The `perl-lsp-ux-tests` crate provides a systematic regression prevention
harness for common first-5-minutes user experiences.  It spawns the real
`perl-lsp` binary over stdio and verifies that the server behaves correctly
in each scenario — not just "didn't crash" but "returned a useful response."

## Quick Start

```bash
# Run all default UX scenarios (currently 18 scenario files):
just ux-tests

# Run full suite including the integration-only 10k-line large-file scenario:
just ux-tests-full

# Run with verbose server stderr output:
UX_TEST_ECHO_STDERR=1 just ux-tests

# Run a single scenario:
cargo test -p perl-lsp-ux-tests --test ux_scenario_01_simple_file
```

`just ux-tests` and `just ux-tests-full` build `target/debug/perl-lsp` first and
export `PERL_LSP_BIN` automatically. Direct `cargo test` invocations still need
either a prebuilt binary or an explicit `PERL_LSP_BIN=/path/to/perl-lsp`.

## Current Scenarios

| # | Scenario | What it tests |
|---|----------|---------------|
| 01 | Simple file | Fresh open, hover on `$x`, no crash |
| 02 | Missing perltidy | Format request degrades gracefully |
| 03 | Missing perl | Server starts, hover/completion work in degraded mode |
| 04 | Missing perlcritic | Diagnostics degrade gracefully |
| 05 | Bad config | Invalid tool paths do not crash the server |
| 06 | Large file | 1k-line (base) / 10k-line (integration-test) file |
| 07 | Multi-file workspace | Multiple modules + cross-file definition |
| 08 | Shebang detection | Files without `.pl` extension but `languageId=perl` |
| 09 | BOM and encoding | UTF-8 BOM, `use utf8`, Unicode in comments |
| 10 | Go-to-definition | Request succeeds end-to-end and returns location-or-empty without crashing |
| 11 | Hover | Request succeeds end-to-end and returns useful structure-or-empty without crashing |
| 12 | Strict diagnostics | `publishDiagnostics` arrives and the payload shape stays valid |
| 13 | Document symbols | Parsable files return structured document symbols |
| 14 | `@INC` conformance | PL701, hover, and goto-definition stay consistent across 6 module-resolution modes |
| 15 | Workspace symbols | `workspace/symbol` finds same-named symbols across workspace folders and carries `workspaceFolderUri` |
| 16 | Folder removal | removing a workspace folder evicts its symbols from `workspace/symbol` results |
| 17 | Deleted file churn | a `didChangeWatchedFiles` Deleted event removes stale symbols and definition targets |
| 18 | Go-to-declaration | declaration request succeeds end-to-end and returns location-or-empty without crashing |

`just ux-tests` runs every default scenario above. `just ux-tests-full` adds the
feature-gated 10k-line large-file case from Scenario 06.

## Workflow Scorecard Contract

The UX harness is also the fixture source for the planned `editor_ux`
scorecard:

- `docs/project/metrics/WORKFLOW_SCORECARDS.md` describes the workflow layer and
  the top-line/current-component rows it owns.
- `.ci/schemas/editor-ux.schema.json` defines the measured scorecard shape.
- `crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json` maps each
  workflow fixture to the rows it can actually back today and the owning
  subsystem.
- `crates/perl-lsp-ux-tests/tests/editor_ux_fixture_matrix.rs` prevents the
  matrix from drifting away from the executable scenario files.

This keeps the workflow scorecard grounded in real harness coverage instead of a
manually curated checklist.

## How to Add a New Scenario

1. Create `crates/perl-lsp-ux-tests/tests/ux_scenario_NN_my_scenario.rs`.
2. Import the harness:
   ```rust
   use perl_lsp_ux_tests::{ScenarioConfig, UxHarness};
   ```
3. Build a `ScenarioConfig`:
   ```rust
   let config = ScenarioConfig::default()
       .with_file("test.pl", "my $x = 1;\n");
   ```
4. Create the harness:
   ```rust
   let harness = UxHarness::new(config).expect("harness setup");
   ```
5. Drive LSP interactions:
   ```rust
   harness.open_file("test.pl", source).expect("didOpen");
   let hover = harness.hover("test.pl", 0, 3).expect("no error");
   ```
6. Assert on outcomes:
   ```rust
   harness.assert_no_crash();
   harness.assert_message_contains("some expected text");
   ```
7. Add a skip guard at the top:
   ```rust
   if !perl_lsp_ux_tests::resolve_binary().is_ok() {
       eprintln!("SKIP: binary not found");
       return;
   }
   ```

## ScenarioConfig Reference

| Field | Type | Default | Purpose |
|-------|------|---------|---------|
| `timeout` | `Duration` | 10s | Per-request timeout |
| `path_restriction` | `Option<Vec<String>>` | `None` (full PATH) | Override child PATH |
| `echo_stderr` | `bool` | `false` | Print server stderr |
| `extra_env` | `Vec<(String, Option<String>)>` | `[]` | Set/unset env vars |
| `workspace_files` | `Vec<(String, String)>` | `[]` | Pre-seed workspace |

### Simulating a Missing Tool

```rust
// Completely empty PATH (no tools at all):
let config = ScenarioConfig { path_restriction: Some(vec![]), ..Default::default() };

// Keep system dirs but remove any entry containing "tidy":
use perl_lsp_ux_tests::env::RestrictedPath;
let restricted = RestrictedPath::current_excluding("perltidy");
let config = ScenarioConfig { path_restriction: Some(/*...*/), ..Default::default() };
```

## UxHarness Method Reference

| Method | Purpose |
|--------|---------|
| `open_file(path, content)` | `textDocument/didOpen` |
| `hover(path, line, char)` | `textDocument/hover` → `Option<Value>` |
| `completion(path, line, char)` | `textDocument/completion` → `Vec<Value>` |
| `format_document(path)` | `textDocument/formatting` → `FormatResult` |
| `definition(path, line, char)` | `textDocument/definition` → `Vec<Value>` |
| `collect_notifications()` | Drain buffered server events (queue becomes empty) |
| `peek_notifications()` | Clone buffered events without removing them |
| `assert_no_crash()` | Assert no crash signatures — uses peek, queue intact |
| `assert_message_contains(needle)` | Assert a message contains `needle` — uses peek |
| `assert_no_message_containing(needle)` | Assert no message contains `needle` — uses peek |
| `root_uri()` | The `file://` URI of the workspace root |

## LspEvent Variants

Events captured from the server's stdout:

```rust
LspEvent::WindowMessage { message_type, message }  // window/showMessage
LspEvent::LogMessage { message_type, message }     // window/logMessage
LspEvent::Diagnostics { uri, diagnostics }         // textDocument/publishDiagnostics
LspEvent::Other { method, params }                 // anything else
```

`message_type` follows LSP spec: 1=Error, 2=Warning, 3=Info, 4=Log.

## Debugging a Failing Scenario

1. Enable server stderr output:
   ```bash
   UX_TEST_ECHO_STDERR=1 cargo test -p perl-lsp-ux-tests --test ux_scenario_02_missing_perltidy -- --nocapture
   ```
2. Check what binary is being resolved:
   ```bash
   PERL_LSP_BIN=/path/to/custom/perl-lsp cargo test -p perl-lsp-ux-tests
   ```
3. Increase the timeout for slow environments:
   ```bash
   UX_TEST_TIMEOUT_MS=30000 cargo test -p perl-lsp-ux-tests
   ```
4. Run a single test with full output:
   ```bash
   cargo test -p perl-lsp-ux-tests --test ux_scenario_07_multi_file_workspace -- scenario_07_definition_request_does_not_crash --nocapture
   ```

## What the Tests Do NOT Cover

- VSCode extension UI (that requires a VSCode test runner — separate scope).
- Protocol-level compliance (covered by `lsp_3_17_*` tests in `perl-lsp`).
- Parser correctness (covered by `perl-parser` tests).
- Performance benchmarks (covered by `criterion` benchmarks).
- Syntax highlighting (tree-sitter grammar, separate scope).

## Coverage Strategy

These tests verify the **user-observable experience**, not internal implementation.
When a scout files a UX blocker issue, the fix workflow is:
1. Builder fixes the root cause.
2. Add a scenario (or assertion) to the relevant `ux_scenario_NN_*.rs` that
   would have caught the regression.
3. The scenario then serves as a permanent regression gate.

## Environment Variables

| Variable | Purpose |
|----------|---------|
| `PERL_LSP_BIN` | Override path to `perl-lsp` binary |
| `UX_TEST_TIMEOUT_MS` | Per-request timeout in milliseconds (default: 10000) |
| `UX_TEST_ECHO_STDERR` | If set, echo perl-lsp stderr to test output |
