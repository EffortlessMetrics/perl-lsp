> For the documentation hub, see [README.md](../README.md).

# Documentation Guide

This project organizes user-facing docs with the [Diátaxis](https://diataxis.fr/) framework:

- **Tutorials** (`docs/tutorials/`): learning-oriented, step-by-step material.
- **How-to guides** (`docs/how-to/`): task-oriented instructions to solve a concrete problem.
- **Reference** (`docs/reference/`): factual, complete lookup documentation.
- **Explanation** (`docs/explanation/`): conceptual context and design rationale.

Use this page to decide where new content belongs and to keep existing docs consistent.

## Quick routing guide

Use these prompts to place content quickly:

| If the reader wants to... | Put the doc in... | Typical shape |
|---|---|---|
| Learn the workflow for the first time | `tutorials/` | Ordered steps with expected results |
| Complete a focused task | `how-to/` | Prerequisites, commands, verification |
| Look up exact behavior or commands | `reference/` | Tables, headings, terse examples |
| Understand tradeoffs or architecture | `explanation/` | Narrative about decisions and constraints |

## Where to put a new document

Ask one question first: **what is the reader trying to do?**

1. **Learn by doing for the first time** → `tutorials/`
2. **Complete a specific task** → `how-to/`
3. **Look up exact behavior, interfaces, or commands** → `reference/`
4. **Understand why the system works this way** → `explanation/`

If a document tries to do more than one of these, split it into multiple pages and cross-link them.

## What each doc type should include

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

- Be precise and scannable with tables, headings, and command blocks.
- Prefer completeness over storytelling.
- Avoid implicit assumptions and hidden defaults.
- Keep examples minimal and behavior-focused.

### Explanation

- Focus on tradeoffs, constraints, and design decisions.
- Connect architecture to user/developer impact.
- Link to reference pages for exact APIs and commands.
- Avoid procedural step lists; move those to tutorials/how-to.

## File naming and structure conventions

- Prefer descriptive uppercase snake case for stable docs already following that convention in `docs/`.
- Keep one primary subject per file.
- Start with a short intro that states the document's purpose.
- Use headings that make sense in isolation when linked directly.
- Add cross-links to the most relevant neighboring docs instead of duplicating content.

## Documentation hygiene checklist

Before merging doc changes:

- Verify internal links resolve.
- Verify command examples run as written where practical.
- Ensure the page’s style matches its Diátaxis category.
- Update [docs/README.md](../README.md) when adding or removing docs.
- Update nearby index pages if the new document changes a common workflow.
- Remove or qualify stale version-specific claims.

## Reviewing an existing doc

When improving an existing page, check for:

1. **Audience drift**: tutorial content inside reference pages, or vice versa.
2. **Staleness**: versions, feature counts, and milestone language that may have changed.
3. **Navigation gaps**: missing “start here,” “see also,” or verification links.
4. **Command ambiguity**: commands that lack working-directory context or success criteria.
5. **Duplication**: content that should be linked from a canonical source instead.

## Current entry points

Start from the documentation hub in [docs/README.md](../README.md):

- Tutorials: [Getting Started](../tutorials/GETTING_STARTED.md)
- How-to: [Installation](../how-to/INSTALLATION.md)
- Reference: [Commands Reference](COMMANDS_REFERENCE.md)
- Explanation: [Pure Rust Parser](../explanation/PURE_RUST_PARSER.md)
- Project status: [Current Status](../project/CURRENT_STATUS.md)
