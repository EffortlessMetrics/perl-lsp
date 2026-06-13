---
description: Spec planner step 3 — produce the ordered implementation checklist and rich acceptance.md
user-invocable: false
---

# Spec Planner: Plan

Produce the implementation checklist and rich acceptance.md. These are the primary artifacts —
the red TDD builder and builder both read them.

**Canonical format**: `docs/reference/SPEC_TEMPLATE.md` defines the exact structure and section
names. acceptance.md must contain ALL six required sections. The section names in this skill
match SPEC_TEMPLATE.md exactly — do not rename them.

## Decision: trivial or non-trivial?

Before writing acceptance.md, determine if the issue is **trivial** (all three hold):
- Changes at most 1 file
- No new public API surface
- Does not touch any protocol handler (LSP/DAP/stdin)

If **non-trivial**: invoke the `spec-builder` workflow (`.claude/workflows/spec-builder.js`) with
`{ issue, subsystem, risk }` to populate §Hazards, §Contracts, §API-Shape, §Test-Grid, and §Blast-Radius
from six parallel haiku angles. Copy the workflow output into acceptance.md verbatim, then add §Behavior.

If **trivial**: populate all six sections manually, marking non-applicable rows `N/A — <reason>`.

## Checklist format

Write `.spec/<issue#>-<specslug>/checklist.md`:

```markdown
# Implementation Checklist: #<issue> — <title>

## Change order (compiles at each step)

### Step 1: <what>
- **File:** `<exact path>` (CREATE if new)
- **Change:** <add field / modify function / add match arm / etc.>
- **Details:** <specific signature, type, or code pattern>
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

- <any ambiguities, missing details, or decisions the builder must make>
```

## Acceptance criteria format (rich — all six sections required)

Write `.spec/<issue#>-<specslug>/acceptance.md`:

```markdown
# Acceptance Criteria: #<issue> — <title>

## §Behavior

| Input / Condition | Expected Result | Notes |
|---|---|---|
| <normal case> | <result> | |
| <edge case> | <result> | |
| <error case> | <result> | |

All tests pass: `cargo test -p <crate>`
No clippy warnings: `cargo clippy -p <crate>`
Formatted: `cargo xtask fmt`

## §Hazards

One row per hazard class. Seeded from docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md for
the issue's subsystem. Mark non-applicable rows `N/A — <reason>`.

| Class | Invariant | Surface (file:fn) | Required adversarial test |
|---|---|---|---|
| ID/ref-space collision | ... | ... | ... |
| Bounds/overflow | ... | ... | ... |
| Protocol-safety | ... | ... | ... |
| Scanner literal/comment blindness | ... | ... | ... |
| Test-encodes-the-bug | ... | ... | ... |
| Coverage/measurement integrity | ... | ... | ... |

**Subsystem-specific defaults consulted**: docs/reference/SUBSYSTEM_HAZARD_DEFAULTS.md — <subsystem>

## §Contracts

| Contract | Source document + section | How this change satisfies or extends it |
|---|---|---|
| <contract name> | PARSER_CONTRACTS.md §N | <description> |

N/A — <if no contracts apply, state so explicitly>

## §API-Shape

| Item | Kind | Signature / Range | Dup-risk (grep result) | Caller count |
|---|---|---|---|---|
| `<Name>` | function/struct/range | <signature> | <grep result> | <count> |

N/A — <if no new public API surface>

## §Test-Grid

| Scenario | Kind | Test name | Invariant discharged |
|---|---|---|---|
| Normal input | positive | `test_<name>_happy` | Basic behavior |
| Empty / null input | negative | `test_<name>_empty` | No panic on empty |
| Out-of-range ID | negative | `test_<name>_oob` | Bounds/overflow |
| Malformed input | negative | `test_<name>_malformed` | Protocol-safety |
| <subsystem-specific adversarial rows> | adversarial | `test_<name>_<condition>` | <class> |

## §Blast-Radius

| Consumer | Crate | Dependency type | Impact | Required update |
|---|---|---|---|---|
| <caller> | <crate> | direct call | none | none |

Must-not-touch boundary: <list>
```

## Context format

Write `.spec/<issue#>-<specslug>/context.md`:

```markdown
# Context: #<issue> — <title>

## Problem

<One paragraph: what is broken or missing and its user/system impact.>

## Why this approach

<Key decisions from plan-review and why chosen over alternatives.>

## Alternatives rejected

- **<Alternative A>**: rejected because <reason>.

## Prior art / duplicates

<Result of prior-art scan. If existing implementation found: name it.
If not found: state that and confirm the new location is canonical.>

## Links

- Issue: #<issue>
- Plan-review comment: <URL>
- PARSER_CONTRACTS.md: <§N if relevant>
- docs/concepts/: <which portable pattern applies>
- docs/learnings/: <which incident motivated this>
- Related issues: #<N> — <how it relates>
```

## Hazard seeding quick reference

Before writing §Hazards, identify the subsystem and seed rows from SUBSYSTEM_HAZARD_DEFAULTS.md:

| Subsystem trigger | Rows to copy |
|---|---|
| `crates/perl-dap/` or `dap*` in perl-lsp-rs | DAP-1 through DAP-7 (select applicable) |
| `crates/perl-parser/`, `crates/perl-lexer/`, `crates/perl-parser-core/` | PARSER-1 through PARSER-4 |
| `crates/perl-lsp/`, `crates/perl-lsp-rs/`, `crates/perl-lsp-*/` | LSP-1 through LSP-4 |
| `xtask/`, `.ci/`, `.github/workflows/` | COV-1 through COV-4 |

Fill in the `Surface` field with the specific file:fn. A row may be omitted only when the surface
is provably not touched — document in context.md, not by silent drop.
