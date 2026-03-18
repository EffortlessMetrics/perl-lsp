> For the documentation hub, see [README.md](../README.md).

# Documentation Guide

This project organizes user-facing docs with the [Diátaxis](https://diataxis.fr/) framework:

- **Tutorials** (`docs/tutorials/`): learning-oriented, step-by-step material.
- **How-to guides** (`docs/how-to/`): task-oriented instructions to solve a concrete problem.
- **Reference** (`docs/reference/`): factual, complete lookup documentation.
- **Explanation** (`docs/explanation/`): conceptual context and design rationale.

Use this page to decide where new content belongs, keep existing docs consistent, and reduce category drift over time.

## Fast classification test

Ask one question first: **what is the reader trying to do right now?**

1. **Learn by doing for the first time** → `tutorials/`
2. **Complete a specific task** → `how-to/`
3. **Look up exact behavior, interfaces, or commands** → `reference/`
4. **Understand why the system works this way** → `explanation/`

If a document tries to do more than one of these jobs, split it into multiple pages and cross-link them.

## Decision table

| Reader need | Best category | Typical shape | Should end with |
|---|---|---|---|
| “Teach me this from the beginning.” | Tutorial | Sequential steps with checkpoints | Next steps into how-to/reference |
| “Help me achieve one concrete outcome.” | How-to | Short prerequisite list and focused procedure | Verification and related tasks |
| “Tell me the exact behavior.” | Reference | Scannable sections, tables, schemas, constraints | Links to explanatory context |
| “Help me understand the tradeoffs.” | Explanation | Narrative reasoning, alternatives, architecture | Links to reference or implementation details |

## Signs a page is in the wrong category

### Tutorial drift

A tutorial probably needs splitting if it:

- Reads like a command catalog instead of a guided lesson.
- Assumes prior project knowledge without introducing it.
- Stops teaching and starts exhaustively documenting edge cases.

### How-to drift

A how-to guide probably needs splitting if it:

- Spends more space on system history than on the task.
- Covers several unrelated goals in one page.
- Cannot be followed independently because key prerequisites are missing.

### Reference drift

A reference page probably needs splitting if it:

- Contains long “why we chose this” sections.
- Walks the reader through a full learning journey.
- Hides defaults, constraints, or variants inside prose.

### Explanation drift

An explanation page probably needs splitting if it:

- Contains numbered setup instructions.
- Looks like a checklist for a task.
- Tries to be the source of truth for command syntax or API details.

## Writing rules by doc type

### Tutorials

- Assume minimal prior context.
- Use numbered steps with expected outcomes.
- Keep narrative flow; avoid large API dumps.
- End with “next steps” links into how-to/reference content.

### How-to guides

- Start with a goal statement (for example, “Set up Neovim for perl-lsp”).
- Provide prerequisites.
- Prefer concise command sequences and verification checks.
- Keep conceptual background short; link to explanation docs instead.

### Reference

- Be precise and scannable (tables, headings, command blocks).
- Prefer completeness over storytelling.
- Avoid implicit assumptions and hidden defaults.
- Keep examples minimal and behavior-focused.

### Explanation

- Focus on tradeoffs, constraints, and design decisions.
- Connect architecture to user/developer impact.
- Link to reference pages for exact APIs and commands.
- Avoid procedural step lists (move those to tutorials/how-to).

## Recommended page template by category

### Tutorial template

1. Goal
2. Prerequisites
3. Step 1, Step 2, Step 3…
4. Verification checkpoints
5. Next steps

### How-to template

1. Goal
2. Prerequisites
3. Procedure
4. Verify the result
5. Troubleshooting or adjacent tasks

### Reference template

1. Scope
2. Commands / schema / API surface
3. Defaults and constraints
4. Examples
5. Related explanation or how-to links

### Explanation template

1. Problem or tension
2. Design choice
3. Tradeoffs and alternatives
4. Operational impact
5. Links to reference and implementation docs

## What belongs outside the four core categories

Not every repository document is a Diátaxis page. These usually live elsewhere:

- **ADRs** in `docs/adr/` for durable architectural decisions.
- **Project status and governance** in `docs/project/`.
- **Specs** in `docs/specs/` when a behavior is being proposed or negotiated.
- **Archive material** in `docs/archive/` for historical records.

When a non-Diátaxis document starts serving end users directly, add or update a Diátaxis page that points to it.

## Documentation hygiene checklist

Before merging doc changes:

- Verify internal links resolve.
- Verify command examples run as written where practical.
- Ensure the page’s style matches its Diátaxis category.
- Update [docs/README.md](../README.md) when adding or removing docs.
- Add cross-links when splitting mixed-purpose content.

## Applying the guide in this repository

Use these pages as reference examples:

- **Tutorial**: [Getting Started](../tutorials/GETTING_STARTED.md)
- **How-to**: [Installation](../how-to/INSTALLATION.md)
- **Reference**: [Commands Reference](COMMANDS_REFERENCE.md)
- **Explanation**: [Pure Rust Parser](../explanation/PURE_RUST_PARSER.md)

Use these directories for non-Diátaxis repository material:

- **Governance and status**: `docs/project/`
- **Decision records**: `docs/adr/`
- **Specifications**: `docs/specs/`
- **Historical records**: `docs/archive/`

## Editing strategy for mixed pages

When you find a page that mixes categories, prefer this sequence:

1. Identify the page's primary reader need.
2. Keep that material in place.
3. Move the off-category material into a new page in the correct directory.
4. Replace the removed content with a short summary and link.
5. Update [docs/README.md](../README.md) if the new page is a user-facing entry point.

That approach preserves links while gradually improving Diátaxis alignment.
