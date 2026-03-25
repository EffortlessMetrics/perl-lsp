# Publication Facts Ledger

One canonical place for all metrics used in articles, talks, and public claims.

**Rule**: Before using ANY number in an article or talk, check this ledger first. If the number is not here, verify and add it.

**Verification**: Run `just verify-publication-facts` to check all auto-computable metrics.

---

## Tier System

Every claim in this ledger carries a tier marker that describes how it was obtained and how it can be updated.

| Tier | Label | Meaning | Auto-verifiable? |
|------|-------|---------|-----------------|
| **A** | Computed | Derived by running a command against the live repo or `.ci/` artifacts. Updates automatically. | Yes — `verify-publication-facts.sh` |
| **B** | Measured | Derived from a single timed measurement (e.g., CI run timing). Refresh by re-running the measurement. | Partially |
| **C** | Estimated | Model estimate with documented assumptions and confidence intervals. Cannot be auto-computed. | No — see methodology docs |
| **D** | External | External source (survey, marketplace). Cannot be computed locally. Requires manual refresh or source link. | No — manual only |
| **N** | Narrative | Qualitative or derived from multiple signals. Not a hard number. | No |

**Staleness thresholds**: Tier A/B metrics flagged as WARNING if >30 days old, ERROR if >90 days old. Tier C/D require manual review on each publication cycle.

---

## Codebase Metrics (Tier A — auto-verified)

| Claim | Verified | Source | Date | Tier | Command |
|-------|----------|--------|------|------|---------|
| Lines of Rust | 597,863 | wc via cat | 2026-03-22 | A | `find crates/ -name "*.rs" -print0 \| xargs -0 cat \| wc -l` |
| Workspace crates | 134 | cargo metadata | 2026-03-25 | A | `cargo metadata --no-deps \| jq '.packages \| length'` |
| LSP features | 98 | features.toml | 2026-03-22 | A | `grep -c '^\[\[feature\]\]' features.toml` |
| Total commits | 3,307 | git log | 2026-03-22 | A | `git log --oneline \| wc -l` |
| Total PRs | 2,823+ | GitHub | 2026-03-22 | A | `gh pr list --state all --limit 1 --json number` |
| Total issues | 2,812+ | GitHub | 2026-03-22 | A | `gh issue list --state all --limit 1 --json number` |
| CPAN corpus files | 4,355 | baseline json | 2026-03-20 | A | `jq .total_files .ci/cpan-corpus-baseline.json` |
| Corpus clean rate (baseline) | 85.4% (3,717/4,355) | baseline json | 2026-03-20 | A | `jq .clean_files .ci/cpan-corpus-baseline.json` |
| Corpus manifest coverage | 47.1% (2,052/4,355) | manifest | 2026-03-22 | A | `wc -l .ci/cpan-corpus-manifest.txt` |
| CPAN known-clean manifest | 2,052 modules | .ci/cpan-corpus-manifest.txt | 2026-03-22 | A | `wc -l .ci/cpan-corpus-manifest.txt` |
| Max daily commits | 308 | git log | 2026-03-21 | A | `git log --format="%ad" --date=format:"%Y-%m-%d" \| sort \| uniq -c \| sort -rn \| head -5` |
| Busiest day | 2026-03-20 | git log | 2026-03-21 | A | same as above |
| Lib tests (workspace) | 2,871 | cargo test | 2026-03-21 | A | `cargo test --workspace --lib 2>&1 \| grep "^test result" \| awk '{sum += $4} END {print sum}'` |
| Total test count | [PENDING VERIFICATION] | — | — | A | Methodology under investigation (see issue #2672) |

**Note on LOC**: The command `find crates/ -name "*.rs" | xargs wc -l | tail -1` produces incorrect results when `xargs` splits into multiple batches (each batch prints its own "total"). Always use `find crates/ -name "*.rs" -print0 | xargs -0 cat | wc -l` for accurate results.

**Corpus note**: Two distinct metrics exist. "Baseline clean rate" counts files that parse without errors from `.ci/cpan-corpus-baseline.json` (85.4% as of 2026-03-20). "Manifest coverage" counts files explicitly verified clean and added to the ratchet manifest (47.1% as of 2026-03-22). Always specify which metric you mean. Note: the 90.9% figure cited in earlier sessions referred to the lib-file sweep, which filters the full corpus to library files only — it is not the same as baseline clean rate.

---

**Test count note**: Three scopes exist. Always specify which you mean:
- **Tier A (merge-gate, canonical)**: `cargo test --workspace --lib --exclude tree-sitter-perl -- --list | grep ': test$' | wc -l` = ~2,811
- **All lib tests**: same without `--exclude tree-sitter-perl` = ~2,949
- **Total (all types)**: lib + doc + integration = ~21,465
- The earlier "6,326" figure was incorrect — it came from an unverified audit with undocumented methodology. See issue #2672.
- `update-current-status.py` uses the Tier A (canonical) methodology.

## Zero-Panic Policy

| Claim | Status | Tier | Note |
|-------|--------|------|------|
| Zero-panic enforcement policy | Verified — policy exists | N | `unwrap`/`expect`/`panic!` banned in production code per CLAUDE.md |
| Zero violations in production code | [PENDING VERIFICATION] | A | Audit in progress: 222 potential violations detected by scan; scope (prod vs test) under investigation |

**Note**: The zero-panic *policy* is real and enforced. The zero-violation *status* is unverified. Do not claim "zero panics in production" until the audit completes.

---

## Swarm Metrics (Tier A — verified from memory files and receipts)

| Claim | Verified | Tier | Source |
|-------|----------|------|--------|
| 150 agents in one session | Yes | A | Era 7 session 1 memory (2026-03-21) |
| ~150 agents in session 2 | Yes | A | Era 7 session 2 report |
| 100 agents in one session | Yes | A | Cycle 5 final memory |
| 57 PRs merged in one session (Era 7 s1) | Yes | A | `gh pr list --state merged --json mergedAt` |
| 56 PRs in one session | Yes | A | Cycle 5 final memory |
| 52+ PRs merged in session 2 | Yes | A | Era 7 session 2 report |
| 200+ PRs merged across Era 7 sessions | Yes | A | `gh pr list --state merged --json mergedAt` |
| 27+ stale issues closed in session 2 | Yes | A | Era 7 session 2 report |
| Deep review bug-finding rate | 100% (every PR) | A | Era 7 s1 session review (2026-03-21) |
| 90% constrained task success | Yes | N | `feedback_agent_success_rate_pattern.md` |
| 50% unconstrained task success | Yes | N | same |
| 75 agent ceiling | Yes | N | `feedback_team_roster_hard_ceiling.md` |

---

## Economics (Tier C — model estimates with documented assumptions)

| Claim | Verified | Tier | Source | Confidence |
|-------|----------|------|--------|-----------|
| DevLT 3-5 min/PR | Model estimate | C | COST_ROI_ANALYSIS.md Section 5 | Derived from ~150-200 hours human time ÷ 190+ PRs. Not measured from CI receipts. |
| $40-79K vs $500K-$1.2M | Model estimate | C | COST_ROI_ANALYSIS.md Section 9 | 35-45 dev-months @ $15-20K/month. Confidence intervals in Section 9. |
| ~3% of weekly budget per session | Yes | A | Era 7 session 2 report | 30 merged PRs in ~2 hours at ~3% weekly budget |
| ~$X per merged PR | [PENDING VERIFICATION] | C | — | Derive from budget % and absolute cost once known |

**Economics note**: "~3% weekly budget for 30 merged PRs in 2 hours" is the verified ratio. Absolute dollar cost not published until confirmed. Cost estimates in COST_ROI.md are Tier C (informed approximations) — the article states this explicitly in its closing note. Confidence intervals exist in COST_ROI_ANALYSIS.md Section 9 but are not reproduced in main articles.

---

## Pipeline Architecture (Tier A)

| Feature | Shipped | Tier | Source | Date |
|---------|---------|------|--------|------|
| research-verifier stage | Yes | A | `.claude/agents/research-verifier.md` | 2026-03-21 |
| accuracy-scout stage | Yes | A | `.claude/agents/accuracy-scout.md` | 2026-03-21 |
| Label state machine | Yes | A | GitHub labels | 2026-03-21 |
| Pipeline stages (full) | Scout → Accuracy-Scout → Research-Verifier → Plan-Review → Build → Review → Green → Merge → Wisdom | A | `.claude/agents/` | 2026-03-21 |

---

## Competitive Claims (Tier D — external, unverified)

| Claim | Tier | Source | Confidence | Refresh path |
|-------|------|--------|-----------|-------------|
| 78% of Perl devs use no LSP | D | 2025 Perl IDE Survey (602 respondents) | High (external) — survey source not linked in any article | Link primary source or re-survey. Mark as "Tier D: 2025 Perl IDE Survey" in articles. |
| PerlNavigator: ~53K VSCode installs | D | VSCode Marketplace | Point-in-time; measurement date unknown | Query VSCode Marketplace API; add date stamp. |
| Perl::LanguageServer: ~293K VSCode installs | D | VSCode Marketplace | Point-in-time; measurement date unknown | Query VSCode Marketplace API; add date stamp. |

**Important**: Install counts in COMPETITIVE_ANALYSIS.md are not date-stamped. They appear as facts but change daily. Before publication, refresh from the VSCode Marketplace and add the measurement date inline.

---

## Corrections (previously wrong)

| Original Claim | Corrected | Why |
|----------------|-----------|-----|
| "321 commits in one day" | 308 (Mar 20, 2026) | Previous record was 152; session activity broke the record |
| "152 max daily commits" | 308 (Mar 20, 2026) | Mar 20 session produced 308 commits |
| "546K lines" | ~598K lines (2026-03-22) | Codebase grew |
| "563K lines" (COST_ROI.md) | ~598K lines | COST_ROI.md used an earlier snapshot |
| "591K lines" | 598K lines (2026-03-22) | Ongoing growth |
| "128 crates" | 134 crates (2026-03-25) | Codebase grew |
| "132-133 crates" | 134 crates (2026-03-25) | Ongoing growth; 134 is the current verified value |
| "97 features" | 98 features | Off by one |
| "2,673 lib tests" | 2,871 lib tests | 198 new tests added across Era 7 |
| "85.7% corpus" without qualifier | Use "85.4% baseline clean rate (3,717/4,355)" | Two distinct corpus metrics exist; must specify which |
| "90.9% corpus" without qualifier | Use "manifest coverage 47.1% (2,052/4,355)" | 90.9% was the lib-file sweep, not full corpus baseline |
| "2,761 total commits" | 3,307 total commits (2026-03-22) | Ongoing growth |
| "2,244+ total PRs" | 2,823+ total PRs (2026-03-22) | Ongoing growth |
| xargs wc -l LOC method | Use `find crates/ -name "*.rs" -print0 \| xargs -0 cat \| wc -l` | xargs batching causes double-counted "total" lines |
| "6,326 test functions" | ~21,465 total / 2,811 lib (Tier A) | Scope confusion: 6,326 was unverified; worktree duplication inflated grep-based counts; see issue #2672 |
| "8:1 test-to-code ratio (6,326/755)" | ratio unverified | Numerator was incorrect; 755 public functions count also unverified |
| "2,871 lib tests" | 2,811 (--exclude tree-sitter-perl) / 2,949 (all) | Previous entry used execution-count methodology; --list methodology is canonical per update-current-status.py |

---

## Non-Automatable Claims Checklist

Review before each publication cycle:

- [ ] **78% Perl devs survey** — Link or cite primary source. If unavailable, mark "Tier D: attributed to 2025 Perl IDE Survey, 602 respondents; source not independently verified."
- [ ] **Install counts** — Refresh from VSCode Marketplace. Add date stamp: "As of [DATE]" in article text.
- [ ] **DevLT 3-5 min/PR** — Add caveat "Model estimate; see COST_ROI_ANALYSIS.md methodology" or measure from actual CI receipts.
- [ ] **Cost estimates** — Ensure confidence intervals from Section 9 are referenced in any article that cites absolute cost figures.
- [ ] **LOC / crate count** — Run `just verify-publication-facts` to confirm current values before any article update.

---

## Verification Log

| Date | Runner | Method | Result |
|------|--------|--------|--------|
| 2026-03-22 | builder agent (issue #2812) | `just verify-publication-facts` | 8 PASS, 0 WARN, 0 ERR |
| 2026-03-21 | manual ledger update | Manual verification of all Tier A metrics | Snapshot captured; xargs LOC bug not yet known |
