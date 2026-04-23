# Codex Desktop Setup

This guide configures `perllsp` for OpenAI Codex Desktop using the same stdio
server command used by other editors.

## Prerequisites

- `perllsp` is installed and available on your `PATH`
- Codex Desktop is installed and signed in
- You opened a workspace/folder containing Perl files

Verify your server first:

```bash
perllsp --version
perllsp --health
```

## Configure perllsp as a custom language server

In Codex Desktop, add a custom language server for Perl and set:

- **Command**: `perllsp`
- **Arguments**: `--stdio`
- **Language / file match**: Perl (`*.pl`, `*.pm`, `*.t`)
- **Workspace root**: your project root folder

> If your Codex Desktop version labels this area differently, look for settings
> that configure an external language server process over stdio.

## Verify It Works

Open a Perl file and confirm you get:

- diagnostics (squiggles / problems)
- hover details on built-ins like `print`
- symbol navigation (definition / references)

If the server does not start, run `perllsp --health` in the same shell
environment and then follow [troubleshooting](../how-to/TROUBLESHOOTING.md).
