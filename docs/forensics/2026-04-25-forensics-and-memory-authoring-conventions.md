# 2026-04-25 — Forensics and Memory Authoring Conventions

**Lens**: How to write forensics docs and memory entries so future agents can act on them
**Purpose**: Style guide for the next forensics author — what to keep, what to compress, what to date-stamp, and why
**Substrate at time of writing**: Anthropic Claude Code orchestrator, Codex 5.4 web upstream, ~135 workspace crates, ~75 memory entries

---

## Why this doc exists

The repo's `docs/forensics/` and `~/.claude/projects/.../memory/` directories are not retrospective journals. They are **prompt material**. They get re-injected into agent system prompts at dispatch time via the memory loader and via cross-link references in scout/plan-review/diff-audit prompts. Their reader is a downstream agent with a job to do, not a human reading after the fact.

Several recent forensics docs have drifted toward narrative essay form (5+ paragraph sections, anecdotal supporting prose, no named pattern). They read fine to a human but degrade as prompt material: agents extract less actionable directive per token, and the implicit "what should I do differently" gets buried under "what we observed."

This doc captures the conventions that follow from the agent-ingestion purpose. Future forensics authors should treat this as a style guide, not a description.

---

## The core insight

**Forensics docs and memory entries serve agent ingestion, not human re-reading. Authoring style follows.**

The wrong default is narrative essay (verbose, low directive density, hard to grep). The wrong overcorrection is pure imperative (brittle — an agent following a rule without knowing *why* has nothing to extrapolate from when the situation drifts off the rule's anticipated path).

The right balance is **imperative spine with brief narrative grounding**: a terse rule, one sentence of why, one sentence of how to apply.

---

## Pattern: narration-room calibration

Pure imperative form looks efficient but fails predictably. An agent told "always check master health before declaring cascade" without knowing *why* will:

- **Freeze on edge cases**: when the symptom looks similar but the toolchain has changed (xtask fmt rewrites its abort behavior, or the cascade hypothesis applies but at a different layer)
- **Follow the rule into a wrong outcome**: applying the master-check on a PR where the failure is actually a stale CI rollup, missing the real root cause
- **Invent reasoning that wasn't there**: confabulating justifications that contradict the original incident's lesson

A small narrative spine — one or two sentences naming the underlying cause or past incident — gives the agent enough grounding to handle these. The narrative is not for entertainment; it is the substrate the agent uses to reason about edge cases the imperative didn't anticipate, contradictions between fragments, and recognition of when the rule no longer applies.

The `feedback_xtask_fmt_false_cascade.md` entry is a clean example: the rule ("verify master health before declaring cascade") would be brittle alone, but the why-block ("xtask fmt aborts at first failure with misleading message; same message appears across N PRs each with their own per-PR fmt drift") lets a future agent recognize the pattern even if the specific tool changes.

---

## Pattern: the three-block format

This is the default shape for new memory entries. Already used by most `feedback_*.md` files; new forensics sections should follow it.

```
{rule, terse imperative — one or two sentences}

**Why:** {one sentence — the underlying cause or past incident}

**How to apply:** {one sentence — when this kicks in, what action to take}
```

Three short blocks. Each load-bearing.

**Worked example** (compressed from `feedback_master_bit_rot_cascade_fixes.md`):

> When N PRs fail the same gate identically, fix master narrowly and `gh pr update-branch` the cluster.
>
> **Why:** Master bit-rot recurs in bursts during high-cluster phases (8 fixes in 3.5h on 2026-04-25); cascade-update unblocks 20-30 PRs per fix.
>
> **How to apply:** When 3+ PRs show identical gate failures, narrow-investigate master before blanket pushing fixes to PR branches.

Compare to the verbose original (~600 words) — the three-block compression keeps every load-bearing fact and discards none of the directive content. The original's prose context is preserved in the `Why` block as a single grounded sentence.

---

## Pattern: half-life dating

Every forensics doc has a half-life because the substrate moves. A 5.4-era calibration becomes wrong-by-default when 5.5 ships, not because it was wrong, but because what it described changed underneath.

Authors should:

- **Date-stamp filenames prominently**: `YYYY-MM-DD-<short-pattern-name>.md`. Date stamp first for chronological sort and half-life identification.
- **Date-stamp section headers** when the content is calibration rather than a stable rule: "as of 2026-04-25, deep-review catches ~100% of correctness bugs in Era 7 sessions" — the qualifier matters.
- **Name the substrate version when it matters**: Codex 5.4 vs 5.5, ChatGPT-Pro vs not, Anthropic-only vs mixed-family downstream, ~135 crates vs post-collapse 30. A reader in 2026-09 needs to know which version of the world the doc described.
- **Write specific calibration numbers with implicit qualifiers**: "6.3% scout error rate" should be read as "6.3% as of <date> against <substrate>". Make the qualifier explicit when the half-life is short.

The `feedback_research_verifier_roi.md` entry's "6.3% scout error rate" was accurate at write time but is already drifting — newer scout prompts have improved that number. The entry would be stronger with an "as-of" tag so the next consolidation pass knows whether to refresh or retire.

---

## Pattern: compression toward agent ingestion

A 5-paragraph narrative is human-readable. The same content compressed to a three-block format is agent-loadable. The next forensics pass should compress older docs toward the agent-loadable shape — keep the why, lose the supporting prose, name the patterns explicitly.

Compression rules of thumb:

- **One pattern, one section.** If a section covers two patterns, split.
- **Name the pattern in the heading.** "Pattern: half-life dating" beats "On the question of dating."
- **Drop incident anecdotes that don't ground the rule.** If the anecdote isn't doing work in the why-block, it's prose tax.
- **Cross-link, don't recap.** When another forensics doc or memory entry covers the same ground, link rather than re-explain.

---

## Pattern: pattern naming over prose description

Name the pattern explicitly so future agents can grep for it. "Wave shape: cluster-collision" plus two sentences beats a paragraph about how PRs collided. Future agents can grep `wave shape:` or `pattern:` and get the playbook instantly.

This is why the `feedback_agent_audit_trail_directories.md` entry is high-value: the term "agent audit trail" is unique enough to grep on, and the entry's rule fires whenever a diff-auditor encounters `.spec/`, `.hermes/`, `.jules/` directories. Without that name, future agents would re-derive the rule each time.

Naming conventions that have proven useful in this repo:

- `Wave shape: <name>` — for orchestration patterns (cluster-collision, promotion-sweep, fire-fix cascade)
- `Failure mode: <name>` — for recurring failures (xtask-fmt false cascade, GraphQL changedFiles drift)
- `Substrate constraint: <name>` — for environmental quirks (CRLF + xargs, Windows MAX_PATH)
- `Verifier pattern: <name>` — for verification ladder structures (verifier-of-verifier, second-agent narrow scope)

---

## Pattern: when narrative IS appropriate

Narrative prose is justified in:

- **The why-block of a three-block entry.** That sentence is the agent's grounding for judgment.
- **Supporting context for time-sensitive observations.** "We observed X under conditions Y as of date Z" — the conditions and date are load-bearing.
- **Case-study docs that exist specifically to teach a pattern.** The pragma-phase-block case study in the docs corpus is a good example: it walks through one specific incident in narrative form because the *reasoning sequence* is what teaches the pattern, not the rule itself.
- **Cross-link blocks at the end of forensics docs.** Listing related entries with a one-line "what it covers" helps the agent decide whether to follow.

Even in these cases, keep paragraphs short (3-5 sentences) and prefer named patterns where possible.

---

## File and section conventions

### File naming

`docs/forensics/YYYY-MM-DD-<short-pattern-name>.md`. Date stamp first for chronological sort. Short pattern name (3-5 words) so the file is greppable by its theme.

### Section structure (recommended for forensics docs)

1. **Problem / what triggered this doc** — one paragraph
2. **The pattern (named, terse)** — heading is the name; body is the rule
3. **Why it works / why it fails** — the narrative grounding (1-3 short paragraphs)
4. **How to apply / how to detect** — concrete checks an agent can run
5. **Failure modes / when this rule doesn't apply** — the boundary conditions
6. **Related forensics + memory entries** — cross-links

Not every section is mandatory. A short doc may collapse 3-6 into a single "How to apply" paragraph. A reference doc (like the failure-mode catalog) may structure entirely around section 2 with sub-patterns.

### Memory entry structure

Three blocks (rule / why / how to apply) inside a YAML-frontmatter wrapper. The frontmatter `description` field is what scout/builder/reviewer prompts surface — write it as a one-sentence directive, not a description.

Compare:

- Weak: `description: notes on xtask fmt failures`
- Strong: `description: green-CI agents keep mis-classifying per-PR fmt failures as master bit-rot because xtask fmt aborts at first failure`

The strong form is itself a directive when an agent reads only the description.

---

## The author's job

Write so that an agent reading this doc as part of a larger prompt context can act on it without further interpretation. If you find yourself writing "we observed..." think instead about what action that observation implies for the agent.

Concrete substitutions:

- "We observed N PRs failing identically" → "When N PRs fail identically, check master before pushing per-PR fixes"
- "It turned out the cause was X" → "Root cause: X. Detect by: <check>"
- "This was surprising because Y" → "Why: Y" (in the why-block)
- "In retrospect we should have..." → "Rule: do <action> before <trigger>"

Every sentence that doesn't push toward an agent action is candidate for cutting or compressing into the why-block.

---

## Concrete examples drawn from existing memory entries

### Example 1: xtask fmt false cascade

The original entry (`feedback_xtask_fmt_false_cascade.md`) is roughly the right shape but has one drift: it includes a bulleted incident list in the "How to apply" section ("Affected PRs in 2026-04-25 session: #6391, #6375, ..."). That list is historical and doesn't help a future agent — those PRs are merged or closed. The 2026-04-25 calibration paragraph at the bottom is load-bearing because it teaches a *different* rule (verify each PR individually even when the cascade hypothesis is correct), so it should be promoted to its own three-block sub-section rather than buried as a follow-up note.

### Example 2: master bit-rot pattern

The `feedback_master_bit_rot_recurrence_pattern.md` and `feedback_master_bit_rot_cascade_fixes.md` and `feedback_master_bitrot_cascade_8plus_pattern.md` entries cover overlapping ground — they're all variants of "when N PRs fail identically, suspect master." A consolidation pass would merge them into one entry with three calibration sub-blocks (3+ instances signal, 8+ instances escalation, fix-and-cascade procedure), each dated. As-is, an agent loading all three gets redundant rules with subtle divergences.

### Example 3: agent audit trail directories

The `feedback_agent_audit_trail_directories.md` entry is well-shaped: the rule names a category (`.spec/`, `.hermes/`, `.jules/` are agent audit trails, keep by default), the why-block grounds it in past incidents, and the 2026-04-25 update tightens the rule (must verify subdir name matches THIS PR's issue, with the awk command). This is the model for how calibration updates should land — as terse rule-tightening, not as narrative addendum.

---

## Failure modes for forensics authors

- **Verbose retrospective**: writing "this session we did X, Y, Z" instead of "when condition K, do action L." If the reader is an agent, the session-narrative form is wasted tokens.
- **Buried rule**: rule appears in paragraph 4 of a section instead of as the section's opening sentence. Agents that read only headings + first sentences will miss it.
- **Unnamed pattern**: section titled "On CI gate timeouts" instead of "Pattern: timeout headroom must include master baseline p95 + 30%". The named version is greppable and self-documenting.
- **Stale calibration without date-stamp**: "6.3% error rate" without "as of <date>" creates a permanent claim from a snapshot observation.
- **Recap of cross-linked content**: re-explaining what `feedback_X.md` says instead of linking to it. Cross-links exist; use them.

---

## When to write a new forensics doc vs. update an existing one

- **New forensics doc**: when the session produced a *novel pattern* not yet captured. The 2026-04-25 failure-mode catalog and process meta-learnings are examples — they catalog patterns specifically discovered or refined in that session.
- **Update existing memory entry**: when calibration drifts (the 6.3% number, the 100% deep-review catch rate, the recommended worktree count). Update in-place with a dated `Update:` line.
- **Consolidate**: when 3+ memory entries cover overlapping ground, propose a consolidation in the next session's forensics doc rather than letting the redundancy compound.

---

## Related forensics and memory entries

- `2026-04-25-failure-mode-catalog.md` — structured reference for known failure modes; uses the named-pattern style consistently
- `2026-04-25-process-meta-learnings.md` — pattern-named sections at the process level; demonstrates the section-structure convention
- `feedback_xtask_fmt_false_cascade.md` — three-block format example (with the drift noted above)
- `feedback_agent_audit_trail_directories.md` — model for tight calibration update
- `feedback_master_bit_rot_recurrence_pattern.md` (+ siblings) — example of overdue consolidation
- `feedback_research_verifier_roi.md` — example of stale calibration needing as-of qualifier

---

## Applies to

This doc would be loaded into the prompt context for:

- **forensic** skill (any agent invoking it for session-end documentation)
- **wisdom-document** and **wisdom-synthesize** skills (post-merge learning capture)
- **agent-wrapup** skill (any agent's final retrospective step)
- **scout-report** and **plan-review-improve** skills (when authoring issue bodies that cross-link to memory entries)
- Any orchestrator session that produces 3+ new memory candidates

The situation_id this doc serves: any session that ends with "I should write this down for the next operator." Read this first before writing.
