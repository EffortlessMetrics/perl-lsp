# GNU nano Setup Guide (Terminal Workflow)

`nano` does not have a built-in LSP client, so it cannot talk to `perllsp`
over `--stdio` like VS Code, Neovim, or Emacs.

This guide gives a pragmatic workflow so `nano` users can still benefit from
`perl-lsp` checks and troubleshooting commands.

## What You Can Use Today

- `perllsp --health` to verify the language server environment
- `perllsp --version` to confirm the installed binary
- Perl compile checks (`perl -c`) for immediate syntax validation while editing

## Quick Start

1. Confirm tooling is installed:

```bash
perllsp --version
perllsp --health
perl -v
```

2. In another terminal, run syntax checks while editing:

```bash
perl -c path/to/file.pl
```

3. If you see parser or indexing issues in downstream tooling, collect logs with:

```bash
RUST_LOG=perl_lsp=debug perllsp --stdio 2> perllsp-debug.log
```

Then stop with `Ctrl+C` after reproducing the issue.

## Optional: Lightweight Re-check Loop

Use this shell loop in a second terminal to re-run `perl -c` after saves:

```bash
while true; do
  clear
  date
  perl -c path/to/file.pl
  sleep 1
done
```

## If You Want Full IDE Features

Features like completion, go-to-definition, hover, rename, and references require
an editor with an LSP client implementation. For that experience, use one of:

- VS Code: [VS_CODE_SETUP.md](VS_CODE_SETUP.md)
- Neovim: [NEOVIM_SETUP.md](NEOVIM_SETUP.md)
- Emacs: [EMACS_SETUP.md](EMACS_SETUP.md)
- Helix: [HELIX_SETUP.md](HELIX_SETUP.md)
- Sublime Text: [SUBLIME_SETUP.md](SUBLIME_SETUP.md)
