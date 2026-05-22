# Release Closeout Audit

> Re-runnable distribution-channel verification checklist. Run after every
> tag is cut. Per-release populations live alongside the release notes
> (e.g. `0.14.0-closeout-audit.md`).

The 0.14.0 closeout left users on 0.13.3 because some channels never
published. This doc is the guardrail that prevents the same gap from
recurring in future releases.

## Why the gap exists

Different channels use different trigger mechanisms. Some fire
automatically on `release:published`; others require explicit
`workflow_dispatch`. If you only tag and assume "the workflow
orchestrator handles the rest," the dispatch-only channels silently stay
on the prior version.

| Channel | Trigger | Auto on tag? |
|---|---|---|
| GitHub Release (binaries) | `release.yml` on tag push | Yes |
| crates.io | `publish-crates.yml` from `release.yml` | Yes (with new-crate burst cap) |
| Homebrew tap | `brew-bump.yml` on `release:published` | Yes (auto-fires; tap-repo PR still needs merge) |
| Scoop bucket | `scoop-bump.yml` on `release:published` | Yes (auto-fires; bucket-repo PR still needs merge) |
| Chocolatey | `chocolatey-bump.yml` on `release:published` | Yes (auto-fires; package submission may queue) |
| VS Code Marketplace | `publish-extension.yml` | **No - `workflow_dispatch` only** |
| Open VSX | `publish-extension.yml` | **No - `workflow_dispatch` only** |
| Docker (Hub + GHCR) | `docker-publish.yml` | **No - `workflow_dispatch` only** |

**Three channels (Docker, VS Code Marketplace, Open VSX) require manual
dispatch.** They are the most common 0.14.0-style "still pending"
channels. Brew/Scoop/Chocolatey auto-fire but each opens a downstream
package-repo PR that must merge before users see the bump.

## Per-channel verification

Run from the publishing repo (`EffortlessMetrics/perl-lsp`) checkout.
Replace `vX.Y.Z` with the actual tag.

### 1. GitHub Release

```bash
gh release view vX.Y.Z --json name,isDraft,isPrerelease,publishedAt,assets \
  | jq '{name, isDraft, isPrerelease, publishedAt, asset_count: (.assets | length)}'
```

Expected: `isDraft=false`, asset count matches the platform matrix
(typically 5 platforms x {tarball, sha256, sig} = ~15).

If draft: `gh release edit vX.Y.Z --draft=false` (this is the trigger
that lets `release:published` fire downstream).

### 2. crates.io

Primary packages (update list if workspace top-level binaries change):

```bash
for crate in perllsp perl-lsp-rs perl-parser perl-dap; do
  echo -n "$crate: "
  cargo search "$crate" --limit 1 | head -1
done
```

Full inventory (all crates listed in `[workspace.metadata.publish.allow]`):

```bash
cargo metadata --format-version=1 --no-deps \
  | jq -r '.metadata.publish.allow[]' \
  | while read crate; do
      printf "%-40s " "$crate"
      cargo search "$crate" --limit 1 | head -1 || echo "NOT FOUND"
    done
```

If any are stuck below `X.Y.Z`: a common cause is the new-crate burst
rate limit (crates.io: burst=5, refill 1/10min). Remediation:
`just publish-new-crates` per `docs/reference/MANUAL_PUBLISH_NEW_CRATES.md`.

### 3. VS Code Marketplace

```bash
# Listing version (BSD grep portable; macOS/Linux both work):
curl -s "https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs" \
  | grep -oE 'data-version="[^"]+"' | head -1 | sed -E 's/.*"([^"]+)"/\1/'
```

Or check programmatically via the marketplace gallery API. If not at
`X.Y.Z`:

```bash
gh workflow run publish-extension.yml -f version=X.Y.Z
```

Then watch the run; this requires `VSCE_PAT` to be set.

### 4. Open VSX

```bash
curl -s "https://open-vsx.org/api/EffortlessMetrics/perl-lsp-rs" \
  | jq '.version'
```

Open VSX is bundled into the same `publish-extension.yml` workflow as
the VS Code Marketplace, but uses `OVSX_PAT` instead. Same dispatch:

```bash
gh workflow run publish-extension.yml -f version=X.Y.Z
```

### 5. Docker

```bash
docker pull effortlessmetrics/perl-lsp:X.Y.Z
docker pull ghcr.io/effortlessmetrics/perl-lsp:X.Y.Z
```

Both should resolve. If either fails with "manifest unknown":

```bash
gh workflow run docker-publish.yml -f version=X.Y.Z
```

Requires `DOCKER_USERNAME` and `DOCKER_PASSWORD` for Hub; GHCR
authenticates via `GITHUB_TOKEN`.

### 6. Homebrew tap

```bash
brew update
brew info --json perllsp | jq '.[0].versions.stable'
```

If still on prior version: `brew-bump.yml` auto-fires on `release:published`
and opens a PR against `EffortlessMetrics/homebrew-perllsp`. Check that
PR was merged:

```bash
gh pr list -R EffortlessMetrics/homebrew-perllsp --state all --limit 5
```

If the PR is open: review and merge it. If the PR doesn't exist:
re-dispatch with `gh workflow run brew-bump.yml -f tag=vX.Y.Z`.

### 7. Scoop bucket

Same pattern as Homebrew:

```bash
gh pr list -R EffortlessMetrics/scoop-perllsp --state all --limit 5
```

Re-dispatch: `gh workflow run scoop-bump.yml -f tag=vX.Y.Z`.

### 8. Chocolatey

```bash
choco search perllsp --exact
```

Chocolatey moderation can queue submissions for hours/days. If
`choco search` returns a stale version, distinguish between "workflow
never fired" and "submitted but queued in moderation":

```bash
# Did the bump workflow run at all for this version?
gh run list --workflow=chocolatey-bump.yml --limit 10 \
  | grep -E "vX\.Y\.Z|completed"
```

If the workflow never ran, dispatch it:

```bash
gh workflow run chocolatey-bump.yml -f tag=vX.Y.Z
```

If the workflow ran successfully but `choco search` is still stale, the
submission is in moderation - nothing to do but wait.

### 9. End-to-end smoke

After the channels above resolve, confirm a fresh install on each
platform works:

```bash
# crates.io install
cargo install perllsp --version X.Y.Z --force
perllsp --version  # -> X.Y.Z

# Homebrew
brew upgrade perllsp
perllsp --version  # -> X.Y.Z

# Docker
docker run --rm effortlessmetrics/perl-lsp:X.Y.Z --version  # -> X.Y.Z
```

For LSP4IJ specifically (the JetBrains plugin that hit the 0.14.x
crash): users typically install `perllsp` via `cargo install`,
Homebrew, or download the GitHub Release binary. Any of those three
must serve the new version for them to actually receive the fix.

## Updating per-release populated audits

After running the checks above for a specific release, create:

```
docs/releases/{X.Y.Z}-closeout-audit.md
```

Populate it from this template with the actual results, then update the
`channels` frontmatter block in the corresponding `vX.Y.Z.md` release
notes file so `notes_status` can flip from `pending` to `closed`.

## Hard rules

- Do not mark a release `notes_status: closed` until every channel above
  resolves to `X.Y.Z` or is documented as deliberately skipped.
- Do not assume `release:published` covers Docker, VS Code Marketplace,
  or Open VSX. Those are dispatch-only.
- A Homebrew/Scoop/Chocolatey auto-bump that opens a tap-repo PR is not
  the same as a user-facing publish. The PR must merge.

## Related

- 0.14.0 release notes (template): [`v0.14.0.md`](v0.14.0.md)
- Manual new-crate publish: [`../reference/MANUAL_PUBLISH_NEW_CRATES.md`](../reference/MANUAL_PUBLISH_NEW_CRATES.md)
- Release process: [`../RELEASE_PROCESS.md`](../RELEASE_PROCESS.md)
- Release checklist: [`../project/RELEASE_CHECKLIST.md`](../project/RELEASE_CHECKLIST.md)
