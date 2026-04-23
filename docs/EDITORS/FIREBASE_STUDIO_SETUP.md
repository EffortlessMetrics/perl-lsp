# Firebase Studio Setup Guide for perl-lsp

Firebase Studio uses a VS Code-compatible editor surface, so perl-lsp works
through the same LSP wiring (`perllsp --stdio`) used by other VS Code-family
clients.

## Prerequisites

- `perllsp` installed in the workspace environment
- Firebase Studio workspace opened at your project root
- A Perl file (`.pl`, `.pm`, `.t`, etc.) in the workspace

Verify the binary in the integrated terminal:

```bash
perllsp --version
perllsp --health
```

## Recommended Setup (VS Code Extension Path)

1. Open the Extensions view in Firebase Studio.
2. Install the **Perl Language Server** extension (`EffortlessMetrics.perl-lsp-rs`).
3. Open a Perl file and confirm diagnostics/completion appear.

The extension auto-manages launch arguments and starts `perllsp` over stdio.

## Manual Setup (Generic LSP Client)

If you prefer a generic client configuration, point it at:

```json
{
  "command": "perllsp",
  "args": ["--stdio"]
}
```

Use the project root as workspace root so module resolution and indexing have
consistent include-path behavior.

## Firebase Studio Tips

- Add `perllsp` to your `.idx/dev.nix` so each imported workspace has the same
  toolchain.
- Keep your workspace opened at repository root (the folder containing `lib/`
  and/or `t/`) to avoid partial indexing.
- If the language server is not detected after install, reload the workspace and
  re-run `perllsp --health` in the integrated terminal.

## Troubleshooting

- No diagnostics/completion: confirm `perllsp --stdio` can be launched from the
  integrated terminal.
- Slow first response: wait for initial workspace indexing to complete.
- Missing cross-file navigation: verify the workspace root points at the
  repository, not a nested subfolder.

For broader troubleshooting, see [docs/how-to/TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
