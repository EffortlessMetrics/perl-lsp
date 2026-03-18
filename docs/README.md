# Perl LSP Documentation

Documentation index for the `perl-lsp` workspace. Use this page as the starting point for user guides, contributor references, and project status material.

## Quick navigation

| If you want to... | Read this |
|---|---|
| Install `perl-lsp` and get your editor working | [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md) |
| Configure a specific editor | [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md) and [EDITORS/](EDITORS/) |
| Troubleshoot setup or runtime issues | [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md) |
| Understand supported LSP capabilities | [reference/LSP_FEATURES.md](reference/LSP_FEATURES.md) and [`../features.toml`](../features.toml) |
| Learn the developer workflow | [../CONTRIBUTING.md](../CONTRIBUTING.md) and [reference/COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md) |
| Review architecture and design decisions | [reference/ARCHITECTURE_OVERVIEW.md](reference/ARCHITECTURE_OVERVIEW.md) and [adr/README.md](adr/README.md) |
| Check current health, metrics, and roadmap | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) and [project/ROADMAP.md](project/ROADMAP.md) |

## Repository snapshot

- Workspace version: **v0.12.0**
- Workspace members: **121 crates**
- Family counts (from `crates/`):
  - `perl-module-*`: 13
  - `perl-lsp-*`: 41
  - `perl-lsp-feature-*`: 8
  - `perl-dap-*`: 9
  - `perl-ts-*`: 5
  - `perl-workspace-*`: 6

Refresh the counts with:

```bash
find crates -maxdepth 1 -mindepth 1 -type d | wc -l
for prefix in perl-module- perl-lsp- perl-lsp-feature- perl-dap- perl-ts- perl-workspace-; do
  printf "%-18s %s\n" "$prefix" "$(find crates -maxdepth 1 -mindepth 1 -type d -name "${prefix}*" | wc -l)"
done
```

## Learn by doing

Tutorials explain end-to-end workflows and are the best place to start when you want context, not just a command.

- [Getting Started](tutorials/GETTING_STARTED.md) — install, verify, and connect an editor
- [DAP User Guide](tutorials/DAP_USER_GUIDE.md) — configure debugger support
- [LSP Development Guide](tutorials/LSP_DEVELOPMENT_GUIDE.md) — extend server behavior safely
- [Execute Command Tutorial](tutorials/EXECUTE_COMMAND_TUTORIAL.md) — add custom LSP commands
- [Comprehensive Testing Guide](tutorials/COMPREHENSIVE_TESTING_GUIDE.md) — understand test lanes and validation
- [Workspace Refactoring Tutorial](tutorials/WORKSPACE_REFACTORING_TUTORIAL.md) — navigate larger codebase changes

## Solve a specific problem

How-to guides are task-oriented and optimized for common maintenance work.

- [Installation](how-to/INSTALLATION.md)
- [Editor Setup](how-to/EDITOR_SETUP.md)
- [Troubleshooting](how-to/TROUBLESHOOTING.md)
- [Debugging](how-to/DEBUGGING.md)
- [Performance Tuning](how-to/PERFORMANCE_TUNING.md)
- [Threading Configuration](how-to/THREADING_CONFIGURATION_GUIDE.md)
- [Dependency Management](how-to/DEPENDENCY_MANAGEMENT.md)
- [Upgrading](how-to/UPGRADING.md)

## Reference

Reference documents are the canonical source for commands, contracts, and detailed technical behavior.

- [Commands Reference](reference/COMMANDS_REFERENCE.md)
- [Configuration](reference/CONFIG.md)
- [FAQ](reference/FAQ.md)
- [Known Limitations](reference/KNOWN_LIMITATIONS.md)
- [Architecture Overview](reference/ARCHITECTURE_OVERVIEW.md)
- [Crate Architecture Guide](reference/CRATE_ARCHITECTURE_GUIDE.md)
- [LSP Implementation Guide](reference/LSP_IMPLEMENTATION_GUIDE.md)
- [LSP Features](reference/LSP_FEATURES.md)
- [Stability Policy](reference/STABILITY.md)
- [Supply Chain Security](reference/SUPPLY_CHAIN_SECURITY.md)

## Explanation and background

These documents explain why the codebase looks the way it does.

- [Pure Rust Parser](explanation/PURE_RUST_PARSER.md)
- [Error Handling Strategy](explanation/ERROR_HANDLING_STRATEGY.md)
- [Slash Disambiguation](explanation/SLASH_DISAMBIGUATION.md)
- [Tree-sitter Compatibility](explanation/TREE_SITTER_COMPATIBILITY.md)
- [LSP Crate Separation Guide](explanation/LSP_CRATE_SEPARATION_GUIDE.md)

## Project status and governance

Use these documents when you need current metrics, planning context, or process details.

- [Current Status](project/CURRENT_STATUS.md)
- [Roadmap](project/ROADMAP.md)
- [Milestones](project/MILESTONES.md)
- [CI](project/CI.md)
- [CI Test Lanes](project/CI_TEST_LANES.md)
- [CI Local Validation](project/CI_LOCAL_VALIDATION.md)
- [Documentation Truth System](project/DOCUMENTATION_TRUTH_SYSTEM.md)

## Additional directories

| Directory | Purpose |
|-----------|---------|
| [adr/](adr/) | Architecture Decision Records |
| [archive/](archive/) | Historical release notes and retired material |
| [benchmarks/](benchmarks/) | Benchmark design and reports |
| [ci/](ci/) | CI and local validation documentation |
| [design/](design/) | Deep design notes for semantic analysis and related systems |
| [EDITORS/](EDITORS/) | Editor-specific setup guides |
| [forensics/](forensics/) | PR archaeology and forensic workflows |
| [handoff/](handoff/) | Agent and swarm workflow packs |
| [issues/](issues/) | Known issues and corpus gap tracking |
| [semantic/](semantic/) | Semantic validation plans and results |
| [specs/](specs/) | Specifications and acceptance criteria |

## Canonical truth sources

| Topic | Source | Verification path |
|------|--------|-------------------|
| Capability catalog | [`../features.toml`](../features.toml) | `just ci-gate` |
| Live metrics | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) | `just status-check` |
| Local validation flow | [project/CI_LOCAL_VALIDATION.md](project/CI_LOCAL_VALIDATION.md) | `nix develop -c just ci-gate` |
| Release planning | [project/ROADMAP.md](project/ROADMAP.md) | Human review |
| Milestones | [project/MILESTONES.md](project/MILESTONES.md) | GitHub milestones |

**Rule:** if a number in another document disagrees with `project/CURRENT_STATUS.md`, treat `project/CURRENT_STATUS.md` as the source of truth unless a newer generated artifact says otherwise.
