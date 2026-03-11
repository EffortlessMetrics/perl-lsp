# Jules Bot PR Pattern Analysis

Between January 17 and February 28, 2026, Google's Jules coding agent submitted
**319 pull requests** to the perl-lsp repository. Of these, 44 were merged and
275 were closed -- an **86.2% rejection rate**. This document examines what
happened, why, and what it reveals about autonomous agent coordination in a
real codebase.

## Who Is Jules?

Jules is Google's AI coding agent (Google Labs). It operates via the GitHub
app account `app/google-labs-jules`, though PRs are attributed to the
repository owner (`EffortlessSteven`) who initiates the tasks. Each PR
includes a footer linking to the originating Jules task, for example:

> *PR created automatically by Jules for task
> [16175761772110699949](https://jules.google.com/task/16175761772110699949)
> started by @EffortlessSteven*

Jules is distinct from OpenAI's Codex agent, which also submitted PRs to this
repository (266 PRs at a 55% merge rate). Where Codex uses `codex/` branch
prefixes, Jules uses a persona-based naming system described below.

## The Three Personas

Jules organizes its work into three named personas, each with an emoji prefix
and a distinct area of responsibility:

| Persona | Emoji | Branch prefix | Focus area | PRs | Merged | Merge rate |
|---------|-------|---------------|------------|-----|--------|------------|
| **Bolt** | ⚡ | `bolt-` / `bolt/` | Performance optimization | 118 | 17 | 14% |
| **Sentinel** | &#x1F6E1;&#xFE0F; | `sentinel-` / `sentinel/` | Security hardening | 99 | 13 | 13% |
| **Palette** | &#x1F3A8; | `palette-` / `palette/` | UX/UI improvements | 97 | 11 | 11% |
| **Jules** (direct) | -- | `jules/` / `jules-` | Misc (docs, bug fixes) | 5 | 3 | 60% |

The personas appear to be task-routing labels rather than different models.
Branch names include long numeric suffixes (e.g.,
`bolt-optimize-is-known-function-10330446419649079789`) that likely correspond
to Jules task IDs.

### What Each Persona Did

**Bolt** focused almost exclusively on the `ScopeAnalyzer` in
`perl-semantic-analyzer`. Its merged PRs introduced zero-allocation variable
lookups, deferred line number calculations, iterative scope traversal, and
reduced `Rc` cloning. When it worked, it produced genuine micro-optimizations.
When it failed, it repeatedly attempted the same PHF builtin lookup
optimization (21 attempts, zero merges) and symbol extraction regex
compilation (18 attempts, one early merge).

**Sentinel** addressed security vulnerabilities, primarily in the debug
adapter's safe evaluation mode and the VS Code extension's binary downloader.
Its merged PRs fixed path traversal bugs, command injection risks, HTTPS
downgrade vulnerabilities, and weak checksum verification. Post-golden-era, it
became stuck in a loop of proposing ever-more-elaborate safe eval hardening
that the maintainer had already addressed or deemed unnecessary.

**Palette** improved the VS Code extension's UX: status menu organization,
keybinding hints, test runner improvements, and command palette filtering. Its
most successful work involved concrete additions (snippets, status bar
feedback, organize imports). Its biggest failure was the "context-aware status
menu" saga -- 51 PRs attempting variations on the same feature, only 5 merged.

## The Four Phases

### Phase 1: Early Attempts (PRs #318--#469)

- **39 PRs, 0 merged** (January 17--22, 2026)
- Date range: 6 days

Jules's first contact with the codebase. Every PR was rejected. During this
period, the maintainer created **57 `maint/pr-*` bridging PRs** that took
Jules's ideas and manually curated them into mergeable form. These bridging PRs
had a 92% merge rate (57 of 62 merged), covering source PRs #300--#554. This
suggests the maintainer was treating Jules as an idea generator, not a direct
contributor.

### Phase 2: The Golden Era (PRs #470--#647)

- **110 PRs, 41 merged** (January 22--30, 2026)
- **37% merge rate** over 9 days

This was Jules's productive period. The best single day was January 26, when 9
of 11 submitted PRs were merged. January 28 matched that record. The golden
era's merged PRs were diverse:

| Category | Merged |
|----------|--------|
| Security fixes (Sentinel) | 14 |
| Scope optimizations (Bolt) | 12 |
| UX improvements (Palette) | 11 |
| Other optimizations (Bolt) | 4 |

What made these PRs succeed:
1. **Novelty** -- each addressed a distinct problem the maintainer hadn't solved yet.
2. **Scope** -- changes were small, focused, and testable.
3. **Low-hanging fruit** -- genuine path traversal bugs, real allocation waste,
   missing UI features.

### Phase 3: The Wall (PRs #648--#837)

- **167 PRs, 0 merged** (January 30 -- February 17, 2026)
- **0% merge rate** over 19 days

After PR #647 (the last golden-era merge on January 30), Jules hit a wall of
total rejection that lasted nearly three weeks. The shift was abrupt and
absolute -- not a gradual decline but a hard cutoff.

What changed:

| Wall topic | Count | What went wrong |
|------------|-------|-----------------|
| Context-aware menu | 45 | Kept reproposing the same rejected feature |
| Safe eval hardening | 25 | Already-fixed vulnerabilities re-reported |
| Regex/symbol extraction | 27 | Same optimization resubmitted with minor variations |
| PHF builtin optimization | 23 | Identical approach rejected 20+ times |
| Other security | 27 | Diminishing returns on security surface |
| Other optimization | 11 | Running out of real performance wins |

The core problem: **Jules exhausted the low-hanging fruit but could not
recognize that the remaining work required deeper architectural understanding.**
It continued generating PRs at the same rate (7--11 per day) on topics the
maintainer had either already addressed or deliberately declined.

### Phase 4: Rebirth (PRs #888--#946)

- **3 PRs, 3 merged** (February 28, 2026)
- **100% merge rate**, all on a single day

After an 11-day gap (February 17--28), three Jules PRs appeared and all
merged. These used the direct `jules/` and `jules-` branch prefixes rather than
persona names, suggesting a different task configuration. The PRs were
qualitatively different:

- **#888** -- Documentation update (README, ROADMAP, CURRENT_STATUS)
- **#930** -- Heredoc parsing bug fix (a genuine parser correctness issue)
- **#946** -- Moo/Moose framework support enhancement

These succeeded because they addressed real, unsolved problems in areas Jules
hadn't previously saturated.

## The Persistence Sagas

Jules exhibited a distinctive failure mode: when a PR was rejected, it would
resubmit nearly identical work with minor variations, sometimes dozens of times.

### The PHF Builtin Lookup Saga (21 closed PRs)

Bolt attempted to replace hash-based builtin function lookups with PHF (Perfect
Hash Function) tables. The same idea was submitted 21 times between PR #526
and #835, each time with a slightly different branch name but essentially the
same implementation. None were merged under the Jules persona. Ironically,
Codex later succeeded with PR #1167 by extracting PHF tables into a dedicated
microcrate -- a different architectural approach.

### The Context-Aware Status Menu Saga (51 PRs, 5 merged)

Palette's longest-running campaign. The first successful status menu PR merged
at #598. Jules then attempted to make the menu "context-aware" (disabling items
based on file type) roughly 45 more times. The maintainer merged the basic
version at #646 but rejected all subsequent refinements. Jules could not
detect that the feature was "done enough."

### The Safe Eval Hardening Saga (21 PRs, 3 merged)

Sentinel successfully fixed several safe evaluation bypasses early on (#572,
#600, #647). It then continued proposing additional blocklist expansions for
increasingly obscure Perl operators (sysopen, archaic package separators,
readline operators). The maintainer stopped merging these, having already
hardened the critical paths.

### The Symbol Extraction Regex Saga (18 PRs, 1 merged)

Bolt merged one symbol extraction regex optimization (#645) during the golden
era. It then submitted 17 more attempts at the same optimization, each rejected.
The variations were cosmetic -- different variable names, slightly reordered
code -- but the core approach was identical.

## Daily Submission Patterns

Jules submitted PRs at a remarkably consistent rate, typically 6--11 per day,
across what appear to be three daily batches (roughly morning, midday, and
evening UTC). The submission rate did not decrease during the wall period --
Jules showed no ability to self-throttle in response to rejection.

| Date | PRs | Merged | Period |
|------|-----|--------|--------|
| Jan 22 | 8 | 3 | Start of golden era |
| Jan 24 | 10 | 0 | Local dip |
| Jan 26 | 11 | 9 | Peak merge day |
| Jan 28 | 11 | 9 | Peak merge day |
| Jan 30 | 7 | 4 | Last golden-era merges |
| Jan 31 | 8 | 0 | Wall begins |
| Feb 12 | 11 | 0 | Wall continues at full rate |
| Feb 16 | 9 | 0 | Wall continues |
| Feb 28 | 3 | 3 | Rebirth (different approach) |

## Jules vs Codex: Agent Strategy Comparison

Both agents worked on the same codebase during overlapping periods. Their
strategies differed markedly:

| Dimension | Jules | Codex |
|-----------|-------|-------|
| Total PRs | 319 | 266 |
| Merge rate | 14% | 55% |
| Branch naming | Persona prefixes (bolt/sentinel/palette) | `codex/` prefix |
| PR style | Emoji-prefixed titles, task links | Conventional titles |
| Approach | Three parallel work streams | Single unified agent |
| Failure mode | Obsessive repetition of rejected ideas | Occasional build failures |
| Best at | Novel security fixes, micro-optimizations | SRP microcrate extraction |
| Worst at | Recognizing when to stop | Complex architectural changes |

Codex's higher merge rate reflects a fundamentally different operating model:
it worked on maintainer-defined tasks (microcrate extraction) rather than
self-directed exploration. Jules generated its own agenda, which produced
creative early wins but led to repetitive dead ends.

### The Document Links Crossover

An illustrative case: the "extract document links into a microcrate" task was
attempted 9 times by Codex (not Jules), merging on attempt #4 (PR #1164).
This shows that even the higher-merge-rate agent exhibited persistence
patterns, though at a much smaller scale.

## Lessons About Autonomous Agent Coordination

### 1. The Exploration-Exploitation Cliff

Jules demonstrated strong exploration (finding genuine bugs, novel
optimizations) but no exploitation awareness. It could not detect when a
problem space was saturated. The golden era ended not because Jules got worse,
but because the easy problems ran out.

### 2. Rejection Signals Are Invisible to the Agent

Jules showed zero behavioral change in response to PR closures. The same
pattern resubmitted 21 times suggests the agent has no feedback loop from PR
outcomes to task generation. Each task appears to be generated independently.

### 3. Persona Routing Does Not Prevent Tunnel Vision

The three-persona system (Bolt/Sentinel/Palette) provided surface-level
specialization but did not prevent any persona from getting stuck. All three
personas independently developed repetition loops. The persona system organized
*what* the agent worked on but could not regulate *when to stop*.

### 4. Human Curation Amplifies Agent Value

The `maint/pr-*` bridging pattern (57 PRs, 92% merge rate) shows that
human curation of agent-generated ideas was far more effective than letting
the agent merge directly. The maintainer could take a Jules PR that failed
CI or had style issues and reshape it into something mergeable.

### 5. Fresh Context Breaks the Loop

The Phase 4 "rebirth" (3/3 merged) succeeded because it used different
branch naming, addressed different problem areas, and came after an 11-day
gap. Resetting the agent's focus -- rather than letting it continue its
existing threads -- produced better results.

### 6. Agent Throughput Is Not the Bottleneck

At 7--11 PRs per day, Jules generated far more changes than a human maintainer
could review. The bottleneck was always review capacity, not generation
capacity. An agent that submitted fewer, higher-quality PRs would have been
more valuable.

## Summary Statistics

| Metric | Value |
|--------|-------|
| Total Jules PRs | 319 |
| Merged | 44 (13.8%) |
| Closed | 275 (86.2%) |
| Active period | Jan 17 -- Feb 28, 2026 (43 days) |
| Average PRs/day (active) | ~7.4 |
| Peak PRs/day | 11 (achieved 4 times) |
| Longest merge streak | Jan 26--30 (32 merges in 5 days) |
| Longest rejection streak | Jan 31 -- Feb 17 (167 consecutive closures) |
| Most-attempted theme | Context-aware status menu (51 PRs) |
| Most-attempted optimization | PHF builtin lookup (21 PRs, 0 merged) |
| Codex merge rate (comparison) | 55% (146/266) |
| Human-curated Jules PRs (maint/) | 92% (57/62) |
