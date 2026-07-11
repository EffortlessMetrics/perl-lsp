# Changelog Workflow

This document describes changelog generation for perl-lsp. Two mechanisms
coexist:

1. **PR-time release-note fragments ([Changie](https://changie.dev)) — the new
   direction.** Each PR records its own release note as a file-based fragment
   (or an evidenced exemption) at the moment the change is made. See
   [PR-time release notes (Changie)](#pr-time-release-notes-changie) below.
2. **Release-time generation ([git-cliff](https://git-cliff.org)) — the current
   execution path.** Unchanged. The sections after the Changie block document
   it.

---

## PR-time release notes (Changie)

> **Status: FOUNDATION / ADVISORY (tracking issue #3784 — the Changie
> program; #3768 is the retrospective baseline artifact only).** This flow is
> wired into a check that always exits 0 or 2 today — never 1 — because the
> blocking boundary (`blocking_enforced_from` in `policy/changelog.toml`) is
> unset. This PR does not change release execution: Cargo versions,
> publishing, and the git-cliff generation documented below remain the
> versioning/tag/publish authority, and git-cliff remains an audit lens. A
> later reviewed cutover makes Changie batching the changelog-preparation
> authority; a follow-up PR decides whether/when fragments become
> merge-blocking.

### Why

Reconstructing a release changelog from hundreds of merged PRs is a release-time
archaeology exercise. Changie moves the release-note decision to **PR time**:
the author who made the change writes its note while the context is fresh, as a
small YAML fragment under `.changes/unreleased/`. At release time
`changie batch <version> --project <p>` folds every fragment into the project's
Keep-a-Changelog file.

### The disposition rule

**Every PR carries exactly one explicit *disposition* — a fragment OR an
exemption-with-reason.** This is *not* "every PR needs a fragment": an evidenced
exemption is a first-class disposition.

**Option A — add a fragment** (user-facing change):

```bash
changie new    # interactive: pick project, component, kind; write the body
```

This writes `.changes/unreleased/<project>-<PR>-<kind>-<HHMMSS>.yaml`. Fragments
carry a `project:` field (`product` → `CHANGELOG.md`, `vscode` →
`vscode-extension/CHANGELOG.md`), a `component`, a `kind`
(Added/Changed/Fixed/Performance/Deprecated/Removed/Security), a body, and custom
metadata (`PR`, optional `Slug`, `Breaking: no|yes`). See `.changes/samples/`
for two worked examples. Do **not** hand-edit `CHANGELOG.md` /
`vscode-extension/CHANGELOG.md` directly in a feature PR — add a fragment
instead; those files are generated.

**Option B — declare an exemption** (no user-facing change). Add a marker line
to the **PR body**:

```
changelog-exempt: <category> — <reason>
```

or add a tracked note under `.changes/exemptions/<slug>.md`. Recognized
`<category>` values (see `policy/changelog.toml`):

| Category | For |
|----------|-----|
| `tests` | test-only changes |
| `ci` | CI / workflow / policy plumbing |
| `refactor` | behavior-preserving internal refactor |
| `generated-status` | auto-generated status / metrics surfaces |
| `docs-no-contract-change` | docs that change no user-facing contract |
| `deps` | dependency lockfile / version bumps |
| `release-prep` | version bump + changelog batch PRs |
| `changelog-tooling` | changes to the changelog tooling itself |

### The advisory check

```bash
# Check the current branch's disposition against origin/main.
cargo xtask changelog check

# Validate + render the sample fragments end-to-end (requires `changie`).
cargo xtask changelog check --self-test
```

The check, for a PR's changed files, verifies that: (a) a valid fragment was
added, OR (b) an explicit exemption is present, OR (c) the PR is a recognized
release-prep. It validates each fragment's project/kind/component/PR metadata,
confirms it renders via `changie batch --dry-run --keep`, and warns when a
feature PR hand-edits a generated changelog. It runs in CI as the non-required
**Changelog Ledger (Advisory)** workflow (`.github/workflows/changelog-advisory.yml`).

`changie` is pinned via the nix devShell (`flake.nix`) for local use and
downloaded at a pinned, checksum-verified version in the advisory CI workflow.

**Exit codes** (see `xtask/src/tasks/changelog.rs` module docs for the full
contract): `0` = policy satisfied or an advisory finding was reported; `1` =
blocking violation (unreachable today — `blocking_enforced_from` is unset);
`2` = instrument/config failure (malformed config, unresolvable changed-file
list, `changie` render crash) — never a policy verdict.

### The three-clock cutoff model

A single "cutoff SHA" is the wrong model: it silently drops any PR merged
between when #3768's manual catalog was *authored* and when #3768 itself
*merged* (e.g. #3765). `policy/changelog.toml` instead declares three
independent clocks:

| Field | Meaning |
|-------|---------|
| `retrospective_covered_through` | Last `main` SHA #3768's manual catalog actually audited (a conservative floor). |
| `advisory_expected_from` | This PR's (#3775) own merge SHA. A disposition is *expected* (missing = reported finding, exit 0) only for PRs whose base is at/after this commit. Empty = not yet armed. |
| `blocking_enforced_from` | A future SHA. A missing disposition is a *blocking* violation (exit 1) only for PRs whose base is at/after this commit. Empty = no blocking path is reachable, ever. |

The half-open interval `(retrospective_covered_through, advisory_expected_from]`
— PRs merged after the retrospective floor but before the advisory boundary
armed — is covered by a separate bridge audit (a later PR), not by this
policy file.

---

## Overview

The perl-lsp project uses automated changelog generation to:
- Maintain consistent, high-quality release notes
- Reduce manual effort during releases
- Ensure all changes are properly documented
- Follow [Keep a Changelog](https://keepachangelog.com) format
- Integrate seamlessly with GitHub releases

## Quick Start

```bash
# Preview unreleased changes
just changelog-preview

# Generate full changelog (overwrites CHANGELOG.md)
just changelog

# Update changelog with unreleased changes (for releases)
just changelog-append
```

## Conventional Commits

The changelog is generated from conventional commit messages. Follow this format:

```
<type>(<scope>): <subject>

[optional body]

[optional footer]
```

### Commit Types

| Type | Description | Changelog Section | Example |
|------|-------------|-------------------|---------|
| `feat` | New feature | ✨ Features | `feat(lsp): add hover support` |
| `fix` | Bug fix | 🐛 Bug Fixes | `fix(parser): handle empty strings` |
| `perf` | Performance improvement | ⚡ Performance | `perf: optimize AST traversal` |
| `refactor` | Code refactoring | ♻️ Refactoring | `refactor(lexer): simplify tokenizer` |
| `docs` | Documentation | 📚 Documentation | `docs: update LSP guide` |
| `test` | Testing | 🧪 Testing | `test(parser): add edge cases` |
| `build` | Build system | 🏗️ Build System | `build: update cargo dependencies` |
| `ci` | CI/CD | 👷 CI/CD | `ci: add benchmark workflow` |
| `chore` | Maintenance | 🔧 Chore | `chore: update gitignore` |
| `security` | Security fix | 🔒 Security | `security: fix path traversal` |
| `revert` | Revert previous commit | ⏪ Reverts | `revert: "feat: add feature X"` |
| `ux` | UX/UI improvement | 🎨 UX/UI | `ux: improve error messages` |
| `style` | Code style (skipped) | _(skipped)_ | `style: format code` |

### Scopes

Scopes indicate which part of the codebase is affected:

- `parser` - Parser library (perl-parser)
- `lsp` - LSP server (perl-lsp)
- `dap` - Debug adapter (perl-dap)
- `lexer` - Lexer (perl-lexer)
- `corpus` - Test corpus
- `extension` - VS Code extension
- `ci` - CI/CD pipelines
- `docs` - Documentation

### Breaking Changes

Mark breaking changes by adding `!` after the type/scope or including `BREAKING CHANGE:` in the footer:

```bash
# Method 1: ! syntax
git commit -m "feat(lsp)!: change API signature"

# Method 2: Footer
git commit -m "feat(lsp): change API signature

BREAKING CHANGE: The `parse()` function now returns Result<T, Error>
instead of Option<T>. Update all callers to handle errors."
```

## Commit Message Examples

### Good Examples

```bash
# Feature with scope
feat(parser): add support for heredoc syntax

# Bug fix with detailed description
fix(lsp): prevent crash on empty file
Handles edge case where document is empty during initialization.
Fixes #123

# Performance improvement
perf(parser): optimize AST traversal in ScopeAnalyzer
Reduces parse time by 30% for large files by using stack-based tracking.

# Breaking change
feat(lsp)!: require Rust 1.70+
Updates MSRV to 1.70 for better error handling support.

BREAKING CHANGE: Minimum supported Rust version is now 1.70.0
```

### Bad Examples

```bash
# Too vague
fix: stuff

# Missing type
added feature

# Not following convention
updated code for issue 123
```

## Justfile Commands

### `just changelog-preview`

Preview unreleased changes without modifying files:

```bash
just changelog-preview
```

This shows what would be included in the next release.

### `just changelog`

Generate complete changelog (overwrites CHANGELOG.md):

```bash
just changelog
```

**Warning**: This regenerates the entire changelog from git history. Use with caution.

### `just changelog-append`

Update CHANGELOG.md with unreleased changes (recommended for releases):

```bash
just changelog-append
```

This prepends new changes to the existing CHANGELOG.md.

### `just changelog-latest`

Show changelog for the latest tag:

```bash
just changelog-latest
```

### `just changelog-range FROM TO`

Generate changelog for a specific range:

```bash
just changelog-range <previous-release> <next-release>
```

## Release Workflow Integration

The changelog is automatically generated during releases via the release orchestration flow:

1. **Trigger**: Dispatch `Version Bump & Changelog Generation` with `version=<0.x.y>`.
2. **Generate**: The workflow bumps `Cargo.toml`, runs `git-cliff`, and creates a PR to merge changelog updates.
3. **Release Orchestration**: After PR merge, dispatch `Release Orchestration` with `version=<0.x.y>`.
4. **Release Workflow**: `release.yml` creates a GitHub release and includes generated release notes.
5. **Publish**: `publish-crates.yml`, `publish-extension`, and `publish-docker` are triggered as configured.

### Turnkey Release Flow

```bash
# Recommended: run both workflow steps through gh automation.
# Canonical command for RC orchestration:
cargo xtask release-turnkey <0.x.y>
```

### Alternative Manual Release Process

```bash
# 1. Generate changelog content
# Use the same canonical flow entrypoint:
cargo xtask release-turnkey <0.x.y> --no-auto-merge --no-wait-release

# 2. Manually review and merge the generated version bump PR.

# 3. Dispatch Release Orchestration manually
gh workflow run "Release Orchestration" \
  --ref master \
  --field version=<0.x.y> \
  --field prerelease=false \
  --field skip_crates=false \
  --field skip_extension=false \
  --field skip_docker=false

# 4. Optionally monitor release/publish workflows in GitHub Actions.

# 5. GitHub Actions creates release notes and publishes artifacts.
```

## Configuration

The changelog generation is configured in `cliff.toml`:

```toml
[changelog]
header = "# Changelog\n\n..."
body = "{% for group, commits in commits | group_by(attribute=\"group\") %}..."
footer = "<!-- Generated by git-cliff -->"

[git]
conventional_commits = true
commit_parsers = [
    { message = "^feat", group = "✨ Features" },
    # ... more parsers
]
```

### Customization

To customize changelog generation:

1. Edit `cliff.toml` to modify:
   - Commit parsers (what goes in which section)
   - Template formatting (emojis, headers, etc.)
   - Filtering rules (skip certain commits)

2. Test changes:
   ```bash
   just changelog-preview
   ```

3. Commit the updated configuration:
   ```bash
   git add cliff.toml
   git commit -m "chore: update changelog config"
   ```

## Installation

### git-cliff

Install git-cliff to use changelog commands:

```bash
# Via cargo
cargo install git-cliff --locked

# Via homebrew (macOS/Linux)
brew install git-cliff

# Via nix
nix-shell -p git-cliff
```

### CI/CD

git-cliff is automatically installed in the release workflow, no manual setup required.

## Best Practices

### 1. Write Clear Commit Messages

```bash
# Good: Specific, actionable
feat(lsp): add semantic token support for variables

# Bad: Vague, no context
update code
```

### 2. Use Conventional Commits Consistently

Every commit should follow the conventional commit format. This ensures:
- Accurate changelog generation
- Proper categorization
- Automatic semantic versioning

### 3. Keep Squash Merges Explicitly Conventional

Use explicit subject/body on squash merges so merged history stays descriptive:

```bash
pr=2943
gh pr merge "$pr" --squash \
  --subject "feat(lsp): improve type definition and implementation with OO inheritance support" \
  --body "PR summary:
- improve OO lookup fallback for type definitions and implementations
- add coverage for inherited methods and signatures
- preserve existing behavior for non-OO dispatch paths"
  --delete-branch
```

### 4. Document Breaking Changes

Always document breaking changes in the commit message:

```bash
feat(lsp)!: change configuration format

BREAKING CHANGE: Configuration now uses TOML instead of JSON.
See [docs/how-to/UPGRADING.md](how-to/UPGRADING.md) for upgrade instructions.
```

### 5. Review Before Release

Always preview the changelog before releasing:

```bash
just changelog-preview
```

## Troubleshooting

### "git-cliff not installed"

Install git-cliff:
```bash
cargo install git-cliff --locked
```

### "No commits found"

Ensure you have commits since the last tag:
```bash
git log $(git describe --tags --abbrev=0)..HEAD
```

### Changelog Missing Commits

Check if commits follow conventional format:
```bash
git log --oneline --pretty=format:"%s" | grep -v "^(feat|fix|docs|style|refactor|perf|test|build|ci|chore|revert|security|ux)"
```

### Unwanted Commits in Changelog

Edit `cliff.toml` to add skip rules:

```toml
commit_parsers = [
    # Skip merge commits
    { message = "^Merge", skip = true },
    # Skip style commits
    { message = "^style", skip = true },
]
```

## Examples

### Generate Changelog for v<0.x.y> Release

```bash
# 1. Check what will be included
just changelog-preview

# 2. Update CHANGELOG.md
just changelog-append

# 3. Review the changes
git diff CHANGELOG.md

# 4. Commit and tag
git add CHANGELOG.md
git commit -m "chore: prepare v<0.x.y> release"
git tag v<0.x.y>
git push origin master --tags
```

### View Changes Between Two Versions

```bash
just changelog-range <previous-release> <next-release>
```

### Regenerate Full Changelog

```bash
# Backup current changelog
cp CHANGELOG.md CHANGELOG.md.backup

# Regenerate from git history
just changelog

# Compare and restore if needed
diff CHANGELOG.md CHANGELOG.md.backup
```

## References

- [git-cliff documentation](https://git-cliff.org/docs/)
- [Conventional Commits](https://www.conventionalcommits.org/)
- [Keep a Changelog](https://keepachangelog.com/)
- [Semantic Versioning](https://semver.org/)

## Related Documentation

- [Release Process](./RELEASE_PROCESS.md) - Complete release workflow
- [Contributing Guide](../CONTRIBUTING.md) - Commit message guidelines
- [CI/CD Documentation](./CI_README.md) - CI pipeline integration
