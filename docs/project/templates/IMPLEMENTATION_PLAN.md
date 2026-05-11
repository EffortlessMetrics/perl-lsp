# Implementation Plan Template

> **Rule**: Indexes provide candidates. Context determines authority.
>
> This template renders structure. The issue spec owns content.
> If the plan and the issue disagree, the issue wins.

Used by spec-planner agents to translate an approved issue into a concrete
ordered plan. Copy into `.spec/<issue-or-lane>/IMPLEMENTATION_PLAN.md` and
fill each section.

---

## 1. Objective

One paragraph. Restate the issue's goal in implementation terms. Reference
the issue number explicitly.

## 2. Branch and worktree

- **Branch**: `<type>/<short-slug>` (e.g. `fix/inc-no-lib-strictness`)
- **Worktree**: `H:\Code\Rust2\wt-<slug>`
- **Base**: `origin/master` at SHA `<short SHA>`

## 3. Files touched

| Path | Change type | Rationale |
|---|---|---|
| `crates/.../foo.rs` | modify | adds the lookup-boundary filter |
| `crates/.../tests/bar.rs` | add | regression test for the filter |

## 4. Sequence

Ordered checklist of steps. Each step should be reversible in isolation.

1. [ ] Step one
2. [ ] Step two
3. [ ] Step three

## 5. Tests added or modified

| Path | Scenario | Expected outcome |
|---|---|---|
| `crates/.../tests/foo.rs::test_name` | description | pass after step 2 |

## 6. Validation matrix

| Gate | Command | Passing criterion |
|---|---|---|
| Unit tests | `cargo test -p <crate>` | all green |
| Workspace lint | `cargo clippy --workspace --lib` | no new warnings |
| PR-fast | `just pr-fast` | exits 0 |

## 7. Status update

- **File**: `docs/project/status/<file>.md`
- **Section**: `## Closeouts` (or the specific subsection)
- **Row content**: one-line summary plus receipt PR number

## 8. Rollback plan

- What reverts cleanly with `git revert <PR-merge-SHA>` and no downstream effects.
- What does **not** revert cleanly (e.g. data migration, schema change, label change) and what the manual fix-up is.
