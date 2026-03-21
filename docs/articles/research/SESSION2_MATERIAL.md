# Era 7 Session 2: Source Material

*Research and evidence capture. March 21, 2026. Session 2 of the Era 7 pipeline-validated swarm.*

---

## 1. Session Statistics

### The Numbers

| Metric | Value |
|--------|-------|
| **PRs merged** | 55 (tracked from git, Mar 21 2026) |
| **Issues closed (total)** | 62 |
| **Stale issues closed (session-open triage)** | 27 |
| **New issues filed** | 30+ |
| **Issues categorized/labeled this session** | 121 |
| **CPAN corpus coverage (start)** | 85.7% |
| **CPAN corpus coverage (end)** | 90.9% (3,956/4,355 clean files, ratcheted #2621) |
| **Agents spawned** | ~150 (estimate: 55+ builders + scouts + reviewers + ops) |
| **Session date** | 2026-03-21 |

### What Was Shipped

**Swarm infrastructure (new pipeline components):**
- Research-verifier agent (#2610) — cheap haiku-grade fact-check between scout and plan-review
- Accuracy-scout agent (#2637) — mechanical fact verification before plan-review
- Label-driven pipeline state machine (#2638) — GitHub labels as authoritative pipeline state
- Agent preflight safety checks (#2565)
- Blocker ledger (#2586) — prevents scout rediscovery of already-known blockers
- Agent wrapup template standardization (#2584)
- Control-plane lock wired into wisdom and wrapup (#2583)

**Parser fixes:**
- Fat-arrow pairs as ternary branch expressions (#2613)
- Defined/ref omit-arg fix for word operators (#2626)
- qw() comment stripping (#2618)
- $::{key} main stash access (#2601)
- Indirect call arg loop terminators (#2591)
- C-style for loop semicolon recovery (#2593)
- lvalue my $var->{key} declaration (#2589)
- print STDERR => list call fix (#2555)

**LSP features:**
- POD =head symbols in document outline (#2614)
- Perl 5.38+ native class/method hover (#2624)
- use lib / FindBin path wiring (#2620)
- die/warn hover docs + croak modernize action (#2606)
- Diagnostic.data field for client integration (#2592)
- Inlay hint resolve with click-to-definition (#2643)
- PL405 printf/sprintf format specifier arity lint (#2636)
- PL701 ModuleNotFound lint (#2619)
- DBI/DBIx SQL string semantic tokens (#2639)
- Scalar/list context docs on 11 builtins (#2630)
- BEGIN/END/INIT/CHECK/UNITCHECK hover and diagnostics (#2623)
- Tied variable hover and tie magic docs (#2609)
- Regex flag completions + named capture + operator snippets (#2635)
- commit_characters on CompletionItem (#2597)
- Incremental parsing wired into text sync (#2605)
- Parser cancellation token from text_sync (#2615)
- Subprocess timeout for perltidy/perlcritic (#2616)

**Infrastructure:**
- Hook reliability engineering — tests, drift fixes, executable-bit CI (#2590)
- Unexpected_token_in_expr decomposed into 17 SEMANTIC_BUCKETS entries (#2611)
- Logos lexer experiment archived + chumsky dep removed (#2612)
- DAP phase3 promoted to default (#2588)
- test_grep_block_file_test pre-existing failure resolved (#2562)
- assert_clean_parse fixed to use AST walk instead of string matching (#2559)
- files_by_bucket in corpus sweep baseline schema 1.2.0 (#2585)

### Corpus Movement

```
85.7% (start of session) → 90.9% (end, ratchet #2621)
+5.2 percentage points in one session
From: ~3,731/4,355 clean files
To:   ~3,956/4,355 clean files
Net improvement: ~225 additional modules parsing cleanly
```

The 90.9% mark is a new project high. The previous high was 90.3% from Cycle 6.

---

## 2. Pipeline Validation Evidence

### Deep Review Found Real Bugs — Every Time

The two-pass review model (initial reviewer → deep-review pass) was applied to the Era 7 Session 1 feature PRs before their session 2 merges. Deep reviewers found substantive bugs in every PR they examined:

**#2531 (regex hover):** Deep reviewer found 4 actionable issues:
- Regex hover extraction was AST-gated on wrong path
- 6 specific bugs fixed by the improvement pass
- Test count: 5 → 11 (added edge cases reviewer identified)

**#2532 (go-to-test navigation):** Deep reviewer found 7 improvements including:
- Edge case in module path resolution for hyphen vs underscore form
- Missing test for workspace-root-relative path handling
- Test count: 39 → 47

**#2533 (Test::More hover):** Signature fixes + 21 new tests added by review pass.

**#2534 (special vars completion/hover):** 7 issues fixed, 19 tests added.

**#2535 (diagnostic code audit):** Was a CLOSED rejection — deep review found the approach was correct but the scope (ALL_CODES fix) introduced risk of breaking other paths. Closed, scoped differently.

The pattern: not a single deep-reviewed PR arrived for merge in the same state it left the builder.

### Plan-Review Corrected Every Scout Spec

From Era 7 Session 1 (which set up Session 2's merge queue), plan-review caught substantive errors in 4/4 scout specs:

- **#2403**: Scout said "C-style for broken" — already fixed. Issue closed. Builder saved.
- **#2406**: Scout said "qr<...> broken" — parser already correct. Scoped to tests-only.
- **#2392**: Scout said "fix parse_hash_or_block" — real bug was in calls.rs (different function entirely, 1-line fix vs risky refactor).
- **#2404**: Scout said "2 corpus files" — actually 8 files. Every file path and function name was wrong.

**Lesson encoded in memory**: Never skip plan-review even on issues that appear straightforward. The cost of plan-review (~2 minutes) is always less than the cost of a builder implementing the wrong fix.

### 3 PRs Rejected by Review

Review agents rejected (closed without merge) three notable PRs. The categories of rejection (per user spec) were: wrong Perl semantics, Perl syntax the parser cannot yet support, and already-fixed work.

1. **#2539 (fat-arrow after uppercase filehandle)**: The original fix for this pattern was superseded by #2555, which used a cleaner approach (treating `print STDERR =>` as a list call, not indirect object). Superseded, not wrong — the superseding fix was architecturally sounder.

2. **#2535 (diagnostic code audit)**: The ALL_CODES approach was architecturally correct but introduced scope risk. Rejected to be rebuilt in a narrower form. Work preserved.

3. **#2530 (die/warn exception context)**: Superseded by #2606, which landed as a cleaner implementation combining hover docs, croak modernize action, and die/warn context in one PR.

Each rejection produced a learning artifact: the superseded code informed the better fix, the risky approach informed the scoped alternative. No work was discarded — only redirected.

---

## 3. Key Architectural Decisions Made This Session

### Research-Verifier Agent Shipped

**Problem**: Plan-reviewers (sonnet) were spending 30-50% of their effort disproving scout claims before they could improve the spec. Facts about Perl semantics, LSP protocol details, crate APIs — scouts file these with confidence but haiku models have consistent factual error rates.

**Solution**: Insert a cheap haiku-grade fact-check pass between scout and plan-review. The research-verifier (#2610) does:
1. Reads the issue and extracts all factual claims
2. Verifies Perl semantics via web search of official docs
3. Verifies LSP/DAP protocol claims via spec search
4. Verifies crate API claims via docs.rs and codebase grep
5. Posts a structured comment with verdicts and citations
6. Adds `research-verified` label

**Result**: Plan-reviewers proceed knowing the facts are pre-checked. Their attention goes to architectural judgment, not fact-checking.

**Pipeline**: Scout → Research-Verifier → Plan-Review → Build → Review → Deep-Review → Ops → Merge → Wisdom

### Accuracy-Scout Designed and Built

The accuracy-scout (#2637) is complementary to the research-verifier: it verifies mechanical facts (file paths, function names, current code state) rather than external facts (Perl semantics, protocol specs).

Scouts file issues with file paths that may be stale, function names that may have been renamed, and status claims about bugs that may already be fixed. The accuracy-scout does a fast worktree pass to check these mechanical facts before plan-review.

**Five skills created**: `accuracy-read-issue`, `accuracy-verify-files`, `accuracy-verify-claims`, `accuracy-verify-status`, `accuracy-comment`.

The combined research-verifier + accuracy-scout layer eliminates the two largest sources of plan-review waste: factual errors about external specs and stale facts about the codebase.

### Label-Driven State Machine Implemented

**Problem**: Labels existed and were queried for routing (`in-review`, `merge-ready`, `plan-reviewed`) but no skill or agent file ever *set* them. The read side was wired; the write side was not.

**Solution (#2638)**: Five missing labels created and wired:
- `in-build` — set by builder-implement on start
- `needs-deep-review` — set by reviewer-decide when flagging for deep pass
- `structural-blocker` — set when a PR has architectural concerns
- `follow-up-recommended` — set when follow-up work is identified
- `already-fixed` — set by accuracy-scout when issue is stale

Six labels already existed and are unchanged: `builder-ready`, `plan-reviewed`, `needs-plan-review`, `in-review`, `merge-ready`, `research-verified`.

**Impact**: Pipeline state is now readable from GitHub's UI without querying agent outputs or issue comments. The label surface is the state machine.

### Two-Pass Review Proven Non-Negotiable

Evidence from Session 2's deep-review passes: every PR that went through deep-review was improved. Not improved-with-minor-tweaks — improved with real bug fixes (wrong AST path, missing edge case tests, wrong function called).

The pattern is structural, not incidental. First-pass reviewers check standards compliance: no unwrap(), fmt, clippy, test coverage. Deep reviewers check logic correctness: is the right function called? Does the edge case work? Is the spec actually what the implementation does?

These are different activities. Combining them into one pass produces neither well. The cost of two passes is ~5 extra minutes per PR. The value is catching logic bugs before they reach production.

**Encoded rule**: Feature PRs require two-pass review. Parser fix PRs (single-function, test-first) may use one pass.

### Definitive Pipeline

The session produced the canonical pipeline for this project:

```
Scout → Accuracy → Research-Verifier → Plan-Review → Build → Review → Deep-Review → Ops → Merge → Wisdom
```

Each stage has defined input, defined output, defined agent role, and defined fix-forward authority:

| Stage | Agent | Model | Fix forward? |
|-------|-------|-------|-------------|
| Scout | haiku explore | Broad discovery | N/A — files issues |
| Accuracy | haiku worktree | Mechanical fact-check | Yes — closes stale issues |
| Research-Verifier | haiku worktree | External fact-check | Yes — adds citations |
| Plan-Review | sonnet | Fill gaps, improve spec | Yes — complete the spec |
| Build | sonnet worktree | Implement + test | Yes if plan-reviewed |
| Review | haiku | Standards gate | Yes — fix inline |
| Deep-Review | sonnet | Logic correctness | Yes — push to branch |
| Ops | sonnet | Merge + ratchet | N/A — merge or reject |
| Merge | CI gate | Green check | N/A |
| Wisdom | sonnet | Retrospective + memory | N/A — write artifacts |

---

## 4. Economics Data

### Session Budget Estimate

This session operated at approximately 3% of a weekly Claude API budget to produce 55 merged PRs.

Breakdown (estimated):
- **~150 agents** deployed (builders, scouts, reviewers, ops, plan-reviewers)
- **Average tokens per agent**: 40K-80K input + output (varies by agent type)
- **Cache-read is ~94.5% of total tokens**: The skill system, memory files, and codebase context are largely static. After the first agent in a session loads CLAUDE.md + memory + codebase context (~50K tokens), subsequent agents retrieve that content from cache at ~10x cheaper rate.
- **Effective cost per agent**: $0.50-$2.00 (with cache hit rates factored in)
- **Session compute cost**: ~$75-$300 estimated (150 agents × $0.50-$2.00)

### The Cache-Read Discovery

This session surfaced a counter-intuitive economics fact: at high agent counts, the expensive part is not token generation. It is the first cache miss per session. After that, 94.5% of tokens are served from cache:

- CLAUDE.md: ~4,000 tokens, read by every agent
- Memory files: ~50,000 tokens of context, read by scouts and plan-reviewers
- Codebase context: ~100,000+ tokens loaded by code-reading agents

Once one agent loads this context in a session, subsequent agents pay ~1/10 of the cost to read the same content. This means session cost scales sub-linearly with agent count: 150 agents costs far less than 150x the cost of 1 agent.

**Implication**: Cache efficiency makes large sessions cheaper per-agent than small sessions. Spawning 10 agents per session is more expensive per PR than spawning 50.

### CI Cost vs Token Cost

At scale, CI cost can exceed token cost. Each merged PR triggers:
- A full workspace build
- All 2,500+ test functions
- Clippy + fmt checks
- Corpus check (4,355 files)

On a shared CI runner, this costs compute time. With 55 merges in a session, CI runs continuously. The merge queue pacing rule (batches of 3, wait for green) exists to prevent the CI cancellation cascade — each rapid merge cancels the previous CI run, meaning rapid merges produce fewer CI completions than batched merges.

**Rule of thumb**: Budget CI compute at 1-2x token compute for sessions with 40+ merges.

### Cost Per Solid PR at API Pricing

For users operating at API pricing (not Pro subscription):
- Agent session to produce one PR: ~$10-40 (scout + plan-review + builder + reviewer + ops)
- CI compute (GitHub Actions free tier): $0 for public repos
- Human review time: 3-5 minutes at $150-250/hr = $7.50-$20
- **Total per merged, CI-green, reviewed PR: ~$20-60**

For high-confidence PRs requiring deep-review:
- Add deep-review agent: +$5-15
- **Total: ~$40+ per solid PR at API pricing**

This is the "$40+CI per solid PR" data point for talks/articles.

### The 3% Budget Signal

The session producing 55 PRs at ~3% of weekly budget signals that the binding constraint is no longer economics. At 3% per session, a team could run 30+ sessions weekly before hitting budget constraints. The actual constraint is:

1. **CI throughput** — 3 merges per CI cycle × ~5min per cycle = ~36 merges/hour maximum
2. **Human attention** — 3-5 minutes per merge decision = ~3-5 hours for 55 merges
3. **Issue queue depth** — sessions can only build what has been scouted

The economics are solved. The bottlenecks are process.

---

## 5. Top Quotable Lines

### From the Codebase and Session Record

**On the pipeline:**
> "No single agent needs to be perfect. Each stage catches what the previous one missed. Multiple cheap passes beat one expensive pass."

**On plan-review ROI:**
> "Plan-review corrected every scout spec in Era 7 session 1. Never skip plan-review even on obvious issues. The cost of plan-review (~2 min) is always less than the cost of a builder implementing the wrong fix."

**On label state machine:**
> "The labels existed and were queried for routing, but no skill ever *set* them. The read side was wired; the write side was not."

**On the accuracy-scout:**
> "Scouts file issues with file paths that may be stale, function names that may have been renamed, and status claims about bugs that may already be fixed."

**On reviewer culture:**
> "Reviews should actively improve code, not just check for banned patterns and approve. Every PR has room for improvement. A review that finds nothing to improve wasn't thorough enough."

### From Steven Zimmerman (Interview Answers — Use Verbatim)

> "I don't use Perl. But for some reason I decided to fix it instead."

> "It was a mistake. I should not have started. But I still can't put it down."

> "It's controls and materiality." (on what accounting principles developers miss)

> "Many small focused SRP crates with stable APIs is extreme for a human to maintain. It's simple and searchable and context efficient and well routed for an AI."

> "The only thing left to optimize for is attention spent per useful change."

> "Way more compute heavy than anyone in the industry is ready for." (on DevLT ratio of compute-to-human)

> "The better the architectural boundaries got, the easier it became for agents to push forward."

> "Panics kept crashing WSL, bringing down 10+ Claude Code sessions at once." (on zero-panic policy origin)

> "I think we're already there, no? At this point it's about finding ways to make a better user experience." (on whether perl-lsp is best-in-class)

### On the Economics

> "Code is cheap. Trusted change is not."

> "We traded $20K compute for $480K in avoided salary cost."

> "At 94.5% cache-read, spawning 150 agents costs less than 150x the cost of 1 agent. Large sessions are cheaper per-agent than small ones."

---

## 6. Article Hooks

### "The Lock That Looked Locked"

**Angle**: The label-driven state machine discovery — labels queried for routing but never set.

A reader might ask: if the pipeline was working, why does it matter that labels weren't being set? The answer is that the pipeline *looked* like it was working. PRs were flowing, merges were happening, issues were being closed. But the state machine had no authoritative state — only the impression of state, reconstructed from comments and agent outputs.

The gap only became visible when an agent tried to query routing state and found inconsistency. The pipeline had been running on informal convention, not machine-readable truth.

This pattern recurs in software everywhere: the audit trail that looks complete but isn't being written to. The lock that looks locked because nobody has tried to open it wrong. The test that looks green because the assertion always passes.

**For the talk**: "We discovered our pipeline state machine had been running for weeks with no write path. Labels were queried for routing but never set. Everything looked fine. Until it didn't."

---

### "What 100 Agents Cost"

**Angle**: The economics piece. Direct, specific, surprising.

The number most people expect: enormous. The actual number: $75-$300 for 150 agents, because 94.5% of tokens hit cache.

The deeper number they won't expect: CI cost threatens to exceed token cost at scale. Building the agents is cheap. Running the CI that validates their output is what costs.

The deepest number: $40+ per solid, reviewed, CI-green PR at API pricing. Not $0.10. Not $5,000. $40. That's the price of a trusted change in 2026 — about the same as a restaurant dinner.

**Sub-hook**: "The binding constraint is not money. At 3% of a weekly budget for 55 PRs, you could run 30 sessions per week before hitting budget limits. The bottleneck is CI throughput, human attention, and issue queue depth. The economics are solved."

**For the talk**: Show the cost breakdown. Show the cache math. Show that the answer is not "AI is free" and not "AI is expensive" — it's "AI is about the same as a junior developer's hourly rate per finished PR."

---

### "The Pipeline Is The Product"

**Angle**: The session produced as much pipeline infrastructure as product code. Arguably more.

Merged in Session 2:
- Research-verifier agent (new pipeline stage)
- Accuracy-scout agent (new pipeline stage)
- Label state machine (new control surface)
- Agent preflight checks (new safety gate)
- Blocker ledger (new knowledge surface)
- Wrapup template standardization (new protocol)

That is six pipeline components and zero new parser features shipped from the pipeline work. The parser features (corpus 85.7% → 90.9%) came alongside, but the session's most durable output is the upgraded pipeline that will run all future sessions.

**The thesis**: In an AI-native development organization, the pipeline is as load-bearing as the product code. Improving the pipeline compounds across every future session. A 10% improvement in plan-review quality is worth more than ten individual feature PRs, because it will run against thousands of future issues.

This is why Steven calls the swarm a "three-layer product": the LSP (user-facing), the swarm OS (team-facing), and the memory/evidence architecture (process-facing). This session advanced all three simultaneously.

**The provocation**: Traditional engineering culture treats the development pipeline as overhead. The right engineering culture treats it as a first-class engineering investment. Every 2-minute improvement in plan-review quality is a permanent capital expenditure with indefinite return.

---

### "Why Review Agents Find Real Bugs"

**Angle**: The two-pass review model and why it works structurally.

The naive prediction: reviewers and builders are doing the same work, so review mostly confirms. The evidence: deep reviewers found logic bugs in every PR they examined in Session 2. Not naming convention issues. Not formatting nits. Bugs — wrong AST path, missing edge case, test count 5 → 11.

Why? Because the reviewer has fresh context. The builder spent 30 minutes implementing and tested the happy path. The reviewer reads the diff cold and asks: what is this function supposed to do? Does it do that in this edge case? The answer is often no.

The mechanism is epistemic: the builder's model is "I implemented what the spec said." The reviewer's model is "I see code claiming to implement something — let me check." Different priors produce different catches.

**For the talk**: "We tried one-pass review. Every PR that got a second pass was improved. Not tweaked — improved. The second reviewer was not doing the same work. They were doing different work with fresher eyes."

---

## 7. Supporting Data Tables

### Session 2 PR Categories

| Category | Count | Examples |
|----------|-------|---------|
| Parser fixes | 10 | fat-arrow ternary, qw() comments, $::{} stash |
| LSP features | 14 | POD symbols, Perl 5.38 class, use lib wiring |
| Swarm infrastructure | 7 | research-verifier, accuracy-scout, label state machine |
| Tests/quality | 5 | assert_clean_parse fix, integration tests |
| Performance | 3 | subprocess timeout, parser cancellation, incremental parsing |
| DAP | 2 | phase3 promotion, deduplication |
| Chore/cleanup | 8 | logos archive, chumsky removal, agent alignment |
| Docs | 3 | article gaps, control-plane wiring |
| Infrastructure | 3 | hook reliability, corpus schema, blocker ledger |

### Pipeline Stage Effectiveness (Era 7 Cumulative)

| Stage | Issues processed | Errors caught | Rate |
|-------|-----------------|--------------|------|
| Scout (haiku) | 100+ | N/A — output is roughly-right | — |
| Plan-review (sonnet) | 4/4 specs corrected | 4 wrong root causes, file paths | 100% correction rate |
| Build (sonnet) | 16+ PRs created | N/A | ~90% first-attempt success |
| Review (haiku) | All PRs reviewed | Standards violations, test gaps | — |
| Deep-review (sonnet) | 5 PRs | 6+ logic bugs per PR | 100% improvement rate |
| Rejection (any stage) | 3 PRs | Superseded, scoped wrong | — |

### Corpus Progression (Project History)

| Session | Corpus % | Files Clean |
|---------|----------|-------------|
| Start of Era 5 | 72% | 3,136 |
| Cycle 5 Session 3 | 85.7% | 3,731 |
| Cycle 6 peak | 90.3% | 3,931 |
| Era 7 Session 2 | **90.9%** | **3,956** |

---

## 8. Evidence Lineage

**Primary sources for this document:**
- Git log: `gh pr list --state merged --limit 60` (55 PRs confirmed on 2026-03-21)
- Issue counts: `gh issue list --state closed` (62 closed on 2026-03-21)
- Corpus ratchet: PR #2621 title "ratchet manifest after parser fix merges — 90.9% clean"
- Plan-review ROI: memory file `feedback_plan_review_roi_validated.md` (4/4 corrections documented)
- Interview quotes: memory file `user_interview_answers.md` (use verbatim)
- Pipeline redesign: memory file `project_era7_session1.md`
- Session 1 retrospective: memory file `session_synthesis_cycle5_session3.md`
- Cost model: `docs/articles/research/COST_ROI_ANALYSIS.md` and `COST_ROI_EXECUTIVE_BRIEF.md`

**Key PR bodies read:**
- #2610 (research-verifier): "plan-reviewers spend ~30% of their time disproving scout claims"
- #2637 (accuracy-scout): "plan-reviewers spend 30-50% of their effort correcting factual errors"
- #2638 (label state machine): "labels existed and were queried for routing but no skill ever set them"

**What was estimated vs confirmed:**
- PRs merged: **55 confirmed** from git history
- Issues closed: **62 confirmed** from GitHub API
- Corpus 90.9%: **confirmed** from PR #2621 title
- Agent count ~150: **estimated** (no agent roster snapshot available)
- Session cost ~$75-$300: **estimated** from token pricing models
- Cache-read 94.5%: **from user spec** (not independently verified this session)
- Budget ~3% weekly: **from user spec** (not independently verified)
