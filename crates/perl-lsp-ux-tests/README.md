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
- hover, goto-definition, goto-declaration, strict diagnostics, and document symbols flows, and
- multi-root `workspace/symbol` disambiguation via `workspaceFolderUri`, and
- workspace-folder removal evicting stale symbols from search results, and
- deleted-file churn evicting stale search results and definition targets.

The harness is now also the source of truth for the workflow UX scorecard
inventory:

- `.ci/schemas/editor-ux.schema.json` defines the eventual measured
  `editor_ux.json` contract.
- `docs/project/metrics/WORKFLOW_SCORECARDS.md` explains how that workflow layer
  fits above the subsystem scorecards.
- `crates/perl-lsp-ux-tests/fixtures/editor_ux_fixture_matrix.json` maps each
  executable scenario to current scorecard rows and subsystem ownership.
- `tests/editor_ux_fixture_matrix.rs` keeps that matrix aligned with the actual
  `ux_scenario_*.rs` files.

## Running the tests

From the workspace root:

```bash
cargo test -p perl-lsp-ux-tests
```

The local `just ux-tests` and `just ux-tests-full` recipes build
`target/debug/perl-lsp` first and set `PERL_LSP_BIN` automatically. Raw
`cargo test` runs still require a prebuilt binary or an explicit
`PERL_LSP_BIN=/path/to/perl-lsp`.

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
