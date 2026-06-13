# Spec Template

**Purpose**: This document defines the canonical structure for every `.spec/<issue#>-<slug>/` directory
produced by the spec-planner. It makes the spec-builder workflow, the spec-planner agent, and the
`acceptance.md` section list agree on names — so deep-review confirms instead of discovers.

**Cross-references**:
- Hazard classes: [docs/agents/SPEC_UPDATE_CHECKLIST.md §8](../agents/SPEC_UPDATE_CHECKLIST.md#8-hazard-class-invariants)
- Subsystem defaults: [docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md](SUBSYSTEM_HAZARD_DEFAULTS.md)
- Contract index: [docs/reference/PARSER_CONTRACTS.md](PARSER_CONTRACTS.md)
- Patterns: [docs/concepts/multi-angle-haiku-early-spec.md](../concepts/multi-angle-haiku-early-spec.md), [docs/concepts/shift-left-ladder.md](../concepts/shift-left-ladder.md)
- Workflow: [`.claude/workflows/spec-builder.js`](../../.claude/workflows/spec-builder.js) — the saved workflow that populates §Hazards, §Contracts, §API-Shape, §Test-Grid, §Blast-Radius in parallel

**When to run the spec-builder workflow**: For any non-trivial change (new feature, new protocol
surface, shared interface change, recurring bug class fix). For trivial changes (one-line constant,
typo, docs-only), populate the sections manually and mark non-applicable rows N/A with a reason.

---

## Directory Structure

```
.spec/
  <issue#>-<slug>/
    checklist.md    # ordered implementation steps, compiles at each step
    acceptance.md   # acceptance criteria — all required sections present
    context.md      # problem, decisions, alternatives-rejected, prior-art, links
```

---

## checklist.md

The builder's step-by-step guide. Every step must compile. The red-TDD builder reads this to know
where to write failing tests. The builder reads it to know the change order.

```markdown
# Implementation Checklist: #<issue> — <title>

## Change order (compiles at each step)

### Step 1: <what>
- **File:** `<exact path>` (CREATE if new)
- **Change:** <add field / modify function / add match arm / new module / etc.>
- **Details:** <specific signature, type, or code pattern expected>
- **Verify:** `cargo check -p <crate>`

### Step 2: <what>
- **File:** `<exact path>`
- **Change:** <description>
- **Details:** <specifics>
- **Depends on:** Step 1
- **Verify:** `cargo check -p <crate>`

...

### Step N: Final verification
- **Verify:** `cargo test -p <crate> && cargo xtask fmt && cargo clippy -p <crate>`

## Callers and consumers

- `<function>` is called from: <list of files>
- `<struct>` is used in: <list of files>

## Scope boundary

Files IN scope: <list>
Files OUT of scope: <everything else — be explicit>

## Flags for builder

- <ambiguities the builder must resolve>
- <decisions the builder must make>
- <anything the spec says but doesn't specify>
```

---

## acceptance.md

The canonical sections are listed below. **All sections are required.** Mark non-applicable rows
`N/A — <reason>` rather than omitting them — omission looks like oversight; `N/A` is explicit.

The spec-builder workflow populates §Hazards, §Contracts, §API-Shape, §Test-Grid, and §Blast-Radius
from six parallel haiku angles. The spec-planner seeds §Behavior and copies applicable rows from
[SUBSYSTEM_HAZARD_DEFAULTS.md](SUBSYSTEM_HAZARD_DEFAULTS.md).

```markdown
# Acceptance Criteria: #<issue> — <title>

## §Behavior

Tabular summary of what the change does. One row per distinct behavior.

| Input / Condition | Expected Result | Notes |
|---|---|---|
| <normal case> | <result> | |
| <edge case> | <result> | |
| <error case> | <result> | |

All tests pass: `cargo test -p <crate>`
No clippy warnings: `cargo clippy -p <crate>`
Formatted: `cargo xtask fmt`

## §Hazards

One row per applicable hazard class. Copied verbatim from
[SUBSYSTEM_HAZARD_DEFAULTS.md](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md) for the issue's
subsystem, then supplemented with any cross-subsystem rows from SPEC_UPDATE_CHECKLIST §8.

Mark non-applicable rows `N/A — <reason>`.

| Class | Invariant | Surface (specific file/fn this change touches) | Required adversarial test |
|---|---|---|---|
| ID/ref-space collision | <invariant from SUBSYSTEM_HAZARD_DEFAULTS> | <file:fn> | <test name> |
| Bounds/overflow | ... | ... | ... |
| Protocol-safety | ... | ... | ... |
| Scanner literal/comment blindness | ... | ... | ... |
| Test-encodes-the-bug | ... | ... | ... |
| Coverage/measurement integrity | ... | ... | ... |

**Subsystem-specific defaults consulted**: [SUBSYSTEM_HAZARD_DEFAULTS.md — <subsystem> section](../reference/SUBSYSTEM_HAZARD_DEFAULTS.md)

## §Contracts

Which contracts from [PARSER_CONTRACTS.md](../reference/PARSER_CONTRACTS.md) (or LSP spec /
DAP spec sections) this change touches or must satisfy. One row per contract.

| Contract | Source document + section | How this change satisfies or extends it |
|---|---|---|
| <contract name> | PARSER_CONTRACTS.md §N | <description> |
| <LSP spec section> | LSP spec §<N> | <description> |

N/A rows must be explicit: `N/A — this change does not touch any indexed parser contract or
external protocol section`.

## §API-Shape

New public types, functions, enum variants, or ID-spaces introduced by this change.
Pre-declaring the shape catches hazard-class-1/2 violations at spec time.

| Item | Kind | Signature / Range | Dup-risk (grep result) | Caller count |
|---|---|---|---|---|
| `<FnName>` | function | `fn foo(x: Bar) -> Result<Baz>` | none found | 0 (new) |
| `<TypeName>` | struct | `struct Foo { ... }` | none found | 0 (new) |
| `<ID range>` | numeric range | `50_000..=59_999` | adjacent: scope refs at `frame_id*10+scope_type` — disjoint? YES | n/a |

N/A — `this change has no new public API surface`.

## §Test-Grid

Enumeration of test cases covering axes of variation. Each row names the test function
that discharges the invariant. The red-TDD builder writes failing versions of these tests.

| Scenario | Kind | Test name | Invariant discharged |
|---|---|---|---|
| Normal input | positive | `test_<name>_happy` | Basic behavior |
| Empty / null input | negative | `test_<name>_empty` | No panic on empty |
| Out-of-range ID | negative | `test_<name>_oob` | Bounds/overflow class |
| Malformed protocol message | negative | `test_<name>_malformed` | Protocol-safety class |
| Delimiter inside string literal | adversarial | `test_<name>_in_string` | Scanner blindness class |
| ID collision between adjacent ranges | adversarial | `test_<name>_id_collision` | ID/ref-space collision class |
| State-transition: call after close | state | `test_<name>_after_close` | Document lifecycle |
| Concurrent open+change | state | `test_<name>_concurrent` | Race-safety |

## §Blast-Radius

Subsystems and crates that consume this change's output. Confirm each is unaffected or
list the required update.

| Consumer | Crate | Dependency type | Impact | Required update |
|---|---|---|---|---|
| `<caller fn>` | `<crate>` | direct call | none — signature unchanged | none |
| `<test suite>` | `<crate>` | test fixture | snapshot update required | update snapshot |
| `<downstream crate>` | `<crate>` | transitive | N/A — no shared surface | none |

Must-not-touch boundary: <list files/modules that must not be modified by this change>

## §Coverage-Map (optional)

Only required when the change touches coverage tooling or when Codecov patch coverage
requires explanation. Skip (omit the section) for standard changes.

| New code path | Covered by | Test file |
|---|---|---|
| `src/foo.rs:bar()` happy path | `test_bar_happy` | `tests/foo.rs` |
| `src/foo.rs:bar()` error arm | `test_bar_error` | `#[cfg(test)]` inline |
```

---

## context.md

The "why" record. Agents and maintainers read this to understand decisions without
reconstructing the issue trail.

```markdown
# Context: #<issue> — <title>

## Problem

<One paragraph: what is broken or missing, what user / system impact it has.>

## Why this approach

<Key decisions from plan-review and why they were chosen over alternatives.>

## Alternatives rejected

- **<Alternative A>**: rejected because <reason>.
- **<Alternative B>**: rejected because <reason>.

## Prior art / duplicates

<Result of the prior-art scan (spec-builder angle C). If a similar function exists,
name it and explain why this is not a duplicate. If no prior art found, state that.>

## Links

- Issue: #<issue>
- Plan-review comment: <URL>
- PARSER_CONTRACTS.md §<N>: <what contract is relevant>
- docs/concepts/: <which portable pattern applies>
- docs/learnings/: <which incident motivated this>
- Related issues: #<N> — <how it relates>
```

---

## Subsystem defaults note

When the issue's subsystem is known, copy the applicable hazard rows verbatim from
[SUBSYSTEM_HAZARD_DEFAULTS.md](SUBSYSTEM_HAZARD_DEFAULTS.md) into §Hazards and fill in
the `Surface` field. The four subsystems with pre-seeded rows are:

| Subsystem | Trigger | Default rows |
|---|---|---|
| DAP | Touches `crates/perl-dap/` or `crates/perl-lsp-rs/src/dap*` | DAP-1 through DAP-7 (select applicable) |
| Parser | Touches `crates/perl-parser/`, `crates/perl-lexer/`, `crates/perl-parser-core/` | PARSER-1 through PARSER-4 (select applicable) |
| LSP | Touches `crates/perl-lsp/`, `crates/perl-lsp-rs/`, `crates/perl-lsp-*/` | LSP-1 through LSP-4 (select applicable) |
| Coverage/CI | Touches `xtask/`, `.ci/`, `.github/workflows/` | COV-1 through COV-4 (select applicable) |

A row may be omitted only when the specific surface is provably not touched — document
the reasoning in `context.md` rather than silently dropping it.

---

## Worked examples (three shapes)

### Shape 1: Parser fix

A bug where the parser panics on a specific input. Single-crate change.

**checklist.md sketch**:
```
Step 1: Add test fixture file (CREATE) — verify cargo check passes
Step 2: Fix the parse arm in crates/perl-parser/src/expressions/foo.rs — verify cargo check
Step 3: cargo test -p perl-parser && cargo xtask fmt && cargo clippy -p perl-parser
```

**acceptance.md highlights**:
- §Hazards rows: PARSER-1 (literal/comment blindness — if the fix touches a scanner),
  PARSER-2 (delimiter pairing — if the fix touches brace counting),
  PARSER-4 (recovery honesty — if the fix changes error-recovery behavior).
  Mark DAP/LSP/COV rows `N/A — parser-only change`.
- §Contracts: cite PARSER_CONTRACTS.md §N if the fix involves a quote-like or known contract.
- §Blast-Radius: confirm no LSP or DAP consumer is broken; run `cargo test --workspace --lib`.

**context.md highlights**:
- Document the specific Perl input that triggered the panic.
- Document the prior-art check: "searched for existing panic handlers in expressions/foo.rs — none found".

---

### Shape 2: LSP feature (new capability)

A new `textDocument/inlayHint` provider. Touches `crates/perl-lsp-*/`.

**checklist.md sketch**:
```
Step 1: Add InlayHintParams struct + handler stub (CREATE or MODIFY) — verify cargo check
Step 2: Add capability declaration in features.toml — verify cargo check
Step 3: Implement provider logic — verify cargo check
Step 4: Register handler in dispatch — verify cargo check
Step 5: cargo test -p perl-lsp-rs && cargo xtask fmt && cargo clippy -p perl-lsp-rs
```

**acceptance.md highlights**:
- §Hazards rows: LSP-1 (request-shape validation), LSP-2 (document lifecycle), LSP-3 (URI normalization).
  Mark DAP/Parser/COV rows `N/A — LSP-only change`.
- §API-Shape: document the new handler signature; grep for existing capability to confirm
  no duplicate implementation.
- §Test-Grid: include state-transition rows (before didOpen, after didClose, rapid didOpen+didChange).
- §Blast-Radius: confirm `features.toml` capability table is consistent with implementation.

**context.md highlights**:
- Cite the LSP spec section (e.g., `textDocument/inlayHint` §3.17.17).
- Document why the implementation location was chosen over alternatives.

---

### Shape 3: Test-only change

Adding adversarial tests to an existing suite. No production code changes.

**checklist.md sketch**:
```
Step 1: Add test cases to existing test file — verify cargo test -p <crate>
Step 2: cargo xtask fmt && cargo clippy -p <crate>
```

**acceptance.md highlights**:
- §Hazards: `N/A — no production code changes; hazard classes apply to implementation, not test-only PRs`.
- §Contracts: cite the contract being tested, but no new contract is introduced.
- §API-Shape: `N/A — no new public API`.
- §Blast-Radius: `N/A — test-only; no production surface changed`.
- §Test-Grid: this IS the test grid — enumerate the added test cases.
- §Coverage-Map: note that new `#[cfg(test)]` inline tests count toward `--lib` profdata.

**context.md highlights**:
- Document what gap the tests fill.
- Document the prior-art check: confirm these exact scenarios were not already covered.
