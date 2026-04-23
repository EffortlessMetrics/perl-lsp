---
description: Spec planner step 3 — produce the ordered implementation checklist
user-invocable: false
---

# Spec Planner: Plan

Produce the implementation checklist. This is the primary artifact —
the red TDD builder and builder both read this.

## Checklist format

Write `.spec/<issue#>-<specslug>/checklist.md`:

```markdown
# Implementation Checklist: #<issue> — <title>

## Change order (compiles at each step)

### Step 1: <what>
- **File:** `<exact path>`
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

Write `.spec/<issue#>-<specslug>/acceptance.md`:

```markdown
# Acceptance Criteria: #<issue>

- [ ] <criterion 1 from spec>
- [ ] <criterion 2>
- [ ] <edge case from oppositional review>
- [ ] All tests pass: `cargo test -p <crate>`
- [ ] No clippy warnings: `cargo clippy -p <crate>`
- [ ] Formatted: `cargo xtask fmt`
```

Write `.spec/<issue#>-<specslug>/context.md`:

```markdown
# Context: #<issue>

## Decision log
- <key decision from plan-review and why>
- <alternative rejected and why>

## Objections addressed
- <from oppositional planner, and how resolved>

## Research findings
- <from research verifier, key confirmed/corrected facts>

## Related issues
- #<related> — <how it relates>
```
