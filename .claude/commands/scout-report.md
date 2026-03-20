---
description: Write scout findings as a builder-ready GitHub issue
argument-hint: "<one-line title of the finding>"
user-invocable: false
---

# Scout Report

File a GitHub issue that a builder can implement WITHOUT re-researching.
This is the scout's primary deliverable. Invoke ONLY after completing all
7 steps of the scout checklist.

## Pre-flight Check

Before filing, verify you have ALL of these. If any are missing, go back
and complete the scout checklist step that produces it:

- [ ] **File:line locations** — exact paths to every relevant code location
- [ ] **Reproduction** — minimal example that triggers the bug/gap
- [ ] **Root cause** — one sentence explaining WHY it fails
- [ ] **Fix options** — 2-3 approaches with tradeoffs
- [ ] **Recommendation** — which option and why
- [ ] **Test spec** — exact test code or command that proves the fix works
- [ ] **Dedup confirmed** — no existing issue or PR covers this

## Template

```bash
gh issue create \
  --title "$ARGUMENTS" \
  --label "swarm-discovered" \
  --body "$(cat <<'ISSUE_EOF'
## Problem

_Exact evidence with file:line references._

<your evidence here — include file paths, line numbers, error messages>

## Root Cause

_One sentence: what's wrong in the code and where._

<e.g., "parse_phase_block in declarations.rs:845 checks for CHECK keyword
before checking if next token is Colon, so CHECK: labels are misidentified
as phase blocks.">

## Options

1. **Option A** — <what to change, which file:line>. Tradeoff: <pro/con>. Effort: <EASY/MEDIUM/HARD>.
2. **Option B** — <what to change, which file:line>. Tradeoff: <pro/con>. Effort: <EASY/MEDIUM/HARD>.

## Recommendation

<which option, one sentence why>

## Builder Spec

_Everything a builder needs to implement this without research._

**File(s) to change:**
- `crates/<crate>/src/<file>.rs:<line>` — <what to change>

**Test to add:**
```rust
#[test]
fn test_<name>() {
    // <exact test code or description>
}
```

**Verify:**
```bash
cargo test -p <crate> -- <test_name> --exact
cargo fmt --all && cargo clippy -p <crate> --tests
```

## Acceptance Criteria

- [ ] <concrete criterion — test passes, metric improves, behavior changes>
- [ ] <second criterion>
- [ ] All existing tests still pass

## Scope

- **Crate(s):** <affected crates>
- **Files:** <file paths>
- **Effort:** EASY (<2h) / MEDIUM (2-8h) / HARD (>8h)
- **Corpus impact:** <N files become clean> (parser issues only)

---
_Filed by scout agent. Builder-ready: no research needed._
ISSUE_EOF
)"
```

## Rules

- ONE issue per distinct finding. Do not bundle.
- Fill in ALL sections. No placeholders. No "TBD" or "needs investigation."
- **Root Cause** must name a specific function and file:line.
- **Builder Spec** must be copy-paste implementable.
- **Test to add** must be actual code, not a description of what to test.
- If you can't fill in the Builder Spec, your investigation was incomplete.
  Go back to scout checklist steps 2-4.
- Label `swarm-discovered` for bugs/improvements, `swarm-architectural`
  for design decisions that need human input.
- After creating the issue, print the URL.
