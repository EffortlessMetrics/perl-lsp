# Eclipse Setup

This guide configures `perllsp` in Eclipse-based IDEs via the Language Server
Protocol (LSP).

## Prerequisites

- Eclipse IDE 2023-12+ (or another recent Eclipse package)
- `perllsp` installed and available on your `PATH`
- Perl files opened from a workspace folder/project

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Option 1 (Recommended): Wild Web Developer

If your Eclipse distribution includes the Wild Web Developer plugin, use its
Language Server configuration:

1. Open **Preferences**.
2. Go to **Languages > Language Servers**.
3. Add a new server definition:
   - **Command**: `perllsp`
   - **Arguments**: `--stdio`
4. Associate the server with Perl file patterns (for example `*.pl`, `*.pm`,
   `*.t`).
5. Restart Eclipse or reload the project.

## Option 2: Generic LSP4E integration

For Eclipse installations using standalone LSP4E support:

1. Install/enable LSP4E support in your Eclipse package.
2. Register a language server launch command:
   - executable: `perllsp`
   - args: `--stdio`
3. Bind the server to Perl content types or file extensions.
4. Reopen Perl files to trigger initialization.

## Troubleshooting

- If Eclipse cannot launch the server, run `perllsp --health` from a terminal
  started in the same environment as Eclipse.
- If diagnostics do not appear, confirm Perl file associations include all
  relevant extensions (`.pl`, `.pm`, `.t`).
- If startup works but features are missing, review
  [../how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
