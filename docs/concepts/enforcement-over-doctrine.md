# Enforcement Over Doctrine — Knowledge Is Not Behavior

*Portable concept. Grounded in perl-lsp 2026-06 synthesis. See also: [slow-stochastic-compiler](slow-stochastic-compiler.md), [gate-names-must-match-failure-classes](gate-names-must-match-failure-classes.md), [non-exhaustive-check-silent-drop](non-exhaustive-check-silent-drop.md), [verify-the-instrument](verify-the-instrument.md).*

---

## The Thesis

A learning encoded only as prose changes behavior almost not at all.

Stochastic agents — and humans — read documentation and then do something else. The unsettling evidence is self-demonstrating: in the perl-lsp 2026-06 session, the orchestrator authored the learning "do not re-run a broken process" and then did exactly that, wrongly declaring #1457 deadlocked and re-running the failed merge gate until it broke master. A red-tdd agent shipped invalid-red (5 of 6 "red" tests passing) immediately **after** the "red tests must actually be red" rule was encoded in the spec. Gate-name and coverage-name confusions recurred after the [gate-names-must-match-failure-classes](gate-names-must-match-failure-classes.md) concept landed. NodeKind exhaustiveness gaps appeared in three consecutive PRs despite a specific learning document.

The author of a rule is not immune to violating it. Neither is a system that merely documents the rule.

Therefore the law: **every learning must name its ENFORCEMENT mechanism, or it is theater.**

---

## The Enforcement Ladder

Not all enforcement is equal. Some mechanisms make the violation impossible. Some make it caught. Some merely document it for awareness. The ladder, in order of effectiveness:

### Compile-time-impossible (the gold rung)

The violation is a type error, a module resolution failure, or a missing trait impl. The code cannot compile if the rule is broken.

**Example**: ID-space collision between DAP variable-refs. Solved by promoting a newtype wrapper (PR #1219 fix) so that ID-space operations are type-safe. Once promoted, no builder can accidentally confuse ref-spaces again — the type checker enforces it.

**Cost**: Highest design cost, zero runtime cost, zero operational cost.

### Lint / static analysis (compiler-ish)

A clippy lint, a custom proc macro, or a focused `cargo check` variant detects the violation before the PR is built.

**Example**: Banned patterns (unwrap, expect, panic in production code). A clippy lint rule enforces it; every builder runs clippy before push.

**Cost**: Medium design cost, zero runtime cost, low operational cost (one more linter pass).

### CI gate (automated catch)

A CI check—xtask validator, focused integration test, snapshot test, coverage gate, or custom check—verifies the rule post-merge. The violation is caught before shipping.

**Example**: Parser-contracts exhaustiveness check. When a new NodeKind variant is added, a focused CI gate verifies that all non-exhaustive consumers are either fixed or explicitly audited. Violation is caught at merge time, not post-release.

**Cost**: Medium design cost (writing the check), low runtime cost (focused check), moderate operational cost (gate latency, maintenance).

### Hazard-default checklist (specification catch)

A spec row in the acceptance criteria or hazard-class default codifies the invariant, and the red-tdd and builder stages verify it as a condition of done. The violation is caught before code is written.

**Example**: [SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md) requires that every parser-fix includes a snapshot test for the new variant and explicit audits of known non-exhaustive consumers. Before a builder ships a parser change, the spec checklist flags missing hazard rows.

**Cost**: Low design cost (add a checklist row), zero runtime cost, low operational cost (agent reads the checklist).

### Agent-instruction-with-trigger (contextual catch)

An agent's instruction includes a specific trigger: "before you commit, check [specific thing]" or "if the PR title contains [keyword], verify [consequence]." The check happens in the moment, when stakes are highest.

**Example**: Before deep-review approves a gate-change PR, the instruction says: "Run the full merge-gate tier locally—not just the gate you changed—and verify that the entire suite passes on current HEAD before signing off."

**Cost**: Low design cost (add to agent docstring), low runtime cost, moderate operational cost (depends on human/agent attention in the moment).

### Prose documentation (behavioral awareness only)

A markdown file, a doctrine article, a concept doc. The reader is expected to understand the rule, internalize it, and apply it without mechanical enforcement.

**Example**: The learning "do not re-run a broken process; diagnose instead" was encoded in four separate concept documents before the 2026-06 session began. It did not prevent the orchestrator from re-running the failed merge gate three times in a row.

**Cost**: Low design cost (write the doc), zero runtime cost, **high operational cost** (relies on human/agent memory and judgment; failure rate is stochastic and recurrent).

---

## The Cost-Effectiveness Principle

**A rule encoded only at the prose rung will fail stochastically.** The failure rate depends on salience (how memorable the rule is) and the cost of failure. A rule about "never delete the production database" is remembered; a rule about "the default cache TTL should be 5 minutes, not 10" is not.

The acceptance test for a new learning:
> What mechanism makes repeating this **caught or impossible**, not merely documented?

If the answer is "the doc," it will not hold.

**Where to invest the enforcement cost:**
- Compile-time-impossible: for invariants that would be catastrophic if violated (ID-space, type mismatches, protocol violations)
- Lint / static analysis: for structural patterns that recur across PRs (banned APIs, naming conventions)
- CI gate: for behavioral contracts that are cheap to verify post-merge (exhaustiveness checks, snapshot regressions, measurement integrity)
- Hazard-default checklist: for domain-specific invariants that belong in the spec (parser contracts, coverage assumptions, DAP threading models)
- Agent-instruction: for critical sequences that require in-the-moment attention (pre-merge gate validation, branch-state verification)
- Prose: only for meta-level guidance, context, and post-incident retrospectives—never as the sole enforcement mechanism for a recurring class

---

## Mapping: When a Learning Lands, What Rung Does It Need?

| Learning Class | Recommended Rung | Why | Example |
|---|---|---|---|
| Type error (ID collision, bounds overflow) | Compile-time-impossible | One violation can cascade through the codebase | DAP ref-space newtype |
| Structural pattern (banned API, naming) | Lint | Detectable syntactically; developer must learn once | clippy unwrap ban |
| Behavioral contract (test validity, gate logic) | CI gate | Behavioral verification is cheapest post-merge; catches all builders uniformly | parser-exhaustiveness check, red-test validation |
| Domain hazard (parser coverage, DAP threading) | Hazard-default checklist | Reduces builder cognitive load; spec documents expected care | SUBSYSTEM_HAZARD_DEFAULTS rows |
| Critical sequence (pre-merge validation, branch state) | Agent-instruction-with-trigger | In-the-moment context; agent has full visibility into the state to verify | "Run full merge-gate tier before signing off on gate-change PR" |
| Context and historical reasoning | Prose documentation | Explains why the rung below exists; helps future agents understand the incident | [slow-stochastic-compiler](slow-stochastic-compiler.md), [verify-the-instrument](verify-the-instrument.md) |

---

## The Recursion: Enforcement Itself Must Be Enforced

The enforcement ladder is itself a subject of doctrine drift. A CI gate that breaks silently and nobody re-fixes becomes documentation. A spec checklist that spec-planner doesn't populate becomes optional. An agent-instruction that is not in the docstring is invisible.

Therefore:

1. **After a learning lands, check the enforcement rung.** If it's prose, file a follow-up issue to promote it: "add mechanical check for [class]".
2. **After a gate-change lands, run the full merge-gate tier locally.** The gate itself cannot test its own correctness; you must.
3. **After a spec hazard row is added, verify the red-tdd stage reads it.** Spec rows that are not read are not enforced.
4. **After a lint rule is added, verify it ships in `cargo clippy`.** A lint rule that doesn't run is not enforcement.

---

## Implications

**[slow-stochastic-compiler](slow-stochastic-compiler.md)** — The enforcement ladder is the cost-tiering principle in action. Compile-time is cheapest; prose is most expensive. Choose the rung that offers sufficient reliability at acceptable cost.

**[gate-names-must-match-failure-classes](gate-names-must-match-failure-classes.md)** — A CI gate named "coverage" that actually validates test count is a misnamed gate. The name must match what it enforces, or agents and humans will apply it to the wrong class of failure.

**[non-exhaustive-check-silent-drop](non-exhaustive-check-silent-drop.md)** — A check that silently skips invalid input (non-exhaustive pattern match) is a prose rule in disguise—no enforcement at all. If the behavior is required, make it a compile error.

**[verify-the-instrument](verify-the-instrument.md)** — Before trusting an enforcement gate, verify that it measures what it claims. A gate that enforces the wrong thing is worse than no gate.

---

## Ground Truth from 2026-06

The perl-lsp 2026-06 synthesis learned these rules through lived failure:

- **Invalid-red persisted** (#1338, #1372, #1445): Red-TDD produced tests that passed immediately because the learning "red tests must actually be red" was prose-only. Once a spec hazard row was added requiring red-test validation (gate #1), the violation was caught pre-merge.
- **Orchestrator re-ran the broken gate** (#1469, #1477, #1478): The learning "do not re-run a broken gate" was in four concept documents and ignored. The fix: an agent-instruction trigger with explicit in-the-moment validation ("run the full tier locally before approving").
- **Gate and coverage names lied** (#1457, #1470, #1469): The gate named "coverage-job" ran integration tests; "coverage-gate" checked test counts, not coverage. The learning "gate names must match failure classes" was encoded as a concept doc in PR #1330 and violated again in PR #1469. The fix: a checklist row in SPEC_UPDATE_CHECKLIST requiring gate-name accuracy, enforced by spec-planner.
- **NodeKind variant silent-drop recurred** (#1362, #1457, #1459): Three PRs in a row added new NodeKind variants without fixing non-exhaustive consumers. The prose learning did not work. The fix: a parser-contracts CI gate that audits exhaustiveness, making violation impossible (rung 3).

Every rung up the ladder prevented a category of recurrence. Prose alone prevented nothing.

---

## When to Break the Rule

Never. The rule stands in every category: enforcement over doctrine. If you find yourself wanting to document-only, ask instead: what mechanism would make this caught or impossible?

If the cost is too high, document **why**—and file a follow-up issue to instrument it later. But do not pretend the documentation is enforcement.
