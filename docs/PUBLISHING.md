> **In progress (2026-04-14):** The publishing pipeline is being simplified as part of the microcrate collapse — see [ADR-0041](adr/0041-microcrate-collapse.md), [PUBLISHING_AFTER_COLLAPSE.md](project/PUBLISHING_AFTER_COLLAPSE.md), and [tracking issue #4410](https://github.com/EffortlessMetrics/perl-lsp/issues/4410). This document describes the current 132-crate pipeline; expect dramatic simplification once the collapse lands.

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
4. Run crates.io dry-run validation (`just prep-crates-io-launch` for launch crates, or `just prep-crates-io-launch all` for full allowlist)
5. Confirm release version and changelog are finalized

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
2. Confirm `cargo search perl-lsp-rs --limit 1` and `cargo search perllsp --limit 1` show the new release version.
3. Confirm `cargo install perllsp` works for the new release version.
4. Update documentation links where versioned examples are present.
5. Announce release in project channels.

- Confirm `Release` and `Publish to crates.io` workflows completed successfully.
- Spot-check package index visibility with `cargo search` for critical crates (`perllsp`, `perl-lsp-rs`, `perl-parser`, `perl-dap`).
- Validate `cargo install perllsp` succeeds and executes `perllsp --version`.

## Pre-Publish Checklist

- [ ] Workspace version updated for the release
- [ ] `CHANGELOG.md` finalized
- [ ] Release tag prepared
- [ ] Required publish dependencies available on crates.io
- [ ] Release checklist completed in `RELEASE_PROCESS.md`

## Turnkey Workflow Integration

To run the entire path from PR creation through publish dispatch:

```bash
cargo xtask release-turnkey <0.x.y>
```

Use `--skip-crates` to run validation and release without crates.io publishing when needed.
