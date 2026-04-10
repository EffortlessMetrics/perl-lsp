# Pre-commit Framework

`perl-lsp` publishes a root-level [`.pre-commit-hooks.yaml`](../../.pre-commit-hooks.yaml) so downstream repos can reuse the same fast local gates.

Use a pinned release tag in your consumer repo. `rev` should point at a tag, not a branch, so the hook set stays reproducible:

```yaml
# .pre-commit-config.yaml in a consumer repo
repos:
  - repo: https://github.com/EffortlessMetrics/perl-lsp
    rev: v0.12.3
    hooks:
      - id: perl-lsp-fmt
      - id: perl-lsp-clippy
      - id: perl-lsp-test
```

Install and run it once from the consumer repo:

```bash
pre-commit install
pre-commit run --all-files
```

These hooks are intentionally fast and map to the repo's PR-fast path:

- `perl-lsp-fmt` runs the format check
- `perl-lsp-clippy` runs the core clippy gate
- `perl-lsp-test` runs the fast library test gate

For the full merge gate, keep using the repo's normal CI gate or pre-push hook flow.
