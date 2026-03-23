# Release Guide

This document covers the end-to-end process for releasing a new version of `perl-lsp`.

## Table of Contents

1. [Prerequisites Checklist](#prerequisites-checklist)
2. [Pre-release Verification Steps](#pre-release-verification-steps)
3. [Triggering the Release Workflow](#triggering-the-release-workflow)
4. [Expected Workflow Runtimes](#expected-workflow-runtimes)
5. [Post-release Verification](#post-release-verification)
6. [Rollback Procedures](#rollback-procedures)

---

## Prerequisites Checklist

### Required Secrets

All five secrets must be configured in the repository under **Settings > Secrets and variables > Actions** before triggering a release. Missing secrets cause silent workflow skips or hard failures.

| Secret | Purpose | Where to get it |
|--------|---------|-----------------|
| `CARGO_REGISTRY_TOKEN` | Publish crates to crates.io | https://crates.io/me under "API Tokens" |
| `VSCE_PAT` | Publish to VS Code Marketplace | https://marketplace.visualstudio.com/manage under "Personal Access Tokens" |
| `OVSX_PAT` | Publish to Open VSX Registry | https://open-vsx.org under account settings |
| `DOCKER_USERNAME` | Push to Docker Hub | Your Docker Hub username |
| `DOCKER_PASSWORD` | Push to Docker Hub | A Docker Hub access token (not your account password) |

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

The release workflow validates this automatically, but verify locally first:

```bash
# Check current CI state for the HEAD commit on master
gh run list --branch master --limit 5
```

---

## Pre-release Verification Steps

Run these locally before triggering the release workflow. The validate job in the orchestration workflow checks most of these, but catching failures locally avoids a partial-start release.

### 1. Tests passing

```bash
export CARGO_TARGET_DIR="/tmp/release-preflight-target"

# Library tests (fast, comprehensive)
cargo nextest run --workspace --lib

# Full workspace tests
cargo nextest run --workspace

# Snapshot tests (these must all pass — stale snapshots block release)
cargo test -p perl-lsp --test lsp_capability_snapshot_test
```

If any snapshot test fails and `.snap.new` files exist as untracked, accept them before releasing:

```bash
cargo insta accept
git add crates/perl-lsp/tests/snapshots/
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
grep -A 5 '## \[0\.12\.0\]' CHANGELOG.md
```

The `## [Unreleased]` section must be empty or contain only the section header — no uncommitted changes should appear there.

### 5. No `v<VERSION>` tag already exists

```bash
git fetch --tags
git tag | grep 'v0\.12\.0'
# Should return nothing
```

### 6. Workspace Cargo.toml version matches intended release

```bash
grep '^version' Cargo.toml | head -1
# Expected output: version = "0.12.0"
```

### 7. All publishable crate versions match

```bash
cargo metadata --format-version=1 --no-deps | python3 -c '
import json, sys
meta = json.load(sys.stdin)
ws = set(meta["workspace_members"])
for pkg in meta["packages"]:
    if pkg["id"] in ws:
        if pkg["version"] != "0.12.0":
            print(f"MISMATCH: {pkg[\"name\"]}@{pkg[\"version\"]}")
'
# Should print nothing
```

---

## Triggering the Release Workflow

The release is triggered via a single manual workflow dispatch. The orchestration workflow (`release-orchestration.yml`) validates prerequisites, creates the git tag, and dispatches all downstream workflows.

### Using `gh` CLI (recommended)

```bash
gh workflow run release-orchestration.yml \
  --field version=0.12.0 \
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
   - **Release version**: `0.12.0` (no `v` prefix)
   - **Mark as prerelease**: unchecked for stable releases
   - **Skip crates.io publishing**: leave unchecked
   - **Skip VSCode extension publishing**: leave unchecked
   - **Skip Docker image publishing**: leave unchecked
5. Click **Run workflow**.

### Workflow inputs

| Input | Description | Default |
|-------|-------------|---------|
| `version` | Release version without `v` prefix, e.g. `0.12.0` | required |
| `prerelease` | Mark the GitHub release as a prerelease | `false` |
| `skip_crates` | Skip crates.io publish (for re-runs after partial failure) | `false` |
| `skip_extension` | Skip VS Code Marketplace publish | `false` |
| `skip_docker` | Skip Docker Hub / GHCR publish | `false` |

### Skipping individual stages on re-run

If the release orchestration fails mid-way (e.g., crates.io succeeds but Docker fails), you can re-run with individual stages skipped:

```bash
# Re-run only Docker (crates and extension already published)
gh workflow run release-orchestration.yml \
  --field version=0.12.0 \
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
| `publish-extension.yml` | Orchestration dispatch | ~5–10 min | Builds VSIX, publishes to VS Code Marketplace and Open VSX Registry |
| `docker-publish.yml` | Orchestration dispatch | ~20–30 min | Builds multi-arch images (amd64, arm64) for GHCR and Docker Hub |
| `brew-bump.yml` | GitHub release published event | ~5–10 min | Updates Homebrew formula with new version and checksums |
| `scoop-bump.yml` | GitHub release published event | ~3–5 min | Updates Scoop manifest |
| `chocolatey-bump.yml` | GitHub release published event | ~3–5 min | Updates Chocolatey package |

**Total expected wall time for a full release: ~50–90 minutes.**

The build, crates, extension, and Docker workflows run in parallel after the tag is created.

---

## Post-release Verification

After all workflows complete, verify that each distribution channel received the release.

### 1. GitHub Release

```bash
gh release view v0.12.0
# Should show assets including:
# - perl-lsp-0.12.0-x86_64-unknown-linux-gnu.tar.gz
# - perl-lsp-0.12.0-aarch64-unknown-linux-gnu.tar.gz
# - perl-lsp-0.12.0-x86_64-unknown-linux-musl.tar.gz
# - perl-lsp-0.12.0-aarch64-unknown-linux-musl.tar.gz
# - perl-lsp-0.12.0-x86_64-apple-darwin.tar.gz
# - perl-lsp-0.12.0-aarch64-apple-darwin.tar.gz
# - perl-lsp-0.12.0-x86_64-pc-windows-msvc.zip
# - SHA256SUMS
# - sbom-spdx.json
# - perl-lsp-rs-0.12.0.vsix
```

### 2. crates.io

```bash
cargo search perl-lsp --limit 1
# Expected: perl-lsp = "0.12.0"
```

### 3. VS Code Marketplace

Visit: https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs

Check that the version shown is `0.12.0`.

### 4. Open VSX Registry

Visit: https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs

### 5. Docker images

```bash
docker pull effortlessmetrics/perl-lsp:0.12.0
docker run --rm effortlessmetrics/perl-lsp:0.12.0 perl-lsp --version
# Expected: perl-lsp 0.12.0
```

```bash
docker pull ghcr.io/effortlessmetrics/perl-lsp:0.12.0
```

### 6. Verify binary checksum

```bash
# Download the Linux binary and verify its SHA256 matches the release
gh release download v0.12.0 --pattern 'perl-lsp-0.12.0-x86_64-unknown-linux-gnu.tar.gz' --pattern SHA256SUMS
sha256sum --check SHA256SUMS --ignore-missing
```

### 7. Post-merge metrics update

After the release merges, the corpus metrics auto-regenerate. No manual step is required.

---

## Rollback Procedures

### GitHub Release (safe — can delete and re-publish)

```bash
# Delete the release (keeps the tag)
gh release delete v0.12.0 --yes

# Delete the tag if needed
git push origin :refs/tags/v0.12.0
git tag -d v0.12.0
```

### crates.io (irreversible — yank, do not delete)

Once published to crates.io, a crate version cannot be deleted. Use `cargo yank` to prevent new projects from depending on it:

```bash
# Yank a specific crate version
cargo yank --version 0.12.0 <crate-name>

# Example: yank perl-lsp
cargo yank --version 0.12.0 perl-lsp
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
# Replace 0.12.0 with the bad version
VERSION=0.12.0
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

Versions cannot be deleted from the VS Code Marketplace. Publish a corrected patch release (`0.12.1`) to supersede the bad version. Contact the Marketplace support team only for critical security issues.

### Open VSX Registry

Same as VS Code Marketplace — publish a patch release to supersede.

### Docker Hub

```bash
# Delete a specific tag via Docker Hub API (requires login)
curl -X DELETE \
  "https://hub.docker.com/v2/repositories/effortlessmetrics/perl-lsp/tags/0.12.0/" \
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
     --field tag=v0.12.0 \
     --field prerelease=false
   ```

---

## Preparing the Next Release

After a release ships, trigger the version bump workflow to prepare the next development cycle:

```bash
# Bump to next minor version (e.g., 0.12.0 -> 0.13.0)
gh workflow run version-bump.yml \
  --field bump_type=minor

# Or specify an exact version
gh workflow run version-bump.yml \
  --field version=0.13.0
```

This creates a `release/v0.13.0` branch with updated `Cargo.toml` and `CHANGELOG.md`, then opens a PR for review.
