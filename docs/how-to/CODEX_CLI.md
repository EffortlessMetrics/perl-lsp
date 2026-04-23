# Codex CLI Contributor Setup

This guide is for contributors using OpenAI Codex CLI in this repository.

## Why this exists

`perl-lsp` has project-specific working rules in [AGENTS.md](../../AGENTS.md).
Codex reads those automatically, but first-time setup friction is usually:

- running from the wrong working directory,
- missing trust for project-local `.codex/` settings,
- using generic commands instead of this repo's `just`/`xtask` workflow.

This page gives a copy/paste baseline that matches the contributor flow used in this repo.

## 1) Use the repository root as your cwd

From the checkout root:

```bash
pwd
```

Expected: the path ends with `perl-lsp`.

If you start Codex from another folder, AGENTS instructions and local project config may not apply.

## 2) Optional: project-level Codex config

Codex supports project-local settings in `.codex/config.toml`. Keep that file minimal and repository-specific, and put personal preferences in `~/.codex/config.toml` instead.

Notes:

- Do not commit secrets in `.codex/config.toml`.
- If your Codex installation requires project trust for local config, mark this checkout as trusted.
- Confirm supported keys against the current Codex config reference before committing shared defaults.

## 3) Start with a repo-aware prompt

Example session prompt:

```text
Read AGENTS.md and docs/reference/COMMANDS_REFERENCE.md, then implement <task>. Run the smallest relevant test and summarize what changed.
```

This avoids generic-language assumptions and steers Codex to the same command surface used by human contributors.

## 4) Prefer repo-native verification commands

For focused work in one crate:

```bash
cargo test -p <crate>
cargo check --all-targets -p <crate>
cargo clippy -p <crate>
```

Before opening a PR (full fast gate):

```bash
just pr-fast
```

Canonical local merge gate:

```bash
nix develop -c just ci-gate
```

## 5) Common failure modes

- **"My change ignored project rules"** → You likely launched Codex outside the repo root.
- **"Codex used wrong commands"** → Prime with `docs/reference/COMMANDS_REFERENCE.md` in the first prompt.
- **"PR title check failed"** → Use `type(scope): description (#NNNN)`; use `(#0000)` when needed.

## Related docs

- [AGENTS.md](../../AGENTS.md)
- [Contributing](../../CONTRIBUTING.md)
- [Commands Reference](../reference/COMMANDS_REFERENCE.md)
- [Troubleshooting](TROUBLESHOOTING.md)
