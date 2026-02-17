# perl-lsp-tooling

External tooling runtime adapters for Perl LSP.

## Scope

- Abstracts subprocess execution for formatter/linter integrations.
- Provides production runtime (`OsSubprocessRuntime`) and test mocks.
- Implements integrations for `perltidy` and `perlcritic`.

## Public Surface

- Runtime interfaces: `SubprocessRuntime`, `SubprocessOutput`, `SubprocessError`.
- Runtime implementation: `OsSubprocessRuntime`.
- Modules: `perltidy`, `perl_critic`, `performance`, and `mock`.

## Workspace Role

Internal infrastructure crate used by formatting, diagnostics, and command execution layers.

## License

MIT OR Apache-2.0.
