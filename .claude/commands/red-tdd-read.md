---
description: Red TDD builder step 1 — read the issue, spec-planner checklist, and existing test patterns
user-invocable: false
---

# Red TDD: Read

Read the issue, the spec-planner's checklist, and the target crate's
existing test patterns.

## Steps

1. Read the issue and comments:
   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Check out the implementation branch (created by spec-planner):
   ```bash
   git fetch origin
   git checkout impl/<issue#>-<specslug>
   ```

3. Read the spec files:
   ```bash
   cat .spec/<issue#>-<specslug>/checklist.md
   cat .spec/<issue#>-<specslug>/acceptance.md
   ```

4. Read existing tests in the target crate to understand patterns:
   - What test framework? (inline `#[cfg(test)]` or `tests/` directory?)
   - What helpers are used? (`LspHarness`, `MockSubprocessRuntime`, `tempfile`, etc.)
   - What import patterns? (`use perl_tdd_support::must;`, `use insta::assert_snapshot;`, etc.)
   - How are test functions named?

5. Identify from acceptance.md:
   - Each criterion that needs a test
   - Edge cases mentioned in oppositional/plan-review comments
   - The exact assertions that define "done"
