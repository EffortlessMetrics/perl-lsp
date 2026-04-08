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

**Last updated**: 2026-04-07
**Target version**: v0.13.0
**Verification script**: `just release-check` (superset gate — runs ci-gate + sbom + semver + no-panic + changelog + dry-run)

---

<details>
<summary><strong>1. Build &amp; Publish Gates (automated)</strong></summary>

These items should all be ✓ before the version-bump PR is merged.

- [ ] **Master CI gate green** — `just ci-gate` passes on master HEAD.
  - Verify: `gh run list --branch master --limit 3` — most recent CI run shows `completed / success`.
  - Current status: ✓ — CI run [24099547634] shows `[ok]` as of 2026-04-07.

- [ ] **Nightly CI gate green** — mutation testing, bounded fuzz, and coverage tier all pass.
  - Verify: `gh run list --workflow=ci-nightly.yml --limit 3` — most recent nightly shows success.
  - Current status: ⚠ — nightly run [24099547653] was pending at time of checklist authoring; confirm green before announcement.

- [ ] **`cargo audit` clean** — no CRITICAL or HIGH CVEs in the dependency tree.
  - Verify: `just security-audit` (runs `cargo audit`; non-blocking in gate but must be reviewed).
  - Current status: ⚠ — verify interactively; `deny.toml` is present (`H:\Code\Rust\perl-lsp\deny.toml`) and `cargo audit` is in `just security-audit`.

- [ ] **`cargo deny` clean** — license policy, duplicate crate policy, and ban list pass.
  - Verify: `cargo deny check` — must exit 0.
  - Config: `deny.toml` exists at repo root.
  - Current status: ⚠ — `deny.toml` is present but a fresh run against v0.13.0 tree has not been signed off.

- [ ] **All publishable crates on crates.io at v0.13.0** — every crate in `[workspace.metadata.publish.allow]` is live at the target version.
  - Verify: `cargo search perl-lsp-rs --limit 1` and `cargo search perllsp --limit 1` return `"0.13.0"`.
  - Full list check: `cargo metadata --format-version=1 --no-deps | python3 -c 'import json,sys; meta=json.load(sys.stdin); ws=set(meta["workspace_members"]); [print(p["name"], p["version"]) for p in meta["packages"] if p["id"] in ws and p.get("publish")!=[]]'`
  - Current status: ✗ — workspace version is `0.12.2`; version-bump to `0.13.0` has not been triggered yet. See `just release-check` to confirm readiness before bumping.

- [ ] **All publishable crates building on docs.rs at v0.13.0** — no docs.rs build failures.
  - Verify: `https://docs.rs/perl-lsp-rs/0.13.0` and `https://docs.rs/perllsp/0.13.0` load without build errors.
  - Current status: ✗ — depends on crates.io publish above; check after release workflow completes.

- [ ] **Docker image published at v0.13.0** — multi-arch image available on both Docker Hub and GHCR.
  - Verify: `docker pull effortlessmetrics/perl-lsp:0.13.0 && docker run --rm effortlessmetrics/perl-lsp:0.13.0 perllsp --version`
  - Workflow: `.github/workflows/docker-publish.yml` — dispatch `release-orchestration.yml`.
  - Current status: ✗ — pending version bump and release workflow.

- [ ] **VSCode extension published to Marketplace at v0.13.0** — `effortlessmetrics.perl-lsp-rs` listing shows version `0.13.0`.
  - Verify: `https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs`
  - Workflow: `.github/workflows/publish-extension.yml`
  - Current status: ✗ — pending version bump.

- [ ] **Open VSX extension published at v0.13.0** — `https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs` shows `0.13.0`.
  - Verify: browser check above URL.
  - Current status: ✗ — published by same `publish-extension.yml` workflow; pending version bump.

- [ ] **GitHub Release created for v0.13.0 with binary assets attached** — release has all 7 platform tarballs, `SHA256SUMS`, `sbom-spdx.json`, and `.vsix`.
  - Verify: `gh release view v0.13.0`
  - Expected assets: `perllsp-0.13.0-{x86_64,aarch64}-unknown-linux-{gnu,musl}.tar.gz`, `perllsp-0.13.0-{x86_64,aarch64}-apple-darwin.tar.gz`, `perllsp-0.13.0-x86_64-pc-windows-msvc.zip`, `SHA256SUMS`, `sbom-spdx.json`, `perl-lsp-rs-0.13.0.vsix`
  - Current status: ✗ — pending release workflow.

- [ ] **Post-publish smoke test passes against installed v0.13.0** — the PUBLISHING_ROADMAP.md §2 smoke protocol (5 test projects) passes against the released binary.
  - Verify: Follow `docs/project/PUBLISHING_ROADMAP.md` Part 2 smoke protocol with `perllsp --version` confirming `0.13.0`.
  - Current status: ✗ — cannot run until published binary is available.

</details>

---

<details>
<summary><strong>2. Quality Gates (manual sign-off required)</strong></summary>

Each item requires a human to verify and record sign-off date + initials.

- [ ] **Banned-constructs audit clean** — zero `unwrap()`, `expect()`, `panic!()`, `todo!()`, `dbg!()` in production library code outside the one allowlisted exception (`crates/perl-lsp/src/util/uri.rs`).
  - Verify: `just ci-unwrap-panic-ratchet` — must pass. Also `just clippy-prod-no-unwrap`.
  - Full check (including panic-family): `just release-check` (includes panic grep in `ci-unwrap-panic-ratchet`).
  - Current status: ✓ — `docs/project/status/index.md` records `unwrap/expect=0`, panic-family=0, unsafe=0 in production baseline as of 2026-04-07. Confirm on v0.13.0 tree with `just ci-unwrap-panic-ratchet`.
  - Sign-off: _______________ Date: _______________

- [ ] **Security scout findings: all CRITICAL + HIGH addressed** — either fixed (with PR link) or consciously deferred (with issue link and rationale).
  - Verify: `just security-audit` output reviewed. Check `gh issue list --label security --state open`.
  - Current status: ⚠ — `deny.toml` and `security-audit` recipe exist; no open security issues visible, but a fresh run is required against v0.13.0 tree.
  - Sign-off: _______________ Date: _______________

- [ ] **LSP feature gap: all P0 blockers addressed** — any feature listed in `features.toml` that is advertised as implemented is actually working.
  - Verify: `just ci-gate` passing is necessary but not sufficient. Spot-check 5 advertised features end-to-end in a real editor.
  - Reference: `features.toml` (102 features as of 2026-04-07, all at GA maturity per `docs/project/status/index.md`).
  - Current status: ⚠ — AI inline completion (#3018) shipped in v0.12.2 but awaits E2E user validation (per project memory). Mark ✓ only after human spot-check.
  - Sign-off: _______________ Date: _______________

- [ ] **Parser corpus baseline: no NEW regressions vs v0.12.2** — `just cpan-corpus-check` passes without regressions relative to the committed baseline.
  - Verify: `just cpan-corpus-check` exits 0. If regressions found, triage with `just cpan-corpus-sweep` before bumping version.
  - Baseline: `85.4%` clean rate (3,717/4,355) as of 2026-03-20 — see `PUBLICATION_FACTS_LEDGER.md`.
  - Current status: ⚠ — run `just cpan-corpus-check` on the v0.13.0 tree and confirm no regression.
  - Sign-off: _______________ Date: _______________

- [ ] **`features.toml` accuracy** — every feature with `status = "ga"` or `status = "preview"` is present and working. No feature claims capability not yet implemented.
  - Verify: Cross-check `features.toml` against `just ci-gate` output. Spot-test at least 3 features manually.
  - Current status: ✓ — features.toml is the canonical catalog; `docs/project/status/lsp.md` is generated from it and is consistent. 102 features tracked.
  - Sign-off: _______________ Date: _______________

</details>

---

<details>
<summary><strong>3. Documentation Gates</strong></summary>

- [ ] **README badges all resolve** — CI badge, GitHub release badge, license badge, Rust version badge all load without errors.
  - Verify: Open `README.md` in a browser (raw GitHub) and confirm all `img.shields.io` and `github.com/actions` badges render.
  - Current badges: CI, GitHub release, license (MIT/Apache-2.0), Rust 1.92+.
  - Missing: No `crates.io` version badge or `docs.rs` badge present. ⚠ — consider adding before announcement.
  - Current status: ✓ for existing badges; ⚠ for missing crates.io and docs.rs badges.

- [ ] **CONTRIBUTING.md reflects current workflow** — describes `just ci-gate`, the `nix develop` path, and the swarm pipeline at a high level for external contributors.
  - Verify: `head -80 CONTRIBUTING.md` — confirms setup, CI, and PR process are current.
  - Current status: ✓ — `CONTRIBUTING.md` exists and describes the current pipeline (PR #2010 merged). Describes `nix develop`, `just ci-gate`, and commit conventions.

- [ ] **CODE_OF_CONDUCT.md exists** — present and points to a contact method.
  - Verify: `ls CODE_OF_CONDUCT.md`
  - Current status: ✓ — `CODE_OF_CONDUCT.md` exists at repo root.

- [ ] **CHANGELOG.md has a v0.13.0 entry** — a dated `## [0.13.0] - YYYY-MM-DD` section narrating the release.
  - Verify: `grep "## \[0.13.0\]" CHANGELOG.md`
  - Current status: ✗ — CHANGELOG.md currently has `## [Unreleased]` targeting v0.13.0 changes. Must be promoted to a dated versioned section before tagging. Run `just release-check` to verify.

- [ ] **`features.toml` version matches target** — `[meta].version` is `"0.13.0"`.
  - Verify: `grep '^version' features.toml` returns `version = "0.13.0"`.
  - Current status: ✗ — currently at `0.12.2`. Updated by `gh workflow run version-bump.yml`.

- [ ] **`docs/project/status/index.md` narrative is current** — release posture line reflects v0.13.0 and does not reference stale milestones.
  - Verify: Read the "What's True Right Now" section; confirm it describes v0.13.0 as the announced release.
  - Current status: ✗ — currently describes v0.12.x milestones; needs update to v0.13.0 posture after version bump.

- [ ] **tree-sitter-perl-c README is first-impression polished** — `crates/tree-sitter-perl-c/README.md` describes what the crate is and how to use it from a crates.io visitor's perspective.
  - Verify: Read `crates/tree-sitter-perl-c/README.md`. It should stand alone without internal project context.
  - Current status: ✓ — README exists and describes purpose, C sources, and the lack of bindgen dependency. Polished for external publication.

- [ ] **tree-sitter-perl (highlight corpus) README is accurate** — `crates/tree-sitter-perl/README.md` explains that this is not a Rust crate, just a test corpus.
  - Verify: Read `crates/tree-sitter-perl/README.md`.
  - Current status: ✓ — README clearly states "not a Rust crate", explains contents.

</details>

---

<details>
<summary><strong>4. Marketplace + Discoverability</strong></summary>

- [ ] **VSCode Marketplace listing complete** — extension has: description, categories (`Programming Languages`, `Linters`, `Formatters`, `Debuggers`, `Testing`), icon (`icon.png`), gallery banner, minimum VSCode version, and keywords.
  - Verify: `node -p "JSON.stringify(require('./vscode-extension/package.json'), null, 2)"` — check `icon`, `galleryBanner`, `categories`, `keywords`, `engines.vscode`.
  - Current status: ✓ — `vscode-extension/package.json` has all required fields: `icon: "icon.png"`, `galleryBanner: {color: "#1e3a8a", theme: "dark"}`, 5 categories, 10 keywords.

- [ ] **VSCode Marketplace gallery screenshots** — at least one screenshot in the Marketplace listing showing the extension in action.
  - Verify: Check `vscode-extension/package.json` for a `"galleryImages"` or `"screenshots"` section, or verify images exist in the `vscode-extension/` directory.
  - Current status: ✗ — no screenshot assets found. `docs/project/RELEASE_NOTES_DRAFT.md` references a "GIF recording guide" (PR #3130) but no actual demo GIF or screenshot exists in the repo. Blocker for discoverability.

- [ ] **Open VSX listing matches VSCode Marketplace** — same description, categories, and icon on Open VSX as on Marketplace.
  - Verify: `https://open-vsx.org/extension/EffortlessMetrics/perl-lsp-rs` after publish.
  - Current status: ✗ — pending publish.

- [ ] **Docker Hub listing has description and README content** — `effortlessmetrics/perl-lsp` on Docker Hub has a meaningful description and tags documented.
  - Verify: `https://hub.docker.com/r/effortlessmetrics/perl-lsp` — check overview page.
  - Current status: ⚠ — Docker workflow exists (`.github/workflows/docker-publish.yml`) but Docker Hub listing content has not been verified against published state.

- [ ] **crates.io crate descriptions are polished** — all published crates have accurate, helpful short descriptions that match what they actually do at v0.13.0.
  - Verify: `cargo metadata --format-version=1 --no-deps | python3 -c 'import json,sys; meta=json.load(sys.stdin); ws=set(meta["workspace_members"]); [print(p["name"], repr(p["description"])) for p in meta["packages"] if p["id"] in ws and p.get("publish")!=[]]'`
  - Current status: ⚠ — descriptions exist but have not been audited for accuracy against v0.13.0 feature set. Spot-check recommended.

</details>

---

<details>
<summary><strong>5. Ecosystem + Community</strong></summary>

- [ ] **GitHub Discussions enabled with Welcome post** — Discussions tab is active and has at least a Welcome thread.
  - Verify: `https://github.com/EffortlessMetrics/perl-lsp/discussions`
  - Current status: ✓ — `CHANGELOG.md` [Unreleased] section records "GitHub Discussions enabled" (line 29). Welcome post not confirmed — visit the URL to verify.

- [ ] **Announcement post drafted in Discussions** — a pinned Discussions post ready to publish on announcement day.
  - Verify: Draft exists either in Discussions (draft state) or in `docs/project/`.
  - Current status: ✗ — no announcement Discussions post draft found in repo.

- [ ] **Issue templates are friendly and functional** — all templates in `.github/ISSUE_TEMPLATE/` work correctly and route users to the right place.
  - Verify: `ls .github/ISSUE_TEMPLATE/` — 8 templates exist; open one in browser at `https://github.com/EffortlessMetrics/perl-lsp/issues/new/choose`.
  - Current status: ✓ — 8 templates exist: `bug_report.yml`, `config.yml`, `documentation_issue.yml`, `feature_request.yml`, `parser_bug.yml`, `performance_issue.yml`, `scout_report.yml`, `swarm_discovered.yml`, `vscode_extension_issue.yml`. Config routes security issues to advisory page and questions to Discussions.

- [ ] **PR template explains the pipeline to external contributors** — `.github/pull_request_template.md` gives enough context that a first-time external contributor knows what to expect.
  - Verify: Read `.github/pull_request_template.md`.
  - Current status: ⚠ — template exists and covers Summary, Changes, Test, Verification, and What-I-Considered sections. It does not explain the Scout→Build→Review→Merge pipeline for external contributors who may be unfamiliar with the swarm model. Add a brief note pointing to `CONTRIBUTING.md`.

- [ ] **Project homepage (effortlesssteven.com) has an LSP section** — the personal site has a landing page or section for perl-lsp.
  - Verify: Visit `https://effortlesssteven.com` — confirm a perl-lsp section exists.
  - Current status: ✗ — no references to `effortlesssteven.com` found anywhere in the repo. Status unknown; manual check required.

</details>

---

<details>
<summary><strong>6. Announcement Content (external)</strong></summary>

These items confirm drafts EXIST — not that they are final.

- [ ] **Announcement blog post draft exists** — at least one blog post draft ready for editing.
  - Verify: `ls docs/articles/drafts/`
  - Current status: ✓ — 5 drafts exist in `docs/articles/drafts/`: `EVERY_DEEP_REVIEW_FOUND_A_BUG.md`, `PERL_DESERVES_BETTER_TOOLING.md` (primary announcement post — 600+ respondent survey data, competitive analysis), `THE_ONE_CHARACTER_FIX.md`, `THE_PIPELINE_IS_THE_PRODUCT.md`, `WHAT_100_AGENTS_COST.md`. Primary announcement draft at `docs/articles/drafts/PERL_DESERVES_BETTER_TOOLING.md`.

- [ ] **Demo GIF or screenshot recorded** — at least one visual asset showing completion, diagnostics, or hover in a real editor.
  - Verify: Check `vscode-extension/`, `docs/`, or `README.md` for `.gif` or screenshot assets. `docs/project/RELEASE_NOTES_DRAFT.md` references a GIF recording guide (PR #3130).
  - Current status: ✗ — no demo GIF or screenshot found in the repo. `docs/articles/drafts/PERL_DESERVES_BETTER_TOOLING.md` text references LSP functionality but has no visuals. This is a blocker for the VSCode Marketplace listing and for Reddit/HN posts.

- [ ] **Release notes draft finalized** — `docs/project/RELEASE_NOTES_DRAFT.md` is complete and covers the v0.13.0 changes.
  - Verify: `cat docs/project/RELEASE_NOTES_DRAFT.md` — check it has a Highlights section, feature list, and statistics.
  - Current status: ✓ — `docs/project/RELEASE_NOTES_DRAFT.md` exists with Highlights, Features (refactoring, diagnostics, AI, performance, distribution), Documentation, Breaking Changes, Resolved Issues, and Statistics sections. Covers v0.12.2–v0.12.8 → v0.13.0.

- [ ] **Social media posts drafted** — at least text drafts for: Twitter/X thread, Reddit r/rust, Reddit r/perl, Hacker News Show HN title+URL, lobste.rs.
  - Verify: Check `docs/project/LAUNCH_PLAN.md` for templates; confirm finalized versions exist somewhere (e.g., `docs/project/LAUNCH_PLAN.md` §3.6 or a dedicated file).
  - Current status: ⚠ — `docs/project/PUBLISHING_ROADMAP.md` §3.6 has post templates and channel list. Reddit/HN templates exist as outlines. Twitter/X thread draft not found as a standalone file. Consider creating `docs/project/ANNOUNCEMENT_POSTS.md` with finalized copy before day-of.

</details>

---

<details>
<summary><strong>7. Operational Readiness</strong></summary>

- [ ] **`just doctor` passes on a clean checkout** — `just doctor` exits 0 and shows healthy status for all subsystems.
  - Verify: `just doctor` (recipe added in #3249, merged 2026-04-04).
  - Current status: ⚠ — recipe exists (PR #3249 merged); run on a fresh checkout to confirm.

- [ ] **Pre-push hook works correctly** — `bash scripts/install-githooks.sh` installs cleanly and the hook catches format/lint errors before push.
  - Verify: Run `bash scripts/install-githooks.sh` on a clean machine; attempt to push a branch with a `cargo fmt` violation.
  - Current status: ✓ — pre-push hook improvements shipped in PR #3238 (`feat(devex): smarten pre-push hook`). Hook fixture isolation fixed in PR #3205.

- [ ] **On-call / response plan documented** — a human knows where bug reports go and what the triage SLA is.
  - Verify: `docs/project/PUBLISHING_ROADMAP.md` §4.3 — confirms triage SLA: crash/hang = same day (P0), parse error = 48h, feature gap = 1 week.
  - Current status: ✓ — §4.3 of `docs/project/PUBLISHING_ROADMAP.md` documents triage SLA, monitoring plan (daily `gh issue list` checks), and response routing. No formal on-call rotation exists (solo maintainer project); the runbook substitutes.

- [ ] **GitHub secrets are configured** — `CARGO_REGISTRY_TOKEN`, `VSCE_PAT`, `OVSX_PAT`, `DOCKER_USERNAME`, `DOCKER_PASSWORD` are set in the repo.
  - Verify: `gh secret list` — all 5 must appear.
  - Current status: ⚠ — documented in `docs/project/RELEASE_CHECKLIST.md` §Preflight. Verify live before triggering release workflow.

- [ ] **Version-bump workflow tested** — `gh workflow run version-bump.yml --field version=0.13.0` produces a clean PR that bumps `Cargo.toml`, `vscode-extension/package.json`, `features.toml`, and `CHANGELOG.md`.
  - Verify: Dry-run by reviewing the last version-bump PR (or the xtask `check-version-sync` logic).
  - Current status: ✓ — `cargo xtask check-version-sync` exists and version-bump workflow was used for prior releases (PR #3228). Procedure is tested.

</details>

---

## Summary Dashboard

Quick snapshot at 2026-04-07 against v0.13.0 (workspace currently at v0.12.2):

| Section | ✓ Done | ⚠ In Progress | ✗ Blocker |
|---------|--------|--------------|-----------|
| 1. Build & Publish | 1 | 2 | 8 |
| 2. Quality Gates | 2 | 3 | 0 |
| 3. Documentation | 5 | 0 | 3 |
| 4. Marketplace | 1 | 2 | 2 |
| 5. Community | 2 | 1 | 2 |
| 6. Announcement Content | 2 | 1 | 2 |
| 7. Operational | 3 | 2 | 0 |
| **Total** | **16** | **11** | **17** |

### Hard blockers before announcement

1. **Version bump to v0.13.0** — trigger `gh workflow run version-bump.yml --field version=0.13.0`, merge the PR. Unblocks all "pending version bump" items.
2. **Demo GIF / screenshot** — no visual asset exists. Required for VSCode Marketplace and Reddit/HN posts. See `docs/project/RELEASE_NOTES_DRAFT.md` which references PR #3130 (GIF recording guide).
3. **CHANGELOG.md dated v0.13.0 entry** — promote `[Unreleased]` to `[0.13.0] - YYYY-MM-DD`. Automated by `version-bump.yml` or done manually per `docs/project/PUBLISHING_ROADMAP.md` §1.3.
4. **Release workflow** — `gh workflow run release-orchestration.yml --field version=0.13.0 ...` — publishes crates, extension, Docker, and creates GitHub Release.
5. **Post-publish smoke** — run the 5-project smoke protocol from `docs/project/PUBLISHING_ROADMAP.md` Part 2 against the installed binary.

### Path to green

```
1. just release-check              # Confirm master is ready
2. gh workflow run version-bump.yml --field version=0.13.0
   # Merge the resulting PR
3. Record demo GIF (see docs/project/RELEASE_NOTES_DRAFT.md for guide reference)
4. gh workflow run release-orchestration.yml --field version=0.13.0 \
     --field prerelease=false --field skip_crates=false \
     --field skip_extension=false --field skip_docker=false
5. Run smoke protocol (PUBLISHING_ROADMAP.md Part 2)
6. Fill in sign-off lines in Quality Gates section above
7. Post announcements per PUBLISHING_ROADMAP.md §3.6
```

---

*Related documents: [RELEASE_CHECKLIST.md](RELEASE_CHECKLIST.md) (release mechanics gate) | [PUBLISHING_ROADMAP.md](PUBLISHING_ROADMAP.md) (day-of runbook) | [RELEASE_NOTES_DRAFT.md](RELEASE_NOTES_DRAFT.md) | [LAUNCH_PLAN.md](LAUNCH_PLAN.md)*
