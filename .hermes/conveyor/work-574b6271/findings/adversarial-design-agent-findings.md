# Adversarial Design Findings — work-574b6271

## Current Approach

The plan proposes to reorder sections in three agent files (`scout-parser.md`, `scout-dap.md`, `accuracy-scout.md`) so that `## Todo list` becomes the final section, moving informational sections (`## Domain context`, `## Invocation`) before it. The rationale is that having the Todo list as the terminal section will enforce it as a "terminal-skill gate" — agents cannot exit without completing the todo list. This broadens PR #4087's fix for `scout-lsp.md` and `reviewer-deep.md` to all multi-step agents.

## Alternative Approaches

### Alternative 1: Fix the Execution Layer, Not the Definition Layer

**Core idea:** Instead of reordering markdown sections, add a runtime enforcement mechanism. When an agent is spawned, the orchestrator or lead agent records the expected todo list steps and verifies completion before the agent can exit. This could be a skill-gate at the orchestration layer that validates all listed skills were invoked.

**Why it might be better:**
- Addresses the root cause: PR #4087's own commit message explicitly states that scout-parser #4084 "ran with a complete todo list and still skipped /scout-report — that's **agent drift**, not a definition gap, **so it cannot be fixed at the definition layer**." If agent drift can't be fixed at the definition layer, reordering sections won't help either.
- Affects all agents uniformly regardless of their internal section ordering
- Can provide actual enforcement with receipts (logged skill invocations)
- Doesn't depend on the LLM interpreting markdown section order as a signal

**Why it might be worse:**
- Requires building new orchestration infrastructure (not just editing markdown files)
- More complex to implement and test
- The existing orchestrator may not have hooks for this validation

**What it sacrifices:**
- The simplicity of a pure documentation/structure fix
- The ability to ship this as a quick PR

---

### Alternative 2: Investigate Why scout-lsp Is "Correct" When It Has the Same Pattern

**Core idea:** Before reordering, perform a genuine root-cause analysis. The research analysis claims `scout-lsp.md` is "correct" despite having `## Todo list` (line 20) followed by `## Domain context` (line 33) — the same structural pattern as the three "wrong" files. Additionally, `scout-lsp.md` has a third section "Write to think" (lines 42-45) that comes AFTER `## Domain context`, making it structurally "worse" than the files being fixed. If the structural hypothesis is correct, scout-lsp should also be marked for change. If it's correct despite the same pattern, the structural hypothesis is wrong.

**Why it might be better:**
- Exposes that the research analysis is internally inconsistent (calling scout-lsp correct despite it having content after Todo list, while calling files with the same pattern "wrong")
- If scout-lsp is actually correct, understanding why prevents unnecessary changes to three other files
- Could reveal that the "correct" pattern is something other than "Todo list last"

**Why it might be worse:**
- May delay the fix if investigation takes time
- If the conclusion is "we don't know why it works," the fix may still be wrong

**What it sacrifices:**
- The confidence that structural position is the lever
- The ability to close issue #4202 quickly

---

### Alternative 3: Accept That Agent Files Cannot Enforce Terminal Skills

**Core idea:** The PR #4087 commit message itself states that definition files cannot prevent agent drift: "it cannot be fixed at the definition layer." Rather than try to force a definition-layer solution, accept that agents will occasionally skip steps, and instead invest in monitoring/detection mechanisms that catch drift when it happens rather than prevent it upstream.

**Why it might be better:**
- Aligns with empirical evidence: even the original #4087 fix acknowledged this limitation
- Focuses resources on observable symptoms (drift detection) rather than hypothesized causes (section order)
- Avoids churn on files that PR #4087 already declared "all fine" less than two weeks ago

**Why it might be worse:**
- Doesn't "fix" issue #4202 as requested
- May be unsatisfying to stakeholders who want a clean solution

**What it sacrifices:**
- Any hope that section reordering produces reliable terminal-skill enforcement

---

## Strongest Argument Against Current Approach

**The current approach contradicts the very PR it is meant to broaden.**

PR #4087 (commit 4a5e999f) explicitly states: *"the #4084 scout-parser ran with a complete todo list and still skipped /scout-report — that's agent drift, not a definition gap, so it cannot be fixed at the definition layer."*

This is a direct admission by the team that wrote #4087 that editing definition files cannot solve agent drift. Issue #4202 asks to "broaden" #4087 by applying its pattern to all multi-step agents, but #4087's own analysis concludes that broadening the pattern (having complete todo lists) does NOT prevent skipping. The proposed fix — reordering sections so Todo list is last — is still entirely a definition-layer change, and PR #4087 says definition-layer changes cannot fix this class of problem.

Additionally, the research analysis is self-contradictory: it marks `scout-lsp.md` as "correct" despite `## Todo list` being followed by `## Domain context` and an additional `## Write to think` section — the same pattern (informational content after Todo list) that marks `scout-parser.md`, `scout-dap.md`, and `accuracy-scout.md` as needing fixing. If the structural hypothesis is correct, scout-lsp should be included in the fix. If scout-lsp is correct as-is, the structural hypothesis is falsified.

---

## Recommended Action

**Modify.** The current approach should not be executed as-is. Before proceeding:

1. **Clarify the contradiction with #4087's own findings** — either obtain new evidence that section reordering prevents drift (whereas complete todo lists don't), or accept that this is a different class of fix and rename/re-scope the issue accordingly.

2. **Resolve scout-lsp.md inconsistency** — either include scout-lsp.md in the changes (if structural ordering is truly the lever) or update the research analysis to explain why scout-lsp is exempt from the pattern it shares with the "wrong" files.

3. **Define what "terminal-skill gate" actually means in code/runtime terms** — without a specification for how section order creates enforcement, the fix is superstitious.

If forced to proceed without clarification: **do Alternative 2 first** (investigate scout-lsp) to determine whether the structural hypothesis is even coherent before touching three files.

---

## Long-Term Cost Assessment

**If we do it the current way (reordering sections):**

- **6 months:** Someone files a new issue noting that agents still skip terminal skills despite reordered sections. The definition-layer hypothesis is re-falsified. We spend another sprint investigating drift, potentially reinvestigating #4087's original finding that "it cannot be fixed at the definition layer."

- **2 years:** The section-ordering convention becomes entrenched lore that everyone repeats ("put Todo list last for terminal-skill enforcement") without anyone being able to explain the mechanism or point to evidence it works. When new agents are created, they follow the convention. Drift still occurs. The convention becomes a superstition rather than a practice grounded in mechanism.

**If we instead invest in the execution layer:**

- **6 months:** Slower initial delivery of the terminal-skill enforcement feature, but it actually works. Agents that skip steps are caught and corrected or logged.

- **2 years:** A robust runtime enforcement mechanism that scales to new agent types without requiring per-file manual editing. The convention becomes " Todo list last" is nice-to-have hygiene, but the real enforcement is at the runtime layer. New agents automatically benefit without anyone having to remember to reorder sections.
