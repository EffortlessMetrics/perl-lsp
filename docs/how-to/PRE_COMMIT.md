# Pre-commit Integration

This repository publishes a `.pre-commit-hooks.yaml` file so contributors can run the same fast checks locally before they commit.

## Quick Start

```yaml
repos:
  - repo: https://github.com/EffortlessMetrics/perl-lsp
    rev: v0.12.3
    hooks:
      - id: perl-lsp-fmt
      - id: perl-lsp-clippy
      - id: perl-lsp-test
```

Then install and run the hooks:

```bash
pre-commit install
pre-commit run --all-files
```

## Notes

- The hooks use `cargo` from your local environment.
- `perl-lsp-fmt` checks formatting with `cargo fmt --all --check`.
- `perl-lsp-clippy` runs `cargo clippy --workspace --all-targets --locked -- -D warnings -A missing_docs`.
- `perl-lsp-test` runs `cargo test --workspace --locked`.
