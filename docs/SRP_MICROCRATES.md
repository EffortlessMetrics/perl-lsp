# SRP Microcrates

This workspace uses **single-responsibility (SRP) microcrates** to keep behavior focused and reusable.

To find and separate SRP-style microcrates from broader workspace crates, run:

```bash
scripts/list-srp-microcrates.py
```

The script scans each `crates/perl-*` crate for SRP signals (`single responsibility`, `SRP`, `microcrate`) in:

- `Cargo.toml`
- `README.md`
- `src/lib.rs`

## Current snapshot

Detected `18` crates with SRP/microcrate signals.

### `perl-module-*` (5)

- `perl-module-name`
- `perl-module-resolution-path`
- `perl-module-resolution-uri`
- `perl-module-token-core`
- `perl-module-token-parser`

### `perl-lsp-feature-*` (6)

- `perl-lsp-feature-contracts`
- `perl-lsp-feature-flags`
- `perl-lsp-feature-grid`
- `perl-lsp-feature-ids`
- `perl-lsp-feature-policy`
- `perl-lsp-feature-profile`

### `perl-lsp-*` (2)

- `perl-lsp-cancellation`
- `perl-lsp-launcher`

### `perl-workspace-*` (2)

- `perl-workspace-folder`
- `perl-workspace-index-slo`

### `perl-ts-*` (2)

- `perl-ts-heredoc-parser`
- `perl-ts-partial-ast`

### Core/misc (1)

- `perl-text-line`

## Notes

- This is an intentionally conservative detector based on explicit SRP wording.
- Add SRP wording to crate docs when introducing new microcrates so they are discovered by this inventory.
