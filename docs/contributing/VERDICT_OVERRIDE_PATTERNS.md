# VERDICT_OVERRIDE_PATTERNS.md

When and how the orchestrator should override verification-agent verdicts. Complements [VERIFICATION_LADDER.md](./VERIFICATION_LADDER.md) (which covers WHO verifies WHAT) with the question of WHEN to disagree with a verdict.

## Why this doc exists

Verification agents (diaboli, oppositional-planner, architecture-reviewer, maintainer) are **advisors, not directives**. The orchestrator owns the routing decision. Blindly honoring every DEFER / CLOSE / BLOCK verdict is both conservative and wasteful — verdicts are bound to a specific scope and premise, and those often change when you look again.

Observed 2026-04-19 Wave G1 session: two diaboli DEFER verdicts (on #4497 and #4499) were reversed by the orchestrator after re-examining premises at reduced scope. Both PRs merged cleanly same day. If both DEFERs had been honored blindly, the session would have ended at 64 published crates instead of 49 — a ~30-40% productivity loss from two unnecessary defers.

## Pattern 1: Scope-pivot on DEFER

**When diaboli or maintainer returns DEFER, the first question is NOT "how long do we wait" — it's "does this defer still apply at reduced scope?"**

DEFER verdicts almost always have an implicit "at the proposed scope" qualifier. The agent wasn't asked to consider scope alternatives; it evaluated what was on the issue. Shrink the scope, and the blocker often evaporates.

### How to apply

1. **Re-read the DEFER rationale**. Identify WHY: risk, timing, noise, churn, premise-flawed?
2. **Ask explicitly**: "If we cut scope to the minimum that matters for v0.13.0, does this rationale still hold?"
3. If **no** — file a PIVOT comment on the issue with the reduced scope + route back to plan-reviewer.
4. If **yes** (genuine structural blocker independent of scope) — honor the DEFER.
5. **Always write the override reasoning on the issue**. Future agents must see the explicit trail.

### Concrete examples from Wave G1

**#4497 `cargo public-api` ratchet**
- Diaboli verdict: DEFER until post-Wave-G3. Rationale: 74-crate baseline would churn 15+ times through G-waves.
- Scope pivot: facade-only (5 crates: perl-lsp-rs, perl-parser, perl-uri, perl-dap, perllsp). Facades don't churn during satellite collapse — that's the whole point of the facade/core split.
- Premise evaporates: baseline-churn concern doesn't apply to 5 crates that don't churn.
- Outcome: built + merged same day as #4504.

**#4499 offline manifest-lint**
- Diaboli verdict: DEFER. Rationale: 6 checks overlap with `cargo package`, allowlist-drift check during active G-waves would fire constantly on expected changes.
- Scope pivot: 2 checks (consolidate existing Python `--check-drift` + add LICENSE-present). Drop the 4 that `cargo package` already catches.
- Premise evaporates: drift check operates on the combined publish allowlist + baseline. G-wave PRs already update both files together (serial merge train); drift is internally consistent per PR.
- Outcome: built + merged same day as #4505.

## Pattern 2: Re-weigh every prior comment

**Every prior comment is a hypothesis bound to a SHA, not authority.**

This applies equally to:
- Scout findings (accuracy-scout found the #4496 scout's "no inter-provider deps" framing was incomplete)
- Research verifications (research-verifier caught the #4498 false premise about `cargo publish --dry-run`)
- Diaboli verdicts (see Pattern 1)
- External AI advisories pasted into the session (ChatGPT / Opus elsewhere — both pages may be cached minutes out of date)
- Even diff-auditor approvals (Wave F #4493 diff-audit was CLEAN but shipped bit-rot later caught as #4502/#4503)
- Tool success reports (TaskUpdate reported "Updated" while the state didn't actually change — #4509)

### How to apply

- **Before routing based on a label**, check the label is receipt-bound to current HEAD via the receipt system. Stale labels are unreliable.
- **Before acting on external advice**, do a 1-minute live-truth sweep: `git log`, `gh pr list`, `gh issue view <N>`. The advice may reference PRs already merged.
- **Before trusting an agent's comment on an old issue**, re-verify the specific fact at current state. 30 seconds of `cat`/`grep`/`gh view` is cheap insurance.
- **When an agent's observation CONFLICTS with yours**, trust your own read. Write a correction comment; don't silently work around.
- **When a tool reports success**, verify by re-reading the underlying state if the cost is low.

## Pattern 3: DEFER is an invitation to re-examine, CLOSE is a verdict

If diaboli returns CLOSE (not DEFER), the situation is different. CLOSE means the premise is broken — a scope-pivot creates a NEW issue, not a continuation. See #4498 → #4499 transition:

- Original #4498 premise (`cargo publish --dry-run` validates against crates.io) was **false** per research-verifier.
- Diaboli verdict: CLOSE.
- Orchestrator action: close #4498 with comment documenting why, file #4499 as a NEW issue with the correct premise (consolidate existing Python drift-check + LICENSE), thread a link from #4498 to #4499.

**Why not repurpose #4498?** The comment trail on #4498 is a teaching artifact — future agents benefit from seeing the broken-premise catch. Repurposing obscures the lesson.

## Related

- [VERIFICATION_LADDER.md](./VERIFICATION_LADDER.md) — claim-type to verifier mapping
- [../project/protocols/layered-verification.md](../project/protocols/layered-verification.md) — theory
- Session retrospective: [../forensics/2026-04-19-wave-g1-collapse-retrospective.md](../forensics/2026-04-19-wave-g1-collapse-retrospective.md)
