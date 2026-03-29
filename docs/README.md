# perl-lsp Documentation

Use this directory as the docs front door. It is organized by task, not by the
project's internal crate layout.

## Canonical Sources

| Topic | Source | Verified By |
| --- | --- | --- |
| Current release line | [`../Cargo.toml`](../Cargo.toml) | Workspace manifest |
| Metrics and receipts | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) | `just status-update` and `just status-check` |
| Roadmap and active milestone | [project/ROADMAP.md](project/ROADMAP.md) | Human review |
| Capability catalog | [`../features.toml`](../features.toml) | `just ci-gate` |
| Local validation flow | [project/CI_LOCAL_VALIDATION.md](project/CI_LOCAL_VALIDATION.md) | `just ci-gate` |

Rule: if a project metric appears outside [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), treat it as stale until reverified.

## I Need To...

- install or upgrade perl-lsp: [how-to/INSTALLATION.md](how-to/INSTALLATION.md)
- configure an editor: [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md)
- fix a broken setup: [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md)
- upgrade an existing install: [how-to/UPGRADING.md](how-to/UPGRADING.md)
- understand what is shipped now: [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md)
- see the current release plan: [project/ROADMAP.md](project/ROADMAP.md)
- work on the codebase: [../CONTRIBUTING.md](../CONTRIBUTING.md)

## Start Here

- New users: [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md)
- Existing installs: [how-to/UPGRADING.md](how-to/UPGRADING.md)
- Editor setup: [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md)
- Contributors: [../CONTRIBUTING.md](../CONTRIBUTING.md)

## Docs by Job

- Tutorials: [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md)
- How-to: [how-to/INSTALLATION.md](how-to/INSTALLATION.md), [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md), [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md), [how-to/UPGRADING.md](how-to/UPGRADING.md)
- Reference: [reference/COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md), [reference/CONFIG.md](reference/CONFIG.md), [reference/LSP_FEATURES.md](reference/LSP_FEATURES.md)
- Project: [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), [project/ROADMAP.md](project/ROADMAP.md), [project/CI.md](project/CI.md)

## Maintenance

```bash
nix develop -c just ci-gate
just status-update
just status-check
```

- Put computed metrics in [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), not scattered through the docs tree.
- Update [project/ROADMAP.md](project/ROADMAP.md) when the active milestone or release framing changes.
- Keep top-level summary docs short and link back to the canonical project docs.
