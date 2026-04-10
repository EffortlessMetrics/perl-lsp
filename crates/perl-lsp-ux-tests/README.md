# perl-lsp-ux-tests

UX regression harness for `perl-lsp` that exercises “first 5 minutes” user scenarios against a real LSP server process.

## What this crate covers

This crate validates user-visible behavior by:

- creating a temporary workspace,
- spawning the actual `perl-lsp` binary,
- issuing real LSP requests (`didOpen`, `hover`, `completion`, formatting, etc.), and
- asserting outcomes from a UX perspective (helpful response, no crash, expected diagnostics/messages).

Scenarios currently include:

- simple-file startup smoke tests,
- missing toolchain binaries (`perl`, `perltidy`, `perlcritic`),
- bad configuration handling,
- large-file handling,
- shebang/encoding behavior,
- multi-file workspace interactions,
- hover, goto-definition, strict diagnostics, and document symbols flows.

## Running the tests

From the workspace root:

```bash
cargo test -p perl-lsp-ux-tests
```

To force integration-gated tests (if present):

```bash
cargo test -p perl-lsp-ux-tests --features integration-test
```

## Environment variables

The harness supports these runtime controls:

- `PERL_LSP_BIN` — override the `perl-lsp` binary path.
- `UX_TEST_TIMEOUT_MS` — per-request timeout in milliseconds (default: `10000`).
- `UX_TEST_ECHO_STDERR` — if set, echo LSP stderr into test output.

## Authoring new UX scenarios

1. Add a new `tests/ux_scenario_XX_*.rs` file.
2. Create a harness with `UxHarness::new(ScenarioConfig::default())`.
3. Seed files via `ScenarioConfig::with_file(...)` or `harness.open_file(...)`.
4. Drive UX actions (`hover`, `completion`, goto-definition, formatting, etc.).
5. Assert no crash and validate response quality.

Keep scenarios focused on user workflows and regression intent.
