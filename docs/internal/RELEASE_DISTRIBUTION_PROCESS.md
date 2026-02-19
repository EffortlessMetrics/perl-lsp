# Release Distribution Process

This document outlines the complete process for distributing perl-lsp v1.0.0 across all platforms and package managers.

## Overview

The release distribution process ensures that perl-lsp v1.0.0 is available to users through multiple channels:

1. GitHub Releases (primary distribution)
2. Package managers (Homebrew, Chocolatey, Scoop)
3. One-liner installation scripts
4. Container registries

## Prerequisites

- Release tag `v1.0.0` must be created on GitHub
- All CI/CD tests must pass for the release commit
- Release notes must be prepared
- Checksums must be generated for all binaries

## Release Checklist

### 1. Prepare Release

- [ ] Update version in `Cargo.toml` to `1.0.0`
- [ ] Update version in all distribution files
- [ ] Ensure all tests pass: `cargo test --workspace`
- [ ] Build release binaries: `cargo build --release --bin perl-lsp -p perl-lsp`
- [ ] Generate release notes
- [ ] Create release tag: `git tag v1.0.0 && git push origin v1.0.0`

### 2. Build Distribution Packages

```bash
# Build all packages
./distribution/build-packages.sh

# This creates:
# - perl-lsp-1.0.0-x86_64-unknown-linux-gnu.tar.gz
# - perl-lsp-1.0.0-aarch64-unknown-linux-gnu.tar.gz
# - perl-lsp-1.0.0-x86_64-apple-darwin.tar.gz
# - perl-lsp-1.0.0-aarch64-apple-darwin.tar.gz
# - perl-lsp-1.0.0-x86_64-pc-windows-msvc.zip
# - perl-lsp_1.0.0_amd64.deb
# - perl-lsp-1.0.0.rpm
```

### 3. Generate Checksums

```bash
# Generate SHA256 checksums for all binaries
sha256sum perl-lsp-1.0.0-* > SHA256SUMS

# Create individual checksum files for each binary
for file in perl-lsp-1.0.0-*; do
  sha256sum "$file" > "${file}.sha256"
done
```

### 4. Create GitHub Release

1. Go to [GitHub Releases](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/releases)
2. Click "Create a new release"
3. Select tag `v1.0.0`
4. Upload all binary files and checksums
5. Add release notes
6. Publish release

### 5. Update Package Managers

#### Homebrew
```bash
# Update formula with new version and SHA256
# File: distribution/homebrew/perl-lsp.rb
# Update: url, sha256, version

# Submit PR to homebrew-core
brew bump-formula-pr perl-lsp --version=1.0.0 --sha256=ACTUAL_SHA256
```

#### Chocolatey
```bash
# Update package files:
# - distribution/chocolatey/perl-lsp.nuspec (version)
# - distribution/chocolatey/tools/chocolateyinstall.ps1 (url, checksum)

# Build and publish package
choco pack
choco push perl-lsp.1.0.0.nupkg --source https://push.chocolatey.org/
```

#### Scoop
```bash
# Update manifest:
# - distribution/scoop/perl-lsp.json (version, url, hash)

# Submit PR to scoop-extras
git clone https://github.com/ScoopInstaller/Extras.git
# Update bucket/perl-lsp.json
# Submit PR
```

### 6. Test Release

```bash
# Test all binaries
./distribution/test-release.sh 1.0.0

# Test installation scripts
# Linux/macOS:
curl -fsSL https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/main/install.sh | bash

# Windows:
irm https://raw.githubusercontent.com/EffortlessMetrics/tree-sitter-perl-rs/main/install.ps1 | iex
```

### 7. Update Documentation

- [ ] Update installation guide with new version
- [ ] Update README.md with latest download links
- [ ] Update website with new version information
- [ ] Announce release on community channels

## Distribution Files

### Core Distribution Files

| File | Purpose | Platform |
|------|---------|----------|
| `distribution/build-packages.sh` | Builds all distribution packages | Linux |
| `distribution/test-release.sh` | Tests release artifacts | Linux |
| `install.sh` | Linux/macOS one-liner installer | Linux/macOS |
| `install.ps1` | Windows PowerShell installer | Windows |

### Package Manager Files

| File | Package Manager | Platform |
|------|----------------|----------|
| `distribution/homebrew/perl-lsp.rb` | Homebrew | macOS/Linux |
| `distribution/chocolatey/perl-lsp.nuspec` | Chocolatey | Windows |
| `distribution/chocolatey/tools/chocolateyinstall.ps1` | Chocolatey | Windows |
| `distribution/scoop/perl-lsp.json` | Scoop | Windows |

### Container Files

| File | Purpose | Platform |
|------|---------|----------|
| `.docker/rust/Dockerfile` | Build environment | All |

## Release Artifacts

### Binary Releases

1. **Linux x86_64**: `perl-lsp-1.0.0-x86_64-unknown-linux-gnu.tar.gz`
2. **Linux aarch64**: `perl-lsp-1.0.0-aarch64-unknown-linux-gnu.tar.gz`
3. **macOS x86_64**: `perl-lsp-1.0.0-x86_64-apple-darwin.tar.gz`
4. **macOS aarch64**: `perl-lsp-1.0.0-aarch64-apple-darwin.tar.gz`
5. **Windows x86_64**: `perl-lsp-1.0.0-x86_64-pc-windows-msvc.zip`

### Package Manager Releases

1. **Debian**: `perl-lsp_1.0.0_amd64.deb`
2. **RPM**: `perl-lsp-1.0.0.x86_64.rpm`
3. **Homebrew**: Updated formula in homebrew-core
4. **Chocolatey**: `perl-lsp.1.0.0.nupkg`
5. **Scoop**: Updated manifest in scoop-extras

## Automated Distribution

### GitHub Actions Workflow

The release process is automated through GitHub Actions:

1. **Trigger**: On tag creation matching `v*`
2. **Build**: Cross-platform compilation
3. **Test**: Comprehensive test suite
4. **Package**: Create distribution packages
5. **Upload**: Attach artifacts to GitHub release
6. **Notify**: Update package managers (where possible)

### CI/CD Pipeline

```yaml
# .github/workflows/release.yml
name: Release
on:
  push:
    tags:
      - 'v*'

jobs:
  build:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
        arch: [x86_64, aarch64]
        exclude:
          - os: windows-latest
            arch: aarch64
    
    steps:
      - uses: actions/checkout@v4
      - name: Build
        run: cargo build --release --bin perl-lsp -p perl-lsp
      - name: Test
        run: cargo test --workspace
      - name: Package
        run: ./distribution/build-packages.sh
      - name: Upload
        uses: actions/upload-release-asset@v1
```

## Verification Process

### Pre-Release Verification

1. **Code Quality**: All clippy warnings resolved
2. **Tests**: Full test suite passing
3. **Documentation**: All documentation updated
4. **Security**: Security scan passed
5. **Performance**: Benchmarks within acceptable range

### Post-Release Verification

1. **Binary Testing**: Test all downloaded binaries
2. **Installation Testing**: Test all installation methods
3. **Integration Testing**: Test with popular editors
4. **Package Manager Testing**: Verify packages install correctly
5. **Documentation Testing**: Verify all links and instructions work

## Rollback Plan

If issues are discovered after release:

1. **Immediate Actions**:
   - Unpublish package manager releases
   - Update GitHub release with warning
   - Notify users through all channels

2. **Fix Process**:
   - Create hotfix branch from release tag
   - Fix identified issues
   - Create patch release (v1.0.1)
   - Follow full release process

3. **Communication**:
   - Update release notes with issue details
   - Communicate fix timeline to users
   - Document lessons learned

## Maintenance

### Ongoing Tasks

1. **Monitor**: Track installation success rates
2. **Update**: Keep package manager formulas current
3. **Support**: Respond to installation issues
4. **Improve**: Enhance distribution process based on feedback

### Metrics to Track

1. **Download counts**: By platform and version
2. **Installation success rates**: By method
3. **Issue reports**: Installation-related problems
4. **Package manager adoption**: Usage through package managers

## Contact Information

- **Release Manager**: [Release Team](mailto:release@effortlessmetrics.com)
- **Package Maintainers**: [Package Team](mailto:packages@effortlessmetrics.com)
- **Security Issues**: [Security Team](mailto:security@effortlessmetrics.com)

## Related Documentation

- [Installation Guide](INSTALLATION.md)
- [Contributing Guidelines](../CONTRIBUTING.md)
- [Security Policy](../SECURITY.md)
- [Release Notes](../RELEASE_NOTES.md)