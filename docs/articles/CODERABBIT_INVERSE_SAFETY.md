# CodeRabbit Skips Big PRs: An Inverse Safety Pattern

**Date**: 2026-04-19
**Session**: Wave G1 collapse on perl-lsp
**Cross-references**: [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) §4, [VERIFICATION_LADDER_PER_LAYER_ROI.md](VERIFICATION_LADDER_PER_LAYER_ROI.md)

---

## TL;DR

CodeRabbit (and likely similar automated review bots) silently skip pull requests with more than 150 files changed. The bot posts a short "skipped, file limit exceeded" comment and performs no code-level review. On perl-lsp's 2026-04-19 Wave G1 collapse session, this affected both G1a (258 files, PR #4506) and G1b (258 files, PR #4510) — the two largest, most structurally complex PRs in the session.

The pattern: **automated review thins out exactly when human review should thicken**. PRs that most need a second pair of eyes get the fewest. This article names the pattern and recommends it be treated as a process signal — large PRs require visible compensatory human (or human-like agent) scrutiny, not silent trust that CodeRabbit caught anything.

---

## The Behavior

On any PR exceeding CodeRabbit's file-count threshold (appears to be 150), the bot posts a single short comment such as:

> _CodeRabbit was unable to review this PR because the changes exceed the file limit._

No inline review. No scope review. No suggestion comments. The `coderabbitai` status check sometimes shows as "success" (it ran; it just decided not to review), which can be misleading — the PR can have all automated checks green and still have had zero automated review.

This is a reasonable default from the bot's perspective. Reviewing 258-file diffs is expensive; the signal-to-noise ratio of bot comments on large refactors is typically poor. Skipping them protects the review pipeline from being overwhelmed.

## The Inverse Safety Problem

The problem is that the *set of PRs most likely to benefit from automated review* is largely the same as *the set CodeRabbit skips*.

- Small PRs (single-file typo fixes, minor refactors): CodeRabbit reviews them. They rarely need it — the scope is obvious, the risk is low.
- Large PRs (structural refactors, collapses, new feature families): CodeRabbit skips them. They almost always need it — the scope is broad, the risk is high, the human reviewer is more likely to miss something.

On perl-lsp's 2026-04-19 session, both G1a (#4506) and G1b (#4510) were 258-file PRs involving 10-15 crate absorptions each. Each had real bugs that other layers caught (green-tdd caught a 3-import regression on #4510; reviewer-deep caught 3 bugs on #4504, though that was a smaller PR). Neither had any signal from CodeRabbit.

## The Compensation

On this session, the failure mode didn't manifest as silent bugs shipping — other layers caught the issues. But the coverage was fragile. Without reviewer-deep's thorough pass on #4504 (which itself was technically below the 150-file threshold), the 3 logic bugs would have shipped. If those had been in the 258-file G1a instead, and if CodeRabbit had been the primary automated review on master-path PRs, the bugs would have shipped with zero automated review intervention.

The compensations that kept this from being a silent failure:

1. **`reviewer-deep` was non-optional.** The perl-lsp pipeline runs reviewer-deep on all non-docs PRs. This policy predates the CodeRabbit skip observation but accidentally covered for it.
2. **`green-tdd` runs on every PR.** Catches regressions the reviewer missed.
3. **Human orchestrator attention** was higher on larger PRs by natural scaling of review effort.
4. **Diff-auditor** provided a final scope-drift check.

Without those four, the CodeRabbit skip would have been a real gap.

## Recommendations

1. **Treat CodeRabbit skip as a signal, not absence-of-signal.** When `reviewer-deep` or `diff-auditor` runs on a PR and sees the "file limit exceeded" comment, they should note it and apply thicker review. PR reviewers should explicitly write "CodeRabbit skipped due to size; compensating with manual walk" in their comments.
2. **Split large PRs when possible.** Wave G1 was deliberately split into G1a + G1b (15 + 10 crates, 258 + 258 files each instead of a monolithic 25-crate / 500-file PR). This helped downstream review but didn't cross CodeRabbit's threshold.
3. **Consider alternative bot review for large PRs.** Some tools (Reviewpad, GitHub Copilot PR Review) have different thresholds or no skip. Layering different bots for different size classes could provide coverage.
4. **Human-reviewer-agent pairing.** On large PRs, ensure `reviewer-deep` is a sonnet-level model spending real time on the diff rather than haiku-level quick scan. The per-PR review cost goes up with PR size, and the budget should match.

## Why It's Worth Naming

CodeRabbit's skip is transparent to careful readers (the comment is visible) and easy to miss in automated pipelines (the status check passes). Like other silent-degradation patterns — receipts that lie, tests that are vacuous, gates that false-positive — this is a case where **the safety appearance is preserved while the safety substance is not**.

Naming the pattern makes it searchable. When a future engineer troubleshoots a production incident traced to a 300-file PR and wonders "why didn't automated review catch this?", they can find the answer.

## Related

- [forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md) §4 — session data
- [WHEN_RECEIPTS_LIE.md](WHEN_RECEIPTS_LIE.md) — companion pattern (other ways safety signals go wrong)
- [VERIFICATION_LADDER_PER_LAYER_ROI.md](VERIFICATION_LADDER_PER_LAYER_ROI.md) — per-layer catch data; reviewer-deep is the layer that compensates
