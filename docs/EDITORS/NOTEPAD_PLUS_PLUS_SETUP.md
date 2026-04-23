# Notepad++ Setup Guide for perl-lsp

This guide shows a minimal, reliable Notepad++ setup for `perllsp` using the
community **LSP Client** plugin.

## Prerequisites

- Notepad++ (Windows)
- `perllsp` installed and available on `PATH`
- Notepad++ plugin **LSP Client** installed from Plugins Admin

Verify the language server first from `cmd.exe` or PowerShell:

```powershell
perllsp --version
perllsp --health
```

## 1) Install the LSP Client plugin

1. Open **Plugins → Plugins Admin...**
2. Search for **LSP Client**
3. Install and restart Notepad++

## 2) Add a perl-lsp client definition

1. Open **Plugins → LSP Client → Settings**
2. Add a client entry that launches `perllsp --stdio`

Use this baseline JSON as a starting point:

```json
{
  "clients": {
    "perl-lsp": {
      "command": ["perllsp", "--stdio"],
      "languages": ["perl"],
      "enabled": true
    }
  }
}
```

If your plugin version uses a different schema, keep the same essentials:

- command: `perllsp --stdio`
- language/file association: Perl (`.pl`, `.pm`, `.t`, `.psgi`)

## 3) Ensure Perl files map to a Perl language

If `.t` or `.psgi` files do not trigger LSP features automatically, map them to
Perl in your Notepad++ language associations so the plugin can attach to the
server.

## 4) Validate end-to-end

1. Open a project folder and a Perl file.
2. Wait for the plugin status to show the client attached.
3. Confirm diagnostics and hover/completion are working.

## Troubleshooting

- **Server not found**: restart Notepad++ from a shell where `perllsp` is on
  `PATH`, or use the absolute path to `perllsp.exe`.
- **No features in open file**: verify the file is recognized as Perl and that
  the plugin actually started the `perl-lsp` client entry.
- **Intermittent startup issues**: enable plugin logging and confirm the launch
  command is exactly `perllsp --stdio`.

For broader diagnostics, see [Troubleshooting](../how-to/TROUBLESHOOTING.md).
