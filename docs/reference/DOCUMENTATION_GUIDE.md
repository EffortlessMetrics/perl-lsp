> For the documentation hub, see [README.md](../README.md).

# Documentation Guide

This project organizes user-facing docs with the [Diátaxis](https://diataxis.fr/) framework:

- **Tutorials** (`docs/tutorials/`): learning-oriented, step-by-step material.
- **How-to guides** (`docs/how-to/`): task-oriented instructions to solve a concrete problem.
- **Reference** (`docs/reference/`): factual, complete lookup documentation.
- **Explanation** (`docs/explanation/`): conceptual context and design rationale.

Use this page to decide where new content belongs, keep existing docs consistent, and split mixed-purpose pages before they become hard to maintain.

## Quick decision table

| Reader intent | Diátaxis type | Primary question | Typical shape |
|---|---|---|---|
| Learning for the first time | Tutorial | “Can you teach me this?” | Ordered walkthrough with expected outcomes |
| Completing a specific task | How-to | “How do I do X right now?” | Short procedure with prerequisites and verification |
| Looking up exact facts | Reference | “What does this do?” | Tables, definitions, command lists, schemas |
| Building a mental model | Explanation | “Why is it designed this way?” | Narrative about tradeoffs, constraints, and rationale |

## Where to put a new document

Ask one question first: **what is the reader trying to do?**

1. **Learn by doing for the first time** → `tutorials/`
2. **Complete a specific task** → `how-to/`
3. **Look up exact behavior, interfaces, or commands** → `reference/`
4. **Understand why the system works this way** → `explanation/`

If a document tries to do more than one of these, split it into multiple pages and cross-link them.

## A simple classification test

Use the “dominant sentence” test before writing or editing:

- If the page mostly says **“first, then, next”**, it is probably a **tutorial**.
- If the page mostly says **“to achieve X, do Y”**, it is probably a **how-to**.
- If the page mostly says **“here are the facts/options/fields”**, it is probably **reference**.
- If the page mostly says **“this exists because…”**, it is probably **explanation**.

When a page fails this test, do not keep adding sections. Split it.

## Writing rules by doc type

### Tutorials

**Use when:** the reader is new and needs a guided success path.

- Assume minimal prior context.
- Use numbered steps with expected outcomes.
- Keep narrative flow; avoid large API dumps.
- End with “next steps” links into how-to/reference content.

**Good tutorial moves**

- Introduce only the concepts needed for the next step.
- Include one happy-path verification after each major stage.
- Name exactly what the reader should see or run next.

**Avoid**

- Exhaustive option listings.
- Branch-heavy troubleshooting.
- Deep architectural digressions.

### How-to guides

**Use when:** the reader already knows enough and wants to solve one problem.

- Start with a goal statement (e.g., “Set up Neovim for perl-lsp”).
- Provide prerequisites.
- Prefer concise command sequences and verification checks.
- Keep conceptual background short; link to explanation docs instead.

**Good how-to moves**

- Optimize for copy-pasteable commands.
- Make success/failure states obvious.
- Keep scope narrow enough to answer one operational question.

**Avoid**

- Introductory teaching material.
- Full API inventories.
- Historical rationale beyond a sentence or two.

### Reference

**Use when:** accuracy, coverage, and scanability matter more than flow.

- Be precise and scannable (tables, headings, command blocks).
- Prefer completeness over storytelling.
- Avoid implicit assumptions and hidden defaults.
- Keep examples minimal and behavior-focused.

**Good reference moves**

- Use stable section names and predictable ordering.
- Define defaults, edge cases, and constraints explicitly.
- Cross-link to tutorials/how-to pages for applied usage.

**Avoid**

- Long introductions.
- Advice that depends on reader experience level.
- Step-by-step narratives unless describing a protocol sequence.

### Explanation

**Use when:** the reader needs the model behind a design or policy.

- Focus on tradeoffs, constraints, and design decisions.
- Connect architecture to user/developer impact.
- Link to reference pages for exact APIs and commands.
- Avoid procedural step lists (move those to tutorials/how-to).

**Good explanation moves**

- Compare alternatives and explain why one was chosen.
- Describe consequences, not just decisions.
- Connect local implementation details to larger project goals.

**Avoid**

- Setup checklists.
- Option-by-option API listings.
- Troubleshooting branches.

## Common anti-patterns

### “Mega page” smell

A page tries to teach, troubleshoot, explain internals, and list every command. Split it into:

- a tutorial for onboarding,
- a how-to for common operations,
- a reference page for exact commands or schemas, and
- an explanation page for rationale.

### FAQ creep into reference

If a reference page accumulates advisory prose like “you probably want...”, move that content into a how-to or FAQ.

### Hidden explanation in how-to guides

If a task guide starts spending multiple sections on tradeoffs or architecture, move that material into `docs/explanation/` and link to it.

### Troubleshooting inside tutorials

Tutorials should protect momentum. Put complex recovery paths in `docs/how-to/TROUBLESHOOTING.md` and link out.

## Recommended page skeletons

### Tutorial skeleton

```text
# Title

## What you will build or learn
## Prerequisites
## Step 1
## Step 2
## Verify the result
## Next steps
```

### How-to skeleton

```text
# Title

## Goal
## Prerequisites
## Steps
## Verify
## Related docs
```

### Reference skeleton

```text
# Title

## Overview
## Commands / Fields / Behavior
## Defaults and constraints
## Related docs
```

### Explanation skeleton

```text
# Title

## Context
## Problem
## Tradeoffs
## Decision or model
## Consequences
## Related docs
```

## Cross-linking rules

Use cross-links to preserve single-purpose pages:

- **Tutorial → How-to/Reference** for next steps and exact lookups.
- **How-to → Reference** for command flags, schemas, and edge cases.
- **How-to → Explanation** for rationale that would otherwise bloat the guide.
- **Explanation → Reference** for exact APIs, commands, and guarantees.
- **Reference → Tutorial/How-to** when readers are likely to need a guided path.

## Documentation hygiene checklist

Before merging doc changes:

- Verify internal links resolve.
- Verify command examples run as written where practical.
- Ensure the page’s style matches its Diátaxis category.
- Update [docs/README.md](../README.md) when adding or removing docs.
- Rename or move misclassified pages instead of patching over structural problems.

## Review checklist for doc PRs

Ask these questions during review:

1. Can I identify the document type in the first screenful?
2. Does every major section serve the same reader intent?
3. Are conceptual digressions linked out instead of embedded?
4. Are exact commands, flags, or schemas located in reference docs?
5. Does the page point readers to the next appropriate doc type?

If any answer is “no,” the page probably needs to be split or reframed.

## Current entry points

Start from the documentation hub in [docs/README.md](../README.md):

- Tutorials: [Getting Started](../tutorials/GETTING_STARTED.md)
- How-to: [Installation](../how-to/INSTALLATION.md)
- Reference: [Commands Reference](COMMANDS_REFERENCE.md)
- Explanation: [Pure Rust Parser](../explanation/PURE_RUST_PARSER.md)
