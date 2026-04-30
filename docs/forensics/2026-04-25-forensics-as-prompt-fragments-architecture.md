# Forensics-as-Prompt-Fragments: An Ingestion Architecture

**Window**: 2026-04-25 synthesis from session conversation
**Audience**: orchestrator, anyone designing agent dispatch wrappers, anyone deciding where to put new methodology knowledge
**Purpose**: capture the architectural property that forensics docs and memory entries are not retrospectives — they are prompt material. Establish the ingestion architecture (index → autoload → context injection) as a load-bearing methodology component, separate from the authoring style.

---

## The architectural claim

The forensics docs in `docs/forensics/` and the memory entries under `feedback_*.md` are *not* documentation in the traditional sense. They are **prompt material**: structured fragments that get re-injected into agent prompts at dispatch time, so each agent inherits the methodology's accumulated learning without rederiving it.

This claim has a specific load-bearing consequence: **the in-repo placement of these artifacts is part of the methodology architecture, not a documentation convention**. The repo is being used as a slow-changing shared cache that every future session and every future agent reads from.

A companion doc (`2026-04-25-forensics-and-memory-authoring-conventions.md`) covers *how to write* these fragments. This doc covers the *ingestion architecture* they participate in — different concern, different decisions.

---

## What "prompt fragment" means structurally

A prompt fragment is a chunk of methodology knowledge that:
- Has a stable identifier (file path + section anchor)
- Carries a load-bearing rule plus enough grounding to apply it (the three-block format: rule + Why + How to apply)
- Is composable with other fragments — the agent's prompt can include N fragments without conflict
- Has a known purpose-to-situation mapping — there exists some agent dispatch context in which including this fragment improves the agent's behavior

Concretely:
- The `feedback_xtask_fmt_false_cascade.md` memory entry is a prompt fragment for green-CI agents (and master bit-rot scouts) — they should not declare a master cascade without verifying master health for the named crate
- The `feedback_take_judgment_on_verdicts.md` memory entry is a prompt fragment for plan-reviewer (and other synthesis agents) — verdicts compose, not vote
- The `2026-04-25-failure-mode-catalog.md` forensics doc is a prompt fragment for any agent making decisions in a recurring-failure-class context

These are not docs to read once and remember. They are fragments to inject into agent prompts whenever the agent's dispatch context matches the fragment's situation.

---

## The four ingestion architecture properties

The methodology gets compounding leverage from the prompt-fragment treatment if (and only if) the architecture supports four properties:

### 1 — Indexable

There must be a structured mapping from `situation_id → list of fragment paths` so a dispatching agent (or orchestrator) can look up "I'm spawning a green-CI agent against this PR cluster" → "include these fragments in the prompt." Without an index, every dispatch either includes no fragments (zero leverage) or includes everything (context exhaustion).

The current state: `MEMORY.md` is close to an index for memory entries but unstructured for autoload purposes. `docs/forensics/INDEX.md` exists but is human-oriented. **Gap**: a machine-loadable index file (TOML/YAML) keyed by situation_id, with each entry pointing to the most-relevant 2-4 fragments. This is one of the small artifacts being added in the same batch as this doc (`docs/forensics/dispatch-index.toml`).

### 2 — Autoloadable

There must be a mechanism that consumes the index and assembles the prompt context for a given dispatch. Today this is implicit — operators paste relevant memory content when spawning agents, or the agent definition references specific memory files in its prose.

The lightest-weight mechanism: agent definitions explicitly reference the situation_ids they need, and a dispatch wrapper (or, until that wrapper exists, the orchestrator itself) reads the index to resolve those situation_ids into file paths and includes the file contents as part of the agent's input.

Heavier-weight options exist (a template-rendering agent harness, a runtime fragment-loader skill, an MCP tool that returns fragments by situation_id) but the lightest version is sufficient to get most of the value. Don't over-engineer.

### 3 — Composable without contradiction

Multiple fragments included in one prompt must not produce an incoherent agent. The risk is real: two fragments written at different times against different substrates may give contradictory advice. The agent has no way to adjudicate.

Mitigations:
- **Each fragment dates and substrate-versions itself** (per the authoring conventions doc). This lets the agent prefer the more recent fragment when they conflict.
- **The index curates carefully** — under each situation_id, list 2-4 fragments, not 12. Forces selection of the most-relevant and most-current.
- **A periodic memory-recalibration agent class** reviews fragments for staleness and consolidates contradictions. Documented as a missing-but-buildable agent class in the methodology blind spots doc.

### 4 — Half-life-aware

Fragments have a half-life because the substrate moves. A 5.4-era calibration fragment becomes wrong-by-default when 5.5 ships, not because it was wrong, but because the conditions it described changed underneath. The architecture must handle staleness explicitly:

- Fragments carry visible date and substrate-version stamps
- The recalibration agent class (memory-recalibrator, just added 2026-04-25) periodically re-verifies time-sensitive calibrations against current substrate
- The index can flag stale entries for agents to weight differently
- Operators have a discipline of demoting (or removing) fragments that have aged past their useful window

Without half-life-awareness, the prompt-fragment architecture decays into "lots of confidently-wrong context being injected into every agent." That's worse than no fragments at all.

---

## Why in-repo placement is load-bearing

The fragments could in principle live anywhere — a wiki, a separate repo, a database. They live in-repo because:

- **Co-located with the code they describe**: a memory entry about parser internals lives in the same repo as the parser. When the code moves, the entry's references break visibly. When the entry is wrong, anyone reading the code can correct it via PR.
- **Versioned with the methodology**: changes to fragments go through PR review. Bad fragments get caught the same way bad code does. The methodology evolves at the same cadence as the code, with the same review gates.
- **Discoverable by future agents**: every agent already has read access to the repo. No additional infrastructure required to make fragments available — they're already there.
- **Visible to humans**: maintainers and contributors can read the fragments without having to be initiated into a separate methodology system. The methodology's mechanics are inspectable.

These properties don't follow from "putting documentation in the repo." They follow from putting *agent-ingestable fragments* in the repo, with the explicit understanding that they're prompt material.

---

## What this changes about how to write new methodology knowledge

The decision tree for "I learned something during this session — where does it go?" follows from the architecture:

1. **Is it agent-actionable?** If no — if it's a personal observation, an interesting framing, or a debate — it doesn't go in the prompt-fragment layer at all. It goes in a session retrospective, a forum post, or just memory.
2. **What situation should trigger including it in an agent prompt?** Identify the situation_id. If you can't name a situation, the knowledge isn't ready to be a fragment yet.
3. **Is there an existing fragment for that situation that should be updated?** Prefer updating over adding — the index gets unwieldy if every learning becomes a new entry.
4. **If a new fragment is warranted, write it in the three-block format** (rule + Why + How to apply, per the authoring conventions doc) and add it to the index under the relevant situation_id.

The decision tree closes a recurring failure mode in the existing system: writing 5-paragraph narrative retrospectives that are useful to humans but not loadable as prompt fragments. The "first form is prompt-ready" framing was too binary (a small narrative spine helps), but the underlying observation was correct: docs not optimized for ingestion produce zero compounding leverage.

---

## The economics of prompt-fragment treatment

Each forensics doc is a one-time investment that amortizes across every future agent dispatch that loads it. Even if a doc costs $5-20 in compute to produce (deep-review-grade synthesis), it pays back across thousands of subsequent agent runs at zero marginal cost.

This matters because it inverts the natural intuition about documentation cost. Writing a 300-line forensics doc feels expensive. But the per-dispatch cost of *including* that doc in an agent's prompt is microscopic, and the leverage is the gap between "agent rederives the knowledge from session transcript" (expensive, error-prone) and "agent reads the knowledge from prompt and acts on it" (cheap, reliable).

The architecture is essentially a **prefetched cache for agent reasoning**. The fragments are the cache entries; the index is the cache key map; the agent dispatch is the cache lookup. Same architectural pattern as any other cache, with the same trade-offs (stale entries, eviction policy, lookup cost).

---

## The memory-recalibrator agent as cache GC

A cache without garbage collection eventually overflows or becomes stale. The memory-recalibrator agent class (added 2026-04-25 as part of this same batch) is the methodology's GC for the prompt-fragment cache. Its job:

- Periodically scan fragments for staleness signals (substrate version stamps older than current substrate, calibration numbers that haven't been re-verified, references to deprecated infrastructure)
- For staleness candidates, re-verify against current state
- Update, consolidate, or remove as appropriate
- Post a consolidated report so operators can see what changed

Without it, the prompt-fragment architecture decays. With it, the architecture stays useful as the substrate evolves.

This is a Conway's-law observation in itself — the methodology needs an agent class to maintain its own meta-layer, not just to act on the product code. The memory-recalibrator agent is the second example of this pattern (after tooling-debt-scout); both were added 2026-04-25 to address recurring blind spots.

---

## Practical implications for orchestrators

When dispatching any agent that has a corresponding situation_id in the index, include the listed fragments in the prompt. Today this is manual; until a dispatch wrapper exists, the orchestrator does it by reading the index and pasting fragment paths into agent prompts.

When the orchestrator notices a recurring pattern across sessions that isn't covered by any existing fragment, file a new fragment (with situation_id) rather than just remembering it. The fragment will pay back across every future relevant dispatch.

When an agent's verdict references a fragment ("per the xtask fmt false cascade memory entry, this looks like per-PR drift not master cascade"), trust the agent more than when it produces the same verdict from raw analysis — the fragment-reference indicates the agent had structured grounding.

When fragments contradict each other in the same prompt, the agent should flag the contradiction. Train (in agent definitions) for this disposition: "if two loaded fragments disagree, surface the conflict in your output with the dates of each fragment so the operator can reconcile."

---

## What this isn't

Some implications people might draw that don't apply:

- **Not "every observation should become a fragment."** Most observations are session-specific and don't generalize. The bar for a new fragment is "this would improve at least one future agent's behavior in a recurring situation." Below that bar, the observation can stay in personal memory or session retrospective.
- **Not "fragments replace agent definitions."** Agent definitions specify *what* the agent does; fragments provide *contextual grounding* for how it does it in this codebase. Both layers are needed.
- **Not "more fragments are better."** The composability constraint and the index-curation discipline mean adding fragments has a cost (the index gets harder to navigate, the per-prompt context grows). Prefer updating to adding.
- **Not "fragments are forever."** Half-life-awareness means some fragments will be retired. That's correct architecture, not loss of knowledge — the recalibrator's consolidations preserve what's still load-bearing.

---

## Related forensics + memory entries

- `2026-04-25-forensics-and-memory-authoring-conventions.md` — how to write fragments (the authoring side, distinct from this ingestion side)
- `2026-04-25-defense-in-depth-verification-architecture.md` — the ladder layer that composes with the prompt-fragment architecture
- `2026-04-25-methodology-blind-spots-conways-law.md` — why the recalibrator agent class exists (to address staleness as a methodology blind spot)
- `docs/forensics/dispatch-index.toml` — the machine-loadable index this architecture depends on (added in same batch as this doc)
- `.claude/agents/memory-recalibrator.md` — the GC agent for the prompt-fragment cache (added in same batch)

---

## Applies to

Reference this doc when:
- Designing a new agent class (does it need to read fragments? which situation_ids?)
- Deciding where to put new methodology knowledge (decision tree above)
- Debating whether to update an existing fragment or add a new one
- Building a dispatch wrapper or fragment-loader infrastructure
- Onboarding a new operator who needs to understand why forensics docs are in-repo
- Reviewing the recalibrator's consolidation reports
