// spec-builder.js — Multi-angle haiku spec-builder for perl-lsp-swarm.
//
// Fans out six independent haiku analysis passes in parallel, then synthesizes
// the results into the §Hazards, §Contracts, §API-Shape, §Test-Grid, and
// §Blast-Radius sections of acceptance.md, plus the prior-art/duplicates block
// for context.md.
//
// Usage:
//   Invoke via the Claude Code Workflow tool with name="spec-builder" and args:
//     { issue: "<N>", subsystem: "DAP|Parser|LSP|Coverage/CI|cross", risk: "low|medium|high" }
//
//   Triggered by spec-planner for non-trivial issues (see .claude/agents/spec-planner.md).
//   For trivial issues (one-line constant, typo, docs-only), populate sections manually
//   and mark N/A rows with a reason — running this workflow is overkill.
//
// Cross-references:
//   docs/reference/SPEC_TEMPLATE.md — canonical acceptance.md section names and format
//   docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md — pre-seeded hazard rows per subsystem
//   docs/agents/SPEC_UPDATE_CHECKLIST.md §8 — six hazard classes and trigger conditions
//   docs/concepts/multi-angle-haiku-early-spec.md — the six-angle pattern this implements
//   docs/concepts/hazard-class-invariants.md — generic hazard class definitions
//   docs/reference/PARSER_CONTRACTS.md — contract index for parser/quote-like/scanner

export const meta = {
  name: "spec-builder",
  description:
    "Multi-angle haiku spec-builder for perl-lsp-swarm. Fans out six independent haiku " +
    "analysis passes in parallel (hazard enumeration, contract pointers, prior-art/duplicate " +
    "check, API shape, test grid, blast radius), then synthesizes the results into the " +
    "acceptance.md sections required by SPEC_TEMPLATE.md. Triggered by spec-planner for " +
    "non-trivial issues. Output: merged acceptance.md sections + context.md prior-art block.",
  whenToUse:
    "When spec-planner needs to populate acceptance.md §Hazards, §Contracts, §API-Shape, " +
    "§Test-Grid, and §Blast-Radius sections for a non-trivial issue. Skip for trivial " +
    "changes (one-line constant, typo, docs-only) — populate manually and mark N/A.",
  phases: [
    {
      id: "phase-1-fan-out",
      name: "Parallel Analysis Fan-Out",
      description:
        "Six independent haiku passes, each from a distinct analytical angle. " +
        "True fan-out — passes do not depend on each other and can run in parallel. " +
        "Each angle catches a different category of spec gap.",
      model: "haiku",
      parallel: true,
      agents: [
        {
          label: "A-hazard-enumeration",
          phase: "phase-1-fan-out",
          prompt:
            "You are angle A — Hazard Enumeration — for issue #${args.issue} (subsystem: ${args.subsystem}).\n\n" +
            "Read the issue spec and the change description. Then:\n\n" +
            "1. Read docs/agents/SPEC_UPDATE_CHECKLIST.md §8 to identify the six hazard classes.\n" +
            "2. Read docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md for the ${args.subsystem} section.\n" +
            "3. For EACH of the six classes, determine: does the described change touch the trigger surface?\n" +
            "   - If YES: copy the row verbatim from SUBSYSTEM_HAZARD_DEFAULTS (or generic taxonomy), " +
            "     fill in the specific Surface (file:fn), and name the required adversarial test.\n" +
            "   - If NO: mark the row 'N/A — <reason>' (e.g., 'N/A — no numeric ID allocation').\n\n" +
            "Output format: a markdown table ready to paste into §Hazards of acceptance.md.\n" +
            "Every row MUST be present — no silent omissions. If you are uncertain whether a " +
            "surface is touched, mark it 'UNCERTAIN — <reason>' rather than omitting it.\n\n" +
            "Six classes to enumerate:\n" +
            "1. ID/ref-space collision\n" +
            "2. Bounds/overflow\n" +
            "3. Protocol-safety\n" +
            "4. Scanner literal/comment blindness\n" +
            "5. Test-encodes-the-bug\n" +
            "6. Coverage/measurement integrity"
        },
        {
          label: "B-contract-pointers",
          phase: "phase-1-fan-out",
          prompt:
            "You are angle B — Contract Pointers — for issue #${args.issue} (subsystem: ${args.subsystem}).\n\n" +
            "Read the issue spec. Then:\n\n" +
            "1. Read docs/reference/PARSER_CONTRACTS.md. For each contract listed, determine whether " +
            "   the described change touches, extends, or must satisfy that contract.\n" +
            "2. For LSP changes: identify which LSP spec sections (e.g., §3.17.17 inlayHint) apply.\n" +
            "3. For DAP changes: identify which DAP spec messages or sequences the change implements.\n" +
            "4. For each contract found: cite the source (doc name + section), state whether the change " +
            "   satisfies, extends, or is constrained by the contract.\n\n" +
            "Output format: a markdown table ready to paste into §Contracts of acceptance.md.\n" +
            "If no contracts apply, output: 'N/A — this change does not touch any indexed contract or " +
            "external protocol section' as the single row.\n\n" +
            "Do NOT list contracts that are clearly inapplicable. Quality over completeness here — " +
            "a false positive forces the builder to investigate and reject. Only list contracts where " +
            "you can name the specific clause that applies."
        },
        {
          label: "C-prior-art-duplicate",
          phase: "phase-1-fan-out",
          prompt:
            "You are angle C — Prior Art / Duplicate Check — for issue #${args.issue}.\n\n" +
            "Read the issue spec. Then search the codebase for existing implementations that solve " +
            "the same problem or provide the same capability:\n\n" +
            "1. Search for functions/structs with the same name or similar purpose using grep.\n" +
            "2. Search for the capability in features.toml if this is an LSP feature.\n" +
            "3. Search for related test patterns that may already cover this scenario.\n" +
            "4. Check docs/reference/PARSER_CONTRACTS.md for an existing canonical implementation.\n\n" +
            "Output two blocks:\n\n" +
            "Block 1 — for acceptance.md §Contracts (prior-art finding):\n" +
            "  If existing: 'Existing: <fn/module at path> — reuse and extend rather than duplicate.'\n" +
            "  If not found: 'No prior art found. New location: <path> is canonical because <reason>.'\n\n" +
            "Block 2 — for context.md §Prior art / duplicates:\n" +
            "  A paragraph naming what was searched and what was (or was not) found. Include the " +
            "  grep commands used as evidence. If a similar implementation exists, explain why this " +
            "  change is not a duplicate (different surface, different behavior, extends rather than duplicates)."
        },
        {
          label: "D-api-shape",
          phase: "phase-1-fan-out",
          prompt:
            "You are angle D — API Shape — for issue #${args.issue} (subsystem: ${args.subsystem}).\n\n" +
            "Read the issue spec. Sketch the public interface this change introduces or modifies:\n\n" +
            "1. For each new function: exact signature (name, params with types, return type).\n" +
            "2. For each new struct/enum: fields with types.\n" +
            "3. For each new numeric ID range: the range, the formula, and a check for disjointness " +
            "   with adjacent ranges (consult docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md DAP-1 for " +
            "   the disjointness requirement if this is a DAP change).\n" +
            "4. For each item: grep for existing definitions with the same name to assess dup-risk.\n" +
            "5. For each item: estimate the initial caller count (0 if new, N if modifying existing).\n\n" +
            "Goal: make interface correctness properties visible BEFORE the builder writes code. " +
            "An interface that makes class-1 and class-2 hazards structurally difficult to trigger " +
            "is preferred over one that requires careful runtime checks.\n\n" +
            "Output format: a markdown table ready to paste into §API-Shape of acceptance.md.\n" +
            "If no new public API surface is introduced, output: 'N/A — no new public API surface'."
        },
        {
          label: "E-test-grid",
          phase: "phase-1-fan-out",
          prompt:
            "You are angle E — Test Grid — for issue #${args.issue} (subsystem: ${args.subsystem}).\n\n" +
            "Read the issue spec. Enumerate the axes of variation in the described change behavior:\n\n" +
            "1. Identify all inputs and their possible states (empty, normal, edge, adversarial).\n" +
            "2. Identify all relevant system states at call time (before/after open, running/stopped, etc.).\n" +
            "3. Identify protocol-version or capability-negotiation axes if applicable.\n" +
            "4. For each cell in the matrix, name:\n" +
            "   - The scenario description\n" +
            "   - The kind (positive / negative / adversarial / state-transition / protocol-safety)\n" +
            "   - The proposed test function name (follows existing crate naming conventions: test_<noun>_<condition>)\n" +
            "   - The invariant it discharges (link to hazard class if applicable)\n\n" +
            "Required minimum rows (mark N/A only if genuinely inapplicable with reason):\n" +
            "- Happy path (positive)\n" +
            "- Empty/null input (negative)\n" +
            "- Out-of-range or oversized input (negative — Bounds/overflow class)\n" +
            "- Malformed/unknown input (negative — Protocol-safety class)\n" +
            "- Input containing the target delimiter inside a string literal (adversarial — Scanner blindness, if applicable)\n" +
            "- ID collision between adjacent ranges (adversarial — ID/ref-space collision, if applicable)\n" +
            "- State-transition: call after resource is closed/terminated (state)\n\n" +
            "Output format: a markdown table ready to paste into §Test-Grid of acceptance.md."
        },
        {
          label: "F-blast-radius",
          phase: "phase-1-fan-out",
          prompt:
            "You are angle F — Blast Radius — for issue #${args.issue} (subsystem: ${args.subsystem}).\n\n" +
            "Read the issue spec. Identify every subsystem and crate that consumes this change's output:\n\n" +
            "1. grep for callers of the functions being modified.\n" +
            "2. List downstream crates that depend on the modified crate (check Cargo.toml dependencies).\n" +
            "3. Identify test suites in other crates that use the modified surface (snapshot tests, " +
            "   integration tests that call the affected API).\n" +
            "4. For each consumer: state the impact (none / snapshot update required / behavior change) " +
            "   and the required update action.\n" +
            "5. Identify the must-not-touch boundary: which files/modules must NOT be modified by this change.\n\n" +
            "Output format: a markdown table ready to paste into §Blast-Radius of acceptance.md, " +
            "followed by a 'Must-not-touch boundary:' line.\n" +
            "If no consumers are affected, output: 'N/A — no external consumers of the modified surface'."
        }
      ]
    },
    {
      id: "phase-2-synthesis",
      name: "Synthesis",
      description:
        "Synthesizer agent merges the six angle outputs into the final acceptance.md sections " +
        "and the context.md prior-art block. Resolves conflicts between angles, deduplicates " +
        "rows, and ensures the output matches the SPEC_TEMPLATE.md section names exactly.",
      model: "haiku",
      parallel: false,
      agents: [
        {
          label: "synthesizer",
          phase: "phase-2-synthesis",
          prompt:
            "You are the synthesizer. You have received the outputs from six parallel haiku " +
            "analysis passes (A through F) for issue #${args.issue}.\n\n" +
            "Your job: merge them into the final acceptance.md sections and one context.md block.\n\n" +
            "## Section names (must match SPEC_TEMPLATE.md exactly)\n\n" +
            "acceptance.md sections to produce:\n" +
            "- ## §Hazards  (from angle A output)\n" +
            "- ## §Contracts  (from angles B + C, merged)\n" +
            "- ## §API-Shape  (from angle D output)\n" +
            "- ## §Test-Grid  (from angle E output)\n" +
            "- ## §Blast-Radius  (from angle F output)\n\n" +
            "context.md block to produce:\n" +
            "- ## Prior art / duplicates  (from angle C block 2)\n\n" +
            "## Merge rules\n\n" +
            "1. §Hazards: Include ALL six class rows from angle A. Never drop a row silently — " +
            "   N/A rows are valid and required. If angle A marked a row UNCERTAIN, flag it for " +
            "   spec-planner to resolve before handing off to red-TDD.\n\n" +
            "2. §Contracts: Merge angle B (protocol/doc contracts) with angle C block 1 (prior-art " +
            "   finding). Deduplicate by contract name. If angle C found an existing canonical " +
            "   implementation, add a 'Reuse:' row citing it.\n\n" +
            "3. §API-Shape: Use angle D output verbatim. If angle C found a prior implementation " +
            "   of the same API shape, add a warning row: 'WARNING: possible duplicate — see §Contracts'.\n\n" +
            "4. §Test-Grid: Use angle E output. Confirm that every adversarial test row named in " +
            "   §Hazards appears in §Test-Grid. If a hazard row has a required adversarial test not " +
            "   in angle E's output, add it.\n\n" +
            "5. §Blast-Radius: Use angle F output verbatim.\n\n" +
            "6. context.md / Prior art: Use angle C block 2 verbatim.\n\n" +
            "## Output format\n\n" +
            "Produce two labeled blocks:\n\n" +
            "--- ACCEPTANCE.MD SECTIONS ---\n" +
            "<paste the five sections, each with a ## §SectionName heading>\n\n" +
            "--- CONTEXT.MD BLOCK ---\n" +
            "## Prior art / duplicates\n" +
            "<paste the prior-art paragraph from angle C>\n\n" +
            "The spec-planner will copy these blocks into the .spec/ files verbatim. " +
            "Section names must match SPEC_TEMPLATE.md exactly — deep-review checks them."
        }
      ]
    }
  ]
};
