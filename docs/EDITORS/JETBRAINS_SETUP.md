# JetBrains Setup (Generic LSP)

Use this guide for JetBrains IDEs that expose a generic LSP integration point
(e.g. IntelliJ IDEA, PhpStorm, WebStorm, RustRover with built-in support or the
LSP Support plugin).

## Prerequisites

- `perllsp` is installed and on your shell `PATH`
- The project opens at the repository root

Verify from a terminal first:

```bash
perllsp --version
perllsp --health
```

## Add `perllsp` as a language server

1. Open IDE settings and navigate to the generic LSP server configuration page.
2. Add a new server named `perl-lsp`.
3. Set the command to `perllsp`.
4. Set arguments to `--stdio`.
5. Scope the server to Perl file patterns (`*.pl`, `*.pm`, `*.t`, `*.psgi`).
6. Use the project root as the working directory.

If the UI offers an `initializationOptions` JSON field, use:

```json
{
  "perl": {
    "workspace": {
      "includePaths": ["lib", ".", "local/lib/perl5"],
      "useSystemInc": false
    }
  }
}
```

## Troubleshooting

- **Server not found**: configure an absolute path to `perllsp` in the command.
- **No diagnostics/completion**: confirm the server is attached to Perl file
  types and not only plain text.
- **Wrong project indexing**: ensure the working directory is the Perl project
  root, then restart the language server.

For shared option semantics, see [CONFIG.md](../reference/CONFIG.md).
