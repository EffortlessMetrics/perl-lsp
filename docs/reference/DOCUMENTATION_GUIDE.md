> For the documentation hub, see [docs/README.md](../README.md).

# Documentation Guide (Diátaxis)

This repository organizes user-facing documentation using the [Diátaxis framework](https://diataxis.fr/):

- **Tutorials**: learning-oriented, step-by-step guides.
- **How-to guides**: task-oriented instructions for a specific outcome.
- **Reference**: factual lookup material and specifications.
- **Explanation**: conceptual background and design rationale.

## Where content belongs

### 1) Tutorials (`docs/tutorials/`)
Use tutorials when a reader needs guided practice.

Examples:
- [Getting Started](../tutorials/GETTING_STARTED.md)
- [LSP Development Guide](../tutorials/LSP_DEVELOPMENT_GUIDE.md)
- [Execute Command Tutorial](../tutorials/EXECUTE_COMMAND_TUTORIAL.md)

### 2) How-to guides (`docs/how-to/`)
Use how-to guides when a reader already knows what they want to accomplish.

Examples:
- [Installation](../how-to/INSTALLATION.md)
- [Editor Setup](../how-to/EDITOR_SETUP.md)
- [Troubleshooting](../how-to/TROUBLESHOOTING.md)

### 3) Reference (`docs/reference/`)
Use reference docs for contracts, schemas, command catalogs, and exact behavior.

Examples:
- [Commands Reference](COMMANDS_REFERENCE.md)
- [LSP Features](LSP_FEATURES.md)
- [Stability Policy](STABILITY.md)
- [Configuration](CONFIG.md)

### 4) Explanation (`docs/explanation/`)
Use explanation docs to describe design decisions and trade-offs.

Examples:
- [Pure Rust Parser](../explanation/PURE_RUST_PARSER.md)
- [Error Handling Strategy](../explanation/ERROR_HANDLING_STRATEGY.md)
- [Slash Disambiguation](../explanation/SLASH_DISAMBIGUATION.md)

## Writing rules

When adding or updating documentation:

1. **Choose one primary Diátaxis type** for the page.
2. **Keep intent pure**:
   - Tutorials teach by doing.
   - How-to guides solve one concrete problem.
   - Reference pages avoid narrative and opinion.
   - Explanations focus on why, trade-offs, and context.
3. **Cross-link instead of mixing styles** (for example, a how-to can link to reference for exact flags).
4. **Use descriptive titles** that match user intent (e.g. “How to debug flaky LSP tests”).

## Quick placement checklist

If the answer to the question is mainly...

- “**Teach me**” → `docs/tutorials/`
- “**How do I**” → `docs/how-to/`
- “**What is / what are the exact details**” → `docs/reference/`
- “**Why is it designed this way**” → `docs/explanation/`

## Maintenance

- Keep [docs/README.md](../README.md) aligned with the main, high-value pages in each Diátaxis category.
- Prefer stable links and avoid linking to non-existent directories.
- When moving docs between categories, update inbound links in `docs/README.md` and relevant section indexes.
