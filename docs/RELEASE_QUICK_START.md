# Release Quick Start Guide

This is a quick reference guide for performing a release. For detailed information, see [RELEASE_PROCESS.md](RELEASE_PROCESS.md).

## One-Command Release

The entire release process can be completed with a single workflow:

```bash
# 1. Trigger version bump
# Go to: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/actions/workflows/version-bump.yml
# Click "Run workflow" and enter version (e.g., 0.9.0)

# 2. Merge the version bump PR
# Review and merge the PR created by the workflow

# 3. Trigger release orchestration
# Go to: https://github.com/EffortlessMetrics/tree-sitter-perl-rs/actions/workflows/release-orchestration.yml
# Click "Run workflow" and enter the same version

# 4. Monitor progress
# All workflows will run automatically
```

## What Gets Released

| Component | Platform | Destination |
|-----------|----------|-------------|
| Binaries | Linux, macOS, Windows | GitHub Releases |
| Crates | 28 workspace crates | crates.io |
| Extension | VSCode, Open VSX | Marketplaces |
| Images | linux/amd64, linux/arm64 | ghcr.io, Docker Hub |
| Formula | macOS, Linux | Homebrew |
| Manifest | Windows | Scoop |
| Package | Windows | Chocolatey |

## Required Secrets

Configure these in GitHub repository settings:

- `CARGO_REGISTRY_TOKEN` - crates.io API token
- `VSCE_PAT` - VSCode Marketplace PAT
- `OVSX_PAT` - Open VSX PAT
- `DOCKER_USERNAME` - Docker Hub username
- `DOCKER_PASSWORD` - Docker Hub password

## Quick Checklist

- [ ] All CI tests passing
- [ ] Version bump PR merged
- [ ] Release orchestration triggered
- [ ] GitHub release created
- [ ] Binaries tested
- [ ] crates.io verified
- [ ] VSCode extension verified
- [ ] Docker images verified
- [ ] Package manager PRs monitored

## Rollback

If something goes wrong:

```bash
# Delete release and tag
gh release delete v0.9.0 --yes
git push origin :refs/tags/v0.9.0

# Fix the issue and re-run release orchestration
```

## Support

For issues or questions:
- Check [RELEASE_PROCESS.md](RELEASE_PROCESS.md) for detailed documentation
- Review workflow logs for specific errors
- Open an issue on GitHub
