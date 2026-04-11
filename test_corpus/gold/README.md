# Gold Corpus — Diagnostics Validation Fixtures

The gold corpus is a curated set of Perl code fixtures with hand-verified expected diagnostics. Each fixture serves as a test case for the LSP diagnostics pipeline, validating that the parser correctly identifies (or does not identify) diagnostics for Perl code patterns.

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
- **`expected.json`** — A JSON file specifying expected diagnostic assertions

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

**Purpose**: Conditional pragma loading via `use if` (strict disabled).

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

The gold corpus directory structure supports sibling assertions for future scorecards:

- `expected_diagnostics.json` — Diagnostics expectations (current)
- `expected_hover.json` — Hover information expectations (future)
- `expected_module.json` — Module resolution expectations (future)
- `expected_completion.json` — Completion suggestions (future)

Each fixture can carry multiple assertion files without conflicts.

## Related Issues

- **#4065** — Bootstrap gold corpus seed fixtures and diagnostics scorecard
