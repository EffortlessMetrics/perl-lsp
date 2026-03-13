# Perl LSP Documentation

Documentation for Perl LSP v0.10.0 — a Language Server Protocol implementation for Perl.

## Repository snapshot

- Workspace version: **v0.10.0**
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

## Tutorials — learn by doing

Step-by-step guides to get you started.

- [Getting Started](tutorials/GETTING_STARTED.md) — Installation and first steps
- [DAP User Guide](tutorials/DAP_USER_GUIDE.md) — Debug Adapter Protocol setup and usage
- [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md) — Build and extend the LSP server
- [Execute Command Tutorial](tutorials/EXECUTE_COMMAND_TUTORIAL.md) — Custom LSP commands
- [Comprehensive Testing Guide](tutorials/COMPREHENSIVE_TESTING_GUIDE.md) — Testing workflows

## How-to guides — solve a problem

Task-oriented instructions for common operations.

- [Installation](how-to/INSTALLATION.md) — Install from source or binary
- [Editor Setup](how-to/EDITOR_SETUP.md) — Configure your editor
- [Troubleshooting](how-to/TROUBLESHOOTING.md) — Common issues and solutions
- [Dependency Management](how-to/DEPENDENCY_MANAGEMENT.md) — Automated updates with Dependabot
- [SemVer Workflow](how-to/SEMVER_WORKFLOW.md) — SemVer checking and API compatibility
- [Coverage](how-to/COVERAGE.md) — Code coverage reports
- [Dead Code Detection](how-to/DEAD_CODE_DETECTION.md) — Find unused code

## Reference — look it up

Precise, complete information for lookup.

- [Commands Reference](reference/COMMANDS_REFERENCE.md) — Full command catalog
- [Architecture Overview](reference/ARCHITECTURE_OVERVIEW.md) — System design and components
- [Crate Architecture Guide](reference/CRATE_ARCHITECTURE_GUIDE.md) — Workspace structure and tiers
- [LSP Implementation Guide](reference/LSP_IMPLEMENTATION_GUIDE.md) — Language Server Protocol details
- [LSP Features](reference/LSP_FEATURES.md) — Supported LSP capabilities
- [Stability Policy](reference/STABILITY.md) — API versioning and compatibility
- [Configuration](reference/CONFIG.md) — Configuration options
- [FAQ](reference/FAQ.md) — Frequently asked questions
- [Known Limitations](reference/KNOWN_LIMITATIONS.md) — Current constraints and workarounds
- [Documentation Guide](reference/DOCUMENTATION_GUIDE.md) — Diataxis framework and standards

## Explanation — understand why

Conceptual discussions and design rationale.

- [Pure Rust Parser](explanation/PURE_RUST_PARSER.md) — Why we built a native parser
- [Error Handling Strategy](explanation/ERROR_HANDLING_STRATEGY.md) — Error philosophy and patterns
- [Debt Tracking](explanation/DEBT_TRACKING.md) — Technical debt management
- [Slash Disambiguation](explanation/SLASH_DISAMBIGUATION.md) — Perl regex vs division parsing

## Project — status and governance

Process, metrics, and project health.

- [Current Status](project/CURRENT_STATUS.md) — Computed metrics and project health
- [Roadmap](project/ROADMAP.md) — Milestones and release planning
- [Milestones](project/MILESTONES.md) — GitHub milestones
- [Lessons](project/LESSONS.md) — What went wrong
- [Casebook](project/CASEBOOK.md) — What went right
- [CI](project/CI.md) — Continuous integration setup
- [CI Test Lanes](project/CI_TEST_LANES.md) — Test lane configuration

## Strategic Documents — planning and direction

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

## Contributing

- [Contributing Guide](../CONTRIBUTING.md) — Development workflow and contribution process

## Quick verification

```bash
nix develop -c just ci-gate       # Canonical local gate
nix develop -c just status-check  # Verify metrics haven't drifted
```

## Canonical Truth Sources

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

Version: v0.10.0
