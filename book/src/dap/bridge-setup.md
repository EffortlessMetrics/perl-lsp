# DAP Bridge Setup Guide

This guide describes how to set up the Debug Adapter Protocol (DAP) bridge for the Perl Language Server. This feature allows you to debug Perl scripts directly in VS Code using the `Perl::LanguageServer` backend.

## Prerequisites

1. **Perl Installation**: Perl 5.10 or later must be installed.
2. **Perl::LanguageServer Module**: The bridge requires the Perl module to communicate with the debug backend.

### Installing Perl::LanguageServer

Install the required Perl module using `cpan` or `cpanm`:

```bash
cpan Perl::LanguageServer
# or
cpanm Perl::LanguageServer
```

Ensure the `perl` executable used by VS Code has access to this module.

## Configuration

You can configure debugging in VS Code using `launch.json`.

### Launch Configuration

To launch a Perl script in debug mode:

```json
{
    "type": "perl",
    "request": "launch",
    "name": "Perl: Launch Script",
    "program": "${workspaceFolder}/script.pl",
    "stopOnEntry": true,
    "args": [],
    "env": {}
}
```

### Attach Configuration

To attach to a running Perl debugging session:

```json
{
    "type": "perl",
    "request": "attach",
    "name": "Perl: Attach",
    "port": 5000,
    "host": "localhost"
}
```

## Troubleshooting

### Debugger Fails to Start

- **Error**: "Perl::LanguageServer not found"
  - **Fix**: Ensure `Perl::LanguageServer` is installed in your Perl environment (`cpan -l Perl::LanguageServer`).
  - **Fix**: Check `perl-lsp.serverPath` setting in VS Code if you are using a custom Perl location.

### Breakpoints Not Hitting

- **Cause**: Path mapping issues between VS Code and Perl interpreter.
- **Fix**: Ensure `program` path matches the file location on disk.
- **Fix**: On Windows, check for drive letter inconsistencies (C: vs c:).

### Connection Refused (Attach Mode)

- **Cause**: The Perl process is not running or listening on the specified port.
- **Fix**: Start the Perl process with debug arguments before attaching.