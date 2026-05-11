# Builder Brief Template

> **Rule**: Indexes provide candidates. Context determines authority.
>
> This template renders structure. The issue spec owns content.
> If the brief and the issue disagree, the issue wins.

Used by spec-planner to hand off to a builder agent. Copy into
`.spec/<issue-or-lane>/BUILDER_BRIEF.md` and fill each section.

---

## 1. Goal

One paragraph. What is the builder being asked to do? Reference the issue.

## 2. Branch / worktree

Already set up by spec-planner. The builder checks out and runs from here.

- **Branch**: `<type>/<short-slug>`
- **Worktree**: `H:\Code\Rust2\wt-<slug>`

## 3. Existing artifacts

| File | Role |
|---|---|
| `.spec/<lane>/IMPLEMENTATION_PLAN.md` | the plan to follow |
| `crates/.../tests/red_<name>.rs` | red test from red-tdd; must turn green |
| `tests/fixtures/<name>.pm` | fixture the red test depends on |

## 4. Implementation map

Pointer to the section of the spec or implementation plan the builder follows.
Example: `IMPLEMENTATION_PLAN.md` §5 (Tests) and §3 (Files touched).

## 5. Acceptance

Exact commands the builder must run before pushing.

```bash
cargo test -p <crate> --test <test_file>
cargo clippy --workspace --lib
just pr-fast
```

PR body must include receipts that downstream agents (green-tdd, reviewer,
diff-auditor) will consume.

## 6. Out of scope

Explicit list of work the builder must **not** do. If the builder hits one of
these while implementing, bump back to spec-planner with a comment — don't
expand scope silently.

- Item one (rationale)
- Item two (rationale)

## 7. Receipts to capture

What to include in the PR body so downstream agents don't have to re-research:

- Commands run and their exit status
- New test names and crates
- Status doc updated (which row)
- Any deviation from the implementation plan, with rationale
