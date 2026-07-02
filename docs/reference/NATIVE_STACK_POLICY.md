# Native Stack Policy

`perl-lsp` ships the native Rust stack by default:

- `perllsp` for LSP.
- `perl-dap` for DAP.
- Native formatting.
- Native critic diagnostics.
- Native parser, workspace, semantic, and test-discovery infrastructure.

External Perl tools such as `perltidy`, `perlcritic`, `Perl::LanguageServer`,
and other legacy debugger backends are **not bundled** and are **not required**
for normal operation.

Those tools may be used only as explicit compatibility, migration, or
conformance comparison surfaces. They are optical benches for validating native
behavior, not product dependencies and not artifacts we ship to users.

## Product-surface rules

Public first-mile docs, Marketplace copy, editor settings, release artifacts, and
the default CLI help should describe the native stack only.

Allowed external-tool references belong in one of these places:

- compatibility or migration reference docs;
- conformance-comparison reports;
- historical ADRs or issue archaeology;
- tests that prove legacy content is isolated from the native product surface.

## Release artifact rule

Release archives should contain the native Rust binaries and supporting metadata:

- `perllsp` / `perllsp.exe`
- `perl-dap` / `perl-dap.exe`
- checksums and release metadata

They should not contain external Perl tooling payloads.
