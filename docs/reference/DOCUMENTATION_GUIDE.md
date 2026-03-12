> For the documentation hub, see [README.md](../README.md). This page defines how perl-lsp uses the Diátaxis framework.

# Documentation Guide

This repository organizes user-facing docs with [Diátaxis](https://diataxis.fr/):

1. **Tutorials** — learning-oriented, end-to-end walkthroughs
2. **How-to guides** — task-oriented instructions for specific outcomes
3. **Reference** — precise lookup material (APIs, specs, command catalogs)
4. **Explanation** — architecture, rationale, and trade-offs

Use this guide to decide **where new docs belong** and to keep existing docs consistent.

## Classification Rules

### 1) Tutorials (`docs/tutorials/`)
Use tutorials when the reader is learning by following a sequence.

**Should include:**
- Prerequisites and expected outcome
- Ordered steps with verification points
- Minimal branching and no exhaustive option matrix

**Examples:**
- [GETTING_STARTED.md](../tutorials/GETTING_STARTED.md)
- [EXECUTE_COMMAND_TUTORIAL.md](../tutorials/EXECUTE_COMMAND_TUTORIAL.md)
- [LSP_DEVELOPMENT_GUIDE.md](../tutorials/LSP_DEVELOPMENT_GUIDE.md)

### 2) How-to guides (`docs/how-to/`)
Use how-to docs when the reader already knows the system and wants one concrete result.

**Should include:**
- Goal-first framing ("How to X")
- Focused commands/config snippets
- Troubleshooting for that specific task only

**Examples:**
- [INSTALLATION.md](../how-to/INSTALLATION.md)
- [EDITOR_SETUP.md](../how-to/EDITOR_SETUP.md)
- [THREADING_CONFIGURATION_GUIDE.md](../how-to/THREADING_CONFIGURATION_GUIDE.md)

### 3) Reference (`docs/reference/`)
Use reference docs for facts readers need to look up quickly.

**Should include:**
- Stable structure and headings
- Complete parameter/behavior detail
- No narrative teaching flow

**Examples:**
- [COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)
- [CONFIG.md](CONFIG.md)
- [LSP_FEATURES.md](LSP_FEATURES.md)
- [STABILITY.md](STABILITY.md)

### 4) Explanation (`docs/explanation/`)
Use explanation docs for design intent, historical context, and architectural trade-offs.

**Should include:**
- Why this design exists
- Alternatives considered or rejected
- Conceptual diagrams/mental models where useful

**Examples:**
- [PURE_RUST_PARSER.md](../explanation/PURE_RUST_PARSER.md)
- [ERROR_HANDLING_STRATEGY.md](../explanation/ERROR_HANDLING_STRATEGY.md)
- [CANCELLATION_ARCHITECTURE_GUIDE.md](../explanation/CANCELLATION_ARCHITECTURE_GUIDE.md)

## Quick Placement Checklist

Before opening a docs PR, confirm:

- If the doc teaches from zero with sequential steps → `tutorials/`.
- If it solves one practical task for experienced users → `how-to/`.
- If it is a catalog/spec/contract readers consult repeatedly → `reference/`.
- If it explains architectural reasoning and trade-offs → `explanation/`.

If content spans multiple categories, split it instead of mixing styles.

## Writing Standards

- Keep command examples runnable in this workspace (`cargo ...`, `just ...`).
- Prefer concise sections and descriptive headings.
- Link laterally to companion docs in other Diátaxis categories.
- Avoid duplicating metrics/version numbers outside canonical status docs.

## Maintenance Workflow

When adding or changing a feature:

1. Update at least one **reference** doc for factual behavior.
2. Add or adjust a **how-to** if the change impacts operator workflow.
3. Add a **tutorial** only when there is new learning flow or onboarding value.
4. Add an **explanation** when architectural intent changed.
5. Update [docs/README.md](../README.md) if navigation changes.

## Related Documentation

- Documentation hub: [docs/README.md](../README.md)
- Command catalog: [COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)
- API doc standards: [API_DOCUMENTATION_STANDARDS.md](API_DOCUMENTATION_STANDARDS.md)
- Contribution workflow: [CONTRIBUTING.md](../../CONTRIBUTING.md)
