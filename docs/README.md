# Perl LSP Documentation

Documentation for Perl LSP v0.12.0 — a Language Server Protocol implementation for Perl.

## Repository snapshot

- Workspace version: **v0.12.0**
- Workspace members: **112 crates**
- Family counts (from `crates/`):
  - `perl-module-*`: 13
  - `perl-lsp-*`: 38
  - `perl-lsp-feature-*`: 8
  - `perl-dap-*`: 9
  - `perl-ts-*`: 5
  - `perl-workspace-*`: 6

To refresh counts:

```bash
for prefix in perl-module- perl-lsp- perl-lsp-feature- perl-dap- perl-ts- perl-workspace-; do
  printf "%-18s %s\n" "$prefix" "$(find crates -maxdepth 1 -mindepth 1 -type d -name "${prefix}*" | wc -l)"
done
```

## Read docs by intent

This documentation hub follows the [Diátaxis](https://diataxis.fr/) framework. Pick the section that matches what you need **right now**:

| If you want to… | Start here | Why |
|---|---|---|
| Learn the product from scratch | [Tutorials](#tutorials--learn-by-doing) | Guided, sequential learning with minimal assumptions |
| Complete a concrete task | [How-to guides](#how-to-guides--solve-a-problem) | Goal-oriented instructions with verification steps |
| Look up commands, behavior, or policy | [Reference](#reference--look-it-up) | Concise, factual lookup material |
| Understand architecture and design tradeoffs | [Explanation](#explanation--understand-why) | Context, rationale, and system-level thinking |

### Quick start paths

- **New user** → [Getting Started](tutorials/GETTING_STARTED.md) → [Installation](how-to/INSTALLATION.md) → [Editor Setup](how-to/EDITOR_SETUP.md)
- **LSP contributor** → [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md) → [Commands Reference](reference/COMMANDS_REFERENCE.md) → [LSP Implementation Guide](reference/LSP_IMPLEMENTATION_GUIDE.md)
- **DAP user** → [DAP User Guide](tutorials/DAP_USER_GUIDE.md) → [DAP Bridge Setup Guide](tutorials/DAP_BRIDGE_SETUP_GUIDE.md)
- **Docs contributor** → [Documentation Guide](reference/DOCUMENTATION_GUIDE.md)

## Tutorials — learn by doing

Step-by-step guides to get you started and build confidence through practice.

- [Getting Started](tutorials/GETTING_STARTED.md) — Installation and first steps
- [DAP User Guide](tutorials/DAP_USER_GUIDE.md) — Debug Adapter Protocol setup and usage
- [DAP Bridge Setup Guide](tutorials/DAP_BRIDGE_SETUP_GUIDE.md) — End-to-end bridge debugging workflow
- [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md) — Build and extend the LSP server
- [Execute Command Tutorial](tutorials/EXECUTE_COMMAND_TUTORIAL.md) — Custom LSP commands
- [Workspace Refactoring Tutorial](tutorials/WORKSPACE_REFACTORING_TUTORIAL.md) — Guided workspace-wide rename workflows
- [Comprehensive Testing Guide](tutorials/COMPREHENSIVE_TESTING_GUIDE.md) — Testing workflows
- [AI Build Guide](tutorials/AI_BUILD_GUIDE.md) — Agent-assisted development setup

## How-to guides — solve a problem

Task-oriented instructions for common operations and operational troubleshooting.

- [Installation](how-to/INSTALLATION.md) — Install from source or binary
- [Editor Setup](how-to/EDITOR_SETUP.md) — Configure your editor
- [Troubleshooting](how-to/TROUBLESHOOTING.md) — Common issues and solutions
- [Upgrading](how-to/UPGRADING.md) — Safely move between releases
- [Dependency Management](how-to/DEPENDENCY_MANAGEMENT.md) — Automated updates with Dependabot
- [SemVer Workflow](how-to/SEMVER_WORKFLOW.md) — SemVer checking and API compatibility
- [Coverage](how-to/COVERAGE.md) — Code coverage reports
- [Dead Code Detection](how-to/DEAD_CODE_DETECTION.md) — Find unused code
- [Performance Tuning](how-to/PERFORMANCE_TUNING.md) — Optimize performance-sensitive paths
- [Security Development Guide](how-to/SECURITY_DEVELOPMENT_GUIDE.md) — Secure development practices
- [Contributing LSP](how-to/CONTRIBUTING_LSP.md) — Common LSP contribution tasks

## Reference — look it up

Precise, complete information for lookup and verification.

- [Commands Reference](reference/COMMANDS_REFERENCE.md) — Full command catalog
- [Architecture Overview](reference/ARCHITECTURE_OVERVIEW.md) — System design and components
- [Crate Architecture Guide](reference/CRATE_ARCHITECTURE_GUIDE.md) — Workspace structure and tiers
- [LSP Implementation Guide](reference/LSP_IMPLEMENTATION_GUIDE.md) — Language Server Protocol details
- [LSP Features](reference/LSP_FEATURES.md) — Supported LSP capabilities
- [Configuration](reference/CONFIG.md) — Configuration options
- [Configuration Schema](reference/CONFIGURATION_SCHEMA.md) — Structured config schema details
- [Stability Policy](reference/STABILITY.md) — API versioning and compatibility
- [Known Limitations](reference/KNOWN_LIMITATIONS.md) — Current constraints and workarounds
- [FAQ](reference/FAQ.md) — Frequently asked questions
- [Documentation Guide](reference/DOCUMENTATION_GUIDE.md) — Diátaxis framework, doc placement, and writing standards

## Explanation — understand why

Conceptual discussions, design rationale, and tradeoff analysis.

- [Pure Rust Parser](explanation/PURE_RUST_PARSER.md) — Why we built a native parser
- [Error Handling Strategy](explanation/ERROR_HANDLING_STRATEGY.md) — Error philosophy and patterns
- [LSP Documentation](explanation/LSP_DOCUMENTATION.md) — Documentation design for LSP features
- [Debt Tracking](explanation/DEBT_TRACKING.md) — Technical debt management
- [Slash Disambiguation](explanation/SLASH_DISAMBIGUATION.md) — Perl regex vs division parsing
- [Tree-sitter Compatibility](explanation/TREE_SITTER_COMPATIBILITY.md) — Parser compatibility tradeoffs

## Project — status and governance

Process, metrics, and project health.

- [Current Status](project/CURRENT_STATUS.md) — Computed metrics and project health
- [Roadmap](project/ROADMAP.md) — Milestones and release planning
- [Milestones](project/MILESTONES.md) — GitHub milestones
- [Lessons](project/LESSONS.md) — What went wrong
- [Casebook](project/CASEBOOK.md) — What went right
- [CI](project/CI.md) — Continuous integration setup
- [CI Test Lanes](project/CI_TEST_LANES.md) — Test lane configuration
- [Documentation Truth System](project/DOCUMENTATION_TRUTH_SYSTEM.md) — Governance for source-of-truth docs

## Strategic documents — planning and direction

High-level planning documents for project direction.

- [Strategic Documentation Index](STRATEGIC_DOCUMENTATION.md) — Navigation hub for all strategic docs
- [Technical Vision](../TECHNICAL_VISION.md) — Long-term technical direction (3-5 years)
- [Roadmap](../ROADMAP.md) — Version milestones and deliverables
- [Now/Next/Later](../NOW_NEXT_LATER.md) — Current quarter priorities

## Other directories

| Directory | Purpose |
|-----------|---------|
| [adr/](adr/) | Architecture Decision Records |
| [archive/](archive/) | Historical documents |
| [benchmarks/](benchmarks/) | Benchmark framework docs |
| [ci/](ci/) | CI-specific documentation |
| [design/](design/) | Semantic analyzer design |
| [EDITORS/](EDITORS/) | Editor-specific setup |
| [forensics/](forensics/) | PR archaeology |
| [issues/](issues/) | Corpus gap tracking |
| [semantic/](semantic/) | Semantic validation |
| [specs/](specs/) | Specification documents |

## Contributing to docs

- [Contributing Guide](../CONTRIBUTING.md) — Development workflow and contribution process
- [Documentation Guide](reference/DOCUMENTATION_GUIDE.md) — Choose the right Diátaxis category before writing

### Diátaxis review checklist

Use this quick check before opening a documentation PR:

- Does the page serve **one primary intent**: tutorial, how-to, reference, or explanation?
- Does the title make the page’s promise clear?
- Does the page link outward to adjacent doc types instead of mixing them together?
- Did you update this hub if you added, removed, or renamed an entry-point document?
- Did you verify commands and internal links where practical?

## Quick verification

```bash
nix develop -c just ci-gate       # Canonical local gate
nix develop -c just status-check  # Verify metrics haven't drifted
```

## Canonical truth sources

| What | Where | Verified By |
|------|-------|-------------|
| Metrics | [CURRENT_STATUS.md](project/CURRENT_STATUS.md) | `just status-check` |
| Plans | [ROADMAP.md](project/ROADMAP.md) | Human review |
| Milestones | [MILESTONES.md](project/MILESTONES.md) | GitHub Milestones |
| Capability catalog | [features.toml](../features.toml) | `just ci-gate` |
| CI lanes | [CI_TEST_LANES.md](project/CI_TEST_LANES.md) | `just ci-gate` |
| Local validation | [CI_LOCAL_VALIDATION.md](project/CI_LOCAL_VALIDATION.md) | `just ci-gate` |
| What went wrong | [LESSONS.md](project/LESSONS.md) | Human review |
| What went right | [CASEBOOK.md](project/CASEBOOK.md) | Human review |

**Rule**: All metrics are computed and live in `CURRENT_STATUS.md`. If you see a number elsewhere, treat it as stale.

---

Version: v0.12.0
