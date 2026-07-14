# CLAUDE.md (perllsp)

## Role

Published Cargo facade for the installable `perllsp` language-server binary. It
exists so `cargo install perllsp` gives users a stable crate/binary name while
the actual implementation lives elsewhere.

## Owns

- `src/lib.rs` -- `pub use perl_lsp::*;` re-export of the full public surface.
- `src/main.rs` -- binary entrypoint; delegates to `perllsp::run_cli(...)`.

Nothing else. This crate intentionally has almost no code of its own.

## Does not own

No LSP protocol handling, no providers, no runtime logic -- all of that lives
in `perl-lsp-rs` (and transitively `perl-lsp-rs-core`). If a change needs more
than a re-export here, it belongs in `perl-lsp-rs`, not this crate.

## Neighbors

- Upstream: `perl-lsp-rs` (the only dependency).
- Downstream: none in-workspace -- this is the top of the dependency graph
  (the crate that gets published/installed).

## Read first

- `src/lib.rs` and `src/main.rs` (both are a few lines).
- `crates/perl-lsp-rs/CLAUDE.md` for the real implementation this facade wraps.

## Focused validation

`cargo test -p perllsp` -- see `tests/cli_smoke.rs` for the binary-level smoke
test. Behavioral coverage of the server itself lives in `perl-lsp-rs` /
`perl-lsp-rs-core`, not here.

## Review hotspots

Any PR that adds logic to this crate (beyond re-exports and CLI arg plumbing)
is drifting from the facade pattern -- push it down into `perl-lsp-rs`.

## Claim boundary

Describes this crate's structure as authored (a thin re-export shim). Makes no
claim about the LSP server's runtime behavior -- see `perl-lsp-rs-core`'s
package-local context for that.
