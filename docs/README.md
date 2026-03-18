# Perl LSP Documentation

This directory is the main documentation hub for `perl-lsp`.

The user-facing documentation is organized with the [Diátaxis](https://diataxis.fr/) framework so readers can quickly choose the right kind of page for what they need right now:

| Need | Start here | Expect |
|------|------------|--------|
| I am new and want a guided path | [`tutorials/`](tutorials/) | Sequential learning, examples, and checkpoints |
| I need to complete a task | [`how-to/`](how-to/) | Short, goal-oriented instructions |
| I need exact facts or commands | [`reference/`](reference/) | Lookup tables, schemas, and authoritative details |
| I need to understand design choices | [`explanation/`](explanation/) | Rationale, tradeoffs, and architecture context |

If you are unsure where to begin, open the [Documentation Guide](reference/DOCUMENTATION_GUIDE.md) for the repository's Diátaxis rules and writing standards.

## Recommended entry points

### Start here if you are a user

- [Getting Started](tutorials/GETTING_STARTED.md) — install `perl-lsp`, connect an editor, and verify the setup.
- [DAP User Guide](tutorials/DAP_USER_GUIDE.md) — configure debugging with the DAP bridge.
- [Editor Setup](how-to/EDITOR_SETUP.md) — jump directly to editor-specific configuration.
- [Troubleshooting](how-to/TROUBLESHOOTING.md) — resolve common installation and runtime issues.

### Start here if you are contributing

- [Contributing Guide](../CONTRIBUTING.md) — repository workflow and expectations.
- [Commands Reference](reference/COMMANDS_REFERENCE.md) — canonical build, test, lint, and CI commands.
- [Crate Architecture Guide](reference/CRATE_ARCHITECTURE_GUIDE.md) — workspace structure and crate families.
- [LSP Implementation Guide](reference/LSP_IMPLEMENTATION_GUIDE.md) — feature architecture and protocol behavior.
- [Current Status](project/CURRENT_STATUS.md) — computed project metrics and health signals.

## Documentation by Diátaxis category

### Tutorials — learn by doing

Use these when you want a guided path from zero to a working outcome.

- [Getting Started](tutorials/GETTING_STARTED.md)
- [DAP User Guide](tutorials/DAP_USER_GUIDE.md)
- [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md)
- [Execute Command Tutorial](tutorials/EXECUTE_COMMAND_TUTORIAL.md)
- [Comprehensive Testing Guide](tutorials/COMPREHENSIVE_TESTING_GUIDE.md)
- [DAP Bridge Setup Guide](tutorials/DAP_BRIDGE_SETUP_GUIDE.md)
- [Workspace Refactoring Tutorial](tutorials/WORKSPACE_REFACTORING_TUTORIAL.md)
- [AI Build Guide](tutorials/AI_BUILD_GUIDE.md)

### How-to guides — solve a problem

Use these when you know the result you want and need the shortest path to it.

- [Installation](how-to/INSTALLATION.md)
- [Editor Setup](how-to/EDITOR_SETUP.md)
- [Troubleshooting](how-to/TROUBLESHOOTING.md)
- [Performance Tuning](how-to/PERFORMANCE_TUNING.md)
- [Threading Configuration Guide](how-to/THREADING_CONFIGURATION_GUIDE.md)
- [Coverage](how-to/COVERAGE.md)
- [Dead Code Detection](how-to/DEAD_CODE_DETECTION.md)
- [Dependency Management](how-to/DEPENDENCY_MANAGEMENT.md)
- [Security Development Guide](how-to/SECURITY_DEVELOPMENT_GUIDE.md)
- [Contributing LSP](how-to/CONTRIBUTING_LSP.md)

For the full set of task-oriented guides, browse [`docs/how-to/`](how-to/).

### Reference — look it up

Use these when accuracy, completeness, and scannability matter more than narrative.

- [Commands Reference](reference/COMMANDS_REFERENCE.md)
- [Architecture Overview](reference/ARCHITECTURE_OVERVIEW.md)
- [Crate Architecture Guide](reference/CRATE_ARCHITECTURE_GUIDE.md)
- [LSP Implementation Guide](reference/LSP_IMPLEMENTATION_GUIDE.md)
- [LSP Features](reference/LSP_FEATURES.md)
- [Configuration](reference/CONFIG.md)
- [Configuration Schema](reference/CONFIGURATION_SCHEMA.md)
- [Stability Policy](reference/STABILITY.md)
- [Known Limitations](reference/KNOWN_LIMITATIONS.md)
- [Documentation Guide](reference/DOCUMENTATION_GUIDE.md)

For the full reference corpus, browse [`docs/reference/`](reference/).

### Explanation — understand why

Use these when you need context, design rationale, or tradeoff analysis.

- [Pure Rust Parser](explanation/PURE_RUST_PARSER.md)
- [Error Handling Strategy](explanation/ERROR_HANDLING_STRATEGY.md)
- [Slash Disambiguation](explanation/SLASH_DISAMBIGUATION.md)
- [Cancellation Architecture Guide](explanation/CANCELLATION_ARCHITECTURE_GUIDE.md)
- [Builtin Function Parsing](explanation/BUILTIN_FUNCTION_PARSING.md)
- [LSP Documentation](explanation/LSP_DOCUMENTATION.md)
- [Debt Tracking](explanation/DEBT_TRACKING.md)

For the full set of conceptual docs, browse [`docs/explanation/`](explanation/).

## Project, governance, and supporting material

These directories support delivery, status tracking, and deeper internal process work. They complement the Diátaxis categories above rather than replacing them.

| Directory | Purpose |
|-----------|---------|
| [project/](project/) | Status reports, health metrics, roadmap, and governance |
| [adr/](adr/) | Architecture Decision Records |
| [benchmarks/](benchmarks/) | Benchmark and performance documentation |
| [ci/](ci/) | CI-specific documentation |
| [design/](design/) | Design notes and working documents |
| [EDITORS/](EDITORS/) | Editor-specific reference material |
| [forensics/](forensics/) | PR archaeology and historical investigations |
| [issues/](issues/) | Known issues, corpus gaps, and resolution plans |
| [semantic/](semantic/) | Semantic analysis planning and validation |
| [specs/](specs/) | Specifications and lifecycle documents |
| [archive/](archive/) | Historical documents retained for reference |
| [handoff/](handoff/) | Handoff and swarm workflow material |

## Canonical truth sources

Use these when you need the authoritative source for a class of information.

| What | Where | Notes |
|------|-------|-------|
| Project metrics | [CURRENT_STATUS.md](project/CURRENT_STATUS.md) | Computed status lives here |
| Feature catalog | [features.toml](../features.toml) | Canonical advertised capability list |
| Build and test commands | [COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md) | Preferred command source |
| API and release stability | [STABILITY.md](reference/STABILITY.md) | Compatibility expectations |
| Project direction | [ROADMAP.md](../ROADMAP.md) | Release and milestone planning |
| Documentation standards | [DOCUMENTATION_GUIDE.md](reference/DOCUMENTATION_GUIDE.md) | Diátaxis placement and writing rules |

## Documentation maintenance rules

When adding or revising docs:

1. Put the page in the Diátaxis category that matches the reader's need.
2. Cross-link neighboring material instead of combining tutorial, how-to, reference, and explanation into one page.
3. Update this index when you add a new major entry point or move an existing one.
4. Prefer linking dynamic project facts to canonical sources rather than copying numbers that will drift.
5. Verify commands and internal links where practical before merging.

## Quick verification

```bash
nix develop -c just ci-gate
nix develop -c just status-check
```
