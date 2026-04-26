# Triage as Learning: What Codex 4-Shot Variants Teach

**Date:** 2026-04-23

When Codex produces 4 PRs per prompt — or 60 across multiple prompts addressing the same topic — the default reaction is "pick one, close the rest." That's cost-efficient but it leaves value on the table.

The variants are **not random noise**. Each is a genuine design exploration against the same spec. Read together, they encode information the winner alone doesn't.

## What to extract during triage

### 1. Convergence = emergent consensus

When 48 of 60 variants name the same helper function, put the receipt JSON in the same top-level key, or split a trait the same way — that's the community-pattern consensus. It's the shape the winner should use, even if the winner you'd otherwise pick structures it differently.

Example from the UX-confidence cluster (#5138-#5196): across ~60 variants, the `workflow_count` field name, the `confidence_signals` tag array, and the `test_corpus/gold/<category>/expected_*.json` fixture layout were near-universal. Any winner choosing differently would have been the outlier — reviewer-deep rejected the one that did (#5184 split on `\n## ` when the actual heading was `#### What shipped`).

### 2. Divergence = the real design question

When 15 variants do approach A, 8 do approach B, and 4 do approach C, the cluster has surfaced the genuine architectural choice you need to make. The relative sizes are weak signal; the fact that three distinct approaches emerged at all means there's no obvious right answer, and you need to decide explicitly.

The UX scorecard cluster had exactly this: **(a) fixture-backed signal collection (`#5154`)**, **(b) xtask plumbing with comprehensive schema+docs (`#5184`)**, **(c) minimal plumbing only (`#5138`)**. The triage agent correctly identified these as architecturally independent and preserved all three — until the deep-review found that (b) and (c) had real bugs and couldn't layer with (a)'s JSON shape. Only then did we collapse to a single winner.

### 3. Edge cases leak through "losers"

A "loser" that used a different test fixture may have caught something the winner missed. The fix-forward on **#5269** was found this way: a sibling "losing" variant had a test asserting parse errors on `my $x = (1 + ;` (unterminated expression). The winner's test asserted parse errors on `print "hi"` (missing `;`) — which is actually **valid Perl**. Without reading the sibling, the winner's false-green test would have shipped.

### 4. What stacked patterns look like

Sometimes the "losers" stacked their fixes — variant A added the helper, variant B added A's helper plus a guard, variant C added B's plus a third case. Read the stack, and you can import the complete fix into the chosen winner even if it was based on a smaller subset. That's how #5107 ended up getting the missing dotfile guard from #5104 during fix-forward.

## When NOT to deep-triage

- Pure-noise clusters (4 identical reword PRs of the same doc line): just close 3. No learning.
- Trivially-equivalent PRs (same regex, same test, same commit message): just close. No learning.
- Malformed clusters (1 PR does real work, 3 are `.tmp` artifacts): just close the artifacts.

Triage-as-learning pays off when the 4-shot Codex wave produced genuinely different attempts — which is its default output mode for non-trivial specs.

## The meta-lesson

**Codex's 4-shot design is a feature.** Don't fight it with "one PR per prompt" back-pressure. The diversity is doing design work for you. Your job as triager is to absorb that work, then commit to the one that matches.

If you only close-and-merge, you treat Codex as a typist. Codex is cheap enough to be a brainstormer too.

---

_Companion: `docs/articles/TWO_MODE_DEV_LOOP.md`, `docs/articles/CONTINUOUS_REVIEW_PATTERNS.md`. Memory: `feedback_gap_analysis_as_codex_prompt.md`, `feedback_codex_ensemble_pattern.md`._
