# 0.12.3 Release Runbook — Pipeline Rehearsal

> **Purpose**: Prove the full publish + release + extension + Docker cycle works
> end-to-end cleanly. 0.12.3 is a rehearsal; once it succeeds, 0.13.0 is cut as
> the public alpha announcement.
>
> **Assumes**: 0.12.2 is confirmed published on crates.io (all crates indexed,
> verified via sparse index). Do not start this runbook if 0.12.2 is incomplete.
>
> **Cross-references**:
> - [`docs/project/PUBLISHING_ROADMAP.md`](PUBLISHING_ROADMAP.md) — machine-executable pre-release checklist
> - [`docs/project/RELEASE_CHECKLIST.md`](RELEASE_CHECKLIST.md) — preflight gate checklist
> - [`docs/project/RELEASE_NOTES_DRAFT.md`](RELEASE_NOTES_DRAFT.md) — release notes source
> - [`.github/workflows/version-bump.yml`](../../.github/workflows/version-bump.yml) — automated version bump workflow
> - [`.github/workflows/release-orchestration.yml`](../../.github/workflows/release-orchestration.yml) — master release orchestration
> - [`.github/workflows/publish-crates.yml`](../../.github/workflows/publish-crates.yml) — crates.io publish pipeline
> - [`.github/workflows/publish-extension.yml`](../../.github/workflows/publish-extension.yml) — VSCode extension publish
> - [`.github/workflows/docker-publish.yml`](../../.github/workflows/docker-publish.yml) — Docker image publish
> - [`CONTRIBUTING.md`](../../CONTRIBUTING.md) — contributing and release process notes

---

## Pre-flight

### 1. Verify master branch CI is green

**What**: Confirm the most recent CI runs on master are all successful.

**Why**: Release orchestration validates CI state before tagging. A red master blocks the workflow automatically; catching it here saves a wasted workflow dispatch.

**Command**:
```bash
rtk gh run list --branch master --limit 5
```

**Expected output**: All 5 runs show `completed / success`. The `ci.yml` workflow must be among them.

**If it fails**: Do not proceed. Identify the failing job, fix the root cause on a feature branch, merge, and re-check. The release-orchestration workflow will also reject a non-green HEAD (see `release-orchestration.yml` step "Check default branch and CI status").

---

### 2. Verify 0.12.2 is fully on crates.io

**What**: Confirm every publishable crate at version 0.12.2 is indexed on the crates.io sparse index, not just the top-level binary crate.

**Why**: The publish workflow in 0.12.3 skips crates that are already in the sparse index. If 0.12.2 has gaps (partial publish), those gaps may surface as ordering errors during the 0.12.3 run. Verifying 0.12.2 completeness isolates the rehearsal to 0.12.3 changes only.

**Command**:
```bash
# Spot-check a sample of leaf crates and the main binary
curl -fsSL https://index.crates.io/pe/rl/perl-lsp-rs | grep '"vers":"0.12.2"' | grep -v '"yanked":true'
curl -fsSL https://index.crates.io/pe/rl/perl-parser | grep '"vers":"0.12.2"' | grep -v '"yanked":true'
curl -fsSL https://index.crates.io/pe/rl/perl-lexer | grep '"vers":"0.12.2"' | grep -v '"yanked":true'
# For a full check, use the publish workflow's built-in verify job (dry-run):
# gh workflow run publish-crates.yml --field version=0.12.2 --field dry_run=true
```

**Expected output**: Each `curl` returns at least one JSON line with `"vers":"0.12.2"` and `"yanked":false`.

**If it fails**: Investigate which crates are missing. The 0.12.2 publish may need a partial re-run via `gh workflow run publish-crates.yml --field version=0.12.2 --field dry_run=false` targeting only the missing crates. Resolve before cutting 0.12.3.

---

### 3. Verify the CHANGELOG.md `[0.12.3]` entry is complete

**What**: Confirm `CHANGELOG.md` contains a versioned `## [0.12.3]` section (not just `[Unreleased]`) with accurate content.

**Why**: The `release-orchestration.yml` workflow reads the CHANGELOG to annotate the git tag. A missing or incomplete entry produces a tag with no release notes. The `release-check` recipe also fails if the section is absent.

**Command**:
```bash
grep -A 20 "## \[0\.12\.3\]" CHANGELOG.md
```

**Expected output**: A section heading `## [0.12.3] - 2026-04-08` followed by change entries under `### Added`, `### Fixed`, `### Changed`, and the publish pipeline / UX subsections.

**Status**: The `[0.12.3]` entry is finalized in `CHANGELOG.md` as of this PR. The section covers all PRs that landed between the v0.12.2 tag and 2026-04-08, including publish pipeline fixes (#3307, #3296, #3304, #3273, #3315, #3316), UX P0 fixes (#3306, #3308, #3309, #3310, #3312), CI hardening (#3294, #3318, #3293, #3297), docs.rs metadata (#3299), and the publish dry-run gate (#3301).

**If it fails**: If placeholder text remains, this PR was not merged. Open a fresh docs PR to finalize the section.

---

### 4. Verify `PRE_ANNOUNCEMENT_CHECKLIST.md` items

**What**: Review the pre-announcement checklist (if it exists in the repo) and confirm any release-blocking items are resolved or consciously deferred.

**Why**: 0.12.3 is a rehearsal; unresolved blockers discovered here may cascade into the 0.13.0 announcement window.

**Command**:
```bash
# Check if the file exists
ls docs/project/ | grep -i pre.announce || echo "file not present"
# If present:
grep -E "^\- \[ \]" docs/project/PRE_ANNOUNCEMENT_CHECKLIST.md || echo "no open items"
```

**Expected output**: Either the file is absent (acceptable for 0.12.3 rehearsal), or all open items (`- [ ]`) are either resolved or marked "deferred to 0.13.0" with a comment.

**If it fails**: For release-blocking items (e.g., secrets not set, broken packaging path), resolve them before tagging. For cosmetic items, document the deferral decision in the checklist file and proceed.

---

### 5. Run the full release-check gate locally

**What**: Execute the extended release validation suite that covers CI gate, release build, SBOM verification, version sync, semver compliance, changelog presence, publish dry-run, and panic audit.

**Why**: `release-orchestration.yml` validates CI state server-side but does not run the local extended gate. This step catches issues (clippy errors, version mismatches, snapshot drift, panic usage) before spending CI minutes on a workflow dispatch.

**Command**:
```bash
export CARGO_TARGET_DIR="/tmp/release-0.12.3-target"
just release-check
```

**Expected output**:
```
=== Extended release checks ===
  CHANGELOG.md has [0.12.3] section
  RELEASE GATE PASSED
```
All sub-steps (`ci-gate`, `release-build`, `sbom-verify`, `version-check`, `semver-check`) must exit 0.

**If it fails**:
| Failure message | Fix |
|----------------|-----|
| `cargo fmt --check` fails | `cargo fmt --all && git commit -m "style: fmt"` |
| Clippy error | Fix the lint, commit |
| `CHANGELOG.md missing section` | See step 3 |
| Version mismatch in `version-check` | See step 8 |
| Snapshot drift (`.snap.new` files) | `cargo insta accept && git add crates/*/tests/snapshots/ && git commit -m "test: accept snapshots"` |
| `semver-check` fails | Breaking change must go to 0.13.0 (minor bump); or document it explicitly |

---

### 6. Run `just doctor` to confirm workspace health

**What**: Execute the workspace health check to confirm no workspace corruption, missing tools, or environment mismatches.

**Why**: Catches environmental issues (wrong Rust toolchain, missing xtask binaries, stale lock files) that can silently corrupt a release build but are not caught by the CI gate.

**Command**:
```bash
just doctor
```

**Expected output**: All health checks report OK. Any warnings about optional tools (e.g., `cargo-machete`) are acceptable if those tools are not required for release.

**If it fails**: Address the reported issues. Common fixes: `rustup update`, `cargo build -p xtask`, `cargo update`. Do not proceed with a corrupted workspace.

---

### 7. Run post-publish smoke test against 0.12.2

**What**: Install `perllsp` from crates.io at version 0.12.2 and verify the installed binary works. This validates that the install path itself is functional before cutting a new release on top of it.

**Why**: If 0.12.2's install path is broken (bad binary, missing dependency, platform issue), 0.12.3 will inherit the same breakage. Confirming 0.12.2 passes smoke establishes a green baseline.

**Command**:
```bash
just smoke-test-release 0.12.2
```

**Expected output**: The smoke test exits 0. The binary prints `perllsp 0.12.2` (or `perl-lsp-rs 0.12.2`) when invoked with `--version`, and basic LSP handshake succeeds.

**If it fails**: The 0.12.2 publish has a defect. Do not cut 0.12.3 until the defect is understood. Options: yank 0.12.2 and publish a fix as 0.12.3, or document the known defect and confirm 0.12.3 fixes it.

---

## Version Bump

### 8. Trigger the version bump workflow

**What**: Dispatch the `version-bump.yml` GitHub Actions workflow with `version=0.12.3` to bump all workspace crate versions and regenerate the changelog.

**Why**: The workspace has 130+ crates. Manual version bumping across all `Cargo.toml` files is error-prone. `just bump-version` (added in #3289) handles all 191 version sites atomically — workspace `Cargo.toml`, every crate manifest, VS Code extension manifest and lockfile, `features.toml`, README, CLAUDE.md, and ROADMAP. It is the simplest local path. The `version-bump.yml` workflow dispatch and `just release-turnkey` are available alternatives that also open a PR automatically.

**Command** (option A — `just bump-version`, simplest local path):
```bash
export CARGO_TARGET_DIR="/tmp/release-0.12.3-target"
just bump-version 0.12.3
```

This updates all 191 version sites. Commit the result and open a PR normally.

**Command** (option B — workflow dispatch via gh CLI, opens PR automatically):
```bash
gh workflow run version-bump.yml \
  --field version=0.12.3 \
  --field bump_type=patch \
  --field prerelease=false
```

**Command** (option C — local xtask, opens PR):
```bash
export CARGO_TARGET_DIR="/tmp/release-0.12.3-target"
just release-turnkey 0.12.3
```

**Expected output** (options A/B/C): All 191 version sites updated to `0.12.3`. For B and C, a PR is opened on branch `release/v0.12.3` with title "Release v0.12.3". Changed files: workspace `Cargo.toml`, `vscode-extension/package.json`, `features.toml`, docs.

**If it fails**: Check for an existing `release/v0.12.3` branch (delete it first if stale). If `cargo-release` is not installed for option C, use option A or B. If the workflow fails to resolve `cargo-release` from GitHub releases, check network access in CI.

---

### 9. Review the version bump PR diff carefully

**What**: Inspect the diff of the `release/v0.12.3` PR before merging.

**Why**: The automated bump touches many files. A scope violation (e.g., bumping a crate that should not change, or incorrect version in `vscode-extension/package.json`) discovered after tagging requires a yank and a new patch release.

**Command**:
```bash
# Find the PR number
gh pr list --head release/v0.12.3
# View the diff
gh pr diff <PR_NUMBER>
```

**Expected diff contents**:
1. `Cargo.toml` (workspace root): `version = "0.12.2"` -> `"0.12.3"`
2. `vscode-extension/package.json`: `"version": "0.12.2"` -> `"0.12.3"`
3. `CHANGELOG.md`: `[0.12.3]` section added or promoted from `[Unreleased]`
4. No other files should change.

**If it fails**: If unexpected files changed (e.g., individual crate `Cargo.toml` files with wrong versions, or lock file drift), investigate the `cargo-release` configuration. Close the PR, clean up the branch, and re-run.

---

### 10. Commit message and PR compliance

**What**: Confirm the auto-generated commit message follows the project convention.

**Why**: The `post-merge-corpus-ratchet.yml` workflow and other automation read commit message prefixes. A non-conformant message may skip automation triggers.

**Expected commit message** (auto-generated by `version-bump.yml`):
```
chore(release): bump version to v0.12.3

- Update workspace version to 0.12.3
- Generate changelog for release
```

**Action**: No manual action needed if the workflow generated the PR correctly. If using option B (`just release-turnkey`), verify the commit message matches the format above before pushing.

---

### 11. Wait for CI green on the bump PR, then merge

**What**: Let CI run on the `release/v0.12.3` PR (full `ci.yml` gate), then merge.

**Why**: The CI gate on the bump PR catches any test regressions introduced by the version string change (unlikely but possible if tests assert on version numbers). Merging without green CI defeats the rehearsal goal.

**Command**:
```bash
# Watch PR checks
gh pr checks <PR_NUMBER> --watch
# Merge when green (use squash to keep history clean)
gh pr merge <PR_NUMBER> --squash --delete-branch
```

**Expected output**: All checks pass. The PR merges cleanly into master. `release/v0.12.3` branch is deleted.

**If it fails**: If a test asserts a hardcoded version string, update the test. Do not skip CI.

---

## Tag and Release

### 12. Dispatch `release-orchestration.yml` — the single entry point

**What**: Trigger the master release orchestration workflow with `version=0.12.3`. This workflow validates the workspace version, verifies CI state, creates the annotated git tag, and dispatches all downstream workflows (release binary build, crates.io publish, VSCode extension, Docker).

**Why**: Direct `git tag` + `git push` is the old manual path (see `docs/project/GA_RUNBOOK.md` — it is stale). The current canonical path is `release-orchestration.yml`, which validates the workspace version matches the input, checks CI state, and creates the tag server-side with the correct changelog annotation. This prevents the "tag points to wrong commit" class of error.

**Command**:
```bash
gh workflow run release-orchestration.yml \
  --field version=0.12.3 \
  --field prerelease=false \
  --field skip_crates=false \
  --field skip_extension=false \
  --field skip_docker=false
```

**Expected output**: The workflow run starts. Navigate to the Actions tab to watch it. The `validate` job confirms workspace version matches `0.12.3` and CI state is `success`. The `create-tag` job pushes `v0.12.3`. The `trigger-release` job dispatches `release.yml`, `publish-crates.yml`, `publish-extension.yml`, and `docker-publish.yml`.

**If it fails**:
- `validate` fails with "Workspace version does not match": The version bump PR did not merge, or merged to a different branch. Verify `grep '^version' Cargo.toml | head -1` on master.
- `validate` fails with "Commit is not in a successful CI state": Master CI is still running or red. Wait or fix.
- `create-tag` fails with "Tag v0.12.3 already exists": A previous attempt partially succeeded. Delete the tag (`git push origin :v0.12.3`) and re-dispatch only if you are certain no downstream workflows ran against it.

---

### 13. Monitor the GitHub Release creation

**What**: Confirm that `release.yml` (dispatched by orchestration) creates the GitHub Release with binaries and the VSIX artifact attached.

**Why**: The crates.io publish workflow (`publish-crates.yml`) is triggered at tag-ref, meaning it uses the same source. The extension workflow (`publish-extension.yml`) waits for the GitHub Release to exist before uploading the VSIX asset. If `release.yml` fails, the VSIX will not be attached to the release page.

**Command**:
```bash
# Find the release.yml run ID
gh run list --workflow release.yml --limit 3
# Watch it
gh run watch <RUN_ID>
```

**Expected output**: `release.yml` completes successfully. `gh release view v0.12.3` shows assets attached (platform binaries + VSIX).

**If it fails**: Check the build matrix for the specific failing platform. The release can be re-run with `gh run rerun <RUN_ID>`. If the GitHub Release was created but assets are missing, re-triggering `publish-extension.yml` will re-attach the VSIX (it uses `--clobber`).

---

## Publish Cascade

### 14. Monitor the crates.io publish workflow

**What**: Watch `publish-crates.yml` as it publishes all workspace crates in topological dependency order.

**Why**: With 130+ crates, the publish takes 30-60 minutes. The workflow retries on transient crates.io failures and checks the sparse index after each publish. A failure mid-cascade means only some crates are at 0.12.3; the workflow's re-run safety (sparse-index fast-path skip) allows safe re-runs.

**Command**:
```bash
gh run list --workflow publish-crates.yml --limit 3
gh run watch <RUN_ID>
```

**Expected output**: All jobs complete successfully. The final `verify` job reports "All Crates Published Successfully" with the crate count. Duration: typically 30-60 minutes.

**If it fails**:
- If a single crate fails after 3 attempts, check `https://index.crates.io/<path>/<crate-name>` directly. A transient crates.io outage may require a re-run (`gh run rerun <RUN_ID>`). The workflow's sparse-index check ensures already-published crates are skipped safely.
- If `validate crate versions` fails: the workspace version on master does not match 0.12.3. This means the bump PR did not merge correctly.
- Do not manually `cargo publish` individual crates — the topological order logic in the workflow handles dev-dependency cycles.

---

### 15. Verify each crate appears on the sparse index

**What**: After `publish-crates.yml` succeeds, do a spot-check of several crates at the sparse index to confirm they are not yanked and the version is correct.

**Why**: The workflow's built-in `verify` job does this automatically, but a manual spot-check on key crates (the binary, the parser, a leaf crate) confirms the automation's output.

**Command**:
```bash
# Check the main binary crate
curl -fsSL https://index.crates.io/pe/rl/perl-lsp-rs \
  | grep '"vers":"0.12.3"' | grep -v '"yanked":true' | head -1

# Check the parser
curl -fsSL https://index.crates.io/pe/rl/perl-parser \
  | grep '"vers":"0.12.3"' | grep -v '"yanked":true' | head -1

# Check the CLI crate (perllsp)
curl -fsSL https://index.crates.io/pe/rl/perllsp \
  | grep '"vers":"0.12.3"' | grep -v '"yanked":true' | head -1
```

**Expected output**: Each command returns one non-empty JSON line with `"vers":"0.12.3"` and no `"yanked":true`.

**If it fails**: If a crate is missing, re-run `publish-crates.yml` — it is safe to re-run (already-published crates are skipped). If a crate is yanked, investigate why and unpublish the yank if it was accidental (`cargo yank --undo`).

---

## Extension Publish

### 16. Verify the VSCode extension publish

**What**: Confirm `publish-extension.yml` completed and the extension is live on both the VSCode Marketplace and Open VSX Registry.

**Why**: `publish-extension.yml` is dispatched by `release-orchestration.yml` automatically. This step confirms it succeeded and the new version is publicly visible. Extension propagation can take up to 30 minutes after the workflow completes.

**Command**:
```bash
gh run list --workflow publish-extension.yml --limit 3
gh run watch <RUN_ID>
```

**Expected output**: The workflow exits 0 with both "Publish to VSCode Marketplace" and "Publish to Open VSX" steps successful.

**If it fails**:
- `VSCE_PAT` or `OVSX_PAT` secret expired: Rotate the secret in repository settings and re-run.
- Extension version already exists: The version was published from a previous attempt. Check Marketplace directly.
- Re-run: `gh workflow run publish-extension.yml --field version=0.12.3`

---

### 17. Verify VSCode Marketplace shows 0.12.3

**What**: Confirm the extension listing on the VSCode Marketplace reflects version 0.12.3.

**Why**: The workflow can exit 0 while the Marketplace CDN propagates. Verifying the public listing confirms end-user installability.

**Command**:
```bash
curl -fsSL "https://marketplace.visualstudio.com/_apis/public/gallery/publishers/EffortlessMetrics/vsextensions/perl-lsp-rs/latest/vspackage" \
  -o /dev/null -w "%{url_effective}" 2>&1 | grep -o "0\.[0-9]*\.[0-9]*"
# OR check via vsce:
npx @vscode/vsce show EffortlessMetrics.perl-lsp-rs --json | python3 -c "import json,sys; d=json.load(sys.stdin); print(d['versions'][0]['version'])"
```

**Expected output**: `0.12.3`

**If it fails**: Marketplace propagation can take up to 30 minutes. Wait and retry. If still missing after 1 hour, check the `publish-extension.yml` workflow logs for silent errors in the `vsce publish` step.

---

### 18. Verify Open VSX shows 0.12.3

**What**: Confirm the Open VSX listing shows 0.12.3.

**Why**: Open VSX is the extension registry for VS Code-compatible editors (VSCodium, Gitpod, etc.). Both registries must be updated for full coverage.

**Command**:
```bash
curl -fsSL "https://open-vsx.org/api/EffortlessMetrics/perl-lsp-rs" \
  | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('version', 'not found'))"
```

**Expected output**: `0.12.3`

**If it fails**: Open VSX can lag 15-30 minutes behind the workflow. If missing after 1 hour, re-run: `gh workflow run publish-extension.yml --field version=0.12.3` (the `ovsx publish` step is idempotent).

---

## Docker Publish

### 19. Verify Docker image build and push completed

**What**: Confirm `docker-publish.yml` completed and both the builder image (`ghcr.io/effortlessmetrics/perl-lsp`) and the runtime image (`ghcr.io/effortlessmetrics/perl-lsp-perl`) are tagged at 0.12.3.

**Why**: `docker-publish.yml` is dispatched by orchestration automatically. It builds multi-arch images (linux/amd64, linux/arm64) for both the Rust builder image and the perl-lsp runtime image.

**Command**:
```bash
gh run list --workflow docker-publish.yml --limit 3
gh run watch <RUN_ID>
```

**Expected output**: All jobs (`build`, `build-perl-runtime`, `publish-dockerhub`, `publish-dockerhub-perl`) succeed. Duration: typically 45-90 minutes for multi-arch.

**If it fails**: Docker multi-arch builds are the most likely to hit transient failures (QEMU timeouts, registry rate limits). Re-run the workflow: `gh workflow run docker-publish.yml --field version=0.12.3 --field push=true`. The build uses GHA cache so re-runs are faster.

---

### 20. Smoke-test the Docker image

**What**: Pull and run the published Docker image to confirm the binary responds correctly.

**Why**: A successful `docker-publish.yml` run does not guarantee the binary inside the image works. This is the end-to-end validation that the image is usable.

**Command**:
```bash
# GHCR image (builder — Rust CI base, not the LSP runtime)
docker run --rm ghcr.io/effortlessmetrics/perl-lsp:0.12.3 perl-lsp-rs --version

# Docker Hub runtime image (perllsp + Perl)
docker run --rm effortlessmetrics/perl-lsp:0.12.3-perl perl-lsp-rs --version
```

**Expected output**: `perl-lsp-rs 0.12.3` (or `perllsp 0.12.3`)

**If it fails**:
- `docker: Error response from daemon: manifest unknown`: The image did not push. Check the `docker-publish.yml` run logs for push errors.
- Binary exits with error: The release binary has a runtime defect. This is a serious regression; open an issue, yank if on crates.io (via `cargo yank`), and publish 0.12.4 with the fix.
- Do not tag the image manually — re-run the workflow to rebuild.

---

## Post-Publish Verification

### 21. Run the full post-publish smoke test

**What**: Execute the xtask smoke test for 0.12.3, which installs from crates.io and validates the binary.

**Why**: This is the end-user install path. It catches packaging issues (missing files, broken `cargo install`, wrong binary name) that only appear when installing from the registry rather than building locally.

**Command**:
```bash
just smoke-test-release 0.12.3
```

**Expected output**: The smoke test exits 0. The installed binary prints the correct version and passes basic health checks.

**If it fails**: Identify whether the failure is in `cargo install` (registry issue) or in the binary itself (runtime issue). For registry issues, check if the crate is fully indexed yet (may take a few minutes after publish). For binary issues, open a bug and plan 0.12.4.

---

### 22. Verify docs.rs builds for key crates

**What**: Check that docs.rs has successfully built documentation for at least 5 key crates at 0.12.3.

**Why**: docs.rs builds are triggered automatically after crates.io publish. A failed docs.rs build is visible to users and signals a public API or feature-flag documentation issue.

**Command**:
```bash
# Check build status via docs.rs API (adjust crate names as needed)
for crate in perl-lsp-rs perl-parser perl-lexer perl-parser-core perl-semantic-analyzer; do
  STATUS=$(curl -fsSL "https://docs.rs/crate/${crate}/0.12.3/status.json" 2>/dev/null \
    | python3 -c "import json,sys; d=json.load(sys.stdin); print(d.get('build_status', 'unknown'))" 2>/dev/null \
    || echo "not_yet")
  echo "${crate}: ${STATUS}"
done
```

**Expected output**: Each crate reports `success`. Note: docs.rs builds are queued and may take 10-30 minutes to appear after crates.io indexing.

**If it fails**: A `failure` status on docs.rs means the documentation did not compile. Common causes: missing feature flags in `doc` attributes, platform-specific code without `cfg` guards. Fix in 0.12.4. A `not_yet` result just means the build is queued; wait and re-check.

---

### 23. Verify the GitHub Release page

**What**: Confirm the GitHub Release at `v0.12.3` has auto-generated notes, the correct tag, and binary assets attached.

**Why**: The GitHub Release page is the primary user-facing artifact discovery point. Missing assets or garbled notes are immediately visible.

**Command**:
```bash
gh release view v0.12.3
```

**Expected output**:
- Tag: `v0.12.3`
- Title: "Release v0.12.3" (or similar)
- Body: Changelog section content (not placeholder text)
- Assets: Platform binaries (`.tar.gz` / `.zip` for linux, macOS, Windows) and the `.vsix` file

**If it fails**: If notes are missing or garbled, edit the release body via `gh release edit v0.12.3 --notes "..."`. If assets are missing, re-trigger `release.yml` or `publish-extension.yml` as appropriate.

---

### 24. Verify cargo install path works

**What**: Confirm `cargo install perllsp` resolves to 0.12.3.

**Why**: `cargo search` and the sparse index confirm publication, but `cargo install` exercises the full install path including build. This is the most common user-facing install method.

**Command**:
```bash
cargo search perllsp --limit 1
# Expected: perllsp = "0.12.3"

cargo search perl-lsp-rs --limit 1
# Expected: perl-lsp-rs = "0.12.3"
```

**Expected output**: Both searches return the 0.12.3 version as the latest.

**If it fails**: `cargo search` hits a different cache layer than the sparse index and may lag by up to 10 minutes. Wait and retry before concluding there is a problem.

---

## Cleanup

### 25. Close release-blocking issues that are now fixed

**What**: Review the GitHub issue tracker for any issues tagged as release-blocking for 0.12.3 and close them with a reference to the release.

**Why**: Leaving fixed issues open creates false signals for future triage and swarm routing.

**Command**:
```bash
gh issue list --label "release-blocking" --state open
# For each resolved issue:
gh issue close <ISSUE_NUMBER> --comment "Fixed in v0.12.3 — see release: https://github.com/effortlessmetrics/perl-lsp/releases/tag/v0.12.3"
```

**Expected output**: No open release-blocking issues remain after closure.

---

### 26. Verify status docs are current

**What**: Confirm the auto-generated status files (`docs/project/status/*.md`) reflect the 0.12.3 state.

**Why**: The `post-merge-status.yml` workflow regenerates status docs after merges. The version bump merge should have triggered this. Verify that the metrics shown are not stale.

**Command**:
```bash
gh run list --workflow post-merge-status.yml --limit 3
# Confirm the most recent run triggered by the version bump merge completed successfully.
```

**Expected output**: A recent successful run of `post-merge-status.yml` exists, triggered by the version bump PR merge.

**If it fails**: Trigger manually: `gh workflow run post-merge-status.yml` or run `just status-update` locally and push.

---

### 27. Post release announcement (after confirming all steps green)

**What**: Post a release announcement in GitHub Discussions (the content is prepared separately from this runbook).

**Why**: This is the human communication step. Only do this after all technical verification passes. A premature announcement followed by a yank is harmful to trust.

**Action**: Navigate to `https://github.com/effortlessmetrics/perl-lsp/discussions` and create a new post in the "Releases" or "Announcements" category. Use the pre-prepared announcement content from `docs/project/RELEASE_NOTES_DRAFT.md` as a starting point, but note that draft targets 0.13.0 — trim to what 0.12.3 actually contains.

---

## Rollback Procedures

The following procedures apply **after** a publish has gone out. Prevention (catch issues in pre-flight) is always preferred.

### If crates.io publish is partially broken

crates.io does not support unpublish. Options:

```bash
# Yank a specific crate version (marks it as do-not-use, does not delete)
cargo yank --version 0.12.3 perl-lsp-rs

# After fixing the issue, publish 0.12.4 with the fix.
# The yank remains; users on 0.12.3 will be warned by cargo.
```

Do not yank every crate — only the ones with the defect. Yanking leaf crates is usually unnecessary unless they have a security or data-corruption issue.

### If the VSCode extension has a critical defect

```bash
# Unpublish from VSCode Marketplace (removes from search; existing installs may auto-update to next version)
npx @vscode/vsce unpublish EffortlessMetrics.perl-lsp-rs@0.12.3

# Unpublish from Open VSX
# Open VSX does not support CLI unpublish; contact open-vsx.org support or publish 0.12.4 immediately.

# Then publish 0.12.4 with the fix.
gh workflow run publish-extension.yml --field version=0.12.4
```

### If the Docker image has a critical defect

```bash
# Retag latest to point to a known-good previous version
docker pull ghcr.io/effortlessmetrics/perl-lsp:0.12.2
docker tag ghcr.io/effortlessmetrics/perl-lsp:0.12.2 ghcr.io/effortlessmetrics/perl-lsp:latest
docker push ghcr.io/effortlessmetrics/perl-lsp:latest

# Rebuild and re-push the fixed image as 0.12.4
gh workflow run docker-publish.yml --field version=0.12.4 --field push=true
```

### If the GitHub Release has wrong notes or missing assets

```bash
# Edit release notes in place (does not affect crates or extension)
gh release edit v0.12.3 --notes "corrected notes here"

# Delete and recreate the release (keeps the tag, removes the release page)
gh release delete v0.12.3 --yes
gh release create v0.12.3 --generate-notes --title "Release v0.12.3"
# Then re-upload assets:
gh release upload v0.12.3 <asset-file> --clobber
```

### General principle

> 0.12.3 is a rehearsal. If something breaks, that is the point — note it, fix it in 0.12.4, and update this runbook before cutting 0.13.0. Do not rush to restore; the breakage is information.

---

## Go/No-Go Decision Point

Before proceeding to 0.13.0 (public alpha announcement), all of the following must be true:

- [ ] `just smoke-test-release 0.12.3` passes
- [ ] All 130+ crates appear in the sparse index at 0.12.3 (verified by `publish-crates.yml` verify job)
- [ ] VSCode Marketplace and Open VSX both show 0.12.3
- [ ] `docker run --rm effortlessmetrics/perl-lsp:0.12.3-perl perl-lsp-rs --version` prints `0.12.3`
- [ ] GitHub Release page is clean (notes, assets, correct tag)
- [ ] docs.rs builds succeeded for key crates
- [ ] No open severity-1 bugs found during the rehearsal

If any box is unchecked, fix the issue and document what failed in the runbook before cutting 0.13.0.
