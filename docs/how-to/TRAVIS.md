# Travis CI

This repository prefers GitHub Actions for new CI setups. Use the Travis
template here only when you are maintaining a legacy consumer repository that
still depends on Travis.

Template path:

- [`templates/ci/travis/.travis.yml`](../../templates/ci/travis/.travis.yml)

The template is intentionally minimal:

- Rust stable, beta, and nightly are checked
- `cargo fmt --all --check` runs first
- `cargo clippy --workspace --all-targets --all-features --locked -- -D warnings` runs next
- `cargo test --workspace --all-features --locked` runs last
- nightly is allowed to fail so the stable lanes stay the signal

If you need extra Travis jobs for old consumers, add them in the consumer
repository rather than expanding this template. Keep multi-arch or
project-specific jobs local to the repository that actually needs them. For
example, add `arch: arm64` or `arch: ppc64le` jobs only in the consumer repo
when you specifically need that coverage.

For new projects, prefer the GitHub Actions workflow in
[`.github/workflows/ci.yml`](../../.github/workflows/ci.yml).
