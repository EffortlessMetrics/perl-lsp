# Cursor Setup Guide for perl-lsp

Cursor is VS Code-compatible, so the fastest setup path is to use the same
`perl-lsp-rs` extension published for VS Code.

## Prerequisites

- Cursor installed
- Internet access to install extensions
- A Perl workspace folder opened in Cursor

## Fast Path (Recommended)

1. Open **Extensions** in Cursor.
2. Search for **Perl Language Server** (`EffortlessMetrics.perl-lsp-rs`).
3. Install the extension.
4. Reload Cursor when prompted.

The extension auto-downloads a matching `perllsp` binary for your platform.

## Verify the Server Is Running

1. Open a Perl file (`.pl`, `.pm`, or `.t`).
2. Run **“Perl: Show Language Server Status”** from the command palette.
3. Confirm features like hover (`K` / mouse hover), go-to-definition, and
   completion are active.

## Manual Fallback (No Extension)

If extension install is blocked in your environment, run `perllsp` directly via
Cursor's generic LSP/client settings:

- Command: `perllsp`
- Args: `--stdio`
- Root: project/workspace folder

Then verify in a terminal:

```bash
perllsp --version
perllsp --health
```

## Troubleshooting

- If Cursor cannot find `perllsp`, launch Cursor from a shell that already has
  `perllsp` on `PATH`.
- If syntax works but IDE features do not, make sure the file language mode is
  Perl.
- If startup still fails, follow [general troubleshooting](../how-to/TROUBLESHOOTING.md).
