# PLSP-SPEC-0015: Native Stack Product Surface

## Status

Draft implementation spec.

## Problem

The project now has native runtime implementations for the main product paths,
but legacy external tools still leak into public setup, status, CLI, and test
surfaces. Those leaks make `perltidy`, `perlcritic`, and
`Perl::LanguageServer` look like ordinary runtime dependencies even when the
intended product story is native-first.

## Goal

Make the native Rust stack the only first-mile product story while preserving
external tools only as explicit compatibility, migration, or conformance
surfaces.

## Non-goals

- Do not delete all compatibility code in a docs/spec cleanup PR.
- Do not claim byte-for-byte parity with external tools unless a receipt proves
  that exact claim.
- Do not cut or prepare a release as part of this cleanup.
- Do not rewrite unrelated documentation.

## Definitions

- **Native surface**: a public guide, README, CLI help path, editor setting, or
  status table intended for ordinary users installing or using the product.
- **Legacy surface**: a clearly labeled compatibility, migration, archive, or
  conformance surface for users who intentionally need external-tool behavior.
- **External tool**: `perltidy`, `perlcritic`, `Perl::LanguageServer`,
  `Devel::TSPerlDAP`, or similar non-workspace Perl tooling.

## Required invariants

1. Native public guides do not require or recommend external tools for normal
   operation.
2. Installed external tools do not change defaults merely by being available on
   `PATH`.
3. Legacy references are quarantined to compatibility/migration/reference
   surfaces.
4. Tests guard the quarantine boundary: native guides assert absence of legacy
   dependencies; legacy docs assert presence of required compatibility setup.
5. Release/archive checks prove external Perl tools are not bundled.

## Work packets

### Packet A: Native-only DAP public docs

Primary files:

- `docs/tutorials/DAP_USER_GUIDE.md`
- `book/src/dap/user-guide.md`
- `crates/perl-dap/README.md`
- `docs/reference/DAP_LEGACY_BRIDGE.md` or a new `docs/reference/DAP_LEGACY_BRIDGE_COMPAT.md`

Acceptance criteria:

- `docs/tutorials/DAP_USER_GUIDE.md` contains no `Perl::LanguageServer`,
  `BridgeAdapter`, `cpan Perl::LanguageServer`, or
  `cpanm Perl::LanguageServer` text.
- The guide says native `perl-dap` requires a local Perl interpreter and the
  shipped native adapter.
- The architecture section describes native `perl-dap` over stdio/TCP driving a
  local Perl debuggee.
- Roadmap-like `Planned Features` language is replaced with a current hardening
  focus.
- The book page is either generated from the canonical guide or reduced to a
  pointer to it.
- Bridge setup, if retained, lives only in a legacy reference document.

### Packet B: DAP dependency test quarantine

Primary file:

- `crates/perl-dap/tests/dap_dependency_tests.rs`

Acceptance criteria:

- Tests assert the public DAP guide does not mention `Perl::LanguageServer`,
  `BridgeAdapter`, or CPAN install commands.
- Tests assert the legacy bridge reference, if retained, documents its
  `Perl::LanguageServer` dependency and CPAN install commands.

### Packet C: DAP CLI/API product stance

Primary files:

- `crates/perl-dap/src/main.rs`
- `crates/perl-dap/src/server/mode.rs`
- `crates/perl-dap/src/server/lifecycle.rs`
- `crates/perl-dap/src/lib.rs`

Acceptance criteria for the soft quarantine path:

- `perl-dap --help` does not present bridge mode as a normal run mode.
- Any retained bridge flag is hidden or explicitly labeled legacy.
- Crate-root docs advertise native runtime capabilities, not a dual native/bridge
  product architecture.

Acceptance criteria for the hard removal path:

- The shipped CLI has no bridge flag.
- Public runtime mode types no longer expose bridge as a supported product mode.
- Bridge compatibility code is removed, archived, or feature-gated away from the
  default product surface.

### Packet D: VS Code settings and help copy

Primary files:

- `vscode-extension/package.json`
- `vscode-extension/README.md`
- `docs/reference/CONFIG.md`

Acceptance criteria:

- Formatting settings no longer say `perltidy` is required.
- Format-on-save copy says the configured formatter engine is used and that the
  default is native.
- Critic settings no longer present external `perlcritic` as the default product
  path.
- Existing legacy-flavored setting names, if retained, are documented as
  compatibility aliases or explicit external/legacy controls.

### Packet E: Native-first execute-command critic path

Primary file:

- `crates/perl-lsp-rs/src/execute_command/provider.rs`

Acceptance criteria:

- `perl.runCritic` uses the native critic by default.
- `perlcritic` on `PATH` does not change default behavior.
- External `perlcritic` runs only when explicitly configured.
- Returned command metadata uses the product term `native` for the native engine.

### Packet F: Formatter engine default guard

Primary areas:

- formatter selection code;
- `perl-lsp-perltidy` compatibility adapter tests;
- config/default tests.

Acceptance criteria:

- Default formatter engine is native.
- `perltidy` on `PATH` does not change default behavior.
- External `perltidy` runs only when explicitly configured.
- Compatibility reporting can parse `.perltidyrc` without requiring external
  formatting as the default path.

### Packet G: Status and downstream contract cleanup

Primary files:

- `docs/project/status/dap.md`
- `docs/reference/DOWNSTREAM_DAP_INTEGRATIONS.md`

Acceptance criteria:

- Distribution readiness tables describe the native DAP product surface.
- Legacy bridge compatibility, if mentioned, is isolated in a separate legacy
  note and not counted as core distribution readiness.
- Downstream integration language says managed native `perl-dap` explicitly.

### Packet H: Negative packaging guard

Primary areas:

- release artifact checks in `xtask`;
- release workflow docs or commands;
- downstream packaging contract docs.

Acceptance criteria:

- Archive checks fail if product archives include `perltidy`, `perlcritic`,
  `Perl::LanguageServer`, `Devel::TSPerlDAP`, or bridge shim payloads.
- Archive checks still confirm the expected native binaries, including
  `perllsp` and `perl-dap`/`perl-dap.exe`.

## Suggested proof commands

Use the smallest command that proves the touched packet. Examples:

```bash
rg -n "Perl::LanguageServer|BridgeAdapter|cpanm Perl::LanguageServer|cpan Perl::LanguageServer" docs/tutorials/DAP_USER_GUIDE.md book/src/dap/user-guide.md crates/perl-dap/README.md
./scripts/cargo-safe test -p perl-dap --test dap_dependency_tests --profile agent --locked
./scripts/cargo-safe check -p perl-dap --all-targets --profile agent --locked
./scripts/cargo-safe test -p perllsp --profile agent --locked
./scripts/cargo-safe xtask fmt
```

## PR guidance

Keep PRs packet-sized. Do not combine DAP docs, VS Code settings, critic runtime
behavior, formatter defaults, and release packaging in one PR unless the
orchestrator explicitly assigns that broader lane.
