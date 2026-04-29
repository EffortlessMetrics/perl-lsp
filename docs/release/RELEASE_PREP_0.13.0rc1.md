# v0.13.0rc1 release prep

This checklist starts the v0.13.0rc1 release-preparation workstream.

## Current baseline

- Workspace version line: `0.12.4` (from `Cargo.toml`).
- The automated version bump command (`cargo run -p perl-ci-hygiene -- bump-version ...`) currently accepts only `X.Y.Z` and rejects prerelease labels such as `0.13.0rc1`.

## Prep steps to run now

1. Confirm release-blocking backlog is empty in `docs/project/WHAT_THE_REPO_STILL_NEEDS.md`.
2. Run release gates from `docs/release/RUNBOOK.md`:
   - `just publish-allowlist-check`
   - `just publish-dry-run`
   - `just release-check`
3. Draft `CHANGELOG.md` section for `0.13.0rc1` (or final `0.13.0`, based on versioning decision).
4. Decide prerelease tag format and tooling support:
   - Option A: adopt stable-only bump to `0.13.0`.
   - Option B: extend `perl-ci-hygiene bump-version` validation to support prerelease semver.

## Decision needed before cut

A release versioning policy decision is required before the bump PR:

- If the project ships prereleases, the bump tooling must accept semver prerelease identifiers.
- If not, prepare for `0.13.0` directly and use release notes to indicate RC status operationally.
