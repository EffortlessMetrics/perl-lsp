# UX Closeout Spec Template

> **Rule**: Indexes provide candidates. Context determines authority.
>
> This template renders structure. The lane spec owns the authoritative content.
> Status docs and receipt PRs hold the final word — if the spec and a status
> doc disagree, the status doc wins.

Copy this file into the issue or `.spec/<lane>.md` and fill each section.
Sections may be empty if not applicable — say so explicitly rather than
deleting the heading.

---

## 1. User-visible problem

One paragraph. What does the user see today, and why does it hurt? Frame this
in terms of the editor experience, not the internal architecture.

## 2. Current behavior

Observed behavior today, with concrete reproduction steps. Include:

- Editor and configuration assumed
- Exact keystrokes / LSP request the user issues
- Observed response (with snippet if useful)
- Why this is wrong from the user's perspective

## 3. Desired behavior

What should happen after the closeout lands. If the lane touches multiple LSP
consumers, describe each one explicitly:

- **PL701 diagnostic** — desired behavior
- **Completion** — desired behavior
- **Goto-definition** — desired behavior
- **Hover** — desired behavior
- **DAP** — desired behavior (if applicable)

## 4. Non-goals

Explicit exclusions. List items the reader might reasonably expect to be in
scope, then say why they aren't.

## 5. Implementation map

One row per call site that needs to change.

| Call site | Current behavior | Desired behavior | Receipt |
|---|---|---|---|
| `crates/.../path.rs:fn_name` | what it does now | what it should do | PR or test that proves it |

## 6. Acceptance matrix

One row per (consumer, scenario) the closeout must satisfy.

| Consumer | Scenario | Expected | Test path |
|---|---|---|---|
| PL701 | `use Foo;` with no `use lib` | resolves | `crates/.../tests/foo.rs::test_name` |

## 7. Validation

Exact commands to run. Be specific — agents copy-paste from this section.

```bash
cargo test -p <crate> --test <test_file>
just pr-fast
```

## 8. Status update

Which status doc gets the closeout row, and what the row should say:

- **File**: `docs/project/status/<file>.md`
- **Section**: `## Closeouts` (or the specific subsection)
- **Row content**: a one-line summary plus the receipt PR number

## 9. Follow-ups

Issues to file after the closeout lands, with one-line rationale each:

- `tooling(...)`: rationale
- `docs(...)`: rationale
