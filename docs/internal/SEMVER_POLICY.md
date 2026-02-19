# SemVer Policy for Perl LSP Project

This document outlines our approach to Semantic Versioning (SemVer) and how we manage API stability.

## Versioning Scheme (v0.x.y)

While we are in the 0.x phase, we follow these conventions:

- **Patch (0.8.x)**: Bug fixes, documentation, internal refactoring, and performance improvements. No breaking changes or new public API features.
- **Minor (0.x.0)**: New features, potentially breaking changes (if necessary), and major refactorings.
- **Major (1.0.0)**: Our first Long-Term Support (LTS) release with a 5-year support commitment and strict API stability guarantees.

## What Constitutes a Breaking Change?

We use `cargo-semver-checks` to automatically identify breaking changes. Categories include:

1. **API Removal**: Removing or renaming a public item (function, struct, enum variant).
2. **Signature Change**: Changing the parameters, return type, or generics of a public function.
3. **Trait Changes**: Removing a trait implementation from a public type.
4. **Feature Flags**: Removing a feature flag or changing default features in a way that affects the public API.
5. **Layout Changes**: Changing the layout of a public struct that breaks serialization compatibility.
6. **MSRV Increase**: Increasing the Minimum Supported Rust Version (must only happen in minor version bumps).

## Development Workflow

Before submitting a Pull Request, you should check for SemVer compatibility:

```bash
just semver-check
```

### If Breaking Changes are Detected

1. **Categorize**: Determine if the change is truly necessary and if it belongs in the current release cycle.
2. **Version Bump**: Ensure the project version is bumped to the next minor version (e.g., 0.8.x -> 0.9.0).
3. **Document**: Add a `## Breaking Changes` section to `CHANGELOG.md` with a migration guide.
4. **Label**: Tag your PR with the `semver:breaking` label.

### Exceptions

- **Experimental Features**: APIs under feature flags marked as "experimental" are exempt from SemVer guarantees until they are stabilized.
- **Internal APIs**: Changes to `pub(crate)` items are not considered breaking changes.
- **Documentation**: Improvements to doc comments do not count as breaking changes.

## Baseline Management

Our CI compares your changes against the latest release tag. Locally, you can also compare against `origin/master` for incremental changes.
