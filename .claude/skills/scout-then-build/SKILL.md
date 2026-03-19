---
name: scout-then-build
description: Research-first workflow that scouts exact scope before launching a constrained builder. Use when a feature or fix is too unconstrained for a direct builder — converts ~50% success into ~90% by eliminating implicit decisions.
argument-hint: "<feature or fix description>"
---

# Scout Then Build

Research converts unconstrained work into constrained work. Target: **$ARGUMENTS**

## Step 1: SCOUT

Launch an Explore agent to find the exact implementation surface. The scout
must deliver ALL of the following before any code changes begin:

```
Scout(subagent_type: "Explore", prompt: "
  Find the exact implementation surface for: $ARGUMENTS

  Deliver:
  1. Exact file paths to modify (absolute, from crate root)
  2. Exact function signatures to implement or enhance
  3. Exact import paths needed (use statements, crate dependencies)
  4. Exact test patterns to follow (copy from nearest similar test)
  5. Exact verify command (cargo fmt && cargo clippy -p <crate> --lib && cargo test -p <crate>)
  6. Any open PRs or issues that overlap (check with gh pr list / gh issue list)

  Search crates/ for the target feature. Read CLAUDE.md in relevant crates.
  Do NOT suggest changes — only report what exists and where changes belong.
")
```

### Scout Quality Gate

The scout output is ready when it answers these questions with file paths, not
prose:

- Where does the code live? (exact files)
- What function signatures are involved? (exact signatures)
- What imports are needed? (exact use/mod statements)
- What test file should new tests go in? (exact path + example test)
- What command verifies the change? (exact shell command)

If any answer is vague ("somewhere in the parser"), the scout must refine.

## Step 2: CONSTRAIN

Transform the scout output into a numbered task list. Rules:

1. Each step must invoke a named skill where applicable (`/coding-standards`,
   `/verify`, `/parser-fix`)
2. Maximum 3 implicit decisions per builder — if the builder would need to
   make a judgment call, the constraint step must resolve it
3. Every step must be verifiable — "implement the feature" is not a step;
   "add method `foo(&self) -> Bar` to `crates/x/src/lib.rs:42`" is

### Constraint Template

```markdown
## Builder Spec: <slug>

### Goal
<one sentence>

### Prerequisites
- [ ] No overlapping open PRs (scout verified)
- [ ] Crate CLAUDE.md read

### Steps
1. Create test file `crates/<crate>/tests/<name>.rs` with tests:
   - `test_<case_1>`: <input> -> <expected>
   - `test_<case_2>`: <input> -> <expected>
2. Add/modify function `<signature>` in `<file>:<line>`
   - <what the function should do>
   - Follow pattern from `<similar_function>` in `<similar_file>`
3. Run `/verify <crate>`
4. Commit: `<type>(<scope>): <description>`

### Out of Scope
- <thing that looks related but should be a separate PR>

### Verify
`cargo fmt --all && cargo clippy -p <crate> --lib && cargo test -p <crate>`
```

## Step 3: BUILD

Launch a worktree builder with the constrained spec. The builder follows the
numbered task list, not a prose prompt.

```
Agent(isolation: "worktree", prompt: "
  <paste the Builder Spec from Step 2>

  Follow the numbered steps exactly. If you encounter something not
  covered by the spec, stop and report rather than improvising.

  After verification passes, commit and create a draft PR.
")
```

## When NOT to Use This Skill

- The change is already fully scoped (exact file + function known) — use
  `/parser-fix` or a direct builder instead
- The change is pure documentation or config — no scout needed
- The scout would take longer than just doing the work directly

## Key Principle

Exploration is cheap. Building on wrong assumptions is expensive. Invest
10 minutes of scouting to save 30 minutes of rework.
