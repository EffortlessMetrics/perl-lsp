# Anatomy of a Session: What Happens When 60 AI Agents Build a Perl LSP for Seven Hours

*A case study in AI-native development from March 19--20, 2026 --- one human, 60+ agents, 38 PRs merged, corpus coverage from 72% to 86.8%, and twelve v0.13 features built in a single sitting.*

---

## The Plan That Wasn't

The session started with a plan. It was a good plan: merge 30 open PRs from the backlog, launch builders from the issue queue, ratchet the CPAN corpus, prepare for 0.12.0. The plan lasted about twenty minutes.

The first thing the triage agent reported back was that there weren't 30 open PRs. There were 95. GitHub's API paginates at 30 results by default, and every previous count had silently truncated at the first page. Nobody had noticed because nobody had asked for the real number. The backlog was three times larger than believed.

This was the session's first lesson, and it arrived before any code was written: **understand before acting**. The plan to merge and build was replaced, on the spot, with a plan to review and scout and *think*. No code for the first two hours. Just research.

It was the right call. The 95 PRs contained 8 duplicate clusters (multiple agents had independently fixed the same bugs), 15 conflict groups (PRs that touched the same files and couldn't merge cleanly together), and a handful of stale branches from two cycles ago that were never closed. Merging blindly would have created a mess. Understanding first created a strategy.

---

## Act 1: Bootstrap and Discovery

The session's opening moves were administrative. Close stale PRs. Identify duplicates. Map conflict groups. Version-bump to 0.12.0. None of this was glamorous. All of it was necessary.

The triage agent clustered the 95 PRs by crate, by conflict surface, and by staleness. Eight clusters of near-duplicates emerged --- cases where two or three agents, launched in different cycles, had independently discovered and fixed the same parser bug. In traditional development, this would be waste. In swarm development, it is information. The duplicate that took a different approach sometimes revealed a better solution. The triage agent's job was not to pick one and discard the rest, but to identify which approach was strongest and whether any of the alternatives contained insights worth incorporating.

Fifteen conflict groups were mapped --- PRs that edited overlapping files and would require careful merge ordering. The merge-order map became the playbook for the session's later merge waves.

Thirty-eight PRs were closed as stale, superseded, or duplicated. The open count dropped from 95 to 57. The remaining 57 were real, current, and ready for review.

| Metric | Before Triage | After Triage |
|--------|--------------|--------------|
| Open PRs | 95 | 57 |
| Duplicate clusters | 8 | 0 (resolved) |
| Conflict groups | 15 | mapped, ordered |
| Stale PRs closed | --- | 38 |

This took about an hour. It was the most valuable hour of the session, because everything that followed was built on a clean foundation.

---

## Act 2: The Research Wave

With the backlog understood, the session pivoted to its most distinctive phase: a research wave of 40+ simultaneous scouts, each investigating a different aspect of the project, the market, or the codebase.

The scouts were not writing code. They were asking questions:

- **Competitive landscape**: What do PerlNavigator, Perl::LanguageServer, and other tools actually support? Where are the gaps?
- **Cost and ROI**: What does it cost to run a swarm session? What is the return?
- **Edge cases**: What Perl constructs does the parser still fail on? Which error buckets are largest?
- **Architecture**: Where are the god files? What should be split?
- **Community**: What do Perl developers actually want from an LSP? What would make them switch?
- **Internal audit**: Is the async code sound? Are there hidden panics? Does the test infrastructure have blind spots?

Forty scouts returned forty reports. Some were expected. Some were not.

### The Finding Nobody Expected

One scout was tasked with auditing `assert_clean_parse`, the test helper that verifies a Perl file parses without errors. It found a case-sensitivity bug. The helper was comparing error messages case-sensitively, but some error paths produced messages with inconsistent capitalization. The result: 56 test cases that appeared to pass were silently not checking what they claimed to check. They parsed the file, got errors, but the error string didn't match the assertion's expected pattern, so the assertion *vacuously succeeded*. Fifty-six tests that proved nothing.

This is the kind of bug that a human reviewer would never find by reading code. The test suite was green. The helper function was correctly implemented. The bug was in the *interaction* between the helper's matching logic and the parser's error message formatting --- a gap that only appears when you audit every call site against every possible error output.

The scout filed an issue. A builder fixed it in the same session.

### The Finding From Outside

One research thread was not run by a scout agent. It was run by a different AI system entirely --- ChatGPT, analyzing the project from the outside, with no access to the codebase or the swarm infrastructure. It was given the project's README, a few articles, and asked: what is this project, really?

Its answer reframed the entire project. It identified three layers that no internal scout had articulated:

1. **The product layer**: A Perl LSP server that aims to be the best in class.
2. **The methodology layer**: A swarm development system that uses the LSP as its proving ground, but is itself a product --- a replicable methodology for AI-native software development.
3. **The evidence layer**: A memory and metrics system that captures what works, what fails, and why --- creating an institutional knowledge base that compounds across sessions.

The project's maintainer had been building all three layers simultaneously without naming them separately. The external analysis gave them names, and with names came clarity about what to prioritize and why.

### The Greenfield Discovery

The competitive landscape scout returned a number that changed the session's strategy: **78% of Perl developers use no LSP at all**. The market was not a fight for share against existing tools. It was a greenfield opportunity. The majority of potential users had never tried language server support for Perl because nothing had been good enough to bother with.

This shifted the v0.13 feature priorities. Instead of chasing feature parity with competitors, the focus moved to first-run experience, installation friction, and the features that would make a skeptical Perl developer try an LSP for the first time.

---

## Act 3: Research Drives Building

The research wave was not an academic exercise. Every scout report became a builder specification. This was the session's core pattern: **constrained tasks from verified findings**.

The builders did not receive vague instructions like "add perlcritic support." They received findings: "The parser already handles 85% of perlcritic's input requirements. The missing piece is wiring the existing `perlcritic` binary detection in `perl-workspace-discovery` to a new diagnostic provider. Here are the 30 lines that need to change."

This constraint pattern --- scout finds the gap, builder fills exactly that gap --- produced a 90% builder success rate. Unconstrained feature requests ("build Moose support") historically succeed about 50% of the time. Constrained specs ("the framework detection infrastructure exists at this path, create a `ClassModel` struct with these fields, wire it to completion at this call site") succeed almost always.

### What Got Built

Twelve v0.13 features were built in this session, each originating from a scout finding:

| Feature | Scout Finding | Builder Action |
|---------|--------------|----------------|
| **Perlcritic diagnostics** | Parser 85% ready, detection exists | Wired 30 lines to new provider |
| **Perl POD crate** | Module resolution works, pod extraction missing | Created `perl-pod` microcrate |
| **Moose/Moo class model** | Framework detection exists | Created `ClassModel` + completion wiring |
| **Parser cancellation** | Long files block the event loop | Added cancellation token threading |
| **Diagnostic debounce** | Rapid typing causes diagnostic storms | Added configurable debounce |
| **Security lints** | No taint-mode or eval warnings | Created lint provider |
| **Unused import detection** | Scope analysis has the data | Wired to diagnostic provider |
| **Pragma hover** | Educational hovers missing for `use strict` etc. | Added pragma documentation |
| **Special variable hover** | `$_`, `@ARGV` etc. undocumented in hover | Added special var docs |
| **Shebang code action** | Missing `#!/usr/bin/perl` suggested as action | Created code action provider |
| **`check-project` CLI** | No batch validation tool | Created CLI subcommand |
| **v-string fix** | `v5.26.0` parsed as identifier | Fixed tokenizer |

Simultaneously, 11 parser fix PRs attacked every major error bucket identified by the scouts. The parser's CPAN corpus coverage --- the percentage of real-world Perl modules it can parse without errors --- moved from 72% to 85.4% to 86.8% across three ratchets in a single session.

### The Merge Waves

Thirty-eight PRs were merged across four waves, paced to avoid the CI cancellation cascade that earlier cycles had discovered the hard way. (When PRs merge in rapid succession, each merge triggers a CI run on the new HEAD, canceling the previous run. Merge three PRs in sixty seconds and you get three CI triggers but only one completion. The earlier PRs never get validated.)

| Wave | PRs Merged | Focus |
|------|-----------|-------|
| 1 | 8 | Infrastructure, stale cleanup |
| 2 | 12 | Parser fixes, corpus ratchet |
| 3 | 10 | Features, documentation |
| 4 | 8 | Remaining features, version bump |

Each wave waited for CI to go green before the next wave began. The merge-order map from Act 1 determined the sequence within each wave, ensuring that conflicting PRs merged in dependency order.

---

## Act 4: The Numbers

Seven hours. One human. Sixty-plus agents. Here is what they produced:

| Metric | Value |
|--------|-------|
| Agents deployed | 60+ |
| PRs created | 55+ |
| PRs merged | 38 |
| CPAN corpus coverage | 72% to 86.8% |
| Lines landed | ~13,500 |
| v0.13 features built | 12 |
| Parser fix PRs | 11 |
| Memory files (start) | 148 |
| Memory files (end) | 160+ |
| Orphaned worktrees cleaned | 52 (260 GB freed) |
| Structural issues filed | 8 |
| Estimated compute cost | $30--50 |

The corpus number deserves elaboration. CPAN corpus coverage measures the percentage of a curated set of real-world Perl modules --- drawn from the Comprehensive Perl Archive Network --- that the parser handles without errors. Moving from 72% to 86.8% in a single session means the parser went from failing on roughly 1 in 4 modules to failing on roughly 1 in 8. The absolute improvement is 14.8 percentage points. The relative improvement in error rate is 53% --- more than half the remaining parse failures were eliminated.

The cost number deserves context. Sixty agents, seven hours, $30--50 in API compute. A single senior developer costs $150--250 per hour. Seven hours of senior developer time is $1,050--1,750. The swarm produced more reviewed, tested, CI-gated changes in seven hours than a senior developer could produce in a month --- at 2--5% of the labor cost.

This is not a fair comparison, because the human was still there, directing. But the human's role was strategic, not mechanical. Set direction. Review results. Adjust priorities. The agents did the reading, writing, testing, reviewing, and merging. The human did the thinking.

---

## Act 5: The Swarm Debugging Itself

The most interesting thing about the session was not the code it produced. It was the session's recursive quality: the swarm improving its own methodology while simultaneously using that methodology to ship software.

### Validator Blind Spots

The `assert_clean_parse` bug was an instance of a deeper pattern: **validators that pass when they should fail**. The test infrastructure was designed to catch parser regressions, but it had a silent failure mode where case mismatches caused assertions to vacuously succeed. The swarm found the bug, fixed the bug, and then filed an issue to audit all other test helpers for similar blind spots.

The swarm was debugging its own quality infrastructure while using that infrastructure to validate its parser changes. Both activities happened in the same session, run by different agents, with no coordination beyond the issue queue.

### Memory Consolidation

The session started with 148 memory files --- operational learnings from five previous development cycles. During the session, 26 memories were merged into 7 (removing redundancy and contradiction), and 12 new memories were written. The memory system was pruned and extended simultaneously.

One new memory captured the session's most important operational insight: **the merge queue, not the agents, is the bottleneck**. The CI pipeline can process about 3 merges per cycle. With 55+ PRs created in a single session, the merge queue backs up. The optimal number of coding agents is not "as many as possible" but approximately 9 --- the number whose output the merge queue can process in a session.

This is a constraint that could only be discovered by hitting it. No amount of planning would have predicted that the CI merge queue, not agent capacity or human attention, would be the binding constraint on throughput. It took a 60-agent session to find the actual ceiling.

### The Three Ratchets

The CPAN corpus ratchet --- the mechanism that prevents parser regressions by recording the current set of cleanly-parsed modules and failing CI if any of them regress --- was applied three times in a single session:

1. **72% to 78%**: After the first wave of parser fixes merged.
2. **78% to 85.4%**: After the second wave, targeting the largest error buckets.
3. **85.4% to 86.8%**: After targeted fixes for edge cases discovered by the scouts.

Each ratchet locked in the gains from the previous wave, making regression impossible. The third ratchet was smaller than the first two because the remaining failures were harder --- not systematic bugs in the parser's handling of common constructs, but edge cases in rarely-used syntax. The law of diminishing returns was visible in real time.

---

## What Went Wrong

An honest accounting requires the failures.

**The PR count hit 64 open.** Despite merging 38 PRs, the session *created* more than it merged. The open PR count peaked at 64 before declining. This is the merge queue bottleneck made visible: the swarm can generate work faster than the pipeline can absorb it.

**Worktree contention.** Some agents were accidentally assigned to shared worktrees instead of isolated ones. When two agents edit files in the same worktree, they step on each other's uncommitted changes. This was a configuration error, not an architectural one --- the worktree isolation system works when used correctly --- but it cost two agents their work.

**False-positive audit findings.** Two scout reports flagged issues that turned out to be intentional design decisions, not bugs. Builders spent time investigating before realizing there was nothing to fix. The cost was small (an hour of agent time) but the pattern is worth noting: scouts that don't verify their findings against the git history can generate false positives.

**Branch conflicts.** Three builders couldn't complete their PRs because the files they needed to edit had been modified by PRs that merged during their build. They had to rebase and retry, which triggered additional CI runs and consumed merge queue capacity. The conflict was predictable from the merge-order map, but the builders were launched before the map was complete.

None of these failures caused data loss. Every agent's work was either merged, PRed, issued, or memoried. Nothing was thrown away. But the failures consumed time and capacity that could have gone to productive work.

---

## What Went Right

**Every scout finding became either a builder spec or an issue.** No research was wasted. The findings that couldn't be built in this session were filed as GitHub issues with full root-cause analysis, ready for the next session's builders.

**No data was lost.** The swarm's most important property is that every unit of work produces a durable artifact: a merged PR, a draft PR, a GitHub issue, or a memory file. Even failed builds produce issues. Even stale PRs produce learnings. The system is designed so that nothing falls through the cracks.

**v0.12.0 shipped.** All release blockers were cleared, the version was bumped, the changelog was written, and the release was tagged. This was not the session's primary goal --- the research wave took priority --- but it happened in parallel with everything else.

**The methodology improved.** Eight new operational learnings were encoded as memory files. The swarm's own processes were debugged and upgraded while the swarm was using them.

---

## The Session as Evidence

This session is, itself, a data point in the argument that AI-native development is a distinct methodology, not just "using AI to write code faster."

The human did not write code. The human did not review diffs line by line. The human did not run tests, fix CI, resolve merge conflicts, or debug parser edge cases. The human did four things:

1. **Set direction**: "Stop building. Start researching. Understand the backlog before acting on it."
2. **Made strategic decisions**: "The external analysis is right --- we're building three products, not one. Prioritize accordingly."
3. **Managed constraints**: "The merge queue can handle three per cycle. Pace the waves."
4. **Captured wisdom**: "That finding about assert_clean_parse --- that's not just a bug fix. That's a pattern. File an issue to audit all validators."

Everything else was delegated. Not delegated in the sense of "go away and come back with code" --- delegated in the sense of a conductor directing an orchestra. The human didn't play any instrument. The human decided what music to play, and when, and how loudly, and whether the tempo needed to change.

The agents played the instruments. Sixty of them, simultaneously, in different keys, producing a coherent output because the methodology --- the skills, the hooks, the memory, the pipeline stages, the merge ordering, the ratchets --- kept them in sync.

### The Research:Build:Ship Ratio

The session's time allocation reveals its methodology:

| Phase | Hours | Percentage |
|-------|-------|------------|
| Research and discovery | 3 | 43% |
| Building and fixing | 2 | 29% |
| Merging and shipping | 1.5 | 21% |
| Meta-improvement | 0.5 | 7% |

A 4:2:1.5 ratio of research to building to shipping. In traditional development, the ratio is inverted: most time goes to writing code, with review and integration as afterthoughts. In this methodology, understanding the problem takes longer than solving it. This is deliberate. The research wave's 43% of session time produced the constrained specs that made the build wave's 90% success rate possible.

Cheap research. Expensive building. If you're going to spend $30--50 on compute, spend $15 understanding the problem and $15 solving it. Don't spend $30 solving and $0 understanding.

---

## The Deeper Pattern

Every tool era in this project's history unlocked a new capability:

- **Opus Direct** (Era 1) unlocked *quality* --- one human, one AI, deep context, careful work.
- **Early Swarms** (Era 2) unlocked *parallelism* --- multiple agents, crate isolation, PR-based workflow.
- **Architecture** (Era 3) unlocked *structure* --- ADRs, mutation testing, Nix, deliberate design.
- **Claude Code** (Era 4) unlocked *velocity* --- skills, hooks, memory, automated pipelines.
- **Team Swarms** (Era 5) unlocked *scale* --- 60+ agents, coordinator model, merge queue pacing.

This session was the first time Era 5 operated at full capacity for a sustained period. The result --- 38 merged PRs, 14.8 points of corpus improvement, 12 features built, 8 methodology improvements --- is not the ceiling. It is the baseline. The next session will start with the memories this session wrote, the issues this session filed, and the methodology improvements this session encoded.

The session after that will be faster still.

That is the compounding effect. Not in the code --- code is cheap. In the *trust infrastructure* that turns code into shipped, tested, reviewed, regression-proof changes. Each session makes the trust infrastructure a little better, which makes the next session a little more productive, which produces more learnings to feed back into the infrastructure.

The methodology was always trying to exist. It just needed enough iterations to discover itself.

---

## Appendix: Session Timeline

| Time | Event |
|------|-------|
| T+0:00 | Session start. Plan: merge 30 PRs, build from issues. |
| T+0:20 | Triage agent reports 95 open PRs (not 30). Plan discarded. |
| T+0:30 | Strategic pivot: research before building. |
| T+1:00 | Triage complete: 38 stale PRs closed, 8 duplicate clusters resolved, 15 conflict groups mapped. |
| T+1:30 | 40+ scout agents deployed simultaneously. |
| T+2:30 | Scout reports arriving. assert_clean_parse bug discovered. External analysis identifies three-layer product. |
| T+3:00 | Research wave complete. Builder specs written from scout findings. |
| T+3:30 | 12 builder agents launched with constrained specs. 11 parser fix agents launched. |
| T+4:30 | First merge wave: 8 infrastructure PRs. |
| T+5:00 | Corpus ratchet 1: 72% to 78%. Second merge wave begins. |
| T+5:30 | Corpus ratchet 2: 78% to 85.4%. Feature PRs arriving. |
| T+6:00 | Third merge wave. v0.12.0 version bump merged. |
| T+6:30 | Corpus ratchet 3: 85.4% to 86.8%. Final merge wave. |
| T+7:00 | Session wind-down. Memory consolidation. Issue filing. Methodology improvements encoded. |

---

*March 20, 2026. perl-lsp v0.12.0. CPAN corpus: 86.8%. Session cost: ~$40. Agents deployed: 60+. Human attention: 7 hours. Trusted changes shipped: 38.*
