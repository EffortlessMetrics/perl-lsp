# Dependency Management Guide

This document describes the automated dependency update strategy for the perl-lsp project.

## Overview

The project uses **GitHub Dependabot** to automatically check for and propose dependency updates. Dependabot is configured to:

- Check for updates weekly (every Monday at 09:00 UTC)
- Group related dependencies together to reduce PR noise
- Automatically label PRs for easy filtering
- Apply appropriate commit message prefixes for changelog generation

## Configuration

The Dependabot configuration is located at `.github/dependabot.yml` and manages three package ecosystems:

1. **Cargo** (Rust dependencies)
2. **GitHub Actions** (workflow dependencies)
3. **npm** (VS Code extension dependencies)

## Update Strategy

### Cargo Dependencies

**Schedule**: Weekly on Monday at 09:00 UTC

**Grouped Dependencies**:
- `serde` - All serde-related crates (serde, serde_json, etc.)
- `tokio` - All tokio async runtime crates
- `tracing` - All tracing/logging crates
- `lsp` - LSP protocol crates (lsp-types, lsp-server, tower-lsp)
- `testing` - Test framework crates (proptest, criterion, rstest, etc.)
- `tree-sitter` - Tree-sitter parser crates
- `pest` - Pest parser generator crates
- `dependencies` - All other patch updates

**Major Version Exclusions**:
Major version updates are **excluded** for these critical dependencies:
- `tree-sitter` - Core parser infrastructure
- `lsp-types` - LSP protocol types
- `tower-lsp` - LSP server framework
- `tokio` - Async runtime

**Rationale**: Major version updates require careful manual review, comprehensive testing, and may involve breaking API changes.

### GitHub Actions

**Schedule**: Weekly on Monday at 09:00 UTC

**Grouped Dependencies**:
- `actions` - All official GitHub actions (checkout, upload-artifact, etc.)
- `rust-toolchain` - Rust toolchain setup actions
- `github-actions` - All other action updates

### npm Dependencies (VS Code Extension)

**Schedule**: Weekly on Monday at 09:00 UTC

**Grouped Dependencies**:
- `vscode` - VS Code API and related packages
- `typescript` - TypeScript and ESLint tooling
- `npm-dependencies` - All other npm packages

**Major Version Exclusions**:
- `vscode` - VS Code engine requires compatibility validation

## Handling Dependabot PRs

### Automatic Checks

All Dependabot PRs automatically trigger:
1. CI gate checks (`just ci-gate`)
2. Full test suite
3. Clippy linting
4. Format validation

### Review Process

#### For Patch Updates (x.y.Z)

Patch updates are generally safe and can be merged quickly:

1. **Verify CI passes** - All gates must be green
2. **Check for breaking changes** - Review the dependency's changelog
3. **Merge** - If CI passes and no issues found

```bash
# Quick verification
nix develop -c just ci-gate

# If all green:
gh pr merge <pr-number> --squash --auto
```

#### For Minor Updates (x.Y.0)

Minor updates may include new features:

1. **Review changelog** - Check for new features and deprecations
2. **Run full test suite** - Ensure compatibility
3. **Check for deprecation warnings**:
   ```bash
   cargo clippy --workspace 2>&1 | grep -i deprecat
   ```
4. **Manual testing** - Test LSP server functionality
5. **Merge** - If everything passes

#### For Major Updates (X.0.0)

Major updates require careful review:

1. **Read migration guide** - Most major updates provide migration docs
2. **Review breaking changes** - Understand API changes
3. **Update code** - Fix any breaking changes
4. **Run comprehensive tests**:
   ```bash
   # All tests
   cargo test --workspace

   # LSP integration tests
   RUST_TEST_THREADS=2 cargo test -p perl-lsp

   # Property tests
   cargo test --release --features slow_tests
   ```
5. **Update documentation** - Document any API changes
6. **Performance check** - Run benchmarks if needed:
   ```bash
   cd xtask && cargo run benchmark
   ```

### Auto-merge Configuration (Optional)

For low-risk updates, you can enable auto-merge:

1. **Enable Dependabot auto-merge** in repository settings
2. **Configure branch protection** to require status checks
3. **Dependabot will auto-merge** patch updates that pass CI

Example configuration for auto-merge via CLI:
```bash
# Enable auto-merge for specific PR
gh pr merge <pr-number> --auto --squash

# Enable for all patch updates from Dependabot
gh pr list --author "app/dependabot" --label "patch" --json number --jq '.[].number' | \
  xargs -I {} gh pr merge {} --auto --squash
```

## Labels and Filtering

Dependabot PRs are automatically labeled:

- `dependencies` - All dependency updates
- `cargo` - Rust dependencies
- `github-actions` - Workflow dependencies
- `npm` - Node.js dependencies
- `vscode-extension` - VS Code extension specific
- `automated` - Automated PRs

**Filter examples**:
```bash
# View all dependency PRs
gh pr list --label "dependencies"

# View only Cargo updates
gh pr list --label "cargo"

# View Dependabot PRs ready to merge
gh pr list --author "app/dependabot" --search "status:success"
```

## Security Updates

Dependabot also creates PRs for security vulnerabilities:

1. **High priority** - Security PRs should be reviewed immediately
2. **Check severity** - Review GitHub Security Advisory
3. **Test thoroughly** - Ensure fix doesn't break functionality
4. **Merge quickly** - Security fixes should be prioritized

**Security advisory check**:
```bash
# Check for security advisories
cargo audit

# Or with detailed info
cargo audit --json | jq '.vulnerabilities'
```

## Monitoring and Maintenance

### Weekly Review Process

Every Monday after Dependabot runs:

1. **Review new PRs** - Check for any new dependency updates
2. **Triage by type**:
   - Security fixes: Immediate review
   - Patch updates: Quick merge if CI passes
   - Minor updates: Review changelog, merge within week
   - Major updates: Schedule for dedicated review
3. **Merge approved PRs** - Batch merge to reduce CI load

### Monthly Audit

Once per month:

1. **Review ignored dependencies** - Check if any should be updated
2. **Verify update policy** - Ensure grouping strategy still makes sense
3. **Check for stale PRs** - Close or update old dependency PRs
4. **Review dependency tree**:
   ```bash
   cargo tree --depth 2
   ```

### Quarterly Review

Every quarter:

1. **Evaluate major version updates** - Plan migrations for major updates
2. **Remove unused dependencies**:
   ```bash
   cargo machete
   ```
3. **Update pinned versions** - Review any version pins in `Cargo.toml`
4. **Benchmark impact** - Check if dependency updates affected performance

## Troubleshooting

### Dependabot PR Conflicts

If a Dependabot PR has merge conflicts:

```bash
# Recreate the PR
gh pr comment <pr-number> -b "@dependabot recreate"

# Or rebase
gh pr comment <pr-number> -b "@dependabot rebase"
```

### CI Failures

If CI fails on a Dependabot PR:

1. **Check error logs** - Review CI output
2. **Test locally**:
   ```bash
   # Fetch the PR
   gh pr checkout <pr-number>

   # Run gate
   nix develop -c just ci-gate
   ```
3. **Fix if needed** - May need to update code
4. **Close PR** - If fix requires significant work, close and handle manually

### Version Conflicts

If Dependabot updates create version conflicts:

1. **Check dependency tree**:
   ```bash
   cargo tree -d  # Show duplicates
   ```
2. **Update parent dependencies** - May need to update multiple crates
3. **Use `cargo update`** - Try resolving with cargo:
   ```bash
   cargo update -p <package-name>
   ```

## Best Practices

### Do's

- ✅ Review changelogs before merging
- ✅ Run full test suite for minor/major updates
- ✅ Group related dependency updates
- ✅ Merge security updates promptly
- ✅ Keep update schedule consistent
- ✅ Use semantic versioning to guide decisions

### Don'ts

- ❌ Blindly merge all updates without review
- ❌ Ignore CI failures
- ❌ Let Dependabot PRs accumulate
- ❌ Skip testing for "small" updates
- ❌ Update major versions without reading migration guides
- ❌ Merge incompatible version combinations

## Configuration Changes

To modify Dependabot behavior, edit `.github/dependabot.yml`:

**Add new group**:
```yaml
groups:
  my-group:
    patterns:
      - "pattern*"
    update-types:
      - "minor"
      - "patch"
```

**Ignore specific updates**:
```yaml
ignore:
  - dependency-name: "crate-name"
    update-types: ["version-update:semver-major"]
```

**Change schedule**:
```yaml
schedule:
  interval: "weekly"  # or "daily", "monthly"
  day: "monday"       # day of week
  time: "09:00"       # UTC time
```

**After making changes**:
1. Commit and push to master
2. Changes take effect on next scheduled run
3. Test by manually triggering: Repository Settings → Code Security → Dependabot → Check for updates

## Related Documentation

- [GitHub Dependabot Documentation](https://docs.github.com/en/code-security/dependabot)
- [Cargo Dependency Management](https://doc.rust-lang.org/cargo/reference/specifying-dependencies.html)
- [Semantic Versioning](https://semver.org/)
- [Security Audit with cargo-audit](https://github.com/rustsec/rustsec)

## Additional Tools

### cargo-outdated

Check for outdated dependencies manually:

```bash
# Install
cargo install cargo-outdated

# Run
cargo outdated --workspace

# Show detailed version info
cargo outdated --workspace --root-deps-only
```

### cargo-audit

Security audit for known vulnerabilities:

```bash
# Install
cargo install cargo-audit

# Run
cargo audit

# Check specific advisories
cargo audit --db <path-to-advisory-db>
```

### cargo-deny

Policy enforcement for dependencies:

```bash
# Install
cargo install cargo-deny

# Check (already configured in deny.toml)
cargo deny check

# Check specific category
cargo deny check advisories
cargo deny check licenses
```

## Support

For issues with dependency updates:

1. Check [Issue #279](https://github.com/your-org/perl-lsp/issues/279) - Original dependency automation issue
2. Review [CLAUDE.md](../CLAUDE.md) - Project development guidelines
3. Consult [CONTRIBUTING.md](../CONTRIBUTING.md) - Contribution workflow

---

**Last Updated**: 2026-01-28
**Configuration Version**: 1.0
**Dependabot Version**: v2
