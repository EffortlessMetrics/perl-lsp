# Haiku 4.5 for Mechanical Work

**Date:** 2026-04-23

Sonnet (and Opus) are expensive. Haiku 4.5 is close enough to Sonnet 4 on the kind of work that dominates agent throughput — mechanical, well-specified, pattern-matching tasks — that defaulting to Sonnet is leaving money on the table.

Empirically: Haiku 4.5 ≈ Sonnet 4 for mechanical work and is easily better than Sonnet 3.5.

## When Haiku is the right model

### Compiled policies
When a failure mode has a known, deterministic fix:

- `clippy::single_match` → `match` → `if let`
- `clippy::needless_clone` → drop the `.clone()`
- `clippy::redundant_field_names` → strike the field
- snapshot drift → `UPDATE_SNAPSHOTS=1 cargo test regenerate_snapshots`
- missing `(#NNNN)` in PR title → look up issue from branch name, edit title
- stale snap.new file in worktree → delete
- test uses `unwrap()` → replace with `must(...)`
- static regex via `LazyLock<Result<Regex, _>>` → rewrite to `LazyLock<Regex>` with `unreachable!()`

For rules like this, Haiku with a short spec outperforms Sonnet with a long prompt. Sonnet's "thinking" surface is wasted.

### Mass-application passes
- Rename across N files
- Add `#[non_exhaustive]` to every public enum in a crate
- Cargo metadata audits (allowlist vs cargo metadata diff)
- Fmt-fix for one-line drift
- Rebase chains where conflicts are mechanical (imports / use paths)

### Verification / sanity checks
- Check that a PR title ends with `(#\d+)` for validate-title CI
- Check that file SHAs on a branch match what you expect
- Check that a test's assertion references a non-`None` value (non-vacuous)

### Label management
- Strip `merge-ready` from PRs that lack `green-ci` + `diff-audited`
- Add `needs-plan-review` to issues filed by scouts
- Close issues as `already-fixed` when a PR with `Closes #NNNN` merged

### Search / lookup / routing
- "Which agent handled this?" queries
- Status rollup summaries
- PR metadata extraction for dashboarding

## When to reach for Sonnet instead

### Correctness review with semantic judgment
- Does this Perl semantics claim match real Perl? (Haiku gets confused on Moose/Moo compile-time BEGIN)
- Does this coordinate-space bug exist, and what's the minimal fix? (Haiku can miss the evolving-document subtlety)
- Is this test vacuous? (requires reasoning about what the code does without the fix)

### Novel bug investigation
When a failure mode is unknown, Sonnet's broader reasoning pays off. Once you've understood the pattern, compile it to a Haiku policy for future occurrences.

### Cross-file architectural review
When the question is "does this PR fit the microcrate layering rules?", Sonnet sees the bigger picture faster.

### Synthesis across many inputs
Gap analyses, forensics, policy writing (like this doc) — Sonnet for divergent thinking, Haiku for execution.

## The policy

**Default to Haiku. Escalate to Sonnet when the task involves novel reasoning, cross-file judgment, or semantic claims about external systems (Perl, LSP spec, etc.).**

**Compile successful Sonnet reasoning into Haiku policies.** Every time Sonnet solves a failure mode, extract the pattern into a compiled rule. Future occurrences run through Haiku at a fraction of the cost.

## Cost shape

Claude 20× Max session budget is ~100× cheaper at Haiku pricing than at Sonnet pricing. A session of mechanical work costs almost nothing in Haiku; the same session in Sonnet eats a real fraction of the budget.

Across a 200-PR throughput session:
- **All-Sonnet:** ~33% session (observed)
- **Mixed (Sonnet for novel, Haiku for mechanical):** projected ~15% session
- **All-Haiku where possible:** projected ~5% session

The 15-20% savings funds deeper Sonnet review on the PRs that deserve it.

## Implementation

In `.claude/agents/*.md` files, set `model: haiku` for agents that handle mechanical work:

- `reviewer` (first-pass standards) — **already Haiku**
- `pr-responder` (bot-comment addressing) — **already Haiku**
- `accuracy-scout` (mechanical fact check) — **already Haiku**
- `research-verifier` (external doc lookup) — **already Haiku**

Candidates to move to Haiku:
- Label-management helper agents
- PR state normalization helpers
- Cascade-update batch runners
- Fmt-fix / clippy-fix workers when the failure is a known rule

Reserve `sonnet` for:
- `reviewer-deep`
- `builder`
- `plan-reviewer`
- `green-refactor`

## The broader point

Haiku 4.5 is good enough that the design question changes from "can Haiku handle this?" to "what pattern am I escalating to Sonnet for, and why?" Most agent work is mechanical. Most mechanical work is Haiku work.

---

_Related: `.claude/agents/*.md` (per-agent model config), `docs/articles/TWO_MODE_DEV_LOOP.md` (Codex as mass-throughput engine)._
