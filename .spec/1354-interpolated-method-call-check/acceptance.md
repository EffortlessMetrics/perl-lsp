# Acceptance Criteria: #1354 — Parser: Interpolated string delimiter check incorrectly flags method calls

## §Behavior

| Input / Condition | Expected Result | Notes |
|---|---|---|
| `"$obj->method()"` | Parse succeeds, no errors | Method calls are not interpolated in Perl strings |
| `"$obj->foo(bar, baz)"` | Parse succeeds, no errors | Method calls with arguments should not trigger paren balancing check |
| `"$x->method1()->method2()"` | Parse succeeds, no errors | Chained method calls are not interpolated |
| `"    -> $class->install_driver($driver"` (from DBI.pm line 785) | Parse succeeds, no errors | Real-world case from CPAN: literal `->` followed by interpolated `$class`, then method call |
| `"$obj->[0]"` | Parse succeeds, no errors | Array dereference IS interpolated (valid case, must still work) |
| `"$obj->{key}"` | Parse succeeds, no errors | Hash dereference IS interpolated (valid case, must still work) |
| `"unmatched $hash{unclosed"` | Parse succeeds, reports error | Unclosed brace in `$hash{` without closing `}` should still be flagged |
| `"$var[unclosed"` | Parse succeeds, reports error | Unclosed bracket in `$var[` without closing `]` should still be flagged |

**All tests pass:** `cargo test -p perl-parser-core`
**No clippy warnings:** `cargo clippy -p perl-parser-core --tests`
**Code formatted:** `cargo xtask fmt`

## §Hazards

| Class | Invariant | Surface (file:fn) | Required adversarial test |
|---|---|---|---|
| **PARSER-1: Literal / escape blindness** | Escaped delimiters inside strings do not affect balance counting. Backslash-escaped `\)` and `\(` within interpolated segments must not affect balance state. | `find_unclosed_interpolation_delimiter` | Test input: `"$x->[\\)]"` — verify escaped paren does not affect balance logic |
| **PARSER-2: Token boundary confusion** | Boundary between interpolated and literal regions is correct. `->` followed by `{` or `[` must be recognized; `->` followed by `(` or identifier must not trigger interpolation logic. | `find_unclosed_interpolation_delimiter` | Test input: `"$x->{a}$y->foo()"` — mixed valid and invalid dereference forms in one string |
| **PARSER-3: Character class misidentification** | Identifier start/continue predicates (`is_identifier_start`, `is_identifier_continue`) correctly distinguish valid Perl identifier characters from method-call markers. | `is_identifier_start`, `is_identifier_continue` | Test input: `"$obj->_method()"` — underscore is valid identifier start; test should pass |
| **PARSER-4: Balance recovery after error** | If an unclosed `{` or `[` is legitimately found, the error is reported once and scanning continues correctly (no cascading errors). | `find_unclosed_interpolation_delimiter` return, `record_unclosed_interpolation_delimiter` | Test input: `"$x->{unclosed $y->foo()"` — unclosed brace should report, not paren imbalance |
| **Test-encodes-the-bug** | The removed code blocks (`->identifier(` and direct `->(`) should never have been reached; removal closes the false-positive path. | `find_unclosed_interpolation_delimiter` lines 113–134 (deleted) | Test: revert the deletion and verify old test suite would fail on the new test cases |
| **Coverage/measurement integrity** | Removal of unreachable code paths does not change code coverage metrics for valid interpolation checks (`$var`, `$var->{}`, `$var->[`). | `find_unclosed_interpolation_delimiter` overall branch coverage | Confirm coverage remains stable after fix (lines 81–112 still covered by existing tests) |

**Subsystem-specific defaults consulted:** `docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md` — Parser (PARSER-1, PARSER-2, PARSER-3, PARSER-4 seeded from defaults; Test-encodes-the-bug and Coverage/measurement integrity added for this fix class)

## §Contracts

| Contract | Source document + section | How this change satisfies or extends it |
|---|---|---|
| Interpolation boundary rules | `PARSER_CONTRACTS.md §Interpolation-Delimiters` | Removes incorrect method-call paren-balancing; aligns with Perl spec that only `$var`, `$var->[...]`, and `$var->{...}` are interpolated in double-quoted strings. `->method()` call syntax is literal text. |
| Error recovery | `PARSER_CONTRACTS.md §Error-Recovery` | Maintains existing error reporting for legitimately unclosed `{` and `[` delimiters; only removes false-positive path for `(` after method calls. |

## §API-Shape

N/A — No new public API surface introduced. The change is purely internal bug fix (deletion of incorrect validation logic). Function signature, error types, and call sites remain unchanged.

## §Test-Grid

| Scenario | Kind | Test name | Invariant discharged |
|---|---|---|---|
| Simple method call | positive | `method_call_string_simple` | Basic `$obj->method()` parsing |
| Method call with args | positive | `method_call_with_args` | Method calls with arguments parse without false errors |
| Chained method calls | positive | `nested_method_calls_in_string` | Multiple method chains in one string do not trigger cascading errors |
| Real-world DBI.pm case | positive | `dbi_pm_line_785_reproduction` | The exact case from issue description parses cleanly |
| Array dereference (valid interp) | positive | `hash_and_array_dereference_are_valid` | `$var->[...]` still works correctly (must not regress) |
| Hash dereference (valid interp) | positive | `hash_dereference_in_string` | `$var->{...}` still works correctly (must not regress) |
| Unclosed hash access | negative | Existing test suite | `$var->{unclosed` still correctly flagged as error |
| Unclosed array access | negative | Existing test suite | `$var->[unclosed` still correctly flagged as error |
| Escaped delimiters | adversarial | To be added by green-tdd | `"$x->[\\)]"` — escaped paren must not affect balance state |
| Mixed valid/invalid in one string | adversarial | To be added by green-tdd | `"$a->{key}$b->method()"` — valid hash deref next to method call |
| Identifier with underscore | adversarial | To be added by green-tdd | `"$obj->_method()"` — underscore is valid identifier start |
| Method name edge cases | adversarial | To be added by green-tdd | `"$obj->DESTROY()"`, `"$obj->new()"` — reserved method names |

## §Blast-Radius

| Consumer | Crate | Dependency type | Impact | Required update |
|---|---|---|---|---|
| `parse_primary_inner` | perl-parser-core | internal call (line 230) | Zero impact — function signature unchanged; only internal logic modified | None |
| All parser tests | perl-parser-core | integration tests | Tests for interpolation should pass with greater accuracy; no false positives | None |
| perl-parser (public API) | perl-parser | re-exports `Parser` from -core | Zero impact — no API changes visible to consumers | None |
| LSP completion/definition providers | perl-lsp-* | indirect (parser is used) | False-positive errors should disappear from diagnostics; better UX in editor | None required (automatic via parser fix) |

Must-not-touch boundary:
- `consume_balanced_in_interpolated_string` — keep this helper function unchanged (still used for valid `->{}` and `->[]`)
- `record_unclosed_interpolation_delimiter` — keep unchanged (call site unchanged)
- `is_identifier_start`, `is_identifier_continue` — keep unchanged (still used by remaining valid-deref code)
- All other parser modules — zero changes
