# perl-lsp Documentation

Use this directory as the short docs front door. It tells you where to go next
without making you learn the workspace layout first. For the full Diataxis-style
map of the docs tree, use [INDEX.md](INDEX.md).

## Canonical Sources

| Topic | Source | Verified By |
| --- | --- | --- |
| Current release line | [`../Cargo.toml`](../Cargo.toml) | Workspace manifest |
| Metrics and receipts | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) | `just status-update` and `just status-check` |
| Roadmap and active milestone | [project/ROADMAP.md](project/ROADMAP.md) | Human review |
| Capability catalog | [`../features.toml`](../features.toml) | `just ci-gate` |
| Local validation flow | [project/CI_LOCAL_VALIDATION.md](project/CI_LOCAL_VALIDATION.md) | `just ci-gate` |

Rule: if a project metric appears outside [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), treat it as stale until reverified.

## Common Routes

| If you need to... | Read this |
| --- | --- |
| get working fast | [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md) |
| set up continuous testing | [how-to/CONTINUOUS_TESTING.md](how-to/CONTINUOUS_TESTING.md) |
| set up pre-commit hooks | [how-to/PRE_COMMIT.md](how-to/PRE_COMMIT.md) |
| install or upgrade | [how-to/INSTALLATION.md](how-to/INSTALLATION.md), [how-to/UPGRADING.md](how-to/UPGRADING.md) |
| set up `perllsp` in GitHub Actions | [how-to/GITHUB_ACTIONS.md](how-to/GITHUB_ACTIONS.md) |
| configure an editor | [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md) |
| troubleshoot a broken setup | [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md) |
| tune performance or threading | [how-to/PERFORMANCE_TUNING.md](how-to/PERFORMANCE_TUNING.md), [how-to/THREADING_CONFIGURATION_GUIDE.md](how-to/THREADING_CONFIGURATION_GUIDE.md) |
| work with DAP workflows | [tutorials/DAP_USER_GUIDE.md](tutorials/DAP_USER_GUIDE.md) |
| understand project architecture | [reference/ARCHITECTURE_OVERVIEW.md](reference/ARCHITECTURE_OVERVIEW.md), [reference/CRATE_ARCHITECTURE_GUIDE.md](reference/CRATE_ARCHITECTURE_GUIDE.md) |
| check known limitations and parser support | [reference/KNOWN_LIMITATIONS.md](reference/KNOWN_LIMITATIONS.md), [reference/PARSER_FEATURE_MATRIX.md](reference/PARSER_FEATURE_MATRIX.md) |
| see what is true now | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) |
| see the current release plan | [project/ROADMAP.md](project/ROADMAP.md) |
| work on the codebase | [../CONTRIBUTING.md](../CONTRIBUTING.md) |
| browse the full docs map | [INDEX.md](INDEX.md) |

## Docs by Type

- Tutorials: [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md), [tutorials/LSP_DEVELOPMENT_GUIDE.md](tutorials/LSP_DEVELOPMENT_GUIDE.md), [tutorials/DAP_USER_GUIDE.md](tutorials/DAP_USER_GUIDE.md), [tutorials/COMPREHENSIVE_TESTING_GUIDE.md](tutorials/COMPREHENSIVE_TESTING_GUIDE.md)
- How-to: [how-to/INSTALLATION.md](how-to/INSTALLATION.md), [how-to/GITHUB_ACTIONS.md](how-to/GITHUB_ACTIONS.md), [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md), [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md), [how-to/CONTINUOUS_TESTING.md](how-to/CONTINUOUS_TESTING.md), [how-to/UPGRADING.md](how-to/UPGRADING.md), [how-to/PRE_COMMIT.md](how-to/PRE_COMMIT.md), [how-to/PERFORMANCE_TUNING.md](how-to/PERFORMANCE_TUNING.md), [how-to/THREADING_CONFIGURATION_GUIDE.md](how-to/THREADING_CONFIGURATION_GUIDE.md), [how-to/SECURITY_DEVELOPMENT_GUIDE.md](how-to/SECURITY_DEVELOPMENT_GUIDE.md)
- Reference: [reference/COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md), [reference/CONFIG.md](reference/CONFIG.md), [reference/LSP_FEATURES.md](reference/LSP_FEATURES.md), [reference/ARCHITECTURE_OVERVIEW.md](reference/ARCHITECTURE_OVERVIEW.md), [reference/CRATE_ARCHITECTURE_GUIDE.md](reference/CRATE_ARCHITECTURE_GUIDE.md), [reference/KNOWN_LIMITATIONS.md](reference/KNOWN_LIMITATIONS.md), [reference/PARSER_FEATURE_MATRIX.md](reference/PARSER_FEATURE_MATRIX.md), [reference/FAQ.md](reference/FAQ.md)
- Project, specs, and explanations: [INDEX.md](INDEX.md), [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), [project/ROADMAP.md](project/ROADMAP.md), [project/CI.md](project/CI.md), [project/FEATURE_GOVERNANCE.md](project/FEATURE_GOVERNANCE.md), [explanation/LSP_DOCUMENTATION.md](explanation/LSP_DOCUMENTATION.md)

## Maintenance

```bash
nix develop -c just ci-gate
just status-update
just status-check
```

- Put computed metrics in [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), not scattered through the docs tree.
- Update [project/ROADMAP.md](project/ROADMAP.md) when the active milestone or release framing changes.
- Keep top-level summary docs short and link back to the canonical project docs.
