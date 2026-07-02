# DAP User Guide: Debugging Perl with VS Code
<!-- Labels: tutorial:dap-setup, how-to:debugging, reference:configuration, phase:native-dap -->

This guide covers the native `perl-dap` Debug Adapter Protocol server shipped with `perl-lsp`.

**Status**: Native `perl-dap` CLI for launch, attach, stepping, stack frames, variables, evaluate, and parser-backed breakpoint validation.  
**Version**: 0.17.0  
**Date**: 2026-06-28

**Dependency note**: Native `perl-dap` requires a local Perl interpreter for debug sessions. Its Rust parser-backed runtime (`perl-parser`, `perl-lexer`, and the `perl-dap-*` support crates) is compiled into the shipped binary; users do not install parser crates separately.

---

## Table of Contents

- [Tutorial: Getting Started with Perl Debugging](#tutorial-getting-started-with-perl-debugging)
  - [Prerequisites](#prerequisites)
  - [Configure VS Code](#configure-vs-code)
  - [First Debugging Session](#first-debugging-session)
- [How-To: Common Debugging Scenarios](#how-to-common-debugging-scenarios)
  - [Launch a Perl Script](#launch-a-perl-script)
  - [Attach to a Running Process](#attach-to-a-running-process)
  - [Debug with Custom Include Paths](#debug-with-custom-include-paths)
  - [Debug with Environment Variables](#debug-with-environment-variables)
  - [Debug on WSL or Remote Systems](#debug-on-wsl-or-remote-systems)
- [Reference: Configuration Options](#reference-configuration-options)
  - [Launch Configuration](#launch-configuration)
  - [Attach Configuration](#attach-configuration)
  - [Advanced Settings](#advanced-settings)
- [Explanation: Native DAP Architecture](#explanation-native-dap-architecture)
- [Troubleshooting](#troubleshooting)

---

## Tutorial: Getting Started with Perl Debugging

### Prerequisites

Before debugging Perl code with VS Code, ensure you have:

1. **Perl**: Perl 5.10 or higher installed and available on PATH.
   ```bash
   perl --version
   ```

2. **VS Code**: Visual Studio Code with `EffortlessMetrics.perl-lsp-rs` installed.

3. **Operating system**: Windows, macOS, Linux, or WSL.

The VS Code extension downloads the managed `perl-dap` binary from the `perl-lsp` release artifacts. You do not install internal Rust crates or Perl debug-adapter modules separately.

### Configure VS Code

Create a launch configuration in your workspace.

1. Open the Command Palette.
2. Run **Debug: Open launch.json**.
3. Choose **Perl** if prompted.

**Basic launch.json**:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Launch Perl Script",
      "program": "${workspaceFolder}/script.pl",
      "args": [],
      "perlPath": "perl",
      "includePaths": ["${workspaceFolder}/lib"],
      "cwd": "${workspaceFolder}",
      "env": {},
      "stopOnEntry": true
    }
  ]
}
```

**Configuration fields**:

- `type`: Must be `"perl"`.
- `request`: `"launch"` starts a new process; `"attach"` connects to an existing debug target.
- `program`: Path to the Perl script to debug.
- `args`: Command-line arguments passed to the script.
- `perlPath`: Perl executable path. Defaults to `"perl"` on PATH.
- `includePaths`: Additional library directories added to the debug session environment.
- `cwd`: Working directory for the debugged process.
- `env`: Environment variables for the debugged process.
- `stopOnEntry`: Pause at the first executable statement.

### First Debugging Session

Create `hello.pl`:

```perl
#!/usr/bin/env perl
use strict;
use warnings;

my $name = "World";
my $greeting = "Hello, $name!";

print "$greeting\n";

for my $i (1..3) {
    print "Count: $i\n";
}

print "Done!\n";
```

Then:

1. Set a breakpoint in the editor gutter.
2. Press `F5` or run **Run > Start Debugging**.
3. Use the Debug toolbar for Continue, Step Over, Step Into, and Step Out.
4. Inspect variables in the Variables panel and evaluate expressions in the Debug Console.
5. Stop debugging with `Shift+F5`.

---

## How-To: Common Debugging Scenarios

### Launch a Perl Script

Use this when you want the debug adapter to start the process.

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug Current File",
  "program": "${file}",
  "perlPath": "perl",
  "cwd": "${fileDirname}",
  "stopOnEntry": false
}
```

Tips:

- Use `${file}` to debug the active file.
- Use `stopOnEntry: true` when validating setup.
- Add `args` for script arguments.

### Attach to a Running Process

The native adapter supports two attach shapes:

- `processId`: local PID attach.
- `host`/`port`: TCP debugger endpoint attach.

**TCP attach**:

```json
{
  "type": "perl",
  "request": "attach",
  "name": "Attach by TCP",
  "host": "localhost",
  "port": 13603,
  "timeout": 5000
}
```

**PID attach**:

```json
{
  "type": "perl",
  "request": "attach",
  "name": "Attach by PID",
  "processId": 12345
}
```

Use attach when debugging long-running services, externally launched processes, or remote workflows with an exposed TCP endpoint.

### Debug with Custom Include Paths

Use `includePaths` when the project uses local libraries that must be visible to the debuggee.

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug with Custom Libs",
  "program": "${workspaceFolder}/bin/app.pl",
  "includePaths": [
    "${workspaceFolder}/lib",
    "${workspaceFolder}/local/lib/perl5"
  ]
}
```

### Debug with Environment Variables

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug with Environment",
  "program": "${workspaceFolder}/script.pl",
  "env": {
    "DEBUG": "1",
    "DATABASE_URL": "dbi:SQLite:dbname=test.db"
  }
}
```

Prefer VS Code environment substitution for secrets:

```json
{
  "env": {
    "API_KEY": "${env:API_KEY}"
  }
}
```

### Debug on WSL or Remote Systems

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug in WSL",
  "program": "${workspaceFolder}/script.pl",
  "perlPath": "/usr/bin/perl",
  "cwd": "${workspaceFolder}"
}
```

Platform notes:

- **WSL**: Use the Perl executable inside the WSL environment.
- **macOS**: Homebrew Perl paths work when supplied through `perlPath`.
- **Windows**: UNC paths and drive letters are normalized by the adapter.

---

## Reference: Configuration Options

### Launch Configuration

| Property | Type | Required | Default | Description |
|---|---|---:|---|---|
| `type` | `string` | Yes | — | Must be `"perl"` |
| `request` | `string` | Yes | — | Must be `"launch"` |
| `name` | `string` | Yes | — | Display name in the debug dropdown |
| `program` | `string` | Yes | — | Perl script path |
| `args` | `string[]` | No | `[]` | Script arguments |
| `cwd` | `string` | No | `${workspaceFolder}` | Working directory |
| `env` | `object` | No | `{}` | Environment variables |
| `perlPath` | `string` | No | `"perl"` | Perl executable path |
| `includePaths` | `string[]` | No | `[]` | Additional library paths |
| `stopOnEntry` | `boolean` | No | `false` | Pause at entry |

VS Code variables such as `${workspaceFolder}`, `${file}`, `${fileDirname}`, and `${env:VAR_NAME}` are supported by the editor before the adapter receives the configuration.

### Attach Configuration

| Property | Type | Required | Default | Description |
|---|---|---:|---|---|
| `type` | `string` | Yes | — | Must be `"perl"` |
| `request` | `string` | Yes | — | Must be `"attach"` |
| `name` | `string` | Yes | — | Display name in the debug dropdown |
| `processId` | `number` | No | — | Local process ID |
| `host` | `string` | No | `localhost` | TCP endpoint host |
| `port` | `number` | No | `13603` | TCP endpoint port |
| `timeout` | `number` | No | `5000` | TCP attach timeout in milliseconds |

### Advanced Settings

#### Path normalization

The native adapter normalizes paths across supported platforms:

- Windows drive letters are normalized.
- UNC paths are preserved.
- WSL and Unix-like paths are handled according to the active debug environment.

#### Environment setup

`includePaths` are passed to the debug session environment so project libraries can be resolved during debugging. Keep launch-specific runtime paths in `launch.json`; keep editor indexing paths in `perl-lsp` workspace settings.

---

## Explanation: Native DAP Architecture

`perl-dap` is the native Debug Adapter Protocol server for Perl. It speaks DAP over stdio or TCP and drives debug sessions through the local Perl interpreter.

```text
VS Code / DAP client
        │ DAP over stdio or TCP
        ▼
perl-dap (Rust)
  - request routing
  - breakpoint validation
  - stack frame, variable, and evaluate handling
  - path and environment setup
        │
        ▼
local Perl interpreter / debuggee
```

The shipped binary includes the Rust parser-backed runtime used for breakpoint validation and source-aware behavior. External comparison tools are not part of the native DAP runtime.

### Current hardening focus

The native adapter already supports launch, attach, stepping, stack frames, variables, evaluate, and parser-backed breakpoint validation. Current work focuses on:

- faster breakpoint/source updates from incremental parser integration,
- deeper workspace-aware debugging flows,
- broader protocol parity across editors,
- continued release-artifact and editor-integration validation.

---

## Troubleshooting

### Perl binary not found

**Symptom**: The debugger cannot launch because Perl is unavailable.

**Fix**:

1. Verify Perl is installed:
   ```bash
   which perl  # Unix/macOS
   where perl  # Windows
   ```
2. Add Perl to PATH, or set `perlPath` explicitly:
   ```json
   {
     "perlPath": "/usr/local/bin/perl"
   }
   ```

### Breakpoints not hitting

Common causes:

1. `program` points to a different file than the one with breakpoints.
2. Syntax errors prevent the script from running.
3. Breakpoints are placed on comments, blank lines, POD, or non-executable locations.

Try `stopOnEntry: true` to confirm the adapter starts and receives the launch configuration.

### Variables not shown

Variable rendering is best-effort and derived from debugger output. For complex structures, expand variables lazily in the Variables panel and use the Debug Console for targeted expression evaluation.

### Path issues

Use absolute paths while troubleshooting. Once the debug session works, move back to VS Code variables such as `${workspaceFolder}` and `${file}`.
