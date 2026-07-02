# DAP User Guide: Debugging Perl with VS Code

<!-- Labels: tutorial:dap-setup, how-to:debugging, reference:configuration, phase:native-dap -->

> This guide follows the **[Diataxis framework](https://diataxis.fr/)** for comprehensive technical documentation:
> - **Tutorial sections**: Step-by-step learning for first-time DAP users
> - **How-to sections**: Task-oriented debugging workflows
> - **Reference sections**: Configuration specifications and options
> - **Explanation sections**: Understanding native DAP architecture and design

**Status**: Native `perl-dap` CLI for launch, attach, stepping, stack frames, variables, evaluate, and breakpoint validation.  
**Version**: 0.17.0  
**Date**: 2026-06-28

**Dependency note**: Native `perl-dap` requires a local Perl interpreter for debug sessions. Its Rust parser-backed runtime (`perl-parser`, `perl-lexer`, and the `perl-dap-*` support crates) is compiled into the shipped binary; users do not install parser crates separately.

---

## Table of Contents

- [Tutorial: Getting Started with Perl Debugging](#tutorial-getting-started-with-perl-debugging)
  - [Prerequisites](#prerequisites)
  - [Step 1: Configure VS Code](#step-1-configure-vs-code)
  - [Step 2: Your First Debugging Session](#step-2-your-first-debugging-session)
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
  - [Native Adapter Architecture](#native-adapter-architecture)
  - [Current Hardening Focus](#current-hardening-focus)
- [Troubleshooting](#troubleshooting)

---

## Tutorial: Getting Started with Perl Debugging

### Prerequisites

Before you begin debugging Perl code with VS Code, ensure you have:

1. **Perl installation**: Perl 5.10 or higher installed and available on PATH.
   ```bash
   perl --version
   ```

2. **VS Code**: Visual Studio Code 1.88 or higher with `EffortlessMetrics.perl-lsp-rs` installed.

3. **Operating system**: Windows, macOS, Linux, or WSL.

The extension manages the `perl-dap` binary from the `perl-lsp` release artifacts. You only need the local Perl interpreter that runs your script.

### Step 1: Configure VS Code

Create a launch configuration in your workspace to enable debugging.

1. **Open Command Palette**: Press `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (macOS).

2. **Create Debug Configuration**: Type "Debug: Open launch.json" and press Enter.

3. **Add Perl Configuration**: If prompted, select "Perl" as the environment. VS Code will generate a `.vscode/launch.json` file.

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
      "stopOnEntry": false
    }
  ]
}
```

**Configuration Explained**:

- `type`: Must be `"perl"` for Perl debugging.
- `request`: `"launch"` starts a new process; `"attach"` connects to an existing debug target.
- `name`: Display name in VS Code's debug dropdown.
- `program`: Path to the Perl script to debug. VS Code variables such as `${file}` are supported.
- `args`: Command-line arguments passed to your script.
- `perlPath`: Path to the Perl executable. Defaults to `"perl"` on PATH.
- `includePaths`: Additional directories added to `@INC` through `PERL5LIB`.
- `cwd`: Working directory for the debugged process.
- `env`: Environment variables to set for the debugged process.
- `stopOnEntry`: Pause at the first executable line before continuing.

### Step 2: Your First Debugging Session

Let's debug a simple Perl script to verify everything works.

1. **Create a test script** (`hello.pl`):

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

2. **Set a breakpoint**: Click in the gutter next to `print "$greeting\n";`.

3. **Start debugging**: Press `F5` or select **Run > Start Debugging**.

4. **Observe the debugger**:
   - Execution pauses at the breakpoint.
   - The Variables panel shows best-effort parsed values from debugger output.
   - The Call Stack shows your script in the execution context.

5. **Step through code**:
   - **Step Over** (`F10`): Execute current line and move to the next one.
   - **Step Into** (`F11`): Enter function calls.
   - **Step Out** (`Shift+F11`): Exit the current function.
   - **Continue** (`F5`): Resume execution until the next breakpoint.

6. **Inspect variables**:
   - Hover over variables to inspect parsed values.
   - Use the Variables panel to explore data structures with lazy expansion.
   - Use the Debug Console to evaluate Perl expressions. Safe mode performs syntactic validation and is not an interpreter sandbox.

7. **Stop debugging**: Press `Shift+F5` or click the red stop square in the debug toolbar.

---

## How-To: Common Debugging Scenarios

### Launch a Perl Script

**Use Case**: Debug a script from start to finish with full control over execution.

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug Script",
  "program": "${file}",
  "stopOnEntry": false
}
```

**Tips**:

- Use `${file}` to debug the currently open file.
- Set `stopOnEntry: true` to pause at the first line of code.
- Add `"args": ["--verbose", "--input=data.txt"]` for command-line arguments.

### Attach to a Running Process

**Use Case**: Connect to an already-running debug target.

The native adapter supports two attach modes:

- `processId`: local PID signal-control mode.
- `host`/`port`: TCP debugger endpoint.

**Attach by TCP**:

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

**Attach by PID**:

```json
{
  "type": "perl",
  "request": "attach",
  "name": "Attach by PID",
  "processId": 12345
}
```

**When to use attach**:

- Debugging long-running daemons or servers.
- Connecting to Perl processes started by external tools.
- Remote debugging scenarios where the TCP endpoint is exposed intentionally.

### Debug with Custom Include Paths

**Use Case**: Your Perl project uses custom library directories that need to be added to `@INC`.

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug with Custom Libs",
  "program": "${workspaceFolder}/bin/app.pl",
  "includePaths": [
    "${workspaceFolder}/lib",
    "${workspaceFolder}/local/lib/perl5",
    "/opt/custom/perl/lib"
  ]
}
```

**How it works**:

- Each path in `includePaths` is added to the `PERL5LIB` environment variable.
- Paths are platform-specific (`;` separator on Windows, `:` on Unix).
- Relative paths are resolved against the workspace root.

### Debug with Environment Variables

**Use Case**: Your script requires specific environment variables.

```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug with Environment",
  "program": "${workspaceFolder}/script.pl",
  "env": {
    "DEBUG": "1",
    "DATABASE_URL": "dbi:SQLite:dbname=test.db",
    "LOG_LEVEL": "debug"
  }
}
```

**Security note**: Avoid committing sensitive credentials to version control. Use VS Code variables or external configuration files:

```json
{
  "env": {
    "API_KEY": "${env:API_KEY}"
  }
}
```

### Debug on WSL or Remote Systems

**Use Case**: Develop on Windows but debug Perl code running in WSL.

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

**Platform-specific notes**:

- **WSL**: Paths starting with `/mnt/c` are automatically translated to `C:\`.
- **macOS**: Supports Homebrew Perl installations such as `/usr/local/bin/perl`.
- **Windows**: Handles UNC paths (`\\server\share`) and drive letters (`C:\`).

---

## Reference: Configuration Options

### Launch Configuration

| Property | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `type` | `string` | Yes | N/A | Must be `"perl"` |
| `request` | `string` | Yes | N/A | Must be `"launch"` |
| `name` | `string` | Yes | N/A | Display name in debug dropdown |
| `program` | `string` | Yes | N/A | Path to Perl script |
| `args` | `string[]` | No | `[]` | Command-line arguments for the script |
| `cwd` | `string` | No | `${workspaceFolder}` | Working directory for debugged process |
| `env` | `object` | No | `{}` | Environment variables |
| `perlPath` | `string` | No | `"perl"` | Path to the Perl executable |
| `includePaths` | `string[]` | No | `[]` | Additional directories for `@INC` |
| `stopOnEntry` | `boolean` | No | `false` | Pause at the first executable line |

### Attach Configuration

| Property | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `type` | `string` | Yes | N/A | Must be `"perl"` |
| `request` | `string` | Yes | N/A | Must be `"attach"` |
| `name` | `string` | Yes | N/A | Display name in debug dropdown |
| `processId` | `number` | No | N/A | Local process ID for signal-control attach mode |
| `host` | `string` | No | `"localhost"` | Hostname or IP address of DAP server |
| `port` | `number` | No | `13603` | Port number of DAP server |
| `timeout` | `number` | No | `5000` | Connection timeout in milliseconds |

### Advanced Settings

#### Path Normalization

The DAP adapter automatically normalizes paths across platforms:

- **Windows**: Drive letters are uppercased, UNC paths are preserved.
- **WSL**: WSL paths are translated when the target platform is known.
- **macOS/Linux**: Symlinks are canonicalized and redundant separators are removed.

#### Environment Setup

The adapter sets `PERL5LIB` from `includePaths`:

```bash
# Unix/macOS:
PERL5LIB=/workspace/lib:/custom/lib perl script.pl

# Windows:
PERL5LIB=C:\workspace\lib;C:\custom\lib perl script.pl
```

#### Argument Escaping

Arguments with spaces are quoted platform-appropriately:

```json
{
  "args": ["--file", "path with spaces.txt", "--verbose"]
}
```

---

## Explanation: Native DAP Architecture

### Native Adapter Architecture

The `perl-dap` CLI is the native Debug Adapter Protocol server for Perl. It speaks DAP over stdio or TCP and drives debug sessions through the local Perl interpreter.

```text
VS Code / DAP client
        │ DAP over stdio or TCP
        ▼
perl-dap (Rust)
  - DAP request routing
  - parser-backed breakpoint validation
  - stack frame / variable / evaluate handling
  - path and environment setup
        │
        ▼
local Perl interpreter / debuggee
```

The parser-backed runtime is part of the shipped Rust binary. Users install the editor extension or release artifact; they do not install internal Rust crates separately.

### Current Hardening Focus

The native adapter already supports launch, attach, stepping, stack frames, variables, evaluate, and parser-backed breakpoint validation. Current work focuses on:

- Faster breakpoint/source updates from incremental parser integration.
- Deeper workspace-aware debugging flows.
- Broader protocol parity across editors.
- Continued release-artifact and editor-integration validation.

---

## Troubleshooting

### Perl Binary Not Found on PATH

**Symptom**: Error "perl binary not found on PATH" when launching the debugger.

**Solution**:

1. Verify Perl is installed:

   ```bash
   which perl  # Unix/macOS
   where perl  # Windows
   ```

2. Add Perl to PATH or specify an absolute path in `launch.json`:

   ```json
   {
     "perlPath": "/usr/local/bin/perl"
   }
   ```

### Breakpoints Not Hitting

**Symptom**: Breakpoints are shown as unresolved or the debugger does not stop.

**Common causes**:

1. **Wrong file path**: Ensure `program` in `launch.json` matches the file with breakpoints.
2. **Syntax errors**: Fix Perl syntax errors that prevent script startup.
3. **Non-executable locations**: Breakpoints on blank lines, comments, POD, and heredoc interiors are rejected by parser-backed validation where source context is available.

**Solution**:

- Set breakpoints on executable Perl statements.
- Check the Debug Console for error messages.
- Try `"stopOnEntry": true` to verify debugger startup.

### Path Issues on WSL

**Symptom**: "Program file does not exist" when debugging on WSL.

**Solution**:

1. Use WSL-style paths in `launch.json`.
2. Let the adapter normalize paths automatically.
3. Verify the file exists in WSL.

### Environment Variables Not Working

**Symptom**: Script does not see environment variables set in `launch.json`.

**Solution**:

1. Verify syntax in `launch.json`.
2. Use `${env:NAME}` to read shell environment variables.
3. Check the Debug Console for environment-related messages.

### Slow Debugger Startup

**Common causes**:

- Large Perl modules with heavy initialization.
- Slow filesystem paths.
- Many `@INC` directories to scan.

**Solution**:

- Reduce `includePaths` to only necessary directories.
- Use local filesystem paths where possible.
- Optimize module loading in your Perl code.

### Debugger Crashes or Hangs

**Solution**:

1. Check Debug Console output.
2. Restart VS Code with **Developer: Reload Window**.
3. Verify the script runs without the debugger:

   ```bash
   perl script.pl
   ```

4. Report an issue with:
   - Debug Console output.
   - VS Code version.
   - Perl version.
   - Operating system.

---

## Getting Help

- **Documentation**: See [DAP Implementation Specification](../../docs/reference/DAP_IMPLEMENTATION_SPECIFICATION.md) for technical details.
- **Security**: See [DAP Security Specification](../../docs/DAP_SECURITY_SPECIFICATION.md) for security considerations.
- **Architecture**: See [Crate Architecture Guide](../../docs/reference/CRATE_ARCHITECTURE_GUIDE.md) for DAP crate design.
- **Issues**: Report bugs at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues).

---

**Version History**:

- **0.17.0** (2026-06-28): Native `perl-dap` guide refreshed as the first-mile DAP path.
