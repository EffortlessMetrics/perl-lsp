# GitHub Configuration

This directory contains GitHub-specific configuration files for the perl-lsp project.

## Files

### Repository Settings

- **[settings.yaml](settings.yaml)** - Repository metadata as code
  - Canonical description and homepage
  - Repository topics (GitHub tags)
  - Default branch and merge strategy settings

### Dependency Management

- **[dependabot.yml](dependabot.yml)** - Automated dependency updates via Dependabot
  - Updates Cargo dependencies weekly
  - Updates GitHub Actions weekly
  - Updates npm dependencies (VS Code extension) weekly
  - Groups related dependencies to reduce PR noise
  - See [../docs/DEPENDENCY_MANAGEMENT.md](../docs/DEPENDENCY_MANAGEMENT.md) for details

### CI/CD Workflows

- **[workflows/](workflows/)** - GitHub Actions workflow definitions
  - `ci.yml` - Main CI gate (format, lint, test)
  - `rust.yml` - Rust-specific CI (multi-version testing)
  - `test.yml` - Comprehensive test suite
  - `benchmark.yml` - Performance benchmarks
  - `release.yml` - Release automation
  - And many more specialized workflows

### Issue Templates

- **[ISSUE_TEMPLATE/](ISSUE_TEMPLATE/)** - Issue templates for bug reports and feature requests

### Pull Request Templates

- **[pull_request_template.md](pull_request_template.md)** - PR template with checklist

### Custom Actions

- **[actions/](actions/)** - Reusable composite actions
  - `setup-rust/` - Rust toolchain setup
  - `upload-receipt/` - Gate receipt upload

## Dependabot Configuration

Dependabot is configured to check for dependency updates weekly on Mondays at 09:00 UTC.

### Key Features

1. **Grouped Updates**: Related dependencies are grouped together (e.g., all serde crates)
2. **Version Control**: Major version updates are excluded for critical dependencies
3. **Auto-labeling**: All PRs are automatically labeled for easy filtering
4. **Commit Prefixes**: Uses conventional commit format (`chore(deps)`)

### Quick Actions

```bash
# View all Dependabot PRs
gh pr list --author "app/dependabot"

# Merge passing patch updates
gh pr list --author "app/dependabot" --search "status:success" --json number --jq '.[].number' | \
  xargs -I {} gh pr merge {} --auto --squash

# Force Dependabot to recreate a PR (resolves conflicts)
gh pr comment <pr-number> -b "@dependabot recreate"
```

For more details, see:
- [Dependency Management Guide](../docs/DEPENDENCY_MANAGEMENT.md)
- [Dependency Quick Reference](../docs/DEPENDENCY_QUICK_REFERENCE.md)

## Workflow Configuration

### CI Configuration

The project uses a modular CI approach with multiple workflows:

- **Fast feedback** - Quick checks run on every PR
- **Comprehensive testing** - Full test suite with multiple configurations
- **Optional checks** - Expensive tests only run when requested or on main branch
- **Local-first** - All CI checks can be run locally with `nix develop -c just ci-gate`

### Workflow Triggers

Most workflows are triggered by:
- `pull_request` on main/master branches
- `workflow_dispatch` for manual triggering
- Some workflows use labels (e.g., `ci:bench` for benchmarks)

### Configuration Files

- **[ci-config.yml](ci-config.yml)** - Centralized CI configuration
- **[../.ci/gate-policy.yaml](../.ci/gate-policy.yaml)** - Gate policy definitions

## Maintenance

### Updating Dependabot Configuration

1. Edit [dependabot.yml](dependabot.yml)
2. Validate YAML syntax:
   ```bash
   python3 -c "import yaml; yaml.safe_load(open('.github/dependabot.yml'))"
   ```
3. Commit and push to master
4. Changes take effect on next scheduled run (or trigger manually via GitHub UI)

### Updating Workflows

1. Edit workflow files in [workflows/](workflows/)
2. Test locally when possible:
   ```bash
   nix develop -c just ci-gate
   ```
3. Use `workflow_dispatch` trigger for manual testing
4. Consider using `concurrency` to prevent duplicate runs

## Resources

- [GitHub Actions Documentation](https://docs.github.com/en/actions)
- [Dependabot Documentation](https://docs.github.com/en/code-security/dependabot)
- [Workflow Syntax](https://docs.github.com/en/actions/using-workflows/workflow-syntax-for-github-actions)

## Support

For issues with GitHub configuration:
1. Check workflow logs via `gh run list` and `gh run view`
2. Review [CI Documentation](../docs/CI_README.md)
3. Consult [Local CI Guide](../docs/LOCAL_CI.md)
