# Publishing Guide

This guide describes how `perl-lsp` is published to crates.io as part of the PR-driven release flow.

## Automated Crates.io Path

Publishing to crates.io is handled by the [`publish-crates`](../.github/workflows/publish-crates.yml) workflow, which is normally triggered by [Release Orchestration](./RELEASE_PROCESS.md):

1. Merge the version bump PR created by `Version Bump & Changelog Generation`.
2. Dispatch `Release Orchestration` with the target `version`.
3. Release orchestration creates the tag and dispatches publish workflows, including `Publish to crates.io`.
4. Publish workflow resolves publish order from workspace metadata and runs crates in dependency order.

## Workspace Coverage

The publish workflow includes every crate with publishing enabled.

To verify currently publishable crates, run:

```bash
cargo metadata --no-deps --format-version=1 |\
  jq '.packages | map(select(.publish == null or (.publish | length > 0))) | map(.name) | length'
```

## Reusable Manual Checks

For investigation or recovery during release:

```bash
# Verify the target version for a single crate
cargo search <crate-name> --limit 1

# Dry-run publish for a single crate through repo tooling
cargo publish --dry-run -p <crate-name>

# Full publish (requires CARGO_REGISTRY_TOKEN in environment)
cargo publish -p <crate-name>
```

## Post-Publish Verification

After publish completes:

- Confirm `Release` and `Publish to crates.io` workflows completed successfully.
- Spot-check package index visibility with `cargo search` for critical crates (`perl-lsp`, `perl-parser`, `perl-dap`).
- Validate `cargo install perl-lsp` succeeds and executes `perl-lsp --version`.

## Turnkey Workflow Integration

To run the entire path from PR creation through publish dispatch:

```bash
scripts/release-turnkey-pr.sh <0.x.y>
```

Use `--skip-crates` to run validation and release without crates.io publishing when needed.
