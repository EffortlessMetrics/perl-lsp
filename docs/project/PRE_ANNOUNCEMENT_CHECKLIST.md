# Pre-Announcement Checklist — v0.13.0 Go/No-Go

> **Purpose**: Make "is v0.13.0 ready to announce?" an objective pass/fail decision, not a vibe.
> Every item has a verification command or evidence link. Sign-off items require a human to fill in the date and initials.

## Status Legend

| Symbol | Meaning |
|--------|---------|
| ✓ | Done — verified with evidence |
| ⚠ | In progress — tracking issue/PR linked |
| ✗ | Blocker — not started or failing |
| — | N/A for this release |

**Last updated**: 2026-04-08
**Target version**: v0.13.0
**Verification script**: `just release-check` (superset gate — runs ci-gate + sbom + semver + no-panic + changelog + dry-run)

---

<details>
<summary><strong>1. Build &amp; Publish Gates (automated)</strong></summary>

These items should all be ✓ before the version-bump PR is merged.

- [ ] **Master CI gate green** — `just ci-gate` passes on master HEAD.
  - Verify: `gh run list --branch master --limit 3` — most recent CI run shows `completed / success`.
  - Current status: ✓ — CI gate consistently green across session merges as of 2026-04-08.

- [ ] **Nightly CI gate green** — mutation testing, bounded fuzz, and coverage tier all pass.
  - Verify: `gh run list --workflow=ci-nightly.yml --limit 3` — most recent nightly shows success.
  - Current status: ⚠ — confirm green before announcement.

- [ ] **`cargo audit` clean** — no CRITICAL or HIGH CVEs in the dependency tree.
  - Verify: `just security-audit` (runs `cargo audit`; non-blocking in gate but must be reviewed).
  - Current status: ⚠ — `deny.toml` present; run against v0.13.0 tree and sign off.

- [ ] **`cargo deny` clean** — license policy, duplicate crate policy, and ban list pass.
  - Verify: `cargo deny check` — must exit 0.
  - Config: `deny.toml` exists at repo root.
  - Current status: ⚠ — `deny.toml` present; run against v0.13.0 tree and sign off.

- [ ] **Publish allowlist complete** — every crate that should be on crates.io is in `[workspace.metadata.publish.allow]`.
  - Verify: `cargo metadata --format-version=1 --no-deps` — cross-check against `Cargo.toml` publish allow list.
  - Current status: ✓ — `perl-workspace-index-monitoring` and `perl-test-generators` added in #3296 (merged 2026-04-08). Allowlist audited for v0.13.0.

- [ ] **LICENSE files present for all publishable crates** — every crate with `publish = true` has a `LICENSE` or `LICENSE-MIT`/`LICENSE-APACHE` file.
  - Verify: `find crates/ -name 'LICENSE*'` — cross-check against publish allow list.
  - Current status: ✓ — #3304 (merged 2026-04-08) corrected/added LICENSE files for 4 previously-broken crates.

- [ ] **docs.rs metadata present for all published crates** — all 128 publishable crates have `[package.metadata.docs.rs]` entries with `all-features = true` and `rustdoc-args = ["--cfg", "docsrs"]`.
  - Verify: `grep -r "metadata.docs.rs" crates/*/Cargo.toml | wc -l` — should match published crate count.
  - Current status: ✓ — #3299 (merged 2026-04-08) added docs.rs metadata across all 128 published crates.

- [ ] **crates.io metadata polished** — all publishable crates have accurate descriptions, categories, and keywords.
  - Verify: `cargo metadata --format-version=1 --no-deps` — spot-check descriptions.
  - Current status: ✓ — #3281 (merged 2026-04-08) polished crates.io metadata across all publishable crates.

- [ ] **tree-sitter-perl-c publishable** — `tree-sitter-perl-c` crate has all required metadata for first publish.
  - Verify: `cargo publish -p tree-sitter-perl-c --dry-run`.
  - Current status: ✓ — #3273 (merged 2026-04-08) polished tree-sitter-perl-c for first publish to crates.io.

- [ ] **Publish dry-run CI gate** — CI runs `cargo publish --dry-run` on every Cargo.toml change to catch regressions before they block release.
  - Verify: `gh workflow list` shows `publish-dry-run.yml` is active.
  - Current status: ✓ — #3301 (merged 2026-04-08) added publish dry-run gate to CI.

- [ ] **Publish rate-limit throttle in place** — workflow throttles crates.io publishes to avoid 429 errors.
  - Verify: `.github/workflows/publish.yml` — confirms per-crate delay between publishes.
  - Current status: ✓ — #3307 (merged 2026-04-08) added publish throttle to avoid 429 rate limits.

- [ ] **vsce publish idempotent** — re-running `publish-extension.yml` on the same version does not fail.
  - Verify: `.github/workflows/publish-extension.yml` — confirm idempotency guard present.
  - Current status: ✓ — #3267 (merged 2026-04-08) made vsce publish idempotent on re-run.

- [ ] **All publishable crates on crates.io at v0.13.0** — every crate in `[workspace.metadata.publish.allow]` is live at the target version.
  - Verify: `cargo search perl-lsp-rs --limit 1` returns `"0.13.0"`.
  - Current status: ✗ — workspace at `0.12.2`; version-bump to `0.13.0` not yet triggered. v0.12.2 publish retrigger in flight (watch-trigger aabddeabc5) as of 2026-04-08 — wait for completion before bumping.

- [ ] **All published crates building on docs.rs at v0.13.0** — no docs.rs build failures.
  - Verify: `https://docs.rs/perl-lsp-rs/0.13.0` and `https://docs.rs/perllsp/0.13.0` load without errors.
  - Current status: ✗ — depends on crates.io publish above; check after release workflow completes.

- [ ] **Docker image published at v0.13.0** — multi-arch image on Docker Hub and GHCR.
  - Verify: `docker pull effortlessmetrics/perl-lsp:0.13.0 && docker run --rm effortlessmetrics/perl-lsp:0.13.0 perllsp --version`
  - Workflow: `.github/workflows/docker-publish.yml`
  - Current status: ✗ — pending version bump and release workflow.

- [ ] **VSCode extension published to Marketplace at v0.13.0** — listing shows version `0.13.0`.
  - Verify: `https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs`
  - Workflow: `.github/workflows/publish-extension.yml`
  - Current status: ✗ — pending version bump.

- [ ] **Open VSX extension published at v0.13.0** — `https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs` shows `0.13.0`.
  - Current status: ✗ — published by same `publish-extension.yml` workflow; pending version bump.

- [ ] **GitHub Release created for v0.13.0 with binary assets attached** — all 7 platform tarballs, `SHA256SUMS`, `sbom-spdx.json`, and `.vsix`.
  - Verify: `gh release view v0.13.0`
  - Expected assets: `perllsp-0.13.0-{x86_64,aarch64}-unknown-linux-{gnu,musl}.tar.gz`, `perllsp-0.13.0-{x86_64,aarch64}-apple-darwin.tar.gz`, `perllsp-0.13.0-x86_64-pc-windows-msvc.zip`, `SHA256SUMS`, `sbom-spdx.json`, `perl-lsp-rs-0.13.0.vsix`
  - Current status: ✗ — pending release workflow.

- [ ] **Post-publish smoke test passes** — 5-project smoke protocol from `docs/project/PUBLISHING_ROADMAP.md` Part 2 passes against released v0.13.0 binary.
  - Verify: Follow smoke protocol with `perllsp --version` confirming `0.13.0`.
  - Current status: ✗ — cannot run until published binary is available. Smoke test CI job added in #3288 (merged 2026-04-08).

- [ ] **0.12.3 pipeline rehearsal completed** — v0.12.3 release runbook executed as a dry-run rehearsal before v0.13.0.
  - Verify: `gh release view v0.12.3` — confirms rehearsal release was created.
  - Current status: ⚠ — runbook documented in #3298 (merged 2026-04-08); [0.12.3] CHANGELOG entry drafted in #3295 (merged 2026-04-08). Execution pending.

</details>

---

<details>
<summary><strong>2. Quality Gates (manual sign-off required)</strong></summary>

Each item requires a human to verify and record sign-off date + initials.

- [ ] **Banned-constructs audit clean** — zero `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` in production library code outside the one allowlisted exception (`crates/perl-lsp/src/util/uri.rs`).
  - Verify: `just ci-unwrap-panic-ratchet` — must pass. Also `just clippy-prod-no-unwrap`.
  - Current status: ✓ — `docs/project/status/index.md` records `unwrap/expect=0`, panic-family=0, unsafe=0 in production baseline as of 2026-04-07. Unsafe blocks documented or eliminated in #3292 (merged 2026-04-08). Confirm on v0.13.0 tree with `just ci-unwrap-panic-ratchet`.
  - Sign-off: _______________ Date: _______________

- [ ] **Security scout findings: all CRITICAL + HIGH addressed** — either fixed (with PR link) or consciously deferred (with issue link and rationale).
  - Verify: `just security-audit` output reviewed. Check `gh issue list --label security --state open`.
  - Current status: ⚠ — no open security issues visible; fresh run required against v0.13.0 tree.
  - Sign-off: _______________ Date: _______________

- [ ] **UX P0 blockers addressed** — all P0 UX issues resolved or merged before announcement.
  - Verify: `gh issue list --label "ux-p0" --state open` returns empty.
  - Current status: ⚠ — 7 UX P0 items tracked; status as of 2026-04-08:
    - ✓ #3291 actionable error messages — merged 2026-04-08
    - ✓ #3269 extension settings schema polish — merged 2026-04-08
    - ✓ #3310 binary distribution model documented — merged 2026-04-08
    - ⚠ #3306 actionable binary download errors — open, in review
    - ⚠ #3308 real LSP startup errors instead of generic message — open, in review
    - ⚠ #3309 warn on undetected workspace root — open, in review
    - ⚠ #3312 specific error for missing Perl interpreter — open, in review
  - Sign-off (after all P0 PRs merge): _______________ Date: _______________

- [ ] **UX regression gate active** — CI runs a UX regression check on every PR touching LSP/DAP/extension.
  - Verify: `gh workflow list` shows `ux-regression.yml` (or equivalent) is active.
  - Current status: ✓ — #3293 (merged 2026-04-08) added UX regression gate to CI.

- [ ] **LSP feature gap: all P0 blockers addressed** — advertised features in `features.toml` are working.
  - Verify: Spot-check 5 advertised features end-to-end in a real editor.
  - Reference: `features.toml` (116 features as of 2026-04-11, corrected from 102 in PR #4107 after a DAP catalog audit).
  - Current status: ⚠ — AI inline completion (#3018) shipped in v0.12.2 but awaits E2E user validation. Mark ✓ only after human spot-check.
  - Sign-off: _______________ Date: _______________

- [ ] **Parser corpus baseline: no NEW regressions vs v0.12.2** — `just cpan-corpus-check` passes.
  - Verify: `just cpan-corpus-check` exits 0.
  - Baseline: 85.4% clean rate (3,717/4,355) as of 2026-03-20.
  - Current status: ⚠ — run on the v0.13.0 tree and confirm no regression.
  - Sign-off: _______________ Date: _______________

- [ ] **`features.toml` accuracy** — every feature with `status = "ga"` or `status = "preview"` is present and working.
  - Verify: Cross-check `features.toml` against `just ci-gate` output. Spot-test at least 3 features manually.
  - Current status: ✓ — `features.toml` is the canonical catalog; 116 features tracked and consistent with `docs/project/status/lsp.md` (87 LSP + 24 DAP + 5 extension; DAP corrected in PR #4107).
  - Sign-off: _______________ Date: _______________

</details>

---

<details>
<summary><strong>3. Documentation Gates</strong></summary>

- [ ] **README badges all resolve** — CI badge, GitHub release badge, license badge, Rust version badge, status badges all load without errors.
  - Verify: Open `README.md` in a browser and confirm all badges render.
  - Current status: ✓ — #3266 (merged 2026-04-08) added status badges to top-level README and tree-sitter crate READMEs.

- [ ] **CONTRIBUTING.md reflects current workflow** — describes `just ci-gate`, `nix develop` path, and pipeline for v0.13.0 public alpha contributors.
  - Verify: `head -80 CONTRIBUTING.md` — confirms setup, CI, and PR process are current.
  - Current status: ✓ — #3264 (merged 2026-04-08) refreshed CONTRIBUTING.md for the v0.13.0 public alpha audience.

- [ ] **CODE_OF_CONDUCT.md exists** — present and points to a contact method.
  - Verify: `ls CODE_OF_CONDUCT.md`
  - Current status: ✓ — exists at repo root.

- [ ] **CHANGELOG.md has a [0.12.2] entry** — v0.12.2 release changes documented with date.
  - Verify: `grep "## \[0.12.2\]" CHANGELOG.md`
  - Current status: ✓ — #3303 (merged 2026-04-08) added the dated [0.12.2] entry to CHANGELOG.md.

- [ ] **CHANGELOG.md has a [0.12.3] draft entry** — pipeline rehearsal release changes documented.
  - Verify: `grep "## \[0.12.3\]" CHANGELOG.md`
  - Current status: ✓ — #3295 (merged 2026-04-08) drafted the [0.12.3] changelog entry.

- [ ] **CHANGELOG.md has a v0.13.0 entry** — a dated `## [0.13.0] - YYYY-MM-DD` section narrating the release.
  - Verify: `grep "## \[0.13.0\]" CHANGELOG.md`
  - Current status: ✗ — `[Unreleased]` section targets v0.13.0 changes. Must be promoted to a dated versioned section before tagging. Automated by `version-bump.yml`.

- [ ] **`features.toml` version matches target** — `[meta].version` is `"0.13.0"`.
  - Verify: `grep '^version' features.toml` returns `version = "0.13.0"`.
  - Current status: ✗ — currently at `0.12.2`. Updated by `gh workflow run version-bump.yml`.

- [ ] **`docs/project/status/index.md` narrative is current** — release posture line reflects v0.13.0.
  - Verify: "What's True Right Now" section describes v0.13.0 as the announced release.
  - Current status: ✗ — currently describes v0.12.x milestones; needs update after version bump.

- [ ] **Per-crate ROADMAP.md headers are fresh** — per-crate ROADMAP files reflect current state.
  - Verify: Spot-check 3 crates under `crates/*/ROADMAP.md`.
  - Current status: ✓ — #3286 (merged 2026-04-08) refreshed stale per-crate ROADMAP.md headers.

- [ ] **tree-sitter-perl-c README is first-impression polished** — stands alone without internal project context.
  - Verify: Read `crates/tree-sitter-perl-c/README.md`.
  - Current status: ✓ — #3273 (merged 2026-04-08) polished tree-sitter-perl-c README for crates.io visitors.

- [ ] **tree-sitter-perl (highlight corpus) README is accurate** — explains this is not a Rust crate.
  - Verify: Read `crates/tree-sitter-perl/README.md`.
  - Current status: ✓ — README clearly states "not a Rust crate" and explains contents.

- [ ] **VSCode Marketplace punch list complete** — all listing fields verified.
  - Verify: `node -p "JSON.stringify(require('./vscode-extension/package.json'), null, 2)"` — check `icon`, `galleryBanner`, `categories`, `keywords`, `engines.vscode`.
  - Current status: ✓ — #3268 (merged 2026-04-08) documented and verified VSCode Marketplace + Open VSX punch list; `vscode-extension/package.json` has all required fields.

- [ ] **Version-bump automation documented and tested** — `version-bump.yml` workflow ready to run.
  - Verify: `gh workflow list` shows `version-bump.yml`; review last version-bump PR for correctness.
  - Current status: ✓ — #3289 (merged 2026-04-08) centralized version-bump automation. `cargo xtask check-version-sync` tested.

- [ ] **0.12.3 release runbook documented** — pipeline rehearsal playbook exists before triggering the real v0.13.0 release.
  - Verify: `ls docs/project/RELEASE_RUNBOOK_0.12.3.md` (or path from #3298).
  - Current status: ✓ — #3298 (merged 2026-04-08) added the 0.12.3 release runbook.

</details>

---

<details>
<summary><strong>4. Marketplace + Discoverability</strong></summary>

- [ ] **VSCode Marketplace listing complete** — description, categories, icon, gallery banner, minimum VSCode version, and keywords all present.
  - Verify: `node -p "JSON.stringify(require('./vscode-extension/package.json'), null, 2)"` — check all fields.
  - Current status: ✓ — `vscode-extension/package.json` has all required fields: icon, galleryBanner, 5 categories, 10 keywords.

- [ ] **VSCode Marketplace gallery screenshots / GIFs** — at least one screenshot showing the extension in action.
  - Verify: Check `vscode-extension/media/` for `.gif` or screenshot assets.
  - Current status: ✗ — **USER ACTION REQUIRED**: no demo GIF or screenshot exists. Tracked in #3302 (open). Three P0 assets needed: `install-health.gif`, `find-references.gif`, `extract-variable.gif`. See `scripts/marketing/render-walkthrough-gif.py` for recording tooling and `demo_workspace/` for source files. Blocks the Marketplace listing going live.

- [ ] **Open VSX listing matches VSCode Marketplace** — same description, categories, and icon on Open VSX.
  - Verify: `https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs` after publish.
  - Current status: ✗ — pending publish.

- [ ] **Docker Hub listing has description and README content** — meaningful description and tags documented.
  - Verify: `https://hub.docker.com/r/effortlessmetrics/perl-lsp`
  - Current status: ⚠ — Docker workflow exists; listing content not verified against published state.

- [ ] **Binary distribution model documented for enterprise users** — users behind corporate proxies/firewalls know how to install.
  - Verify: Check `docs/` or extension README for enterprise/manual install guidance.
  - Current status: ✓ — #3310 (merged 2026-04-08) documented binary distribution model for enterprise users.

</details>

---

<details>
<summary><strong>5. Ecosystem + Community</strong></summary>

- [ ] **GitHub Discussions enabled with Welcome post** — Discussions tab active and Welcome thread exists.
  - Verify: `https://github.com/EffortlessMetrics/perl-lsp/discussions`
  - Current status: ✓ — Discussions enabled. Welcome post presence: verify URL.

- [ ] **Announcement post drafted in Discussions** — a pinned Discussions post ready to publish on announcement day.
  - Verify: Draft exists in Discussions (draft state) or `docs/project/`.
  - Current status: ✗ — no announcement Discussions post draft found in repo.

- [ ] **Issue templates are friendly and functional** — all templates in `.github/ISSUE_TEMPLATE/` route users correctly.
  - Verify: `ls .github/ISSUE_TEMPLATE/` — 8+ templates exist; open one in browser.
  - Current status: ✓ — 8 templates exist with proper routing. UX-specific templates and triage docs added in #3277 (merged 2026-04-08).

- [ ] **PR template explains the pipeline to external contributors** — `.github/pull_request_template.md` gives first-time contributors enough context.
  - Verify: Read `.github/pull_request_template.md`.
  - Current status: ⚠ — template exists (Summary, Changes, Test, Verification, What-I-Considered). Does not explain Scout→Build→Review→Merge pipeline; add a brief note pointing to `CONTRIBUTING.md`.

- [ ] **Project homepage has an LSP section** — personal site has a landing page or section for perl-lsp.
  - Verify: Visit `https://effortlesssteven.com`.
  - Current status: ✗ — no references in repo; manual check required.

</details>

---

<details>
<summary><strong>6. Announcement Content (external)</strong></summary>

These items confirm drafts EXIST — not that they are final.

- [ ] **Announcement blog post draft exists** — at least one blog post draft ready for editing.
  - Verify: `ls docs/articles/drafts/`
  - Current status: ✓ — 5 drafts exist: `EVERY_DEEP_REVIEW_FOUND_A_BUG.md`, `PERL_DESERVES_BETTER_TOOLING.md` (primary — 600+ words with survey data and competitive analysis), `THE_ONE_CHARACTER_FIX.md`, `THE_PIPELINE_IS_THE_PRODUCT.md`, `WHAT_100_AGENTS_COST.md`.

- [ ] **Demo GIF or screenshot recorded** — at least one visual asset showing completion, diagnostics, or hover in a real editor.
  - Verify: Check `vscode-extension/media/` for `.gif` or screenshot assets.
  - Current status: ✗ — **USER ACTION REQUIRED**: no demo GIF or screenshot found. Tracked in #3302. Required for VSCode Marketplace listing and Reddit/HN posts. See #3302 for full recording spec and `scripts/marketing/render-walkthrough-gif.py` for tooling.

- [ ] **Release notes draft finalized** — `docs/project/RELEASE_NOTES_DRAFT.md` is complete and covers v0.13.0.
  - Verify: `cat docs/project/RELEASE_NOTES_DRAFT.md` — check Highlights, feature list, and statistics.
  - Current status: ✓ — `RELEASE_NOTES_DRAFT.md` exists with Highlights, Features, Breaking Changes, Resolved Issues, and Statistics sections.

- [ ] **Social media posts drafted** — text drafts for: Twitter/X thread, Reddit r/rust, Reddit r/perl, HN Show HN, lobste.rs.
  - Verify: Check `docs/project/LAUNCH_PLAN.md` §3.6 or `docs/project/ANNOUNCEMENT_POSTS.md`.
  - Current status: ⚠ — `docs/project/PUBLISHING_ROADMAP.md` §3.6 has outlines and channel list. Twitter/X thread standalone draft not found. Consider creating `docs/project/ANNOUNCEMENT_POSTS.md` with finalized copy before day-of.

- [ ] **Announcement content reflects session deliverables** — blog post and social posts reference the new UX fixes, docs.rs metadata, full publish allowlist, and other 2026-04-08 session items.
  - Verify: Read `docs/articles/drafts/PERL_DESERVES_BETTER_TOOLING.md` — confirm it describes current feature set.
  - Current status: ⚠ — drafts authored before the 2026-04-08 session; need a final read-through to incorporate session work before posting.

</details>

---

<details>
<summary><strong>7. Operational Readiness</strong></summary>

- [ ] **`just doctor` passes on a clean checkout** — `just doctor` exits 0 and shows healthy status for all subsystems.
  - Verify: `just doctor` (recipe added in #3249, merged 2026-04-04).
  - Current status: ⚠ — recipe exists; run on a fresh checkout to confirm.

- [ ] **Pre-push hook works correctly** — installs cleanly and catches format/lint errors before push.
  - Verify: `bash scripts/install-githooks.sh` on a clean machine.
  - Current status: ✓ — pre-push hook improvements shipped in #3238. Hook fixture isolation fixed in #3205.

- [ ] **On-call / response plan documented** — triage SLA and bug report routing documented.
  - Verify: `docs/project/PUBLISHING_ROADMAP.md` §4.3 — confirms triage SLA.
  - Current status: ✓ — §4.3 documents triage SLA (crash/hang = same day P0, parse error = 48h, feature gap = 1 week) and monitoring plan.

- [ ] **GitHub secrets are configured** — `CARGO_REGISTRY_TOKEN`, `VSCE_PAT`, `OVSX_PAT`, `DOCKER_USERNAME`, `DOCKER_PASSWORD` set in repo.
  - Verify: `gh secret list` — all 5 must appear.
  - Current status: ⚠ — documented in `docs/project/RELEASE_CHECKLIST.md` §Preflight; verify live before triggering release workflow.

- [ ] **Version-bump workflow tested** — `gh workflow run version-bump.yml --field version=0.13.0` produces a clean PR bumping `Cargo.toml`, `vscode-extension/package.json`, `features.toml`, and `CHANGELOG.md`.
  - Verify: Review last version-bump PR or `cargo xtask check-version-sync` logic.
  - Current status: ✓ — version-bump automation centralized in #3289 (merged 2026-04-08); `cargo xtask check-version-sync` tested.

- [ ] **v0.12.2 publish complete and verified** — v0.12.2 is live on crates.io before triggering v0.13.0.
  - Verify: `cargo search perl-lsp-rs --limit 1` returns `"0.12.2"`.
  - Current status: ⚠ — v0.12.2 publish retrigger in flight (watch-trigger aabddeabc5) as of 2026-04-08. Do not bump to v0.13.0 until this completes.

</details>

---

## Summary Dashboard

Quick snapshot at **2026-04-08** against v0.13.0 (workspace currently at v0.12.2):

| Section | ✓ Done | ⚠ In Progress | ✗ Blocker |
|---------|--------|--------------|-----------|
| 1. Build & Publish | 7 | 3 | 7 |
| 2. Quality Gates | 3 | 4 | 0 |
| 3. Documentation | 11 | 0 | 3 |
| 4. Marketplace | 2 | 1 | 2 |
| 5. Community | 2 | 1 | 2 |
| 6. Announcement Content | 2 | 2 | 1 |
| 7. Operational | 3 | 3 | 0 |
| **Total** | **30** | **14** | **15** |

*Baseline at 2026-04-07: 16 done / 11 in-progress / 17 blockers. Session (2026-04-08) added 14 new ✓ items, net reduced blockers by 2 (17 → 15), added in-progress tracking for UX P0 PRs still open.*

### Hard blockers before announcement

1. **Demo GIFs / screenshots** (USER ACTION REQUIRED) — #3302: 3 P0 GIFs must be recorded before VSCode Marketplace listing or announcement posts. Recording guide: `scripts/marketing/render-walkthrough-gif.py`. Source files: `demo_workspace/`.
2. **v0.12.2 publish to complete** — watch retrigger aabddeabc5; do not version-bump until confirmed live.
3. **4 UX P0 PRs to merge** — #3306, #3308, #3309, #3312 are open and in review.
4. **Version bump to v0.13.0** — `gh workflow run version-bump.yml --field version=0.13.0`. Unblocks all "pending version bump" items.
5. **CHANGELOG.md dated v0.13.0 entry** — automated by `version-bump.yml`.
6. **Release workflow** — `gh workflow run release-orchestration.yml --field version=0.13.0 ...` — publishes crates, extension, Docker, creates GitHub Release.
7. **Post-publish smoke** — run 5-project smoke protocol from `docs/project/PUBLISHING_ROADMAP.md` Part 2.

### Announcement content (USER ACTION REQUIRED)

- **Record demo GIFs** — see #3302 for full spec (P0 required before Marketplace goes live)
- **Finalize announcement copy** — drafts in `docs/articles/drafts/`; update to reflect 2026-04-08 session deliverables before posting
- **Write social posts** — outlines in `PUBLISHING_ROADMAP.md §3.6`; finalize as `docs/project/ANNOUNCEMENT_POSTS.md`

### Path to green

```bash
# Prerequisites (wait for these):
# - v0.12.2 publish retrigger completes (watch aabddeabc5)
# - PRs #3306, #3308, #3309, #3312 merge

1. just release-check                  # Confirm master is ready
2. gh workflow run version-bump.yml --field version=0.13.0
   # Review and merge the resulting PR
3. Record 3 P0 demo GIFs (see #3302; use scripts/marketing/render-walkthrough-gif.py)
4. gh workflow run release-orchestration.yml --field version=0.13.0 \
     --field prerelease=false --field skip_crates=false \
     --field skip_extension=false --field skip_docker=false
5. Run smoke protocol (PUBLISHING_ROADMAP.md Part 2)
6. Fill in sign-off lines in Quality Gates section above
7. Post announcements per PUBLISHING_ROADMAP.md §3.6
```

---

*Related documents: [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md) (release mechanics gate) | [PUBLISHING_ROADMAP.md](PUBLISHING_ROADMAP.md) (day-of runbook) | [RELEASE_NOTES_DRAFT.md](RELEASE_NOTES_DRAFT.md) | [LAUNCH_PLAN.md](LAUNCH_PLAN.md)*
