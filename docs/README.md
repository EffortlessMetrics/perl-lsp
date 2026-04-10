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
| see what is true now | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) |
| see the current release plan | [project/ROADMAP.md](project/ROADMAP.md) |
| work on the codebase | [../CONTRIBUTING.md](../CONTRIBUTING.md) |
| understand crate boundaries before changing code | [reference/CRATE_ARCHITECTURE_GUIDE.md](reference/CRATE_ARCHITECTURE_GUIDE.md) |
| follow command recipes for build/test/dev workflows | [reference/COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md) |
| implement or debug LSP behavior | [reference/LSP_IMPLEMENTATION_GUIDE.md](reference/LSP_IMPLEMENTATION_GUIDE.md) |
| work on debugger flows (DAP) | [tutorials/DAP_USER_GUIDE.md](tutorials/DAP_USER_GUIDE.md) |
| browse the full docs map | [INDEX.md](INDEX.md) |

## Docs by Type

- Tutorial: [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md)
- How-to: [how-to/INSTALLATION.md](how-to/INSTALLATION.md), [how-to/GITHUB_ACTIONS.md](how-to/GITHUB_ACTIONS.md), [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md), [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md), [how-to/CONTINUOUS_TESTING.md](how-to/CONTINUOUS_TESTING.md), [how-to/UPGRADING.md](how-to/UPGRADING.md), [how-to/PRE_COMMIT.md](how-to/PRE_COMMIT.md)
- Reference: [reference/COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md), [reference/CONFIG.md](reference/CONFIG.md), [reference/LSP_FEATURES.md](reference/LSP_FEATURES.md)
- Explanation and project docs: [INDEX.md](INDEX.md), [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), [project/ROADMAP.md](project/ROADMAP.md), [project/CI.md](project/CI.md)

## Maintenance

```bash
nix develop -c just ci-gate
just status-update
just status-check
```

- Put computed metrics in [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), not scattered through the docs tree.
- Update [project/ROADMAP.md](project/ROADMAP.md) when the active milestone or release framing changes.
- Keep top-level summary docs short and link back to the canonical project docs.
- When adding a new guide, add at least one pointer from this front door so new contributors can discover it quickly.
