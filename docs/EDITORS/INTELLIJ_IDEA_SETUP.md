# IntelliJ IDEA Setup (LSP4IJ)

This guide covers running `perllsp` inside IntelliJ IDEA through the
[LSP4IJ](https://github.com/redhat-developer/lsp4ij) plugin.

If you have not installed the server binary yet, start with
[INSTALLATION.md](../how-to/INSTALLATION.md).

## Prerequisites

- IntelliJ IDEA (Community or Ultimate)
- LSP4IJ plugin installed and enabled
- `perllsp` on your `PATH`

Verify the server first:

```bash
perllsp --version
perllsp --health
```

## Configure the Language Server

1. Open **Settings** → **Languages & Frameworks** → **Language Servers**.
2. Add a new server.
3. Use these values:
   - **Command**: `perllsp`
   - **Arguments**: `--stdio`
   - **File types**: Perl (`*.pl`, `*.pm`, `*.t`, `*.psgi`)
   - **Working directory**: `$ProjectFileDir$`
4. Apply and restart the language server.

## Verify It Works

Open a Perl file and confirm:

- diagnostics appear for syntax errors
- completion results show local symbols
- go-to-definition jumps to `sub` or variable declarations

## Troubleshooting

- **Server not found**: run `perllsp --version` in a terminal opened from
  IntelliJ to verify `PATH` inheritance.
- **No diagnostics/completion**: confirm the LSP mapping includes Perl file
  types and that the project root is the workspace folder.
- **Unexpected behavior**: check IntelliJ's LSP client logs and compare with
  [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
