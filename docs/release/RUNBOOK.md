# Release Pipeline Runbook

Pre-release checks, release execution, and recovery procedures for the 6 failure
modes from the 2026-04-08 incident, plus the GIT_DIR clobber risk and multi-PR
Cargo.toml race documented in project memory.

---

## Pre-Release Checklist

Run these checks **before** triggering the release orchestration workflow.

### 1. Version bump

```bash
just bump-version X.Y.Z
git diff                          # review: Cargo.toml, features.toml, vscode-extension/package.json
git commit -am "chore(release): bump version to X.Y.Z"
git push origin HEAD
# Open PR, wait for CI green, merge
```

### 2. CHANGELOG update

```bash
grep "## \[X.Y.Z\]" CHANGELOG.md || echo "MISSING — add version section before release"
```

The CHANGELOG must have a `## [X.Y.Z]` section. The release orchestration workflow
checks for this and emits a warning if it finds only `## [Unreleased]`.

### 3. features.toml consistency check

```bash
just ci-parser-features-check   # checks GA+advertised features have tests, no duplicates
```

### 4. Crate allowlist drift check (most important)

The publish allowlist at `[workspace.metadata.publish.allow]` in `Cargo.toml` is
hand-maintained. New crates are silently excluded until a live publish fails.

```bash
just publish-allowlist-check
# equivalent: cargo metadata --format-version=1 --no-deps | python3 scripts/publish-topo.py --check-drift
```

To list all workspace crate names for manual comparison against the allowlist:

```bash
cargo metadata --no-deps --format-version 1 | \
  python3 -c 'import json,sys; [print(p["name"]) for p in json.load(sys.stdin)["packages"]]' | sort
```

Crates with `publish = false` in their `Cargo.toml` should be absent from the
allowlist; all others should be present.

### 5. LICENSE file presence check

```bash
ls LICENSE-MIT LICENSE-APACHE 2>&1  # both must exist at workspace root
```

Missing files cause `cargo publish` to fail with a packaging error that looks like
a manifest validation failure rather than a missing-file error.

### 6. Duplicate TOML section check

Before merging any PR that adds a new `[section]` to `Cargo.toml` (see FM-2 for recovery):

```bash
python3 -c "
import re,sys; text=open('Cargo.toml').read()
headers=re.findall(r'^\[[\w.\"]+\]',text,re.MULTILINE)
from collections import Counter
[print('DUPLICATE:',h) or sys.exit(1) for h,n in Counter(headers).items() if n>1]
print('OK')
"
```

### 7. Publish dry-run

```bash
just publish-dry-run
```

This packages every allowlisted crate in topological order (stripping dev-deps,
mirroring the actual publish workflow) without touching crates.io. This gate runs
automatically in CI on PRs that touch `Cargo.toml`.

### 8. Full release gate

```bash
just release-check  # release-gate + semver-check + changelog section check
```

---

## Release Execution

Once the version-bump PR is merged and all pre-release checks pass:

```bash
# 1. Confirm master is green and the workspace version matches
grep '^version' Cargo.toml | head -1
gh run list --branch master --limit 5

# 2. Trigger release orchestration (creates tag, dispatches publish cascade)
gh workflow run release-orchestration.yml \
  -f version=X.Y.Z \
  -f prerelease=false

# 3. Monitor the cascade
gh run list --workflow=release-orchestration.yml --limit 5
gh run list --workflow=publish-crates.yml --limit 5
gh run list --workflow=publish-extension.yml --limit 5
```

The orchestration workflow validates version + CI state, creates annotated tag
`vX.Y.Z`, then dispatches `release.yml`, `publish-crates.yml`,
`publish-extension.yml`, and `docker-publish.yml` in parallel.

---

## Failure Modes

### FM-1: Allowlist Drift

**Symptoms.** `publish-crates.yml` computes the topological order and either emits
`ERROR: Publish allowlist is empty` or skips a crate entirely, leaving gaps in the
published set. A new crate added to the workspace but not added to
`[workspace.metadata.publish.allow]` is silently omitted — no error, just a missing
publish.

**Root cause.** The allowlist at `[workspace.metadata.publish.allow]` in the root
`Cargo.toml` is hand-maintained. The topological-sort script filters to only
allowlisted crates, so a missing entry means the crate is never published. The
publish-dry-run CI gate does catch this, but only on PRs that touch `Cargo.toml` —
not on workspace restructures that don't touch the root manifest.

**Recovery.**

```bash
# Find the missing crate(s)
just publish-allowlist-check

# Add to Cargo.toml [workspace.metadata.publish.allow] in correct topo order
# Then re-trigger the publish workflow:
gh workflow run publish-crates.yml -f version=X.Y.Z -f dry_run=false
```

Retrigger is **safe**: the workflow fast-paths any crate already present in the
crates.io sparse index and only publishes what is missing.

---

### FM-2: Duplicate TOML Keys

**Symptoms.** `cargo metadata` (or `cargo build`) fails with:

```
error: could not parse TOML file
  --> Cargo.toml:N:1
   |
 N | [workspace.dependencies]
   | ^^^^^^^^^^^^^^^^^^^^^^^^ duplicate key
```

The workspace CI breaks for everyone. The failure typically surfaces when two PRs
each add a section header (e.g., `[workspace.dependencies]`) to `Cargo.toml` and
both are merged without resolving the structural duplication.

**Root cause.** Two concurrent PRs independently add the same TOML section. Git
detects no textual conflict because the hunks are at different line numbers, so the
merge succeeds silently. TOML parsing then fails at runtime. (See project memory:
multi-PR Cargo.toml race.)

**Recovery.** Run the duplicate-header check from the pre-release checklist (step 6)
to locate the duplicate. Manually merge the two sections into one, then:

```bash
cargo metadata --no-deps --format-version 1 > /dev/null  # verify parseable
git commit -m "fix(cargo): deduplicate TOML section header"
```

Retrigger is **safe** after the fix is merged.

---

### FM-3: 429 Rate Limit (crates.io)

**Symptoms.** CI log contains:

```
429 Too Many Requests
Publish attempt N hit 429 rate limit, waiting 60s before retry...
```

or `cargo publish` exits non-zero with `"too many requests"` in stderr.

**Root cause.** crates.io enforces two separate rate limits:
- *Updating an existing crate*: burst 30, then 1 per 12 s steady-state.
- *Publishing a brand-new crate*: burst 5, then 1 per 60 s steady-state.

The publish workflow sleeps 13 s between crates to stay under the update limit. If
many crates are *new* to crates.io (first-ever publish), the 5-burst new-crate limit
exhausts quickly, causing 429s. The workflow retries with a 60 s back-off, but a
wave of new crates may still fail all 3 attempts.

**Recovery.**

For new crates, use the manual staggered script instead of the bulk workflow:

```bash
# Dry-run first (safe):
DRY_RUN=true just publish-new-crates

# Live run (requires CARGO_REGISTRY_TOKEN):
CARGO_REGISTRY_TOKEN=<token> just publish-new-crates
```

See `docs/reference/MANUAL_PUBLISH_NEW_CRATES.md` for full context.

If you already triggered the workflow and it partially succeeded, retriggering is
**safe**: the sparse-index fast-path skips already-published crates.

---

### FM-4: Exit-on-Failure Cascade

**Symptoms.** The publish job stops partway through the crate list. Subsequent
crates that depend on the failed crate are never attempted. CI exits 1 and the step
summary lists `FAILED CRATES`.

**Root cause.** The publish loop accumulates failures in `FAILED_CRATES[]` and
continues, but if the *compute-order* job fails (e.g., an empty allowlist or a
parsing error), all downstream jobs are skipped entirely because they `needs:
compute-order`. The verify job runs `if: always()` to catch partial publish runs,
but it can only report what the compute-order job produced.

**Recovery.**

1. Check whether `compute-order` passed:
   ```bash
   gh run view <run-id> --log-failed
   ```
2. If `compute-order` failed, fix the allowlist or TOML issue first (see FM-1/FM-2).
3. Retrigger:
   ```bash
   gh workflow run publish-crates.yml -f version=X.Y.Z -f dry_run=false
   ```

Retrigger is **safe**: the sparse-index fast-path skips already-published crates.

---

### FM-5: Missing LICENSE Files

**Symptoms.** `cargo publish -p <crate>` exits non-zero with:

```
error: failed to prepare local package for uploading
  ...
  license-file "LICENSE-MIT" does not appear to exist
```

or the production-gates-validation script reports:

```
Gate FAILED: License Files Missing
```

**Root cause.** `cargo publish` packages the crate's source directory plus any
`license-file` entries declared in `Cargo.toml`. If the workspace root
`LICENSE-MIT` or `LICENSE-APACHE` files are absent (e.g., accidentally deleted,
or a new crate declares a `license-file` pointing to a nonexistent path), packaging
fails for every crate that references those files.

**Recovery.**

```bash
# Verify files at workspace root:
ls -la LICENSE-MIT LICENSE-APACHE

# If missing, restore from git:
git show HEAD:LICENSE-MIT > LICENSE-MIT
git show HEAD:LICENSE-APACHE > LICENSE-APACHE

# Re-run the dry-run gate to confirm:
just publish-dry-run
```

Retrigger is **safe** after the files are restored and pushed.

---

### FM-6: Retrigger Safety

**Summary.** Retriggering `publish-crates.yml` after any partial failure is always
safe. The workflow checks the crates.io sparse index before each `cargo publish`
call and skips crates that are already present at the target version. This means a
re-run is idempotent — it publishes only what is missing.

```bash
gh workflow run publish-crates.yml -f version=X.Y.Z -f dry_run=false
```

To verify which crates are already published before retriggering:

```bash
# Check a specific crate in the sparse index:
CRATE=perl-parser; VERSION=X.Y.Z
PATH_KEY=$(python3 -c "n='${CRATE}'.lower(); print(('1/'+n if len(n)==1 else '2/'+n if len(n)==2 else '3/'+n[0]+'/'+n if len(n)==3 else n[:2]+'/'+n[2:4]+'/'+n))")
curl -fsSL "https://index.crates.io/${PATH_KEY}" | grep '"vers":"'${VERSION}'"'
```

---

## README Clobber Risk (GIT_DIR Env Leak)

**Symptoms.** After a pre-push hook runs xtask, `git diff HEAD` shows an unexpected
change to `README.md` (or another generated file) that was not authored in the
current branch. The file contains content from a different worktree.

**Root cause.** When a git pre-push hook spawns a subprocess (e.g., `cargo xtask
...`), git injects `GIT_DIR`, `GIT_WORK_TREE`, and related environment variables
into the hook's environment. If xtask spawns a child `git` process without clearing
these variables, the child operates against the hook's worktree (the one that
triggered the push) rather than the subprocess's own working directory. Any `git`
write operation in the child (e.g., `git commit`, file write via a git command)
lands in the wrong worktree. This was fixed in xtask's `run_git` helper
(see `xtask/src/tasks/hook_checks.rs` Layer 2 — GIT_DIR isolation comment).

**Prevention.** Any xtask function that spawns a `git` subprocess must call
`.env_remove("GIT_DIR").env_remove("GIT_WORK_TREE").env_remove("GIT_INDEX_FILE")`
on the `Command` builder. Write-once generated files (README.md, CHANGELOG sections)
should have integrity checks that fail loudly if content changes unexpectedly.

**Recovery.**

```bash
# If README or generated file was clobbered in a recent commit:
git show HEAD:README.md > /tmp/good-readme.md
# compare with what's actually there
git diff HEAD README.md

# Restore and amend, or create a fix commit:
git restore README.md
git commit --allow-empty -m "fix: restore README.md after GIT_DIR clobber"
```

---

## Multi-PR Cargo.toml Race

Two PRs can introduce a structural conflict in `Cargo.toml` that git does not
detect as a merge conflict (e.g., both add `[workspace.dependencies]` at different
line numbers). Git merges cleanly; TOML parsing fails at runtime.

**Prevention.** After merging PR A, rebase PR B and run the duplicate-header check
(pre-release checklist step 6) before merging:

```bash
gh pr update-branch <PR-B-number>
# then run step 6 check
```

Limit concurrent builders to one per file when work touches the root `Cargo.toml`.

---

## `gh run rerun` vs `gh pr update-branch`

When master receives a CI fix (e.g., a test or workflow change), do not use
`gh run rerun` on queued PRs. `gh run rerun` re-runs the *existing* CI run which
still uses the old workflow/test code. Instead:

```bash
# Pull the CI fix into the PR branch from master:
gh pr update-branch <PR-number>

# CI then triggers fresh from the updated branch tip.
```

Use `gh run rerun` only to retry transient infrastructure failures (network errors,
runner OOM) on the *same* code.

## Quick Reference

| Failure | Recovery command | Retrigger safe? |
|---------|-----------------|-----------------|
| Allowlist drift | `just publish-allowlist-check` then fix + retrigger | Yes |
| Duplicate TOML key | Deduplicate section + merge fix | Yes |
| 429 rate limit | `CARGO_REGISTRY_TOKEN=... just publish-new-crates` | Yes |
| Exit-on-failure cascade | Fix root cause + `gh workflow run publish-crates.yml` | Yes |
| Missing LICENSE files | `git show HEAD:LICENSE-MIT > LICENSE-MIT` | Yes |
| Retrigger safety | Always safe; sparse-index fast-path skips published crates | Yes |
| GIT_DIR clobber | `git restore <file>`; fix xtask `.env_remove("GIT_DIR")` | N/A |
| Multi-PR TOML race | `gh pr update-branch`; dedup headers | After fix |
