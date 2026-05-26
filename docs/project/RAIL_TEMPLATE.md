# Rail Burndown Template

A **rail** in perl-lsp is the `@INC`-shaped pattern:

> **Existing substrate** + **small connector PR(s)** = **user-visible upside**.

Each rail has a roadmap doc following the canonical shape below. Coworker
agents (codex, factory-droid) and human contributors pick rails from
[`RAILS_INDEX.md`](RAILS_INDEX.md). A rail's doc is the single source of
truth for: what's already built, what connectors remain, which agent owns
them, and what "closed" means.

> This template is **structure only**. New rail docs instantiate it; the
> existing rail docs may continue to use their current shape until they
> are refreshed in their own PR.

## Canonical rail-doc shape

Every new rail doc SHOULD use the section order below. Sections marked
**(required)** must be present; **(optional)** may be omitted if not
applicable.

```markdown
# <Rail Name> Burndown

> **Substrate (already built)**: <what exists, with PRs / files / crates>
> **Connector gap**: <the small wiring that makes the substrate user-trustworthy>
> **0.14.0 upside**: <user-visible value of closing this rail>

## Status

| Phase | Issue | Builder-ready? | PR | Receipt |
|---|---|---|---|---|
| 1. <phase name> | #N | yes/no | #N or — | command / file |
| 2. <phase name> | #N | yes/no | — | — |
| 3. <phase name> | #N | no (depends on 2) | — | — |

## Exit criteria

A rail is "closed" when ALL of:

- [ ] All phases land or are explicitly deferred with a successor issue.
- [ ] The user-facing receipt is reproducible (named command in this doc).
- [ ] The status doc (`docs/project/status/<rail>.md` or equivalent) is updated.
- [ ] The claim boundary is recorded.

## Claim boundary

What this rail proves and does NOT prove. The "does NOT prove" line is mandatory.

## Receipts

Copy-pasteable validation commands. Reproducible by any agent or human.

## Related

- Umbrella issue: #N
- Architecture: <doc path / issue>
- Status doc: <doc path>
- Adjacent rails: <list>
- Forensics: <if applicable>

## Do not combine

What this rail's PRs must NOT include. Adjacent-line conflicts, scope
creep, premature optimization.

## Lane assignment

Which coworker agent owns the rail's PRs. Options: codex (clippy /
mechanical refactors), factory-droid (policy validators), orchestrator
(rollout docs only), specific builder (named).

When lane = **codex**, add explicit anti-interference directions so codex
stays in its own lane when multiple rails are active in parallel:

- Name the exact files or globs codex may touch for this rail.
- Name adjacent files/areas that are off-limits because they belong to
  other active rails.
- Require codex to stop and hand back if the needed edit crosses into a
  different lane's owned surface.
- Require a pre-edit check (`git diff --name-only` and current rail doc)
  to confirm no unrelated lane files are already dirty.
- Require PR scope to one lane row/phase only; no "while I'm here"
  cleanup from neighboring rails.
```

## Why this shape

- **Substrate / connector / upside header**: forces every rail to name
  *what is already trustworthy* and *the small wiring* that lets the user
  feel it. Rails that can't answer this cleanly are usually still in
  scout-design territory and should be reframed before they consume
  builder time.
- **Status table**: one row per phase. `builder-ready` is the only column
  the routing orchestrator and coworker agents read to decide pickup.
- **Exit criteria**: every rail closes on the same four conditions.
  Receipts and claim boundary are not optional — they are how the user
  trusts that "the rail is done".
- **Claim boundary**: mandatory "does NOT prove" line prevents scope
  creep into adjacent rails. The `@INC` rail's claim boundary is the
  template here: it proves cross-consumer agreement; it does not prove
  CPAN coverage.
- **Do not combine**: the failure mode is adjacent-line conflicts when
  two rails touch the same file (e.g. `clippy.toml` for Rust 1.95 + strong
  clippy rails). Naming the conflict surface up front lets PR authors and
  reviewers reject mixed PRs without re-deriving the rule.
- **Lane assignment**: codex / factory-droid / orchestrator / named
  builder. The lane is the rail's contract with the coworker agents.
  For codex-owned rails, include explicit allowed/off-limits file
  boundaries so parallel lanes do not collide in the same repo session.

## Filled examples

The three examples below show how the template maps onto rails that
already exist in the repo. They are illustrative — refer to each rail's
own doc for live truth.

### Example 1 — `@INC` strictness (closed)

> **Substrate (already built)**: `EffectiveIncContext` (#8504), prefix
> module scan (#8498), startup-`@INC` probe (#8497, #8518), workspace
> include-root resolution (#8496). See
> [`docs/project/status/module_resolution.md`](status/module_resolution.md).
> **Connector gap**: thread `EffectiveIncContext` into workspace-symbol
> lookups so no-lib strictness is honored at filter time (#8537, #8544).
> **0.14.0 upside**: completion / goto-def / hover stop offering modules
> the runtime cannot reach.

Status: all four consumers (PL701, completion, goto-def, hover)
agree across the seven resolution modes. Verified by `cargo test -p
perl-lsp-ux-tests --test ux_scenario_14_inc_conformance`.

Claim boundary: proves cross-consumer agreement on `@INC` modes for the
fixtures in scenario 14; does **not** prove correctness on every CPAN
module shape, and does **not** cover compiler-substrate module-request
facts (those live in `compiler_facts.md` / #8242).

Lane: closed; no further coworker pickup.

### Example 2 — Rust 1.95 / clippy cleanup

> **Substrate (already built)**: Rust 1.95 toolchain + MSRV (#8509);
> nine 1.94/1.95 lints already cleaned (#8511, #8520-#8523, #8538). See
> [`docs/development/RUST_1_95_ROLLOUT.md`](../development/RUST_1_95_ROLLOUT.md).
> **Connector gap**: 11 remaining ladder rows (C-1..RP-2). Each row is a
> single-purpose PR that removes one workspace clippy allow or tightens
> one policy.
> **0.14.0 upside**: clean clippy gate at the workspace level; release
> readiness for the next minor.

Status: every ladder row has a filed tracking issue: #8561
collapsible_match; #8562 useless_vec / vec_init_then_push; #8559
assertions_on_constants; #8563 manual_range_contains; #8564
unwrap-in-tests; #8565 rustc lint floor; #8567 / #8569 / #8571
no-panic baseline; #8574 file policy narrow; and #8576 / #8579
release prep. Cross-link tracker: #8584.

Claim boundary: proves the clippy allow set shrinks to zero with no
behavior change; does **not** prove no-panic adequacy (that's the
N-series), and does **not** prove release-readiness (that's RP-1/RP-2).

Lane: **codex**. Each row is a mechanical clippy / policy edit; codex
ships these well. Builder lane only for the no-panic baseline rows
(#8569, #8571) and release prep (#8576, #8579).

Do not combine: never bundle ladder rows. The doc says it explicitly;
the failure mode is adjacent-line conflicts in `clippy.toml`.

### Example 3 — Codecov rollout

> **Substrate (already built)**: Codecov upload in nightly CI, parser
> branch coverage baseline (`.ci/coverage-baseline.txt`), Test Analytics
> receipts on PR-fast / gate / UX lanes. See
> [`docs/ci/codecov-rollout.md`](../ci/codecov-rollout.md).
> **Connector gap**: tighten Codecov's posture so it accurately reflects
> the single `parser-branch` lane that's actually uploaded, stays out of
> branch-protection theater, and emits a coverage receipt.
> **0.14.0 upside**: Codecov becomes one well-scoped evidence lane
> alongside parser corpus / UX tests / `ripr` / mutation, instead of a
> noisy false-blocker.

Status: 8-PR ladder (Cov-1..Cov-8) defined in the doc; Cov-7 and Cov-8
are explicitly marked optional / late. Each row is single-purpose.

Claim boundary: Codecov answers "did tests execute this scoped Rust
surface, and did branch coverage regress beyond the accepted budget?";
it does **not** prove parser correctness, `@INC` correctness, LSP / DAP
completeness, CPAN coverage, mutation adequacy, no-panic cleanliness, or
release readiness.

Lane: **codex** for Cov-1 (config), Cov-3 / Cov-4 / Cov-5 / Cov-6 (docs +
policy); builder lane for Cov-2 (receipt emission in `ci-nightly.yml`)
and Cov-7 / Cov-8 (workflow extraction + ratchet calibration).

Do not combine: never bundle ladder rows; never combine Codecov edits
with parser-corpus or mutation-lane changes — they are distinct evidence
lanes with distinct claim boundaries.
