# Native Stack Product Policy

This policy defines the product line agents should preserve while cleaning up
legacy external-tool references.

## Product rule

`perl-lsp` ships the native Rust stack by default:

- `perllsp` for LSP;
- `perl-dap` for DAP;
- native formatter;
- native critic diagnostics;
- native parser, workspace, semantic, and indexing crates.

External Perl tools such as `perltidy`, `perlcritic`, and
`Perl::LanguageServer` are not bundled and are not required for normal
operation. They may appear only as explicit compatibility, migration, or
conformance-comparison adapters.

## Public-surface rule

First-mile product surfaces must describe the native path only. This includes:

- root/product README sections;
- `docs/tutorials/` user guides;
- editor extension marketplace copy and settings descriptions;
- crate READMEs when they are the primary package landing page;
- status/readiness tables that describe distribution readiness;
- CLI help for default shipped binaries.

Legacy adapters must not be introduced in those surfaces as prerequisites,
recommended setup, normal run modes, or default behavior.

## Allowed legacy surfaces

Legacy external tooling references are allowed when the surrounding document is
explicit about compatibility or migration scope. Preferred locations are:

- `docs/reference/*LEGACY*`;
- `docs/reference/archive/`;
- migration or compatibility how-to pages;
- conformance reports and receipts;
- tests that prove legacy references are isolated from native-first surfaces.

## Default-behavior rule

Installed external tools must not change default behavior merely by being on
`PATH`.

- Formatting defaults to the native formatter. `perltidy` may run only when an
  explicit external/compatibility engine is selected.
- Critic diagnostics default to the native critic engine. `perlcritic` may run
  only when an explicit external/compatibility engine is selected.
- Native DAP launch/attach requires a local Perl interpreter and the shipped
  `perl-dap` binary; `Perl::LanguageServer` must not be required for the native
  DAP path.

## Packaging rule

Release archives should contain product binaries and runtime assets owned by
this workspace. They must not bundle external Perl tooling payloads such as
`perltidy`, `perlcritic`, `Perl::LanguageServer`, or
`Devel::TSPerlDAP`.

## Regression search

Agents working on this cleanup should start with a targeted search like:

```bash
rg -n "requires perltidy|requires perlcritic|Perl::LanguageServer.*required|BridgeAdapter|cpanm Perl::LanguageServer|cpan Perl::LanguageServer" \
  docs/tutorials docs/reference crates/perl-dap vscode-extension/package.json book/src
```

Allowed hits must either be in a legacy/compatibility document or in tests that
assert legacy wording stays out of first-mile native docs.
