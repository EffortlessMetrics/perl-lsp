# Release Process Documentation

This document describes the complete release process for perl-lsp, including automated workflows, distribution channels, and rollback procedures.

## Table of Contents

- [Overview](#overview)
- [Prerequisites](#prerequisites)
- [Release Workflow](#release-workflow)
- [Distribution Channels](#distribution-channels)
- [Rollback Procedures](#rollback-procedures)
- [Troubleshooting](#troubleshooting)
- [Release Checklist](#release-checklist)

## Overview

The perl-lsp release process is fully automated and supports:

- **Multi-platform binaries**: Linux (x86_64, aarch64), macOS (x86_64, aarch64), Windows (x86_64)
- **Package managers**: Homebrew, Scoop, Chocolatey
- **Docker images**: Multi-arch (linux/amd64, linux/arm64)
- **VSCode extension**: VSCode Marketplace and Open VSX
- **crates.io**: Crates in `[workspace.metadata.publish.allow]`

### Release Architecture

```
┌─────────────────────────────────────────────────────────────────┐
│                     Release Orchestration                        │
│                   (release-orchestration.yml)                    │
└─────────────────────────────────────────────────────────────────┘
                              │
        ┌─────────────────────┼─────────────────────┐
        │                     │                     │
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   Release    │    │  Publish to  │    │  Package     │
│   Workflow   │    │  crates.io   │    │  Manager     │
│              │    │              │    │  Updates     │
└──────────────┘    └──────────────┘    └──────────────┘
        │                     │                     │
        ▼                     ▼                     ▼
┌──────────────┐    ┌──────────────┐    ┌──────────────┐
│   GitHub     │    │   VSCode     │    │   Docker     │
│   Release    │    │  Extension   │    │   Images     │
│              │    │              │    │              │
└──────────────┘    └──────────────┘    └──────────────┘
```

## Prerequisites

### Required Secrets

Configure the following secrets in GitHub repository settings:

| Secret | Description | Required For |
|--------|-------------|--------------|
| `CARGO_REGISTRY_TOKEN` | crates.io API token | Publishing to crates.io |
| `VSCE_PAT` | VSCode Marketplace PAT | Publishing VSCode extension |
| `OVSX_PAT` | Open VSX PAT | Publishing to Open VSX |
| `DOCKER_USERNAME` | Docker Hub username | Publishing Docker images |
| `DOCKER_PASSWORD` | Docker Hub password | Publishing Docker images |

### Required Permissions

The following GitHub permissions must be granted to workflows:

- `contents: write` - Create releases and tags
- `id-token: write` - SLSA provenance
- `attestations: write` - Build attestations
- `packages: write` - Publish to GitHub Container Registry

## Release Workflow

### Recommended Turnkey PR Flow

The preferred flow is fully PR-driven and starts from the repository default branch (`master`), using `gh` to dispatch workflows and merge the version bump PR:

```bash
# from a clean working tree aligned with origin/master
git fetch origin master
git checkout master
git reset --hard origin/master

# optional but recommended: use the turnkey orchestrator
cargo xtask release-turnkey <0.x.y>
```

Or run the two workflow dispatches manually with `gh`:

```bash
# 1) Generate bump PR and changelog
gh workflow run "Version Bump & Changelog Generation" \
  --ref master \
  --field version=<0.x.y>

# 2) Merge the release/v<0.x.y> PR from release-bot UI or API

# 3) Dispatch release orchestration
gh workflow run "Release Orchestration" \
  --ref master \
  --field version=<0.x.y> \
  --field prerelease=false \
  --field skip_crates=false \
  --field skip_extension=false \
  --field skip_docker=false
```

### Step 1: Version Bump and Changelog Generation

For manual control, trigger the version bump workflow to prepare for release:

```bash
# Via GitHub UI
1. Go to Actions tab
2. Select "Version Bump & Changelog Generation"
3. Click "Run workflow"
4. Either enter a version (e.g., `0.x.y`) or rely on
   bump type (major/minor/patch) to auto-increment the current workspace version.
5. Select bump type only if you are not setting an explicit version.
6. Click "Run workflow"
```

This will:
- Bump the workspace version in `Cargo.toml`
- Generate changelog using git-cliff
- Create a pull request with the changes

### Step 2: Review and Merge Version Bump PR

Review the version bump PR:

1. Check that the version is correct
2. Review the changelog for completeness
3. Verify all breaking changes are documented
4. Merge the PR

### Step 3: Trigger Release Orchestration

After merging the version bump PR, trigger the release orchestration:

```bash
# Via GitHub UI
1. Go to Actions tab
2. Select "Release Orchestration"
3. Click "Run workflow"
4. Enter version (e.g., <0.x.y>)
5. Configure options:
   - prerelease: Mark as prerelease (default: false)
   - skip_crates: Skip crates.io publishing (default: false)
   - skip_extension: Skip VSCode extension (default: false)
   - skip_docker: Skip Docker images (default: false)
6. Click "Run workflow"
```

This will:
- Validate the release
- Create and push the git tag
- Trigger all release workflows

### Step 4: Monitor Release Progress

Monitor the following workflows:

1. **Release** - Builds binaries and creates GitHub release
2. **Publish to crates.io** - Publishes crates in `[workspace.metadata.publish.allow]`
3. **Publish VSCode Extension** - Publishes to VSCode Marketplace and Open VSX
4. **Publish Docker Images** - Builds and pushes multi-arch images
5. **Homebrew Auto-Bump** - Creates PR to Homebrew
6. **Scoop Auto-Bump** - Creates PR to Scoop
7. **Chocolatey Auto-Bump** - Creates PR to Chocolatey
8. **Winget Manifest Refresh** - Refreshes the repo-local winget manifest

### Step 5: Verify Release

After all workflows complete, verify:

1. **GitHub Release**
   - Check that all binaries are uploaded
   - Verify release notes are correct
   - Download and test a binary

2. **crates.io**
   - Verify all crates are published
   - Check that versions match release version
   - Test `cargo install perllsp`

3. **VSCode Extension**
   - Check VSCode Marketplace for new version
   - Check Open VSX for new version
   - Test extension installation

4. **Docker Images**
   - Verify images are pushed to ghcr.io
   - Verify images are pushed to Docker Hub
   - Test `docker run effortlessmetrics/perl-lsp`

5. **Package Managers**
   - Monitor Homebrew PR status
   - Monitor Scoop PR status
   - Monitor Chocolatey PR status
   - Review the winget manifest refresh PR and submit the upstream `winget-pkgs` PR manually

## Distribution Channels

### crates.io

The publish workflow computes dependency order from workspace metadata and publishes crates listed in `[workspace.metadata.publish.allow]`.
The exact crate list and count are printed in the `Compute publish order` step of the `publish-crates` workflow.


**Installation:**
```bash
cargo install perllsp
```

### GitHub Releases

Binaries are published for all platforms:

| Platform | Target | Format |
|----------|--------|--------|
| Linux x86_64 (GNU) | x86_64-unknown-linux-gnu | tar.gz |
| Linux aarch64 (GNU) | aarch64-unknown-linux-gnu | tar.gz |
| Linux x86_64 (musl) | x86_64-unknown-linux-musl | tar.gz |
| Linux aarch64 (musl) | aarch64-unknown-linux-musl | tar.gz |
| macOS x86_64 | x86_64-apple-darwin | tar.gz |
| macOS aarch64 | aarch64-apple-darwin | tar.gz |
| Windows x86_64 | x86_64-pc-windows-msvc | zip |

**Installation:**
```bash
# Download and extract
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v<0.x.y>/perllsp-<0.x.y>-x86_64-unknown-linux-gnu.tar.gz
tar xzf perllsp-<0.x.y>-x86_64-unknown-linux-gnu.tar.gz
sudo cp perllsp-<0.x.y>-x86_64-unknown-linux-gnu/perllsp /usr/local/bin/
```

### Homebrew

Homebrew formula is automatically updated via PR to Homebrew/homebrew-core.

**Installation:**
```bash
brew install perl-lsp
```

### Scoop

Scoop bucket is automatically updated via PR to ScoopInstaller/Main.

**Installation:**
```bash
scoop bucket add extras
scoop install perl-lsp
```

### Chocolatey

Chocolatey package is automatically updated via PR to chocolatey-community/chocolatey-coreteampackages.

**Installation:**
```powershell
choco install perl-lsp
```

### Winget

Winget uses a repo-local manifest source under `distribution/winget/`.
The release workflow refreshes that manifest from the same Windows release asset
and checksum data used by Scoop and Chocolatey. Submitting the manifest to
`winget-pkgs` remains a manual follow-up until that external approval flow is in scope.

**Installation from a local manifest:**
```powershell
winget install --manifest .\distribution\winget\perl-lsp.yaml
```

### Windows Package-Manager Verification

The repo-owned verification story is intentionally narrower than the user-facing
install story:

- Automated: the release workflow publishes the Windows zip and
  `SHA256SUMS`; the Scoop and Chocolatey bump workflows download that asset,
  recompute the checksum, and rewrite the repo-owned manifests through
  `distribution/windows/update-manifests.ps1`.
- Guard rails: both bump workflows fail if release placeholders remain in the
  manifests after the update step.
- Manual: upstream PR acceptance in the Scoop and Chocolatey package repos,
  then a real Windows install check with `scoop install perl-lsp` or
  `choco install perl-lsp`, followed by `perllsp --health` and editor/PATH
  discovery.

Run `powershell -NoLogo -NoProfile -File scripts/check-windows-distribution.ps1`
to audit the repo-side claims above.

### Docker

Multi-arch Docker images are published to:

- GitHub Container Registry: `ghcr.io/EffortlessMetrics/perl-lsp`
- Docker Hub: `effortlessmetrics/perl-lsp`

**Installation:**
```bash
# From GitHub Container Registry
docker pull ghcr.io/EffortlessMetrics/perl-lsp:latest

# From Docker Hub
docker pull effortlessmetrics/perl-lsp:latest

# Run
docker run --rm -v ${PWD}:/workspace effortlessmetrics/perl-lsp:latest
```

### VSCode Extension

VSCode extension is published to:

- VSCode Marketplace: `EffortlessMetrics.perl-lsp-rs`
- Open VSX: `EffortlessMetrics.perl-lsp-rs`

**Installation:**
```bash
# From VSCode Marketplace
code --install-extension EffortlessMetrics.perl-lsp-rs

# From Open VSX
code --install-extension EffortlessMetrics.perl-lsp-rs --extensions-dir ~/.vscode-oss/extensions
```

## Rollback Procedures

### Scenario 1: GitHub Release Issue

If the GitHub release has issues:

1. **Delete the release**
   ```bash
   gh release delete v<0.x.y> --yes
   ```

2. **Delete the tag**
   ```bash
   git push origin :refs/tags/v<0.x.y>
   git tag -d v<0.x.y>
   ```

3. **Fix the issue** (e.g., update release.yml)

4. **Re-run release orchestration**

### Scenario 2: crates.io Publishing Issue

If a crate publish fails:

1. **Check the error** in the publish-crates.yml workflow logs

2. **Fix the issue** (e.g., update Cargo.toml)

3. **Re-publish the specific crate**
   ```bash
   cargo publish -p <crate-name>
   ```

4. **If version already published**, create a patch release

### Scenario 3: VSCode Extension Issue

If the VSCode extension has issues:

1. **Yank the extension** (if published)
   - Contact VSCode Marketplace support
   - Submit yank request

2. **Fix the issue** in vscode-extension/

3. **Re-publish manually**
   ```bash
   cd vscode-extension
   vsce publish <version>
   ```

### Scenario 4: Package Manager PR Issues

If a package manager PR has issues:

1. **Close the PR** and let it be recreated automatically

2. **Or manually fix the PR**
   - Edit the formula/package
   - Update checksums
   - Submit changes

### Scenario 5: Full Rollback

For a complete rollback:

1. **Delete GitHub release and tag** (see Scenario 1)

2. **Yank crates.io versions** (if necessary)
   ```bash
   # Note: crates.io does not support yanking
   # Must create a new release with fixes
   ```

3. **Create hotfix release**
   - Bump version to patch release
   - Fix issues
   - Re-run release process

## Troubleshooting

### Workflow Failures

**Issue: Release workflow fails**
- Check workflow logs for specific error
- Verify all secrets are configured
- Ensure runner has sufficient resources

**Issue: crates.io publish fails**
- Check `CARGO_REGISTRY_TOKEN` is valid
- Verify crate version doesn't already exist
- Check for dependency issues

**Issue: Docker build fails**
- Check Dockerfile syntax
- Verify base image is available
- Check for platform-specific issues

### Binary Issues

**Issue: Binary doesn't work on target platform**
- Verify target triple is correct
- Check for missing dynamic libraries
- Test on actual target platform

**Issue: Binary is too large**
- Enable strip in release workflow
- Use musl for static linking
- Optimize build settings

### Package Manager Issues

**Issue: Homebrew PR not created**
- Check brew-bump.yml logs
- Verify release assets are available
- Check GitHub token permissions

**Issue: Scoop PR not created**
- Check scoop-bump.yml logs
- Verify Windows binary is available
- Check GitHub token permissions

**Issue: Chocolatey PR not created**
- Check chocolatey-bump.yml logs
- Verify Windows binary is available
- Check GitHub token permissions

**Issue: Winget manifest not updated**
- Check winget-bump.yml logs
- Verify Windows binary is available
- Confirm `distribution/winget/perl-lsp.yaml` still matches the release asset layout

## Release Checklist

### Pre-Release

- [ ] All CI tests passing on master branch
- [ ] Version bump PR created and reviewed
- [ ] Changelog generated and reviewed
- [ ] Breaking changes documented
- [ ] Migration guide updated (if needed)
- [ ] Documentation updated
- [ ] All secrets configured
- [ ] Release notes prepared

### Release

- [ ] Version bump PR merged
- [ ] Release orchestration triggered
- [ ] GitHub release created
- [ ] All binaries uploaded
- [ ] Release notes verified
- [ ] crates.io publishing complete
- [ ] VSCode extension published
- [ ] Docker images published
- [ ] Package manager PRs created

### Post-Release

- [ ] Download and test binaries
- [ ] Test `cargo install perllsp`
- [ ] Test VSCode extension
- [ ] Test Docker images
- [ ] Monitor package manager PRs
- [ ] Merge package manager PRs
- [ ] Update website (if applicable)
- [ ] Announce release (blog, social media)
- [ ] Close release-related issues
- [ ] Create next release issue

## Release Notes Template

```markdown
## Release v{VERSION}

### Highlights

- Feature 1
- Feature 2
- Bug fix 1

### Installation

```bash
# Using cargo
cargo install perllsp

# Using Homebrew (macOS/Linux)
brew install perl-lsp

# Using Scoop (Windows)
scoop install perl-lsp

# Using Chocolatey (Windows)
choco install perl-lsp

# Using Docker
docker pull effortlessmetrics/perl-lsp:latest
```

### Changes

See [CHANGELOG.md](CHANGELOG.md) for detailed changes.

### Upgrade Notes

- Breaking change 1 (if any)
- Migration step 1 (if any)

### Checksums

All binaries include SHA256 checksums in their packages.

### Downloads

- [Linux x86_64 (GNU)](https://github.com/EffortlessMetrics/perl-lsp/releases/download/v{VERSION}/perl-lsp-{VERSION}-x86_64-unknown-linux-gnu.tar.gz)
- [Linux aarch64 (GNU)](https://github.com/EffortlessMetrics/perl-lsp/releases/download/v{VERSION}/perl-lsp-{VERSION}-aarch64-unknown-linux-gnu.tar.gz)
- [Linux x86_64 (musl)](https://github.com/EffortlessMetrics/perl-lsp/releases/download/v{VERSION}/perl-lsp-{VERSION}-x86_64-unknown-linux-musl.tar.gz)
- [Linux aarch64 (musl)](https://github.com/EffortlessMetrics/perl-lsp/releases/download/v{VERSION}/perl-lsp-{VERSION}-aarch64-unknown-linux-musl.tar.gz)
- [macOS x86_64](https://github.com/EffortlessMetrics/perl-lsp/releases/download/v{VERSION}/perl-lsp-{VERSION}-x86_64-apple-darwin.tar.gz)
- [macOS aarch64](https://github.com/EffortlessMetrics/perl-lsp/releases/download/v{VERSION}/perl-lsp-{VERSION}-aarch64-apple-darwin.tar.gz)
- [Windows x86_64](https://github.com/EffortlessMetrics/perl-lsp/releases/download/v{VERSION}/perl-lsp-{VERSION}-x86_64-pc-windows-msvc.zip)
```

## Additional Resources

- [Roadmap](project/ROADMAP.md)
- [Commands Reference](reference/COMMANDS_REFERENCE.md)
- [API Documentation Standards](reference/API_DOCUMENTATION_STANDARDS.md)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [crates.io Documentation](https://doc.rust-lang.org/cargo/reference/publishing.html)
