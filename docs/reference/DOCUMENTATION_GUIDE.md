> For the documentation hub, see [README.md](../README.md).

# Documentation Guide

This project organizes user-facing docs with the [Diátaxis](https://diataxis.fr/) framework:

- **Tutorials** (`docs/tutorials/`): learning-oriented, step-by-step material.
- **How-to guides** (`docs/how-to/`): task-oriented instructions to solve a concrete problem.
- **Reference** (`docs/reference/`): factual, complete lookup documentation.
- **Explanation** (`docs/explanation/`): conceptual context and design rationale.

Use this page to decide where new content belongs and to keep existing docs consistent.

## Quick classification matrix

Use this matrix when you are unsure which section should own a new page:

| Reader intent | Primary question | Correct Diátaxis type | Typical output shape |
| --- | --- | --- | --- |
| Learning | “Can you teach me this from scratch?” | Tutorial | Sequential walkthrough with checkpoints |
| Executing | “How do I do _X_ right now?” | How-to | Short, outcome-driven procedure |
| Looking up | “What exactly does this setting/command mean?” | Reference | Dense, complete, scannable facts |
| Understanding | “Why was this designed this way?” | Explanation | Narrative rationale and tradeoffs |

If the page title starts with “How do I…”, it is usually a how-to.  
If the page title starts with “What is…/Why…”, it is usually explanation.

## Where to put a new document

Ask one question first: **what is the reader trying to do?**

1. **Learn by doing for the first time** → `tutorials/`
2. **Complete a specific task** → `how-to/`
3. **Look up exact behavior, interfaces, or commands** → `reference/`
4. **Understand why the system works this way** → `explanation/`

If a document tries to do more than one of these, split it into multiple pages and cross-link them.

## Split mixed-content pages safely

When a page mixes multiple Diátaxis modes, use this refactor sequence:

1. Keep the original page as the **primary type** only.
2. Move task steps into a how-to or tutorial page.
3. Move API/flag/contract details into reference.
4. Move design history and tradeoffs into explanation.
5. Add “See also” links between all split pages.

This keeps each document easy to scan and lowers maintenance drift.

## Writing rules by doc type

### Tutorials

- Assume minimal prior context.
- Use numbered steps with expected outcomes.
- Keep narrative flow; avoid large API dumps.
- End with “next steps” links into how-to/reference content.

### How-to guides

- Start with a goal statement (e.g., “Set up Neovim for perl-lsp”).
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

## Common anti-patterns

Avoid these recurring issues when editing docs:

- **How-to with long background chapters**: move deep rationale into explanation.
- **Reference with imperative “do this now” tone**: move procedures into how-to.
- **Tutorial that assumes internal architecture knowledge**: add prerequisite links or simplify.
- **Explanation that contains command-by-command setup**: extract those commands to how-to.
- **Single page trying to be all four types**: split and cross-link.

## Documentation hygiene checklist

Before merging doc changes:

- Verify internal links resolve.
- Verify command examples run as written where practical.
- Ensure the page’s style matches its Diátaxis category.
- Update [docs/README.md](../README.md) when adding or removing docs.
- Add at least one reciprocal “See also” link when introducing a new sibling page.

## Current entry points

Start from the documentation hub in [docs/README.md](../README.md):

- Tutorials: [Getting Started](../tutorials/GETTING_STARTED.md)
- How-to: [Installation](../how-to/INSTALLATION.md)
- Reference: [Commands Reference](COMMANDS_REFERENCE.md)
- Explanation: [Pure Rust Parser](../explanation/PURE_RUST_PARSER.md)
