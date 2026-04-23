# Continuous Integration

This repository uses two default PR jobs and a small set of opt-in workflows.
Keep this doc aligned with the command flow in
[CONTRIBUTING.md](../CONTRIBUTING.md) and
[CI_LOCAL_VALIDATION.md](./CI_LOCAL_VALIDATION.md).

## Default PR Path

### PR Smoke

Runs on every pull request for fast feedback.

- `cargo fmt --all`
- `just clippy-core`
- `just test-core`

### Merge Gate

Runs on `merge-ready`, pushes to `main` or `master`, and manual workflow runs.
This is the server-side equivalent of the local `nix develop -c just ci-gate`
gate.

- `just gates`
- receipt upload and status reporting

## GitLab CI Template

Use [`templates/ci/gitlab/.gitlab-ci.yml`](../../templates/ci/gitlab/.gitlab-ci.yml)
as the starting point for GitLab-hosted repos or self-managed runners.

The template keeps the GitLab jobs intentionally small:

- `fmt`, `clippy`, and `test` run as separate jobs in the same stage so GitLab
  can schedule them in parallel.
- Cargo state is cached under `.cargo/` and `target/`, keyed by `Cargo.lock`.
- The default image is the stock Rust image, but you can swap in a GitLab
  Container Registry image if you want to preinstall toolchains or share a
  custom CI environment.

Example registry image override:

```yaml
default:
  image: $CI_REGISTRY_IMAGE/ci/rust:1.92-bookworm
```

## Local Equivalents

Use these commands in the same order when iterating locally:

```bash
just devex
just pr-fast
nix develop -c just ci-gate
just ci-full
just status-update
just status-check
just release-check
```

## Opt-In Workflows

The label-gated nightly workflow is `ci-nightly.yml`. Its active labels are:

- `ci:coverage`
- `ci:bench`
- `ci:strict`
- `ci:mutation`

The security workflow is `ci-security.yml`. It runs on path-sensitive pushes
and scheduled checks.

The full label catalog is documented in [`.github/ci-config.yml`](../../.github/ci-config.yml).
That file is the source of truth for reserved labels such as `ci:lsp`,
`ci:determinism`, `ci:audit`, `ci:semver`, and `ci:all-tests`.

## Release Prep

Use `just release-check` before tagging or publishing a release candidate.
That command layers release-specific checks on top of the merge gate.
