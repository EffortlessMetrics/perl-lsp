# 2026-05-03 — `Attach VSIX to GitHub Release` Race vs. Release Creation

**Lens**: `Attach VSIX to GitHub Release` ran in parallel with the GitHub release-creation step and lost the race. The VSIX never made it to the release. Recovered manually because the publish job had stored the VSIX as a workflow artifact for separate reasons.

## What failed

Inside `Publish VSCode Extension` workflow run `25274154490`:

```
Attach VSIX to GitHub Release	Create GitHub Release asset	2026-05-03T08:30:33.6151643Z
##[error]Release v0.13.3 was not available in time for VSIX attachment.
##[error]Process completed with exit code 1.
```

The release was created by a different workflow (`Release` run `25274153685`, which succeeded). The `Attach VSIX` job in `Publish VSCode Extension` was supposed to upload the VSIX as an asset of that release. It ran before the release was queryable via the GitHub API.

Result: `v0.13.3` GitHub release was created with 9 assets (binaries + SHA256SUMS + sbom-spdx.json) but **no VSIX**. Users could install the extension from Marketplace/Open VSX, but anyone trying to install from the GitHub release directly couldn't find the VSIX.

## Why the race exists

The two workflows run in parallel (both triggered by the same orchestration run). The `Publish VSCode Extension` workflow does:

1. Build VSIX
2. Publish to Marketplace
3. Publish to Open VSX
4. **Attach VSIX to GitHub Release** ← this step
5. Smoke published extension
6. Summary

Step 4 assumes the GitHub release exists. The release is created by the parallel `Release` workflow, which has its own job graph. There's no explicit dependency between the two.

When the `Release` workflow runs slow (or `Publish VSCode Extension` runs fast), step 4 hits a race window where the release tag exists but the GitHub release object isn't queryable yet.

## Recovery

The publish step had stored the VSIX as a workflow artifact (`actions/upload-artifact@v4`) for *debug visibility* — not specifically as a recovery point, but the artifact was there:

```bash
$ gh api repos/EffortlessMetrics/perl-lsp/actions/runs/25274154490/artifacts \
  --jq '.artifacts | map({name, archive_download_url, size_in_bytes})'
[{"archive_download_url":"https://api.github.com/.../artifacts/6769294311/zip",
  "name":"perl-lsp-rs-0.13.3.vsix","size_in_bytes":6246085}]
```

Recovery commands:

```bash
mkdir -p /tmp/vsix-attach && cd /tmp/vsix-attach
gh api repos/EffortlessMetrics/perl-lsp/actions/artifacts/6769294311/zip > vsix-artifact.zip
unzip -o vsix-artifact.zip
gh release upload v0.13.3 perl-lsp-rs-0.13.3.vsix \
  --repo EffortlessMetrics/perl-lsp --clobber
```

Verification:

```bash
$ gh release view v0.13.3 --json assets --jq '.assets | map(.name)'
["perl-lsp-rs-0.13.3.vsix","perllsp-0.13.3-aarch64-apple-darwin.tar.gz", ..., "SHA256SUMS"]
# 10 assets — VSIX now present
```

Total recovery time: ~3 minutes including artifact download + manual upload.

## What this taught about artifacts

The VSIX artifact was uploaded for debug purposes, not because anyone had thought "we might need this for recovery." It became the recovery vehicle by accident.

**The pattern is generalizable**: every join-point in a multi-step orchestration should produce an artifact. When the join-point fails, the artifact is the recovery vehicle. Without the artifact, the only recovery is to re-run the entire upstream sequence — which for a publish to Marketplace/Open VSX is risky (re-publishing usually requires bumping a version or unpublishing first).

This is now codified as "in-flight backup" in `docs/articles/EVIDENCE_DURABILITY_TIERS.md`.

## The fix (workflow-side)

Two options:

**Option A: introduce an explicit dependency.** `Publish VSCode Extension`'s `Attach VSIX` job should `needs:` the release-creation job, even if it lives in a separate workflow. GitHub Actions supports cross-workflow dependencies via `workflow_run` events but it's complex.

**Option B: make `Attach VSIX` idempotent + retry.** Poll for the release's existence with timeout, then upload with `--clobber` so re-runs are safe. This is the smaller change.

Option B is simpler. Idempotent attach + ~5-minute retry budget would have made this race invisible.

## Why this is "workflow," not "product"

The VSIX itself was correct; the publish to Marketplace and Open VSX both succeeded. The product (extension) was correct. The *coordination between the publish workflow and the release workflow* was wrong. Workflow-class failure per `../reference/FAILURE_CLASSIFICATION.md`.

## Detection signal

After a release dispatch, immediately check the GitHub release for asset count:

```bash
gh release view v$VERSION --repo <owner>/<repo> --json assets --jq '.assets | length'
```

For a complete release this should be 10 (7 binaries + VSIX + SHA256SUMS + sbom). If it's 9, the VSIX is missing — recovery is the manual upload from artifacts above.

## Related

- Forensics: `2026-05-03-validate-release-squash-timing-race.md` (different timing race in the same orchestration)
- Articles: `../articles/RELEASES_FAIL_AT_SEAMS.md` (race-class seam failures), `../articles/EVIDENCE_DURABILITY_TIERS.md` (artifacts as in-flight backup)
- Reference: `../reference/RELEASE_PROOF_PROTOCOL.md` (asset count is now in the proof packet)
