# Eclipse Setup Guide for perl-lsp

This guide configures `perllsp` in Eclipse using an LSP-capable plugin.

## Prerequisites

- Eclipse IDE (current release recommended)
- An installed LSP client plugin (for example: **Wild Web Developer** or another plugin that allows custom language servers)
- `perllsp` on your `PATH`

Validate the server in a terminal first:

```bash
perllsp --version
perllsp --health
```

## Configure a Perl Language Server in Eclipse

1. Open Eclipse preferences.
2. Go to the plugin's language server configuration page.
3. Add a new server definition for Perl files.
4. Set the server command to:

   ```text
   perllsp --stdio
   ```

5. Associate the server with Perl file patterns (for example `*.pl`, `*.pm`, `*.t`, `*.psgi`).
6. Apply changes and restart Eclipse if prompted.

## Workspace Recommendations

- Open the project root as the Eclipse workspace root so cross-file features can index properly.
- Keep Perl modules and test files inside the opened workspace tree.

## Troubleshooting

- If the server does not start, verify Eclipse can find `perllsp` from its environment.
- If diagnostics/completions do not appear, open the LSP plugin logs and confirm initialization succeeded.
- If indexing appears incomplete, reopen the project root and restart the language server session.

For general debugging steps, see [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
