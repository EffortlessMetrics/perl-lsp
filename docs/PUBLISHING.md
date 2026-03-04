# Publishing Guide

This guide covers crates.io publishing for release alignment during the initial and subsequent release train.

## Publishing Model

Publishing is handled by the GitHub workflow [`publish-crates`](../.github/workflows/publish-crates.yml), which:

- computes publish order from workspace metadata
- filters out crates with `publish = false` / `publish = []`
- runs each crate publish in dependency order
- verifies each published version

This is the same path used by the `release-orchestration` workflow.

## Automated Crates.io Path

1. Create or confirm an account on [crates.io](https://crates.io)
2. Authenticate locally with `cargo login`
3. Ensure release checks pass (`just ci-full`, `just security-scan`, `just semver-check`)
4. Confirm release version and changelog are finalized

## Recommended Path (Automated)

1. Complete the release branch and tag workflow as documented in [`RELEASE_PROCESS.md`](RELEASE_PROCESS.md).
2. In GitHub Actions, run **Release Orchestration** with:
   - `version: <release version>` (for example `0.x.y`)
   - `skip_crates: false`
3. Validate that the **Publish to crates.io** workflow completes and reports all crates published.

## Workspace Coverage

Crates listed in `[workspace.metadata.publish.allow]` are published in dependency order.
To inspect the configured publish allowlist, run:

```bash
cargo metadata --no-deps --format-version=1 |\
  jq '.metadata.publish.allow'
```

To inspect the exact publish order used by the workflow, read the "Compute topological order" output in the workflow run.

For local packaging dry-runs with workspace path patching, use:

```bash
# package every crate in [workspace.metadata.publish.allow]
scripts/cargo-package-workspace-dry-run.sh

# package specific crates
scripts/cargo-package-workspace-dry-run.sh perl-parser perl-lsp perl-dap
```

## Manual Fallback (Use with caution)

If automated publish fails and needs recovery, publish remaining crates one-by-one using the workflow summary order:

```bash
# Verify the target version for a single crate
cargo search <crate-name> --limit 1

# Dry-run publish for a single crate
cargo publish --dry-run -p <crate-name>

# Full publish (requires CARGO_REGISTRY_TOKEN in environment)
cargo publish -p <crate-name>
```

## Post-Publish Verification

After publish completes:

1. Verify `RELEASE_NOTES.md` and release artifacts are complete.
2. Confirm `cargo install perl-lsp` works for the new release version.
3. Update documentation links where versioned examples are present.
4. Announce release in project channels.

- Confirm `Release` and `Publish to crates.io` workflows completed successfully.
- Spot-check package index visibility with `cargo search` for critical crates (`perl-lsp`, `perl-parser`, `perl-dap`).
- Validate `cargo install perl-lsp` succeeds and executes `perl-lsp --version`.

## Pre-Publish Checklist

- [ ] Workspace version updated for the release
- [ ] `CHANGELOG.md` finalized
- [ ] Release tag prepared
- [ ] Required publish dependencies available on crates.io
- [ ] Release checklist completed in `RELEASE_PROCESS.md`

## Turnkey Workflow Integration

To run the entire path from PR creation through publish dispatch:

```bash
scripts/release-turnkey-pr.sh <0.x.y>
```

Use `--skip-crates` to run validation and release without crates.io publishing when needed.
