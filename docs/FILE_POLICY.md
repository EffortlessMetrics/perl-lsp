# File policy

Rust and `xtask` are the default implementation surfaces for this repository.
Non-Rust files are allowed where they serve a clear product, test, release, or CI
purpose, but the Rust 1.95 / 0.14.0 rollout targets receipt-backed file policy so
those surfaces do not expand anonymously.

## Legitimate non-Rust surfaces

The rollout map recognizes these legitimate non-Rust areas:

- Perl fixtures and corpus data.
- Tree-sitter C/native parser bindings.
- VS Code extension surfaces.
- GitHub workflows.
- CI scripts.
- Generated docs/status artifacts.
- Release metadata.

## Target ledger fields

The planned non-Rust allowlist ledger should require each entry to include:

- `id`
- `glob`
- `kind`
- `language`
- `surface`
- `classification`
- `owner`
- `reason`
- `covered_by`
- `created`
- `review_after`

Broad globs should also include `broad_glob_reason`.

## Rollout boundary

This documentation PR does not add enforcement. Enforcement belongs in the later
file-policy checker PR described in
[`docs/ci/perl-lsp-rust-1.95-rollout.md`](ci/perl-lsp-rust-1.95-rollout.md).
