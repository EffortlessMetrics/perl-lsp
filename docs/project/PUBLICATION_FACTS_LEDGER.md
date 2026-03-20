# Publication Facts Ledger

One canonical place for all metrics used in articles, talks, and public claims.
**Rule**: Before using ANY number in an article or talk, check this ledger first. If the number is not here, verify and add it.

## Codebase Metrics

| Claim | Verified | Source | Date | Command |
|-------|----------|--------|------|---------|
| Lines of Rust | 563,228 | wc -l | 2026-03-20 | `find crates/ -name "*.rs" \| xargs wc -l \| tail -1` |
| Workspace crates | 132 | cargo metadata | 2026-03-20 | `cargo metadata --no-deps \| jq '.packages \| length'` |
| Total commits | 2,761 | git log | 2026-03-20 | `git log --oneline \| wc -l` |
| Total PRs | 2,244+ | GitHub | 2026-03-20 | `gh pr list --state all --limit 1 --json number` |
| Total issues | 2,239+ | GitHub | 2026-03-20 | `gh issue list --state all --limit 1 --json number` |
| LSP features | 98 | features.toml | 2026-03-20 | `grep "^\[" features.toml \| wc -l` |
| CPAN corpus files | 4,355 | baseline json | 2026-03-20 | Read `.ci/cpan-corpus-baseline.json` |
| Corpus coverage | 85.7% | sweep | 2026-03-20 | `just cpan-corpus-sweep` |
| Mutation score | 87% | memory file | 2026-03-19 | `scout_security_and_reliability.md` |
| Max daily commits | 152 | git log | 2026-03-20 | `git log --format="%ad" --date=format:"%Y-%m-%d" \| sort \| uniq -c \| sort -rn \| head -5` |
| Busiest day | 2026-03-04 | git log | 2026-03-20 | same as above |

## Swarm Metrics

| Claim | Verified | Source |
|-------|----------|--------|
| 100 agents in one session | Yes | Cycle 5 final memory |
| 56 PRs in one session | Yes | Cycle 5 final memory |
| 90% constrained task success | Yes | `feedback_agent_success_rate_pattern.md` |
| 50% unconstrained task success | Yes | same |
| 75 agent ceiling | Yes | `feedback_team_roster_hard_ceiling.md` |

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
| "97 features" | 98 features | Off by one |
