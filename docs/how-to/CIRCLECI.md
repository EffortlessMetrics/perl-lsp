# CircleCI

This repository publishes a CircleCI template under [`templates/ci/circleci/config.yml`](../../templates/ci/circleci/config.yml).

The template is intentionally minimal and follows the repo's fast PR path with a single matrix job over three checks:

- `cargo fmt --all --check`
- `cargo clippy -p perl-parser -p perl-lexer --locked -- -D warnings -A missing_docs`
- `cargo test -p perl-parser -p perl-lexer --lib --locked`

To use it in a consumer repo:

1. Copy the template into `.circleci/config.yml`, or adapt the jobs to your own workflow.
2. Pin your Rust image or toolchain version if you need reproducible builds.
3. Keep Cargo caches enabled so registry and git dependencies stay warm across runs.

Example consumer workflow:

```yaml
version: 2.1

workflows:
  version: 2
  ci:
    jobs:
      - check:
          matrix:
            parameters:
              kind: [fmt, clippy, test]
```

If you need custom artifact collection, store `target/` or a narrower subdirectory in your own project after the fast checks complete.
