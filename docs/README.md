# perl-lsp Documentation

This directory is the main documentation home for the repository. Treat it as a navigation hub, not a frozen snapshot of project metrics.

## Canonical Truth Sources

| Topic | Source | Verified By |
| --- | --- | --- |
| Current release line | [`../Cargo.toml`](../Cargo.toml) | Workspace manifest |
| Metrics and receipts | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) | `just status-update` and `just status-check` |
| Roadmap and active milestone | [project/ROADMAP.md](project/ROADMAP.md) | Human review |
| Capability catalog | [`../features.toml`](../features.toml) | `just ci-gate` |
| Local validation flow | [project/CI_LOCAL_VALIDATION.md](project/CI_LOCAL_VALIDATION.md) | `just ci-gate` |

Rule: if you see a project metric duplicated outside [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), treat it as suspect until reverified.

## Start Here

- Users: [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md)
- Editor setup: [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md)
- Upgrade existing installs: [how-to/UPGRADING.md](how-to/UPGRADING.md)
- Contributors: [../CONTRIBUTING.md](../CONTRIBUTING.md)
- Current project posture: [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md)
- Active milestone: [project/ROADMAP.md](project/ROADMAP.md)
- Historical analyses and launch material: [articles/README.md](articles/README.md)

## Tutorials

- [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md)
- [tutorials/DAP_USER_GUIDE.md](tutorials/DAP_USER_GUIDE.md)
- [tutorials/LSP_DEVELOPMENT_GUIDE.md](tutorials/LSP_DEVELOPMENT_GUIDE.md)
- [tutorials/EXECUTE_COMMAND_TUTORIAL.md](tutorials/EXECUTE_COMMAND_TUTORIAL.md)
- [tutorials/COMPREHENSIVE_TESTING_GUIDE.md](tutorials/COMPREHENSIVE_TESTING_GUIDE.md)

## How-To Guides

- [how-to/INSTALLATION.md](how-to/INSTALLATION.md)
- [how-to/UPGRADING.md](how-to/UPGRADING.md)
- [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md)
- [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md)
- [how-to/DEPENDENCY_MANAGEMENT.md](how-to/DEPENDENCY_MANAGEMENT.md)
- [how-to/SEMVER_WORKFLOW.md](how-to/SEMVER_WORKFLOW.md)
- [how-to/COVERAGE.md](how-to/COVERAGE.md)
- [how-to/DEAD_CODE_DETECTION.md](how-to/DEAD_CODE_DETECTION.md)
- [../distribution/linux/README.md](../distribution/linux/README.md)

## Reference

- [reference/COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md)
- [reference/ARCHITECTURE_OVERVIEW.md](reference/ARCHITECTURE_OVERVIEW.md)
- [reference/CRATE_ARCHITECTURE_GUIDE.md](reference/CRATE_ARCHITECTURE_GUIDE.md)
- [reference/LSP_IMPLEMENTATION_GUIDE.md](reference/LSP_IMPLEMENTATION_GUIDE.md)
- [reference/PROPERTY_TESTING.md](reference/PROPERTY_TESTING.md)
- [reference/LSP_FEATURES.md](reference/LSP_FEATURES.md)
- [reference/STABILITY.md](reference/STABILITY.md)
- [reference/CONFIG.md](reference/CONFIG.md)
- [reference/FAQ.md](reference/FAQ.md)
- [reference/KNOWN_LIMITATIONS.md](reference/KNOWN_LIMITATIONS.md)
- [reference/DOCUMENTATION_GUIDE.md](reference/DOCUMENTATION_GUIDE.md)

## Explanation

- [explanation/PURE_RUST_PARSER.md](explanation/PURE_RUST_PARSER.md)
- [explanation/ERROR_HANDLING_STRATEGY.md](explanation/ERROR_HANDLING_STRATEGY.md)
- [explanation/DEBT_TRACKING.md](explanation/DEBT_TRACKING.md)
- [explanation/SLASH_DISAMBIGUATION.md](explanation/SLASH_DISAMBIGUATION.md)

## Project Docs

- [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md)
- [project/ROADMAP.md](project/ROADMAP.md)
- [project/MILESTONES.md](project/MILESTONES.md)
- [project/LESSONS.md](project/LESSONS.md)
- [project/CASEBOOK.md](project/CASEBOOK.md)
- [project/CODEBASE_CURIOSITIES.md](project/CODEBASE_CURIOSITIES.md)
- [project/CI.md](project/CI.md)
- [project/CI_TEST_LANES.md](project/CI_TEST_LANES.md)

## Historical Analyses

- [articles/README.md](articles/README.md)
- [articles/FIVE_ERAS.md](articles/FIVE_ERAS.md)
- [articles/SWARM_METHODOLOGY.md](articles/SWARM_METHODOLOGY.md)
- [articles/ZERO_PANIC.md](articles/ZERO_PANIC.md)
- [articles/PARSING_PERL.md](articles/PARSING_PERL.md)
- [articles/CURIOSITIES.md](articles/CURIOSITIES.md)

## Strategic Docs

- [STRATEGIC_DOCUMENTATION.md](STRATEGIC_DOCUMENTATION.md)
- [../ROADMAP.md](../ROADMAP.md)
- [../NOW_NEXT_LATER.md](../NOW_NEXT_LATER.md)
- [../TECHNICAL_VISION.md](../TECHNICAL_VISION.md)

## Other Directories

| Directory | Purpose |
| --- | --- |
| [adr/](adr/) | Architecture Decision Records |
| [archive/](archive/) | Historical docs |
| [articles/](articles/) | Historical analyses plus article research notes |
| [benchmarks/](benchmarks/) | Benchmark docs |
| [ci/](ci/) | CI-specific docs |
| [design/](design/) | Design notes |
| [EDITORS/](EDITORS/) | Editor-specific setup |
| [../distribution/](../distribution/) | Package and release templates |
| [forensics/](forensics/) | PR archaeology |
| [issues/](issues/) | Gap tracking and investigations |
| [semantic/](semantic/) | Semantic validation |
| [specs/](specs/) | Specifications |

## Documentation Maintenance

```bash
nix develop -c just ci-gate
just status-update
just status-check
```

- Put computed metrics in [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), not scattered through the docs tree.
- Update [project/ROADMAP.md](project/ROADMAP.md) when the active milestone or release framing changes.
- Keep top-level summary docs short and linked back to the canonical project docs.
