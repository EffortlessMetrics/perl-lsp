# Claude Code Setup

This guide shows how to wire `perllsp` into Claude Code as an external Language
Server Protocol (LSP) backend.

> If `perllsp` is not installed yet, follow [INSTALLATION.md](../how-to/INSTALLATION.md)
> first.

## 1) Verify the server binary

```bash
perllsp --version
perllsp --health
```

Both commands should succeed before you configure Claude Code.

## 2) Configure Claude Code to launch `perllsp`

Use this command in your Claude Code LSP settings:

```text
perllsp --stdio
```

Use your Perl workspace root as the project folder so workspace-scoped features
(rename, references, symbols, diagnostics) can resolve files correctly.

## 3) Recommended initialize payload fields

If Claude Code lets you customize initialize parameters, include:

- `rootUri` (or `workspaceFolders`)
- `capabilities` (may be minimal `{}`)
- `clientInfo` (recommended, optional)

Example payload skeleton:

```json
{
  "rootUri": "file:///path/to/project",
  "capabilities": {},
  "clientInfo": {
    "name": "claude-code"
  }
}
```

`perllsp` accepts minimal capabilities and negotiates feature support from what
the client declares.

## 4) Smoke test after connecting

Open a `.pl` or `.pm` file and validate these quickly:

1. Diagnostics appear for obvious syntax errors.
2. Hover works on builtins/symbols.
3. Go to definition resolves local symbols.
4. Completion appears inside Perl code.

If any step fails, use [TROUBLESHOOTING.md](../how-to/TROUBLESHOOTING.md).
