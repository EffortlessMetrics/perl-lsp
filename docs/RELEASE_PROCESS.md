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
- **crates.io**: All 28 workspace crates

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

### Step 1: Version Bump and Changelog Generation

Trigger the version bump workflow to prepare for release:

```bash
# Via GitHub UI
1. Go to Actions tab
2. Select "Version Bump & Changelog Generation"
3. Click "Run workflow"
4. Enter version (e.g., 0.9.0)
5. Select bump type (major/minor/patch)
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
4. Enter version (e.g., 0.9.0)
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
2. **Publish to crates.io** - Publishes all 28 crates
3. **Publish VSCode Extension** - Publishes to VSCode Marketplace and Open VSX
4. **Publish Docker Images** - Builds and pushes multi-arch images
5. **Homebrew Auto-Bump** - Creates PR to Homebrew
6. **Scoop Auto-Bump** - Creates PR to Scoop
7. **Chocolatey Auto-Bump** - Creates PR to Chocolatey

### Step 5: Verify Release

After all workflows complete, verify:

1. **GitHub Release**
   - Check that all binaries are uploaded
   - Verify release notes are correct
   - Download and test a binary

2. **crates.io**
   - Verify all crates are published
   - Check that versions match release version
   - Test `cargo install perl-lsp`

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

## Distribution Channels

### crates.io

All 28 workspace crates are published to crates.io in dependency order:

```
perl-lexer → perl-parser-core → perl-position-tracking →
perl-symbol-types → perl-symbol-table → perl-uri →
perl-diagnostics-codes → perl-semantic-analyzer →
perl-workspace-index → perl-refactoring →
perl-incremental-parsing → perl-tdd-support →
perl-lsp-protocol → perl-lsp-transport → perl-lsp-tooling →
perl-lsp-formatting → perl-lsp-diagnostics →
perl-lsp-semantic-tokens → perl-lsp-inlay-hints →
perl-lsp-navigation → perl-lsp-completion →
perl-lsp-code-actions → perl-lsp-rename →
perl-lsp-providers → perl-parser → perl-lsp →
perl-dap → tree-sitter-perl-rs
```

**Installation:**
```bash
cargo install perl-lsp
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
wget https://github.com/EffortlessMetrics/perl-lsp/releases/download/v0.9.0/perl-lsp-0.9.0-x86_64-unknown-linux-gnu.tar.gz
tar xzf perl-lsp-0.9.0-x86_64-unknown-linux-gnu.tar.gz
sudo cp perl-lsp-0.9.0-x86_64-unknown-linux-gnu/perl-lsp /usr/local/bin/
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

- VSCode Marketplace: `effortlesssteven.perl-lsp`
- Open VSX: `effortlesssteven.perl-lsp`

**Installation:**
```bash
# From VSCode Marketplace
code --install-extension effortlesssteven.perl-lsp

# From Open VSX
code --install-extension effortlesssteven.perl-lsp --extensions-dir ~/.vscode-oss/extensions
```

## Rollback Procedures

### Scenario 1: GitHub Release Issue

If the GitHub release has issues:

1. **Delete the release**
   ```bash
   gh release delete v0.9.0 --yes
   ```

2. **Delete the tag**
   ```bash
   git push origin :refs/tags/v0.9.0
   git tag -d v0.9.0
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

## Release Checklist

### Pre-Release

- [ ] All CI tests passing on main branch
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
- [ ] Test `cargo install perl-lsp`
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
cargo install perl-lsp

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

- [Production Readiness Roadmap](../plans/production_readiness_roadmap.md)
- [Commands Reference](COMMANDS_REFERENCE.md)
- [API Documentation Standards](API_DOCUMENTATION_STANDARDS.md)
- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [crates.io Documentation](https://doc.rust-lang.org/cargo/reference/publishing.html)
