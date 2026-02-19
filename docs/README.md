# Perl LSP Documentation

Documentation for Perl LSP v0.9.1 — a Language Server Protocol implementation for Perl.

## Use it

- [Getting Started](GETTING_STARTED.md) — Installation and first steps
- [Editor Setup](EDITOR_SETUP.md) — Configure your editor
- [FAQ](FAQ.md) — Frequently asked questions
- [Troubleshooting](TROUBLESHOOTING.md) — Common issues and solutions
- [Known Limitations](KNOWN_LIMITATIONS.md) — Current constraints and workarounds

## Understand it

- [Architecture Overview](ARCHITECTURE_OVERVIEW.md) — System design and components
- [Crate Architecture Guide](CRATE_ARCHITECTURE_GUIDE.md) — Workspace structure and dependency tiers
- [LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md) — Language Server Protocol details
- [DAP User Guide](DAP_USER_GUIDE.md) — Debug Adapter Protocol setup and usage

## Contribute

- [Contributing Guide](../CONTRIBUTING.md) — Development workflow and contribution process
- [Development Guide](DEVELOPMENT.md) — Setting up the development environment
- [Commands Reference](COMMANDS_REFERENCE.md) — Build, test, and development commands
- [Test Infrastructure Guide](TEST_INFRASTRUCTURE_GUIDE.md) — Testing framework and tools

## Canonical Truth Sources

| What | Where | Verified By |
|------|-------|-------------|
| Metrics | [CURRENT_STATUS.md](CURRENT_STATUS.md) | `just status-check` |
| Plans | [ROADMAP.md](ROADMAP.md) | Human review |
| Milestones | [MILESTONES.md](MILESTONES.md) | GitHub Milestones |
| Capability catalog | [features.toml](../features.toml) | `just ci-gate` |
| CI lanes | [CI_TEST_LANES.md](CI_TEST_LANES.md) | `just ci-gate` |
| Local validation | [CI_LOCAL_VALIDATION.md](CI_LOCAL_VALIDATION.md) | `just ci-gate` |
| What went wrong | [LESSONS.md](LESSONS.md) | Human review |
| What went right | [CASEBOOK.md](CASEBOOK.md) | Human review |

## Quick verification

```bash
nix develop -c just ci-gate   # Canonical local gate
nix develop -c just status-check  # Verify metrics haven't drifted
```

## Further reading

- [Documentation Standards](DOCUMENTATION_GUIDE.md) — Diataxis framework, feature index, learning paths
- [Project Orientation](ORIENTATION.md) — What needs attention right now
- [Documentation Site](https://effortlessmetrics.github.io/perl-lsp/) — Searchable mdBook site

---

Version: v0.9.1
