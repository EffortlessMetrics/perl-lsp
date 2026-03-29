# perllsp

`perllsp` is the Cargo package for the `perl-lsp` language server.

If you want the Perl language server binary, this is the package to install:

```bash
cargo install perllsp
```

That installs the `perl-lsp` binary.

## Why this crate exists

The implementation crate is published as [`perl-lsp-rs`](https://docs.rs/perl-lsp-rs), but
the user-facing Cargo package is `perllsp`. This crate keeps install docs and docs.rs routing
simple while delegating the actual server implementation to `perl-lsp-rs`.

## Quick start

```bash
perl-lsp --version
perl-lsp --health
perl-lsp --stdio
```

## Library facade

This crate re-exports the `perl-lsp-rs` library surface so docs.rs and Cargo users land on the
public package name first.

## Related crates

- [`perl-lsp-rs`](https://docs.rs/perl-lsp-rs): implementation crate for the language server
- [`perl-parser`](https://docs.rs/perl-parser): parser and analysis engine
- [`perl-dap`](https://docs.rs/perl-dap): Perl debug adapter
