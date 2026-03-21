# Publication Facts Ledger

One canonical place for all metrics used in articles, talks, and public claims.
**Rule**: Before using ANY number in an article or talk, check this ledger first. If the number is not here, verify and add it.

## Codebase Metrics

| Claim | Verified | Source | Date | Command |
|-------|----------|--------|------|---------|
| Lines of Rust | 563,228 | wc -l | 2026-03-20 | `find crates/ -name "*.rs" \| xargs wc -l \| tail -1` |
| Workspace crates | 133 | cargo metadata | 2026-03-21 | `cargo metadata --no-deps \| jq '.packages \| length'` |
| Total test count | [PENDING VERIFICATION] | — | — | Methodology under investigation (scout audit in progress) |
| Total commits | 2,761 | git log | 2026-03-20 | `git log --oneline \| wc -l` |
| Total PRs | 2,244+ | GitHub | 2026-03-20 | `gh pr list --state all --limit 1 --json number` |
| Total issues | 2,239+ | GitHub | 2026-03-20 | `gh issue list --state all --limit 1 --json number` |
| LSP features | 98 | features.toml | 2026-03-20 | `grep "^\[" features.toml \| wc -l` |
| CPAN corpus files | 4,355 | baseline json | 2026-03-20 | Read `.ci/cpan-corpus-baseline.json` |
| Corpus clean rate (baseline) | 85.7% | sweep | 2026-03-20 | `just cpan-corpus-sweep` — files that parse without errors |
| Corpus manifest coverage | 90.9% | manifest | 2026-03-21 | Files in manifest / total corpus files |
| Mutation score | 87% | memory file | 2026-03-19 | `scout_security_and_reliability.md` |
| Max daily commits | 152 | git log | 2026-03-20 | `git log --format="%ad" --date=format:"%Y-%m-%d" \| sort \| uniq -c \| sort -rn \| head -5` |
| Busiest day | 2026-03-04 | git log | 2026-03-20 | same as above |

**Corpus note**: Two distinct metrics exist. "Baseline clean rate" counts files that parse without errors (was 85.7% as of 2026-03-20). "Manifest coverage" counts files verified clean and added to the ratchet manifest (90.9% as of 2026-03-21 session 2). Always specify which metric you mean.

## Zero-Panic Policy

| Claim | Status | Note |
|-------|--------|------|
| Zero-panic enforcement policy | Verified — policy exists | `unwrap`/`expect`/`panic!` banned in production code per CLAUDE.md |
| Zero violations in production code | [PENDING VERIFICATION] | Audit in progress: 222 potential violations detected by scan; scope (prod vs test) under investigation |

**Note**: The zero-panic *policy* is real and enforced. The zero-violation *status* is unverified. Do not claim "zero panics in production" until the audit completes.

## Swarm Metrics

| Claim | Verified | Source |
|-------|----------|--------|
| 100 agents in one session | Yes | Cycle 5 final memory |
| 56 PRs in one session | Yes | Cycle 5 final memory |
| 90% constrained task success | Yes | `feedback_agent_success_rate_pattern.md` |
| 50% unconstrained task success | Yes | same |
| 75 agent ceiling | Yes | `feedback_team_roster_hard_ceiling.md` |
| ~150 agents in session 2 | Yes | Era 7 session 2 report |
| 52+ PRs merged in session 2 | Yes | Era 7 session 2 report |
| 27+ stale issues closed in session 2 | Yes | Era 7 session 2 report |

## Economics

| Claim | Verified | Source | Note |
|-------|----------|--------|------|
| ~3% of weekly budget per session | Yes | Era 7 session 2 report | 30 merged PRs in ~2 hours at ~3% weekly budget |
| ~$X per merged PR | [PENDING VERIFICATION] | — | Derive from budget % and absolute cost once known |

**Economics note**: "~3% weekly budget for 30 merged PRs in 2 hours" is the verified ratio. Absolute dollar cost not published until confirmed.

## Competitive Claims

| Claim | Source | Confidence |
|-------|--------|-----------|
| 78% of Perl devs use no LSP | 2025 Perl IDE Survey (602 respondents) | High (external) |
| PerlNavigator: 53K VSCode installs | VSCode Marketplace | High (external, may change) |

## Corrections (previously wrong)

| Original Claim | Corrected | Why |
|----------------|-----------|-----|
| "321 commits in one day" | 152 (Mar 4) | Likely counted across branches or multiple days |
| "546K lines" | 563K lines | Codebase grew |
| "128 crates" | 132 crates | Codebase grew |
| "132 crates" | 133 crates | Codebase grew (verified 2026-03-21) |
| "97 features" | 98 features | Off by one |
| "90.9% corpus" without qualifier | Use "manifest coverage 90.9%" | Two distinct corpus metrics exist; must specify which |
