# Changelog Update

Generate a changelog entry for recent commits following the project's Keep a Changelog format.

## Instructions

1. **Get the last release tag**:
   ```bash
   git describe --tags --abbrev=0
   ```

2. **Gather commits since that tag**:
   ```bash
   git log $(git describe --tags --abbrev=0)..HEAD --oneline --no-merges
   ```

3. **Categorize commits** into Keep a Changelog sections:
   - **Added** - New features
   - **Changed** - Changes in existing functionality
   - **Deprecated** - Soon-to-be removed features
   - **Removed** - Removed features
   - **Fixed** - Bug fixes
   - **Security** - Vulnerability fixes

4. **Cross-reference with PRs** if commit messages include PR numbers:
   ```bash
   gh pr view <number> --json title,body
   ```

5. **Update CHANGELOG.md**:
   - Add entries under `## [Unreleased]`
   - Group by category with `### Added`, `### Changed`, etc.
   - Use imperative mood ("Add feature" not "Added feature")
   - Include PR numbers as links: `(PR #123)`

## Format Example

```markdown
### Added

- **Feature Name** (PR #123): Brief description of what was added

### Fixed

- **Bug Description** (PR #124): What was broken and how it was fixed
```

## Output

After analyzing commits, edit CHANGELOG.md directly with the new entries. Do not create a new file.
