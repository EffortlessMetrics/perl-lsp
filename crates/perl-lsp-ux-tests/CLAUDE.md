# perl-lsp-ux-tests

End-to-end UX scenarios that exercise multiple LSP consumers (completion,
diagnostics, goto-definition, hover, code-actions) against the same fixture.

## Test Threading

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-ux-tests -- --test-threads=2
```

## Verify

```bash
cargo fmt --all
cargo clippy -p perl-lsp-ux-tests --tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp-ux-tests -- --test-threads=2
```

## Fixture shape: prefix vs exact, by consumer

A UX scenario typically asserts multiple LSP consumers see the same logical
state. **Consumer parity does NOT mean identical source text** — it means
semantically equivalent fixture intent, with the right source form for each
consumer.

| Consumer        | Valid fixture shape                                      |
| --------------- | -------------------------------------------------------- |
| completion      | prefix form, cursor inside the prefix: `use Gre<cursor>` |
| PL701 diagnostic| exact module form, fully resolved: `use GreetModule;`    |
| goto-definition | exact module form: `use GreetModule;`                    |
| hover           | exact module form: `use GreetModule;`                    |
| document-link   | exact module form                                        |
| code-actions    | exact module form (action triggers off resolved symbol)  |

**Why**: completion-fixture text is incomplete by design — the prefix is what
the user has typed *so far*. Asking goto-definition or PL701 to resolve a
prefix is a category error; the symbol isn't named yet. The two fixture
shapes must coexist in the scenario, not collapse to one.

**Tell that a fixture is miscategorized**: an ignored test or `#[ignore]` with
a FIXME pointing to "completion returns no result" while the assertion is on
goto-definition. That's the symptom of mixing prefix-form source with an
exact-symbol consumer. The fix is two fixtures (one prefix-cursor, one exact),
not one tortured fixture.

**Historical**: `scenario_14` removed a `#[ignore]`'d test in #8524 for exactly
this reason — it asserted goto-definition on `use Gre`. See
`tests/ux_scenario_14_inc_conformance.rs` for the current pattern: separate
`*_COMPLETION_SOURCE` and `*_SOURCE` constants per fixture.

## After a scenario change, run scenario-14 specifically

```bash
RUST_TEST_THREADS=2 cargo test -p perl-lsp-ux-tests --test ux_scenario_14_inc_conformance
```

This scenario is the @INC strictness conformance grid — touches completion,
PL701, goto-def, hover. Most fixture-shape regressions surface here first.
