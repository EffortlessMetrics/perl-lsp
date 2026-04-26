# Broad Scope Codex Prompts Produce Layer Diversity, Not Duplicates

**Status:** Triage pattern derived from 2026-04-24 encoding-cluster experience.

## The pattern

When a Codex task is given a broadly scoped prompt — *"fix encoding"*, *"improve completion"*, *"harden URI parsing"*, *"expand keyword coverage"* — the 4-shot outputs tend to each pick a different slice of the stack. Not overlapping implementations of the same thing; **different parts of the same problem**.

A naive title-based dup-sweep collapses the layer-diversity into a single PR and throws away 3 useful contributions.

## Example: the 2026-04-24 encoding cluster

A broad "improve character encoding / mojibake handling" Codex task produced 12+ PRs. Title similarities suggested they were duplicates. File-path analysis revealed they were each on a different layer:

| Layer | PRs |
|---|---|
| `workspace.rs` file reads — **same layer, real dupes** | #5740, #5741, #5743 |
| `util/mod.rs` shared `decode_text_bytes` helper | #5742 |
| `navigation.rs` LSP provider | #5742 |
| `perl-uri` URI parsing mojibake detection | #5738 |
| `perl-critic` tool output mojibake | #5739 |
| `perl-parse` CLI binary mojibake | #5736, #5737 |
| URI module UTF-8 path | #5732 |
| `position-tracking` UTF-8 mid-codepoint clamp | #5733 |
| Code-actions UTF-8 pragma detection | #5734, #5735 |

Only the first row was a real same-layer duplicate cluster. Every other entry represented a different attack surface and needed to merge independently.

Closing the wider cluster as duplicates would have left the repo with ONE of:
- No UTF-16 on the file-read path (one layer)
- No UTF-8-aware URI parsing (different layer)
- No CLI mojibake repair (different layer)
- No position-tracking safety (different layer)
- No code-actions pragma awareness (different layer)

Keeping them as distinct PRs preserves the full-stack encoding posture.

## Triage rule

**File-path triage comes before title triage.**

```bash
gh pr diff <N> --name-only
```

Two PRs with similar titles touching disjoint file sets: **complementary**. Keep both.

Two PRs with similar titles touching the same file: **real duplicate cluster**. Pick winner.

Two PRs touching overlapping lines in the same function with incompatible approaches: **pick one**, close the other.

## The deeper signal

Codex's 4-shot-per-prompt design is a **design exploration engine**, not a parallel implementation engine. When the prompt is narrow ("rename `is_valid_identifier` to `is_symbol_identifier`"), 4 shots produce 4 near-identical variants — triage is straightforward dup-selection.

When the prompt is broad ("harden URI parsing"), 4 shots produce 4 different architectural responses — triage is architectural synthesis. Each shot asks "where does this problem live?" and gives its best guess. Collectively, they map the problem's true scope.

The orchestrator's job is **not to collapse** them — it is to synthesize across them. The pattern that emerges is a spec-implicit-in-the-cluster:

> "Encoding handling in this codebase spans workspace file reads, URI parsing, CLI binaries, position-tracking, and code-action pragma detection. Each layer needs its own fallback semantics."

That's a richer statement than any single PR could produce.

## Anti-pattern

A dup-sweep agent that closes PRs by title regex (`docs(editors) add <editor> setup`) is fine for editor-docs clusters (those legitimately are duplicates — same file, same purpose).

The same agent running on `(fix|feat)(<crate>): <generic verb> X` is likely to over-close. Before running dup-sweep on any framework/fix/feat cluster, verify the PRs touch overlapping files via `gh pr diff --name-only`.

## Checklist

Before calling a cluster "duplicates":

- [ ] Did I fetch each PR's file list?
- [ ] Do 3+ PRs touch genuinely overlapping code paths (not just same crate)?
- [ ] If they touch different files, is there an interface boundary that makes them complementary?
- [ ] If they touch the same file but different functions, are the changes composable?

If any checkbox is unclear, **keep the PRs open** and escalate to architecture-reviewer. The cost of an extra review pass is lower than the cost of closing a layer of the stack.

## Related memory

- `feedback_broad_scope_codex_stack_diversity.md` — the original memory entry
- `feedback_codex_ensemble_pattern.md` — 4-shot-per-prompt design
- `CODEX_HALLUCINATION_TRIAGE.md` — a different failure mode of the same Codex generation pattern
