> For the documentation hub, see [README.md](../README.md).

# Documentation Guide

This project organizes user-facing documentation with the [Diátaxis](https://diataxis.fr/) framework:

- **Tutorials** (`docs/tutorials/`): learning-oriented, step-by-step material.
- **How-to guides** (`docs/how-to/`): task-oriented instructions to solve a concrete problem.
- **Reference** (`docs/reference/`): factual, complete lookup documentation.
- **Explanation** (`docs/explanation/`): conceptual context, tradeoffs, and design rationale.

Use this page to decide where new content belongs, keep existing docs consistent, and avoid mixing multiple reader intents into a single page.

## The first question to ask

Before writing, ask: **what does the reader need from this page right now?**

| Reader need | Best doc type | Typical opening |
|---|---|---|
| “Teach me this from the beginning.” | Tutorial | A guided scenario with sequential steps |
| “Help me complete a task.” | How-to | A goal statement and prerequisites |
| “Tell me the exact behavior.” | Reference | A definition, table, schema, or API surface |
| “Help me understand the rationale.” | Explanation | A design problem, constraint, or tradeoff |

If the page answers more than one of those needs equally, split it into separate pages and cross-link them.

## Decision tree for new docs

1. **Is the reader expected to learn while following the page?**
   - Yes → write a **tutorial**.
2. **Is the reader trying to finish a known task?**
   - Yes → write a **how-to guide**.
3. **Is the reader mainly verifying facts, interfaces, defaults, or commands?**
   - Yes → write **reference** material.
4. **Is the reader trying to understand why the system behaves this way?**
   - Yes → write an **explanation**.

When none of the above feels dominant, the page is probably trying to do too much.

## Writing rules by doc type

### Tutorials

**Purpose**: Help a newcomer succeed through guided practice.

Use tutorials when the reader benefits from a sequence, context, and expected results.

**Do**

- Assume minimal prior context.
- Use numbered steps with expected outcomes.
- Keep a single learning path from start to finish.
- End with next steps into how-to and reference material.

**Avoid**

- Large API dumps.
- Comprehensive edge-case coverage.
- Branching into multiple alternative workflows unless the comparison is itself the lesson.

### How-to guides

**Purpose**: Help a reader solve a specific problem quickly.

Use how-to guides when the reader already understands the general area and needs practical instructions.

**Do**

- Start with a goal statement such as “Set up Neovim for perl-lsp”.
- Include prerequisites and constraints.
- Prefer concise command sequences and verification checks.
- Keep conceptual background short and link to explanation docs instead.

**Avoid**

- Long conceptual introductions.
- Teaching the entire subsystem from scratch.
- Reference-style exhaustiveness.

### Reference

**Purpose**: Provide exact information for lookup.

Use reference pages for command syntax, config fields, schemas, policies, capability catalogs, and API contracts.

**Do**

- Be precise and scannable with tables, headings, and focused code blocks.
- Prefer completeness over narrative flow.
- State defaults, invariants, and edge behavior explicitly.
- Keep examples minimal and behavior-focused.

**Avoid**

- Storytelling or motivational prose.
- Long procedural walkthroughs.
- Hidden assumptions.

### Explanation

**Purpose**: Build understanding.

Use explanation pages for architecture, tradeoffs, historical choices, and conceptual models.

**Do**

- Focus on constraints, tradeoffs, and design decisions.
- Connect architecture to user or developer impact.
- Link to reference pages for exact APIs and commands.
- Help readers build mental models.

**Avoid**

- Checklists that primarily exist to complete a task.
- Numbered procedures better suited for tutorials or how-to guides.
- Exhaustive API documentation.

## Common anti-patterns

These are the most common ways docs drift out of Diátaxis alignment:

- **Tutorials turning into reference**: a guided page starts accumulating every option and exception.
- **How-to guides turning into explanation**: task pages open with long architecture essays.
- **Reference turning into tutorial**: lookup pages embed end-to-end walkthroughs.
- **Explanation turning into operations runbooks**: conceptual pages become procedural incident docs.

When this happens, keep the current page focused on its primary job and move the overflow into a separate, linked page.

## Cross-linking rules

Good Diátaxis documentation is connected, not isolated.

- Tutorials should link to how-to or reference pages for deeper follow-up.
- How-to guides should link to explanation pages for rationale and reference pages for exact syntax.
- Reference pages should link outward sparingly to the most relevant tutorial or how-to for practical application.
- Explanation pages should link to reference pages whenever readers may need exact details next.

A useful pattern is: **one page, one job, many links**.

## Recommended page shapes

Use these lightweight templates when creating or refactoring a page.

### Tutorial shape

1. Goal and what you will build or prove
2. Prerequisites
3. Numbered steps
4. Verification or expected outcome
5. Next steps

### How-to shape

1. Goal
2. Prerequisites or environment assumptions
3. Procedure
4. Verification
5. Troubleshooting or related links

### Reference shape

1. Scope
2. Definitions, commands, schema, or tables
3. Examples of exact behavior
4. Constraints, defaults, edge cases
5. Related docs

### Explanation shape

1. Problem or design context
2. Constraints and tradeoffs
3. Chosen approach
4. Implications
5. Related reference/how-to material

## Repository-specific guidance

In this repository, the docs hub at [docs/README.md](../README.md) is the main entry point for user-facing content.

- Add new top-level entry-point docs to the relevant section of `docs/README.md`.
- Prefer placing operational guidance in `docs/how-to/`.
- Prefer placing architecture rationale in `docs/explanation/`.
- Keep specs, governance, and exact interfaces in `docs/reference/` unless they are explicitly project planning artifacts.
- Use `docs/project/` for project status, governance, roadmap, and process health rather than end-user product docs.

## Documentation hygiene checklist

Before merging documentation changes:

- Verify internal links resolve.
- Verify command examples run as written where practical.
- Ensure the page’s style matches its Diátaxis category.
- Update [docs/README.md](../README.md) when adding or removing entry-point docs.
- Check whether an oversized page should be split instead of expanded.

## Refactoring an existing page

When improving an existing doc, use this sequence:

1. Identify the page’s **primary reader intent**.
2. Remove or relocate sections that serve a different intent.
3. Add links to the new destination pages instead of duplicating content.
4. Normalize the opening so the purpose is obvious within the first screenful.
5. Update the docs hub if the page becomes a recommended entry point.

## Current entry points

Start from the documentation hub in [docs/README.md](../README.md):

- Tutorials: [Getting Started](../tutorials/GETTING_STARTED.md)
- How-to: [Installation](../how-to/INSTALLATION.md)
- Reference: [Commands Reference](COMMANDS_REFERENCE.md)
- Explanation: [Pure Rust Parser](../explanation/PURE_RUST_PARSER.md)
