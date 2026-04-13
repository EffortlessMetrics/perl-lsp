---
description: Canonical issue templates for scout agents filing GitHub issues
---

# Scout Issue Templates

Canonical templates for scout agents filing GitHub issues.
All scout commands MUST use these templates — do NOT hand-roll `gh issue create` bodies.

---

## Full Scout Report

Use when: a scout has completed a full investigation and has root cause, options, and a builder spec.

```bash
gh issue create \
  --title "$TITLE" \
  --label "swarm-discovered" \
  --label "needs-plan-review" \
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
\`\`\`rust
#[test]
fn test_<name>() {
    // <exact test code or description>
}
\`\`\`

**Verify:**
\`\`\`bash
cargo test -p <crate> -- <test_name> --exact
cargo xtask fmt && cargo clippy -p <crate> --tests
\`\`\`

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

### Rules

- ONE issue per distinct finding. Do not bundle.
- Fill in ALL sections. No placeholders. No "TBD" or "needs investigation."
- **Root Cause** must name a specific function and file:line.
- **Builder Spec** must be copy-paste implementable.
- **Test to add** must be actual code, not a description of what to test.
- If you can't fill in the Builder Spec completely, **fill in what you can and note your uncertainty.** A plan-reviewer will verify and improve.
- Label `swarm-discovered` for bugs/improvements, `swarm-architectural` for design decisions that need human input.
- After creating the issue, print the URL.

---

## Discovery Issue (Lightweight)

Use when: an agent discovers something incidentally during other work and it's not a full investigation.

```bash
gh issue create \
  --title "$TITLE" \
  --label "swarm-discovered" \
  --body "$(cat <<'ISSUE_EOF'
_Discovered by <agent-type> while working on <branch>._

## Context

<what you found, why it matters — enough that no one re-investigates>

## Files

- `<path>:<line>` — <why this file matters>

## Suggested Approach

<if you have one — optional>
ISSUE_EOF
)"
```

---

## Domain-Specific Sections

Domain scout commands add these as subsections within the canonical Full Scout Report structure:

### Parser Scout — SLICE Definition

Append after **Root Cause** in the Problem section:

```
**SLICE Definition:**
- `error_bucket`: which category
- `perl_construct`: the minimal triggering code
- `root_cause_files`: parser source files involved
- `files_touched`: files that would need changes
- `estimated_complexity`: low/medium/high
```

### DAP Scout — DAP Metadata

Append after **Root Cause**:

```
**DAP Metadata:**
- `crate`: which DAP crate
- `current_test_count`: number of existing tests
- `loc`: lines of code
- `suggested_tests`: what to test
- `related_issues`: linked GitHub issues
```

### Security Scout — Severity Ranking

Add as a section immediately after **Problem**:

```
## Severity

- **[critical/high/medium/low]** <finding name> — <one-line justification>
```

Include one severity line per finding if the issue covers multiple items.
