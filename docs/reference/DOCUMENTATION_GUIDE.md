> For the documentation hub, see [docs/README.md](../README.md).

# Documentation Guide

This repository uses the [Diátaxis](https://diataxis.fr/) framework to keep documentation predictable for readers and maintainable for contributors.

## The four documentation modes

| Mode | Reader mindset | Primary question | Typical content | Avoid |
|------|----------------|------------------|-----------------|-------|
| **Tutorial** | Learning | “Can you teach me?” | Guided steps, checkpoints, expected outcomes | Exhaustive option lists |
| **How-to** | Doing | “How do I accomplish X?” | Short procedure, prerequisites, verification | Long conceptual detours |
| **Reference** | Looking up | “What exactly does it do?” | Facts, defaults, commands, schemas, tables | Narrative storytelling |
| **Explanation** | Understanding | “Why is it designed this way?” | Rationale, tradeoffs, architecture, history | Step-by-step instructions |

## Choose the right home for a page

Ask one question first: **what does the reader need at the moment they open this page?**

1. **A guided first experience** → `docs/tutorials/`
2. **A direct path to a specific outcome** → `docs/how-to/`
3. **Precise facts or complete behavior** → `docs/reference/`
4. **Context, rationale, or tradeoffs** → `docs/explanation/`

If a draft tries to do more than one of these jobs, split it into multiple pages and connect them with links.

## A simple decision test

Use this quick triage before creating or moving a document:

- If the reader should follow the page from top to bottom, it is usually a **tutorial**.
- If the reader will likely arrive from a search query like “how do I…”, it is usually a **how-to**.
- If the reader may scan headings, tables, or command snippets for one exact fact, it is usually **reference**.
- If the page answers “why not the alternative?” or “what constraint shaped this?”, it is usually **explanation**.

## Writing rules by doc type

### Tutorials

**Purpose:** help a newcomer succeed for the first time.

**Do:**

- Assume minimal prior context.
- Use numbered steps in a meaningful sequence.
- Include checkpoints or expected outcomes after important steps.
- Keep the reader moving toward a concrete end state.
- End with “next steps” links to relevant how-to and reference pages.

**Do not:**

- Dump every configuration option.
- Branch heavily for many variants unless the tutorial remains readable.
- Turn the page into a maintenance checklist.

### How-to guides

**Purpose:** help a reader complete a known task efficiently.

**Do:**

- Start with a single goal statement.
- List prerequisites and assumptions up front.
- Prefer concise command sequences and short explanations.
- Include a verification step when possible.
- Link to explanation docs for background and reference docs for exact details.

**Do not:**

- Re-teach the full product from scratch.
- Mix multiple unrelated goals into one page.
- Hide required prerequisites in the middle of the procedure.

### Reference

**Purpose:** provide authoritative facts for lookup.

**Do:**

- Optimize for scanning with headings, tables, and short focused examples.
- Record defaults, edge cases, interfaces, and exact command syntax.
- Be explicit about assumptions, limits, and version-sensitive behavior.
- Keep wording precise and stable.

**Do not:**

- Tell a long story.
- Rely on implied context or unstated defaults.
- Use a reference page as a tutorial substitute.

### Explanation

**Purpose:** help readers build a correct mental model.

**Do:**

- Focus on design choices, tradeoffs, constraints, and architecture.
- Compare alternatives when that clarifies why the current design exists.
- Connect implementation decisions to user or contributor impact.
- Link to reference pages for exact APIs and commands.

**Do not:**

- Present step-by-step task flows as the main content.
- Bury the key rationale under procedural detail.
- Duplicate long reference sections.

## Cross-linking rules

Good Diátaxis docs are connected, not isolated.

- Tutorials should link forward to relevant how-to and reference pages.
- How-to guides should link sideways to explanation for rationale and to reference for exact syntax.
- Reference pages should link outward to tutorials or how-to guides only when a task-oriented next step helps.
- Explanation pages should link to reference pages for exact interfaces and to tutorials/how-to guides when readers may want to apply the idea.

## Repository-specific expectations

For this repository:

- Update [docs/README.md](../README.md) when you add a new major entry point, rename a page, or move a page between categories.
- Prefer pointing volatile facts such as metrics, counts, and status to [project/CURRENT_STATUS.md](../project/CURRENT_STATUS.md) instead of duplicating numbers.
- Keep editor-specific setup primarily in `docs/EDITORS/` and link to it from task-oriented pages in `docs/how-to/`.
- Treat ADRs in `docs/adr/` as decision records, not substitutes for tutorials or reference pages.

## Review checklist for doc changes

Before merging documentation work, verify that:

- The page clearly fits one Diátaxis mode.
- The title and opening paragraphs match the page's job.
- Internal links resolve.
- Command examples are still valid where practical.
- The page links to adjacent material instead of absorbing unrelated content.
- `docs/README.md` still reflects the best entry points.

## Examples from this repository

- **Tutorial:** [Getting Started](../tutorials/GETTING_STARTED.md)
- **How-to:** [Installation](../how-to/INSTALLATION.md)
- **Reference:** [Commands Reference](COMMANDS_REFERENCE.md)
- **Explanation:** [Pure Rust Parser](../explanation/PURE_RUST_PARSER.md)

When in doubt, optimize for reader intent, keep each page narrow in purpose, and split mixed content into linked pages.
