# Release Guide

This is the operational release runbook for `perl-lsp`.
Use it with [docs/project/RELEASE_CHECKLIST.md](docs/project/RELEASE_CHECKLIST.md)
for the preflight gate and [docs/project/PUBLISHING_ROADMAP.md](docs/project/PUBLISHING_ROADMAP.md)
for the release-day sequence and post-release verification.

## Table of Contents

1. [Prerequisites Checklist](#prerequisites-checklist)
2. [Pre-release Verification Steps](#pre-release-verification-steps)
3. [Triggering the Release Workflow](#triggering-the-release-workflow)
4. [Expected Workflow Runtimes](#expected-workflow-runtimes)
5. [Post-release Verification](#post-release-verification)
6. [Rollback Procedures](#rollback-procedures)

---

## Prerequisites Checklist

Use this section to confirm the release inputs are ready before dispatching the workflow.

### Required Secrets

All release-channel secrets must be configured in the repository under
**Settings > Secrets and variables > Actions** before triggering a release.
Missing secrets cause channel skips or hard failures.

| Secret | Purpose | Where to get it |
|--------|---------|-----------------|
| `CARGO_REGISTRY_TOKEN` | Publish crates to crates.io | https://crates.io/me under "API Tokens" |
| `VSCE_PAT` | Publish to VS Code Marketplace | https://marketplace.visualstudio.com/manage under "Personal Access Tokens" |
| `OVSX_PAT` | Publish to Open VSX Registry | https://open-vsx.org under account settings |
| `DOCKER_USERNAME` | Push to Docker Hub | Your Docker Hub username |
| `DOCKER_PASSWORD` | Push to Docker Hub | A Docker Hub access token (not your account password) |
| `HOMEBREW_TAP_TOKEN` | Open bump PRs in `EffortlessMetrics/homebrew-tap` | Fine-grained PAT with contents read/write, pull requests read/write, metadata read |

`GITHUB_TOKEN` is provided automatically — no action needed.

To verify secrets exist (does not reveal values):

```bash
gh secret list
```

### Version Consistency

The version in `Cargo.toml` (workspace root), `CHANGELOG.md`, and the VSCode extension `vscode-extension/package.json` must all match the intended release version.

```bash
# Check workspace version
grep '^version' Cargo.toml | head -1

# Check VSCode extension version
node -p "require('./vscode-extension/package.json').version"

# Check CHANGELOG has an entry for the release version (not just Unreleased)
grep '## \[' CHANGELOG.md | head -5
```

### CI Green on master

Verify the latest `master` run is green before dispatching the release workflow:

```bash
# Check current CI state for the HEAD commit on master
gh run list --branch master --limit 5
```

---

## Pre-release Verification Steps

Run these locally before triggering the release workflow. The validate job in the orchestration workflow checks most of these, but catching failures locally avoids a partial-start release.

Set the intended version once and reuse it in the commands below:

```bash
export VERSION=X.Y.Z
export TAG="v${VERSION}"
```

### 1. Tests passing

```bash
export CARGO_TARGET_DIR="/tmp/release-preflight-target"

# Library tests (fast, comprehensive)
cargo nextest run --workspace --lib

# Full workspace tests
cargo nextest run --workspace

# Snapshot tests (these must all pass — stale snapshots block release)
cargo test -p perl-lsp --test lsp_cap_snap
```

If any snapshot test fails and `.snap.new` files exist as untracked, accept them before releasing:

```bash
cargo insta accept
git add crates/perl-lsp-rs/tests/snapshots/
git commit -m "test: accept updated insta snapshots"
```

### 2. Clippy clean

```bash
cargo clippy --workspace --lib
```

### 3. No stale untracked `.snap.new` files

```bash
git status | grep '\.snap\.new'
# Should return nothing
```

### 4. CHANGELOG has a complete entry for the release version

```bash
# The release version section must exist and contain content
grep -F -A 5 "## [$VERSION]" CHANGELOG.md
```

The `## [Unreleased]` section must be empty or contain only the section header — no uncommitted changes should appear there.

### 5. No `v<VERSION>` tag already exists

```bash
git fetch --tags
! git tag | grep -q "^${TAG}$"
# Should exit successfully
```

### 6. Workspace Cargo.toml version matches intended release

```bash
grep '^version' Cargo.toml | head -1
# Expected output: version = "X.Y.Z"
```

### 7. All publishable crate versions match

```bash
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, os, sys
target = os.environ["VERSION"]
meta = json.load(sys.stdin)
ws = set(meta["workspace_members"])
for pkg in meta["packages"]:
    if pkg["id"] in ws:
        if pkg["version"] != target:
            print(f"MISMATCH: {pkg[\"name\"]}@{pkg[\"version\"]}")
'
# Should print nothing
```

### 8. Install surface and release-note checks

These checks guard the user-facing install paths and GitHub Release asset
chooser that downstream users see first.

```bash
cargo xtask install-surface-check
bash scripts/check_release_history.sh

cargo xtask release-notes --tag "$TAG" --output "/tmp/${TAG}-body.md"
grep -q 'Which file should I download?' "/tmp/${TAG}-body.md"
grep -q 'x86_64-unknown-linux-gnu' "/tmp/${TAG}-body.md"
grep -q 'x86_64-unknown-linux-musl' "/tmp/${TAG}-body.md"
```

If Docker is available, also run the installer target-selection self-test:

```bash
bash scripts/tests/test-install-target-selection.sh
```

### 9. Release-artifact surface check (DAP binary contract)

The release workflow builds and packages both `perllsp` and `perl-dap` into
every archive, and downstream integrations (VS Code / Open VSX, LSP4IJ) depend
on `perl-dap` being present. The release workflow gates on this automatically
(see `.github/workflows/release.yml`), but you can verify a locally-built or
downloaded set of archives the same way. Point `--dist` at a directory holding
the `perllsp-<version>-<triple>.{tar.gz,zip}` archives plus the consolidated
`SHA256SUMS`:

```bash
cargo xtask release artifact-check --dist dist --version "$VERSION"
```

The check enforces the contract in
[`docs/reference/downstream-dap-integrations.json`](docs/reference/downstream-dap-integrations.json):
each archive contains the required LSP and DAP binaries, Unix binaries carry the
executable bit, every contract target triple is present, and every archive is
listed (with a matching digest) in the consolidated `SHA256SUMS`. Pass
`--allow-partial` when validating an intentionally incomplete set.

---

## Triggering the Release Workflow

The release is triggered via a single manual workflow dispatch. The orchestration workflow (`release-orchestration.yml`) validates prerequisites, creates the git tag, and dispatches all downstream workflows.

### Using `gh` CLI (recommended)

```bash
gh workflow run release-orchestration.yml \
  --field version="$VERSION" \
  --field prerelease=false \
  --field skip_crates=false \
  --field skip_extension=false \
  --field skip_docker=false
```

### Using the GitHub UI

1. Navigate to the repository **Actions** tab.
2. Select **Release Orchestration** from the left sidebar.
3. Click **Run workflow**.
4. Fill in the fields:
   - **Release version**: `X.Y.Z` (no `v` prefix)
   - **Mark as prerelease**: check only when GitHub should display the release as a prerelease
   - **Skip crates.io publishing**: leave unchecked
   - **Skip VSCode extension publishing**: leave unchecked
   - **Skip Docker image publishing**: leave unchecked
5. Click **Run workflow**.

### Workflow inputs

| Input | Description | Default |
|-------|-------------|---------|
| `version` | Release version without `v` prefix, e.g. `X.Y.Z` | required |
| `prerelease` | Mark the GitHub release as a prerelease | `false` |
| `skip_crates` | Skip crates.io publish (for re-runs after partial failure) | `false` |
| `skip_extension` | Skip VS Code Marketplace publish | `false` |
| `skip_docker` | Skip Docker Hub / GHCR publish | `false` |

### Skipping individual stages on re-run

If the release orchestration fails mid-way (e.g., crates.io succeeds but Docker fails), you can re-run with individual stages skipped:

```bash
# Re-run only Docker (crates and extension already published)
gh workflow run release-orchestration.yml \
  --field version="$VERSION" \
  --field skip_crates=true \
  --field skip_extension=true \
  --field skip_docker=false
```

---

## Expected Workflow Runtimes

| Workflow | Triggered by | Expected runtime | What it does |
|----------|-------------|-----------------|--------------|
| `release-orchestration.yml` (validate + tag) | Manual dispatch | ~5–15 min | Version/CI validation, creates annotated git tag, dispatches downstream workflows |
| `release.yml` (build + GitHub release) | Orchestration dispatch | ~25–40 min | Builds binaries for 7 platforms (4 Linux, 2 macOS, 1 Windows), creates GitHub release with SHA256SUMS and SBOM |
| `publish-crates.yml` | Orchestration dispatch | ~60–90 min | Publishes all crates to crates.io in topological dependency order with 3-attempt retry and index wait per crate |
| `publish-extension.yml` | Orchestration dispatch | ~10–30 min | Builds VSIX, publishes to VS Code Marketplace and Open VSX Registry, then runs published-package smokes |
| `docker-publish.yml` | Orchestration dispatch | ~20–30 min | Builds multi-arch images (amd64, arm64) for GHCR and Docker Hub |
| `brew-bump.yml` | GitHub release published event | ~5–10 min | Updates `EffortlessMetrics/homebrew-tap` formula with new version and checksums |
| `scoop-bump.yml` | GitHub release published event | ~3–5 min | Updates Scoop manifest |
| `chocolatey-bump.yml` | GitHub release published event | ~3–5 min | Updates Chocolatey package |
| `winget-bump.yml` | GitHub release published event | ~3–5 min | Refreshes the repo-local winget manifest |

**Total expected wall time for a full release: ~50–90 minutes.**

The build, crates, extension, and Docker workflows run in parallel after the tag is created.

---

## Post-release Verification

After all workflows complete, verify that each distribution channel received the release.

### 1. GitHub Release

```bash
gh release view "$TAG"
# Should show assets including:
# - perllsp-X.Y.Z-x86_64-unknown-linux-gnu.tar.gz
# - perllsp-X.Y.Z-aarch64-unknown-linux-gnu.tar.gz
# - perllsp-X.Y.Z-x86_64-unknown-linux-musl.tar.gz
# - perllsp-X.Y.Z-aarch64-unknown-linux-musl.tar.gz
# - perllsp-X.Y.Z-x86_64-apple-darwin.tar.gz
# - perllsp-X.Y.Z-aarch64-apple-darwin.tar.gz
# - perllsp-X.Y.Z-x86_64-pc-windows-msvc.zip
# - SHA256SUMS
# - sbom-spdx.json
# - perl-lsp-rs-X.Y.Z.vsix
```

### 2. crates.io

```bash
cargo search perllsp --limit 1
# Expected: perllsp = "X.Y.Z"

cargo search perl-lsp-rs --limit 1
# Expected: perl-lsp-rs = "X.Y.Z"
```

### 3. VS Code Marketplace

Visit: https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs

Check that the version shown is `X.Y.Z`.

Then run the published Marketplace package smoke:

```bash
gh workflow run vscode-published-extension-smoke.yml \
  --repo EffortlessMetrics/perl-lsp \
  --field version="$VERSION" \
  --field source=marketplace
```

### 4. Open VSX Registry

Visit: https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs

Check that the version shown is `X.Y.Z`.

Then run the published Open VSX package smoke:

```bash
gh workflow run vscode-published-extension-smoke.yml \
  --repo EffortlessMetrics/perl-lsp \
  --field version="$VERSION" \
  --field source=open-vsx
```

### 5. Docker images

```bash
docker pull "effortlessmetrics/perl-lsp:${VERSION}"
docker run --rm "effortlessmetrics/perl-lsp:${VERSION}" perllsp --version
# Expected: perllsp X.Y.Z
```

```bash
docker pull "ghcr.io/effortlessmetrics/perl-lsp:${VERSION}"
```

### 6. Homebrew auto-bump

The `brew-bump.yml` workflow triggers automatically on `release.published`. It downloads all four Homebrew platform archives (`perllsp-${VERSION}-{x86_64,aarch64}-{apple-darwin,unknown-linux-gnu}.tar.gz`), validates them against `SHA256SUMS`, generates `Formula/perllsp.rb` through `cargo xtask update-homebrew`, and creates a bump PR in `EffortlessMetrics/homebrew-tap`.

To verify the workflow ran:

```bash
# Check that the workflow completed successfully
gh run list --workflow brew-bump.yml --limit 5

# Check the bump PR was created
gh pr list --repo EffortlessMetrics/homebrew-tap --search "perllsp" --state open
```

If the workflow did not trigger automatically, run it manually:

```bash
gh workflow run brew-bump.yml \
  --repo EffortlessMetrics/perl-lsp \
  --field tag="$TAG" \
  --field include_prerelease=true
```

After the tap PR merges, test the public user command (requires macOS or Linuxbrew):

```bash
brew update
brew install effortlessmetrics/tap/perllsp || brew upgrade perllsp
perllsp --version
perllsp --health
perl-dap --version
brew test effortlessmetrics/tap/perllsp
```

### 7. Verify binary checksum

```bash
# Download the Linux binary and verify its SHA256 matches the release
gh release download "$TAG" --pattern "perllsp-${VERSION}-x86_64-unknown-linux-gnu.tar.gz" --pattern SHA256SUMS
sha256sum --check SHA256SUMS --ignore-missing
```

### 8. Release install-surface receipt

Capture one short receipt after the channel checks pass. This gives the release
an auditable install-surface summary without requiring future reviewers to
reconstruct the result from individual workflow runs.

```bash
mkdir -p target/receipts
cat > "target/receipts/release-install-surface-${TAG}.md" <<EOF
# Release install surface ${TAG}

- Release notes chooser: pass
- GitHub release asset layout: pass
- Homebrew bump: pass
- Public tap smoke: pass
- VS Code source smoke: pass
- VS Code Marketplace published smoke: pass
- Open VSX published smoke: pass
- Installer target-selection: pass
- Install-surface check: pass
EOF
```

### 9. Post-merge metrics update

After the release merges, the corpus metrics auto-regenerate. No manual step is required.

---

## Rollback Procedures

### GitHub Release (safe — can delete and re-publish)

```bash
# Delete the release (keeps the tag)
gh release delete "$TAG" --yes

# Delete the tag if needed
git push origin ":refs/tags/${TAG}"
git tag -d "$TAG"
```

### crates.io (irreversible — yank, do not delete)

Once published to crates.io, a crate version cannot be deleted. Use `cargo yank` to prevent new projects from depending on it:

```bash
# Yank a specific crate version
cargo yank --version "$VERSION" <crate-name>

# Example: yank the public facade crate
cargo yank --version "$VERSION" perllsp

# Example: yank the implementation crate
cargo yank --version "$VERSION" perl-lsp-rs
```

The crates are published in topological order. If the workflow fails mid-way, earlier crates in the publish order are already live. Yank each published crate individually. The publish order is computed by `publish-crates.yml` from `cargo metadata`; run this to see the order:

```bash
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
allow = meta.get("metadata", {}).get("publish", {}).get("allow", [])
print("\n".join(allow))
'
```

To yank all at once after a botched release:

```bash
# Replace X.Y.Z with the bad version
VERSION=X.Y.Z
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
for name in meta.get("metadata", {}).get("publish", {}).get("allow", []):
    print(name)
' | while read crate; do
  cargo yank --version "$VERSION" "$crate" || true
done
```

### VS Code Marketplace

Versions cannot be deleted from the VS Code Marketplace. Publish a corrected patch release to supersede the bad version. Contact the Marketplace support team only for critical security issues.

### Open VSX Registry

Same as VS Code Marketplace — publish a patch release to supersede.

### Docker Hub

```bash
# Delete a specific tag via Docker Hub API (requires login)
curl -X DELETE \
  "https://hub.docker.com/v2/repositories/effortlessmetrics/perl-lsp/tags/${VERSION}/" \
  -H "Authorization: Bearer <token>"
```

For GHCR (GitHub Container Registry), delete the package version from the repository's **Packages** tab in the GitHub UI, or via:

```bash
gh api --method DELETE /orgs/EffortlessMetrics/packages/container/perl-lsp/versions/<version-id>
```

### Recovering from a mid-release failure

If `release-orchestration.yml` fails after the tag is created but before all downstream workflows finish:

1. Check which workflows completed successfully in the Actions tab.
2. Re-trigger `release-orchestration.yml` with `skip_*` flags for the stages that already succeeded.
3. If the tag was pushed but the GitHub release was not created, run `release.yml` directly:
   ```bash
   gh workflow run release.yml \
     --field tag="$TAG" \
     --field prerelease=false
   ```

---

## Preparing the Next Release

After a release ships, trigger the version bump workflow to prepare the next development cycle:

```bash
# Bump to the next minor version.
gh workflow run version-bump.yml \
  --field bump_type=minor

# Or specify an exact version
gh workflow run version-bump.yml \
  --field version=NEW_VERSION
```

This creates a `release/vNEW_VERSION` branch with updated `Cargo.toml` and `CHANGELOG.md`, then opens a PR for review.

---

## Release History Updates

Every public release **must** update three surfaces. See [RELEASE_HISTORY.md](RELEASE_HISTORY.md) for the canonical ledger.

### Before publishing

- [ ] Create `docs/releases/vX.Y.Z.md` (use an existing file as template).
  **Required** — `release-orchestration.yml` refuses to tag without this file,
  and `release.yml` uses its body (minus YAML frontmatter) as the GitHub
  Release body.
- [ ] If the release ships Linux GNU and musl binaries, include a
  `Which file should I download?` section or link to
  `docs/how-to/INSTALLATION.md`. The GitHub asset list exposes raw target
  triples; release notes must explain that most Linux users choose `gnu`, while
  `musl` is mainly for Alpine Linux and musl-based containers.
- [ ] Add a new row to `RELEASE_HISTORY.md` for vX.Y.Z (fill what you know; mark unknowns with `—`)
- [ ] Ensure `CHANGELOG.md` has a `[X.Y.Z]` section with links to:
  - `docs/releases/vX.Y.Z.md`
  - GitHub Release URL
  - Compare range `vPrev...vX.Y.Z`
- [ ] Preview the extracted release body locally before dispatching the workflow:
  ```bash
  cargo xtask release-notes --tag vX.Y.Z
  ```
  What the command prints is exactly what will become the GitHub Release
  body. GitHub then appends its auto-generated "What's Changed" PR list
  below that body (via `generate_release_notes: true` in `release.yml`).

### After publishing

- [ ] Capture GitHub Release metadata:
  ```bash
  gh release view vX.Y.Z --json tagName,publishedAt,url,body,assets,targetCommitish
  ```
- [ ] Update `docs/releases/vX.Y.Z.md` front-matter with:
  - `release_date_utc` (from `publishedAt`)
  - `tag_commit` (resolve tag to commit SHA)
  - Actual asset list
- [ ] Update `RELEASE_HISTORY.md` row with:
  - Asset count and summary
  - Channel outcomes (crates.io, VS Code Marketplace, Open VSX, Docker, Homebrew tap)

### Release PR template

Use [`.github/pull_request_template_release.md`](.github/pull_request_template_release.md) for release-only PRs so the release-history surface checks are visible during review.

### Verification

```bash
# Release-history surface consistency check
# 1. Notes file exists
test -f docs/releases/vX.Y.Z.md

# 2. Ledger row exists
grep 'X.Y.Z' RELEASE_HISTORY.md

# 3. CHANGELOG section exists
grep '\[X.Y.Z\]' CHANGELOG.md

# 4. Curated body parses cleanly and includes install chooser guidance
cargo xtask release-notes --tag vX.Y.Z --output /tmp/body.md && test -s /tmp/body.md
grep -q 'Which file should I download?' /tmp/body.md
grep -q 'x86_64-unknown-linux-gnu' /tmp/body.md
grep -q 'x86_64-unknown-linux-musl' /tmp/body.md

# 5. Install surface guard passes
cargo xtask install-surface-check

# 6. GitHub Release matches
gh release view vX.Y.Z --json tagName,assets --jq '{tag: .tagName, assets: [.assets[].name]}'

# 7. GitHub Release body starts with the curated content (not a PR dump)
gh release view vX.Y.Z --json body --jq '.body' | head -1   # should be "# vX.Y.Z"
```

### Release PR template

Use [`.github/pull_request_template_release.md`](.github/pull_request_template_release.md)
when opening release-prep PRs so release-history surfaces are consistently
updated and reviewed.
