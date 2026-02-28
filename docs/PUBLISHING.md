# Publishing Guide

This guide covers crates.io publishing for release alignment during the initial and subsequent release train.

## Publishing Model

Publishing is handled by the GitHub workflow [`publish-crates`](../.github/workflows/publish-crates.yml), which:

- computes publish order from workspace metadata
- filters out crates with `publish = false` / `publish = []`
- runs each crate publish in dependency order
- verifies each published version

This is the same path used by the `release-orchestration` workflow.

## Prerequisites

1. Create or confirm an account on [crates.io](https://crates.io)
2. Authenticate locally with `cargo login`
3. Ensure release checks pass (`just ci-full`, `just security-scan`, `just semver-check`)
4. Confirm release version and changelog are finalized

## Recommended Path (Automated)

1. Complete the release branch and tag workflow as documented in [`RELEASE_PROCESS.md`](RELEASE_PROCESS.md).
2. In GitHub Actions, run **Release Orchestration** with:
   - `version: <release version>` (for example `1.0.0`)
   - `skip_crates: false`
3. Validate that the **Publish to crates.io** workflow completes and reports all crates published.

To inspect the current workspace publish set locally:

```bash
cargo metadata --no-deps --format-version=1 \
  | jq '.packages | map(select(.publish == null or (.publish | length > 0))) | length'
```

To inspect the exact publish order used by the workflow, read the "Compute topological order" output in the workflow run.

## Manual Fallback (Use with caution)

If automated publish fails and needs recovery, publish remaining crates one-by-one using the workflow summary order:

```bash
cargo publish --package <crate-name> --dry-run  # Verify inputs and artifacts
cargo publish --package <crate-name>            # Publish a single crate
```

## Post-Publishing

1. Verify `RELEASE_NOTES.md` and release artifacts are complete.
2. Confirm `cargo install perl-lsp` works for the new release version.
3. Update documentation links where versioned examples are present.
4. Announce release in project channels.

## Version Checklist

- [ ] Workspace version updated for the release
- [ ] `CHANGELOG.md` finalized
- [ ] Release tag prepared
- [ ] Required publish dependencies available on crates.io
- [ ] Release checklist completed in `RELEASE_PROCESS.md`
