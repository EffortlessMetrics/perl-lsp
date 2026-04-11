# Investigation Depth Exceeds the Brief

*A short meta-pattern from the 2026-04-11 perl-lsp session: summaries are
compressed, forensics reliably decompress them, and the specifics that fall
out during the decompression are the parts that actually drive decisions.*

---

## TL;DR

During the 2026-04-11 session the orchestrator produced several one-line
framings of session findings -- "a PR was caught just before merging", "the
scout found 8 undersold subsystems", "research-verifier caught the false
premise", "the walk_node workaround was based on the same false premise".
Every one of those sentences is true. Every one of them was also missing
something specific and operationally load-bearing, and a later forensic writeup
reliably surfaced the missing thing: a 9-minute merge-ready window, a scout
whose own numbers were stale, two review agents running in parallel rather than
in sequence, a 2-second gap between a stale-close and a merge. The meta-pattern
is that summary compression is **lossy in a predictable direction** -- it
preferentially drops quantitative specifics and temporal geometry -- and that
**every summary worth reading is worth writing the deep forensic for**, because
the forensic reliably recovers ~3-5 sub-findings per summary that would
otherwise be lost.

This is not a process failure or a rigor failure. It is an intrinsic property
of how summaries work, and naming it means the pipeline can budget for the
decompression step on purpose.

---

## The four examples

### Example 1 — "The PR was caught just before merging"

**Summary framing:** PR #4090, a false-premise pragma fix, was caught by a
parallel research-verifier agent before it could merge.

**What the forensic ([#4127](https://github.com/EffortlessMetrics/perl-lsp/pull/4127))
surfaced:**

- `merge-ready` was applied at **10:58 UTC** and held at **~11:07 UTC**. That
  is a **9-minute window** in which, if ops had happened to run and the CI
  rollup looked green, the false-premise fix would have merged silently.
  "Caught just before merging" doesn't convey "9 minutes"; "9 minutes" does.
- The catch was the **5th check** performed on the PR, not the 1st. Four prior
  checks -- the scout's mental model, the builder's 6 BDD tests, a 10:42
  failed closure attempt using `perl -c`, and the deep reviewer's trace-through
  of the implementation -- each **reinforced** the false premise rather than
  catching it. Only the 5th check (research-verifier running `perl -e` with an
  experiment whose output would differ under the false hypothesis) was actually
  diagnostic.
- The 10:42 failed closure attempt is itself a sub-lesson hidden inside the
  sub-lesson. Someone with the right directional instinct reached for Perl
  verification **19 minutes before the catch** -- but used `perl -c`
  (compile-only) instead of `perl -e` (execute). `perl -c` returned `syntax OK`
  on the false-premise code because a strict-vars error happens at runtime, not
  at compile time. The right lens, the wrong flag. The PR kept moving.

Neither the merge-ready window, the check ordering, nor the `-c` vs `-e`
distinction would have surfaced if someone had read "research-verifier caught
it" and stopped.

### Example 2 — "The scout found 8 undersold subsystems"

**Summary framing:** a substrate-undersell scout catalogued 8 subsystems
(refactoring, hover, completion, code actions, inlay hints, semantic tokens,
benchmarks, workspace index) that were undersold in `features.toml`.

**What the forensic ([#4128](https://github.com/EffortlessMetrics/perl-lsp/pull/4128))
surfaced:**

- The scout's own numbers were themselves undersold. The scout cited "15
  semantic token types + 7 modifiers" for `perl-lsp-semantic-tokens`. The
  deep-dive agent ran an actual grep against `semantic_tokens.rs:159-209` and
  found **23 types + 13 modifiers**. The scout was reading a crate-local
  `CLAUDE.md` that hadn't been updated as the legend grew.
- `perl-refactoring` is not just "a crate the catalog doesn't mention" -- it is
  **264 `#[test]` functions across 6,284 test lines**, covering workspace-wide
  rename, import optimization, modernization, extract-module, inline-subroutine,
  and scoped rename. The single largest undersell of the session, and "8
  undersold subsystems" gives you none of those adjectives.
- `perl-lsp-hover` is not "13 generation functions" -- it is **2,839 lines
  across 41 functions**, with specialized renderers for POD markdown, XS
  typemap, Moo attributes, inherited-method resolution, and method dispatch
  chains. The scout's 13 was a conservative floor drawn from a stale source.
- `perl-lsp-completion` has **18 source files** (including test_more helpers,
  DBI type inference, XS API completion, Moo option keys), not the "6+
  specialized sources" the scout estimated.

The underselling pattern was **recursive**. The scout's source was already
downstream of reality, so the investigation *of* underselling was itself
undersold. The real rollup is bigger than the scout's summary -- and nobody
knew until someone did the grep. That recursion is invisible in the
8-subsystems framing and unmissable in the forensic.

### Example 3 — "Research-verifier caught the false premise"

**Summary framing:** the research-verifier agent dispatched on #4090 caught
that the PR's phase-block pragma claim was false.

**What the forensic ([#4127](https://github.com/EffortlessMetrics/perl-lsp/pull/4127))
surfaced:**

- The research-verifier was running **in parallel** with the deep reviewer, not
  after it. The deep reviewer posted `reviewed-deep` and approval at
  **10:57**; the research-verifier posted the false-claim finding at **11:01**.
  The sequence was: deep-review approval -> **4-minute gap** ->
  research-verifier finding -> orchestrator hold.
- The deep reviewer's approval was therefore **already invalid by the time it
  was posted** because the research-verifier's verification work was in
  progress and would catch the error within four minutes. The two agents were
  operating on the same PR with overlapping time windows and reached
  contradictory conclusions in the same five-minute span.
- "Research-verifier caught it" doesn't convey that **near-simultaneity**, and
  the near-simultaneity is the operationally significant part. It means the
  two agents were already running as complements, not as sequential review
  layers -- which is worth naming as a deliberate dispatch tactic:
  **research-verifier runs in parallel with deep review, not after it**.

If the research-verifier had been gated on "after reviewed-deep, before
merge-ready", the ops queue could have picked the PR up first. Parallel
dispatch was the only reason the catch arrived in time. That is a tactic; "the
right agent caught the error" is not.

### Example 4 — "The walk_node workaround was based on the same false premise"

**Summary framing:** PR #4052 (eval-STRING pragmas) had merged with a
`walk_node` `PhaseBlock` body scan that was based on the same false premise as
#4090. Both layers of the fix shared the misconception.

**What the forensic ([#4127](https://github.com/EffortlessMetrics/perl-lsp/pull/4127))
surfaced:**

- Issue **#4084** closed stale at **10:22:00 UTC**. PR **#4052** merged at
  **10:22:02 UTC**. **Two seconds apart.** The `walk_node` workaround landed
  two seconds after the three failing integration tests that would have proved
  it wrong were silenced by a stale-close.
- The temporal proximity set up a **two-layer corroboration trap**: had #4090
  also merged, master would have contained two independent-looking edits (one
  at the `PragmaTracker` layer, one at the diagnostic layer) that agreed with
  each other about the false premise. Two layers of the stack making the same
  claim looks like **independent confirmation**. It wasn't -- both layers were
  downstream of the same shared blind spot -- but to a future reviewer
  questioning either edit, "the other layer does the same thing" would be the
  first defense.
- "Based on the same false premise" communicates the logical relationship and
  hides the **2-second window** that allowed the trap to assemble. Two seconds
  is the story: a scout spec, an accidental stale-close, and a merge all
  converged in the same wall-clock instant. Any reasonable coordination
  protocol would have prevented the near-miss if it had fired in time, but no
  protocol has 2-second resolution.

The temporal proximity is the interesting fact. The summary discards it.

---

## The meta-pattern generalized

All four examples have the same shape. In each case the summary is **true**,
the forensic detail is **specific and new**, and the specifics are the parts
that would inform a future decision:

- "9 minutes at merge-ready" tells you whether the current ops cadence is tight
  enough for the risk tolerance. "Caught just before merging" doesn't.
- "Scout cited 15+7, reality is 23+13" tells you that scout data itself needs
  verification against live tree state, and that stale crate-local docs are a
  vector for recursive underselling. "8 undersold subsystems" doesn't.
- "Research-verifier was parallel, not sequential" tells you to dispatch the
  two review classes concurrently. "The right agent caught it" doesn't.
- "2 seconds between #4084 closing and #4052 merging" tells you that any
  coordination protocol slower than 2 seconds can't prevent this class of
  trap -- which points at structural fixes rather than process fixes. "Both
  layers shared the false premise" doesn't.

The pattern is not that summaries are careless, or that the forensics are
finding things the summary authors missed. The pattern is that **compression
is lossy in a predictable direction**, and the decompression step reliably
recovers what was lost.

---

## Epistemological framing

Summary compression loses two things preferentially: **quantitative specifics**
and **temporal geometry**. Abstract narratives -- "caught just before merging",
"found 8 subsystems", "was based on the same false premise" -- survive because
they carry the moral of the story. Concrete numbers -- "9 minutes", "23 types",
"2 seconds", "10:22:00 vs 10:22:02" -- don't survive because they don't feel
load-bearing at the moment of summarization. The moral is what the author
thought the reader needed; the numbers are what the moral was built on top of.
Once the moral is the only thing surviving the compression, the numbers are
gone until someone goes back and re-derives them.

This matters for a project that wants to **ratchet**. Ratcheting requires
specific numbers. You can't ratchet "undersold substrate"; you can ratchet "23
semantic token types, up from 15". You can't ratchet "the PR was caught in
time"; you can ratchet "9-minute merge-ready window, target under 5". Which
means that for every summary-level finding that is going to inform a ratchet or
a structural decision, the deep-dive step **has to happen reliably** -- not
just when someone remembers it, not just when the finding looks interesting.

---

## Operational implication

**Every summary worth reading is worth writing the deep forensic for.** Not
because the summary is wrong -- the summary is compressed -- but because the
forensic step reliably surfaces sub-findings that the summary never carried.
The cost is modest (one agent, 30-60 minutes, one file); the value is "you
learn things you didn't know you didn't know".

When the orchestrator produces a summary framing -- "X happened", "we caught
Y", "the scout found Z" -- that is the **signal to dispatch a deep-forensic
agent**, not a stopping point. The summary is the hypothesis. The forensic is
the verification.

A concrete heuristic from this session: **if you can describe a session finding
in one sentence, that sentence is probably losing ~3-5 material sub-findings.**
The four examples above each produced roughly that many novel specifics when
they were decompressed into forensics. If the ratio holds in future sessions,
every summary-level finding is the tip of an iceberg with measurable mass
below the waterline.

The pairing rule, stated plainly: **a summary that would otherwise become
load-bearing for a decision demands a forensic before the decision is made.**
Summaries that are informational only ("the session merged 8 PRs", "CI was
green") don't need the treatment. Summaries whose sub-findings would change
*what is done next* do.

---

## What NOT to do

- **Don't forensic-ify every observation.** The cost is nonzero. The rule is
  "do the forensic when the sub-findings would affect a decision", not "do the
  forensic for every summary the orchestrator produces". Most session-level
  summaries are informational, not decision inputs.
- **Don't treat the summary as wrong.** It's not wrong; it's compressed. The
  forensic augments the summary, it doesn't replace it. Summary-level framings
  are still how session-level context propagates; they just shouldn't be the
  last word when the sub-findings matter.
- **Don't assume the summary's author missed something.** The pattern is
  intrinsic to compression, not to any individual author's rigor. The scout
  whose "15+7" turned out to be "23+13" wasn't sloppy -- they were reading the
  best available intermediate source, and the intermediate source was itself
  out of date. The pattern survives careful authors.

---

## Related reading

- **[#4127](https://github.com/EffortlessMetrics/perl-lsp/pull/4127) --
  PR #4090 false-premise cascade case study.** The forensic that surfaced the
  9-minute merge-ready window, the 5-check anatomy, the 10:42 `perl -c`
  sub-lesson, the parallel-dispatch geometry between deep reviewer and
  research-verifier, and the 2-second gap between #4084 closing and #4052
  merging. Source for examples 1, 3, and 4.
- **[#4128](https://github.com/EffortlessMetrics/perl-lsp/pull/4128) --
  Underselling pattern deep-dive (102 -> 116 -> ~150).** The forensic that
  surfaced the recursive underselling (scout cited 15+7, reality was 23+13),
  the `perl-refactoring` 264-test / 6,284-line scope, the hover 2,839-line /
  41-function scope, and the three structural root causes. Source for
  example 2.
- **[#4117](https://github.com/EffortlessMetrics/perl-lsp/issues/4117) --
  2026-04-11 session wisdom retrospective.** Project-level meta with 7 patterns
  across the whole session. This article goes narrower on one of them (the
  summary-vs-forensic compression gap) rather than re-covering the ground.
- **[#4125](https://github.com/EffortlessMetrics/perl-lsp/pull/4125) --
  Swarm-operations learnings from the 2026-04-11 session.** Covers orchestration
  mechanics and agent coordination; complementary to this article, which is
  about knowledge compression independent of orchestration.
- **[#4062](https://github.com/EffortlessMetrics/perl-lsp/issues/4062) --
  Layered metric scorecard design.** The umbrella issue this article threads
  into: if the project wants to ratchet metrics, it needs the forensic step
  to happen reliably so that the specific numbers survive the compression.

---

## A note on this article

This article is itself an example of the pattern it describes. The observation
"the orchestrator's summary-level framings kept losing material detail that the
forensics recovered" was itself a one-sentence summary during the 2026-04-11
session. Writing it down as a short article forced the decompression: naming
four examples rather than one, naming the recursion in example 2, naming the
parallel-dispatch tactic in example 3, naming the 2-second window in example 4.
None of those specifics were part of the original observation. The article
found them by doing to the meta-pattern what the meta-pattern says to do to
summaries.

That is not a coincidence. It is the test.
