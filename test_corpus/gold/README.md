# Gold Corpus — Diagnostics Validation Fixtures

The gold corpus is a curated set of Perl code fixtures with hand-verified expected behavior. Each fixture can carry diagnostics and editor-intelligence expectations used by headless LSP UX scorecard tests.

## Directory Structure

```
test_corpus/gold/
├── hello_world/
│   ├── fixture.pl         # Perl source code to test
│   └── expected.json      # Expected diagnostic assertions
├── missing_strict/
│   ├── fixture.pl
│   └── expected.json
├── v5_40_suppresses_strict/
│   ├── fixture.pl
│   └── expected.json
└── [10+ more fixtures]
```

Each fixture is a subdirectory containing:

- **`fixture.pl`** — A Perl source file representing a single semantic pattern
- **`expected.json`** — Optional JSON file specifying expected diagnostic assertions
- **`expected_hover.json` / `expected_goto.json` / `expected_completion.json` / `expected_symbols.json`** — Optional editor-intelligence expectations

## JSON Format Specification

The `expected.json` file follows this schema:

```json
{
  "diagnostics": [
    {
      "assertion": "no_diagnostics"
    }
  ]
}
```

### Assertion Types

| Assertion | Fields | Meaning |
|-----------|--------|---------|
| `no_diagnostics` | — | No diagnostics should be emitted for this fixture |
| `no_diagnostic` | `code: String` | The specified diagnostic code should NOT be present |
| `diagnostic_present` | `code: String`, `byte_offset?: usize`, `message_contains?: String` | A diagnostic with the given code should be present; optionally at a specific byte offset or containing a message substring |
| `diagnostic_count` | `code: String`, `count: usize` | Exactly N diagnostics with the given code should be emitted |

### Complete Example

```json
{
  "diagnostics": [
    {
      "assertion": "diagnostic_present",
      "code": "PL100",
      "byte_offset": 24,
      "message_contains": "strict"
    }
  ]
}
```

## Bootstrap Fixtures (Initial 10)

### 1. hello_world

**Purpose**: Minimal sanity check with no expected diagnostics.

### 2. missing_strict

**Purpose**: Validates detection of missing `use strict`.

### 3. v5_40_suppresses_strict

**Purpose**: Validates that `use v5.40` implicitly enables `strict` and `warnings`.

### 4. open_lexical_filehandle

**Purpose**: Idiomatic lexical filehandle with three-argument open.

### 5. push_arrayref

**Purpose**: Dereferencing an arrayref with the `@$ref` syntax in `push`.

### 6. local_special_var

**Purpose**: Localizing special variables like `$/` for paragraph mode reading.

### 7. map_with_default_var

**Purpose**: Using the implicit `$_` topic variable in `map` and `grep`.

### 8. eval_string_pragma

**Purpose**: Dynamic pragma loading via `eval STRING`.

### 9. use_if_strict

**Purpose**: Conditional pragma loading via `use if $] >= 5.020, 'strict'` — validates that the `use if` idiom for runtime-conditional strict enablement is recognized and does not trigger PL100.

### 10. parse_error_recovery

**Purpose**: Parse error recovery with incomplete assignment (missing RHS).

## Running the Test Suite

To run the gold corpus diagnostics test suite:

```bash
cargo test -p perl-lsp-diagnostics --test diagnostics_gold_suite -- --nocapture
```

Or use the justfile target:

```bash
just metrics-diagnostics
```

Output includes:
- Per-fixture pass/fail status
- Diagnostic codes found vs. expected
- Summary: passed, failed, total
- Precision metrics

## Extending the Corpus

To add a new fixture:

1. Create a subdirectory under `test_corpus/gold/` with a descriptive name
2. Write your Perl code to `fixture.pl`
3. Write expected assertions to `expected.json` using the schema above
4. Run the test suite to validate

## Integration with Other Scorecards

The gold corpus directory structure supports sibling assertions for scorecards:

- `expected.json` — Diagnostics expectations (current)
- `expected_hover.json` — Hover information expectations
- `expected_goto.json` — Goto-definition expectations
- `expected_completion.json` — Completion suggestions
- `expected_symbols.json` — Document symbol expectations
- `expected_module.json` — Module resolution expectations

Each fixture can carry multiple assertion files without conflicts.

## Module Resolution Fixtures (5)

These fixtures exercise the consumer-consistency harness
(`ux_scenario_14_inc_conformance`). Each fixture declares which module it
exercises, what `@INC` resolution mode is in play, and checks that the three
consumers (PL701 diagnostic, goto-definition, hover) agree on the outcome.

### Sidecar Format

Each module-resolution fixture carries an `expected_module.json` sidecar:

```json
{
  "module": "Greet",
  "resolution_mode": "workspace_config",
  "consumers": {
    "PL701_diagnostic": "no_error",
    "goto_definition": "resolves",
    "hover": "resolves"
  }
}
```

### Module Resolution Fixtures

| Fixture | Resolution Mode |
|---------|----------------|
| `inc_relative_include_path` | Workspace config `includePaths: ["lib"]` |
| `inc_use_lib_lexical` | In-source `use lib 'lib'` pragma |
| `inc_no_lib_cancellation` | `use lib` then `no lib` — must NOT resolve |
| `inc_findbin_relative` | `use FindBin; use lib "$FindBin::Bin/lib"` |
| `inc_system_inc` | System `@INC` via injected tempdir |

### inc_relative_include_path

**Purpose**: Validates that workspace-level `includePaths` configuration resolves a relative `lib/` path.

### inc_use_lib_lexical

**Purpose**: Validates that a lexical `use lib 'lib'` pragma in the source enables module resolution.

### inc_no_lib_cancellation

**Purpose**: Validates that `use lib 'lib'` followed by `no lib 'lib'` cancels resolution — the module must NOT resolve.

### inc_findbin_relative

**Purpose**: Validates that `use FindBin; use lib "$FindBin::Bin/lib"` resolves a module relative to the script location.

### inc_system_inc

**Purpose**: Validates that a module present in system `@INC` (via injected tempdir) resolves correctly.

### Running the Consumer Consistency Harness

```bash
cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance -- --nocapture
```

### Running the Editor-Intelligence Gold Harness

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs --test editor_intelligence_scorecard -- --nocapture
```

## Related Issues

- **#4065** — Bootstrap gold corpus seed fixtures and diagnostics scorecard
- **#4067** — Module resolution consumer consistency harness
