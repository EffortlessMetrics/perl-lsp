# DAP User Guide: Debugging Perl with VS Code
<!-- Labels: tutorial:dap-setup, how-to:debugging, reference:configuration, phase:bridge-implementation -->

> This guide follows the **[Diataxis framework](https://diataxis.fr/)** for comprehensive technical documentation:
> - **Tutorial sections**: Step-by-step learning for first-time DAP users
> - **How-to sections**: Task-oriented debugging workflows
> - **Reference sections**: Configuration specifications and options
> - **Explanation sections**: Understanding DAP architecture and design

**Status**: Native adapter CLI (launch + attach + stepping + evaluate) + BridgeAdapter guide (Perl::LanguageServer)
**Version**: 0.9.x
**Date**: 2025-10-04

**Note**: The `perl-dap` CLI runs the native adapter (launch + attach) and does not require Perl::LanguageServer. The bridge adapter steps below apply only if you are running the BridgeAdapter library or Perl::LanguageServer DAP directly.

---

## Table of Contents

- [Tutorial: Getting Started with Perl Debugging](#tutorial-getting-started-with-perl-debugging)
  - [Prerequisites](#prerequisites)
  - [Step 1 (BridgeAdapter only): Install Perl::LanguageServer](#step-1-bridgeadapter-only-install-perllanguageserver)
  - [Step 2: Configure VS Code](#step-2-configure-vs-code)
  - [Step 3: Your First Debugging Session](#step-3-your-first-debugging-session)
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
- [Explanation: DAP Architecture](#explanation-dap-architecture)
  - [Adapter Modes (Native CLI + BridgeAdapter)](#adapter-modes-native-cli--bridgeadapter)
  - [Future Roadmap](#future-roadmap)
- [Troubleshooting](#troubleshooting)

---

## Tutorial: Getting Started with Perl Debugging

### Prerequisites

Before you begin debugging Perl code with VS Code, ensure you have:

1. **Perl Installation**: Perl 5.10 or higher installed and available on PATH
   ```bash
   perl --version  # Should output Perl version
   ```

2. **VS Code**: Visual Studio Code 1.70 or higher with Perl LSP extension installed

3. **Operating System**: Windows, macOS, Linux, or WSL

4. **Perl::LanguageServer** (BridgeAdapter only): required for the bridge path

### Step 1 (BridgeAdapter only): Install Perl::LanguageServer

The DAP bridge requires the Perl::LanguageServer CPAN module for debugging functionality.

**Install via CPAN**:
```bash
cpan Perl::LanguageServer
```

**Install via cpanm** (recommended):
```bash
cpanm Perl::LanguageServer
```

**Verify Installation**:
```bash
perl -e "use Perl::LanguageServer::DebuggerInterface; print qq{OK\n};"
```

If the verification succeeds, you'll see `OK` printed. If you see an error, the module installation failed.

### Step 2: Configure VS Code

Create a launch configuration in your workspace to enable debugging.

1. **Open Command Palette**: Press `Ctrl+Shift+P` (Windows/Linux) or `Cmd+Shift+P` (macOS)

2. **Create Debug Configuration**: Type "Debug: Open launch.json" and press Enter

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
      "env": {}
    }
  ]
}
```

**Configuration Explained**:
- `type`: Must be `"perl"` for Perl debugging
- `request`: `"launch"` to start a new process, `"attach"` to connect to a running process/debugger
- `name`: Display name in VS Code's debug dropdown
- `program`: Path to the Perl script to debug (supports VS Code variables like `${file}`)
- `args`: Command-line arguments to pass to your script
- `perlPath`: Path to perl binary (defaults to `"perl"` on PATH)
- `includePaths`: Additional directories to add to `@INC` (Perl's include path)
- `cwd`: Working directory for the debugged process
- `env`: Environment variables to set for the debugged process

### Step 3: Your First Debugging Session

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

2. **Set a breakpoint**: Click in the gutter (left of line numbers) at line 8 (`print "$greeting\n";`). A red dot appears.

3. **Start debugging**: Press `F5` or select "Run > Start Debugging" from the menu.

4. **Observe the debugger**:
   - Execution pauses at line 8
   - Variables panel shows best-effort parsed values from debugger output
   - Call stack shows your script in the execution context

5. **Step through code**:
   - **Step Over** (`F10`): Execute current line, move to next
   - **Step Into** (`F11`): Enter function calls
   - **Step Out** (`Shift+F11`): Exit current function
   - **Continue** (`F5`): Resume execution until next breakpoint

6. **Inspect variables**:
   - Hover over variables to inspect parsed values
   - Use the Variables panel to explore data structures with lazy expansion
   - Use the Debug Console to evaluate Perl expressions (safe mode by default)

7. **Stop debugging**: Press `Shift+F5` or click the red stop square in the debug toolbar.

**Congratulations!** You've successfully debugged your first Perl script with VS Code.

---

## How-To: Common Debugging Scenarios

### Launch a Perl Script

**Use Case**: Debug a script from start to finish with full control over execution.

**Configuration**:
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
- Use `${file}` to debug the currently open file
- Set `stopOnEntry: true` to pause at the first line of code
- Add `"args": ["--verbose", "--input=data.txt"]` for command-line arguments

### Attach to a Running Process

**Use Case**: Connect to an already-running debug target.

The native adapter supports two attach modes:
- `processId`: local PID signal-control mode
- `host`/`port`: TCP debugger endpoint (for shim/bridge workflows)

**Attach Configuration**:
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

**When to Use**:
- Debugging long-running daemons or servers
- Connecting to Perl processes started by external tools
- Remote debugging scenarios (change `host` to remote IP)

### Debug with Custom Include Paths

**Use Case**: Your Perl project uses custom library directories that need to be added to `@INC`.

**Configuration**:
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

**How It Works**:
- Each path in `includePaths` is added to `PERL5LIB` environment variable
- Paths are platform-specific (`;` separator on Windows, `:` on Unix)
- Relative paths are resolved against `${workspaceFolder}`

### Debug with Environment Variables

**Use Case**: Your script requires specific environment variables (API keys, database URLs, feature flags).

**Configuration**:
```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug with Environment",
  "program": "${workspaceFolder}/script.pl",
  "env": {
    "DEBUG": "1",
    "DATABASE_URL": "dbi:SQLite:dbname=test.db",
    "API_KEY": "your-api-key-here",
    "LOG_LEVEL": "debug"
  }
}
```

**Security Note**: Avoid committing sensitive credentials to version control. Use VS Code variables or external configuration files:

```json
{
  "env": {
    "API_KEY": "${env:API_KEY}"
  }
}
```

**Note**: The `${env:API_KEY}` syntax reads from your shell environment, avoiding hardcoded secrets.

### Debug on WSL or Remote Systems

**Use Case**: Develop on Windows but debug Perl code running in WSL (Windows Subsystem for Linux).

**WSL Configuration**:
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

**Platform-Specific Notes**:
- **WSL**: Paths starting with `/mnt/c` are automatically translated to `C:\`
- **macOS**: Supports Homebrew perl installations (e.g., `/usr/local/bin/perl`)
- **Windows**: Handles UNC paths (`\\server\share`) and drive letters (`C:\`)

---

## Reference: Configuration Options

### Launch Configuration

Complete schema for `"request": "launch"` configurations.

| Property | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `type` | `string` | ✅ Yes | N/A | Must be `"perl"` |
| `request` | `string` | ✅ Yes | N/A | Must be `"launch"` for launching new process |
| `name` | `string` | ✅ Yes | N/A | Display name in debug dropdown |
| `program` | `string` | ✅ Yes | N/A | Path to Perl script (absolute or relative to workspace) |
| `args` | `string[]` | ❌ No | `[]` | Command-line arguments for the script |
| `cwd` | `string` | ❌ No | `${workspaceFolder}` | Working directory for debugged process |
| `env` | `object` | ❌ No | `{}` | Environment variables (key-value pairs) |
| `perlPath` | `string` | ❌ No | `"perl"` | Path to perl binary |
| `includePaths` | `string[]` | ❌ No | `[]` | Additional directories for `@INC` (sets `PERL5LIB`) |
| `stopOnEntry` | `boolean` | ❌ No | `false` | Pause execution at first line of code |

**VS Code Variable Substitution**:

Launch configurations support VS Code variables for dynamic paths:

- `${workspaceFolder}`: Absolute path to the workspace folder
- `${file}`: Absolute path to the currently open file
- `${fileBasename}`: Name of the currently open file (e.g., `script.pl`)
- `${fileDirname}`: Directory containing the currently open file
- `${env:VAR_NAME}`: Value of environment variable `VAR_NAME`

**Example with Variables**:
```json
{
  "type": "perl",
  "request": "launch",
  "name": "Debug Current File",
  "program": "${file}",
  "cwd": "${fileDirname}",
  "includePaths": ["${workspaceFolder}/lib"],
  "env": {
    "HOME": "${env:HOME}"
  }
}
```

### Attach Configuration

Complete schema for `"request": "attach"` configurations.

| Property | Type | Required | Default | Description |
|----------|------|----------|---------|-------------|
| `type` | `string` | ✅ Yes | N/A | Must be `"perl"` |
| `request` | `string` | ✅ Yes | N/A | Must be `"attach"` for connecting to running process |
| `name` | `string` | ✅ Yes | N/A | Display name in debug dropdown |
| `processId` | `number` | ❌ No | N/A | Local process ID for signal-control attach mode |
| `host` | `string` | ❌ No | `"localhost"` | Hostname or IP address of DAP server |
| `port` | `number` | ❌ No | `13603` | Port number of DAP server |
| `timeout` | `number` | ❌ No | `5000` | Connection timeout in milliseconds |

**Example Attach Configuration**:
```json
{
  "type": "perl",
  "request": "attach",
  "name": "Attach to Remote Perl Process",
  "host": "192.168.1.100",
  "port": 13603,
  "timeout": 10000
}
```

### Advanced Settings

#### Path Normalization

The DAP adapter automatically normalizes paths across platforms:

- **Windows**: Drive letters uppercased (`c:\` → `C:\`), UNC paths preserved (`\\server\share`)
- **WSL**: WSL paths translated (`/mnt/c/Users/Name` → `C:\Users\Name`)
- **macOS/Linux**: Symlinks canonicalized, redundant separators removed

#### Environment Setup

The adapter sets `PERL5LIB` from `includePaths`:

```bash
# For includePaths: ["/workspace/lib", "/custom/lib"]
# Unix/macOS:
PERL5LIB=/workspace/lib:/custom/lib perl script.pl

# Windows:
PERL5LIB=C:\workspace\lib;C:\custom\lib perl script.pl
```

#### Argument Escaping

Arguments with spaces are automatically quoted platform-appropriately:

```json
{
  "args": ["--file", "path with spaces.txt", "--verbose"]
}
```

**Becomes**:
- **Windows**: `--file "path with spaces.txt" --verbose`
- **Unix**: `--file 'path with spaces.txt' --verbose`

---

## Explanation: DAP Architecture

### Adapter Modes (Native CLI + BridgeAdapter)

The `perl-dap` CLI uses the native adapter to drive `perl -d` directly. A BridgeAdapter library is available to proxy between VS Code and Perl::LanguageServer, but it is not wired into the CLI yet.

```
┌─────────────────────────────────────────────────────────────┐
│                    VS Code Extension                        │
│  - DAP client (JSON-RPC 2.0 over stdio)                     │
│  - Launch configuration management                          │
└───────────────────────────┬─────────────────────────────────┘
                            │ DAP Protocol (stdio)
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                     perl-dap (Rust)                         │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ DebugAdapter (native, CLI default)                    │  │
│  │  - Drives perl -d directly                             │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ BridgeAdapter (library)                               │  │
│  │  - Proxies to Perl::LanguageServer DAP                │  │
│  └───────────────────────────────────────────────────────┘  │
│  ┌───────────────────────────────────────────────────────┐  │
│  │ Platform Layer                                        │  │
│  │  - Cross-platform perl binary resolution             │  │
│  │  - Path normalization (Windows/WSL/macOS/Linux)      │  │
│  │  - Environment variable setup (PERL5LIB)             │  │
│  └───────────────────────────────────────────────────────┘  │
└───────────────────────────┬─────────────────────────────────┘
                            │ perl -d / Perl::LanguageServer
                            ↓
┌─────────────────────────────────────────────────────────────┐
│                      Perl Runtime                           │
└─────────────────────────────────────────────────────────────┘
```

**BridgeAdapter Compatibility Role**:

1. **Interoperability**: Supports environments already standardized on Perl::LanguageServer DAP
2. **Migration Path**: Lets teams move incrementally from bridge-based workflows to native `perl-dap`
3. **Isolation**: Bridge code remains separate from native adapter runtime paths

**BridgeAdapter Trade-offs**:

- ✅ **Pros**: Fast implementation, proven debugging backend, cross-platform compatibility
- ⚠️ **Cons**: External dependency on Perl::LanguageServer CPAN module, additional process overhead

### Future Roadmap

**Native Adapter Roadmap**

The CLI already uses the native adapter with launch + attach + evaluation support. Remaining roadmap items focus on deeper protocol parity and hardening.

**Planned Features**:
- AST-based breakpoint validation (leveraging `perl-parser`)
- Incremental parsing integration (<1ms breakpoint updates)
- Workspace navigation for cross-file debugging
- Enhanced performance (<50ms breakpoint operations)

**Phase 3: Production Hardening (Planned)**

- Comprehensive security validation (path traversal prevention, safe eval)
- Performance benchmarking and optimization
- Advanced DAP features (conditional breakpoints, logpoints, hit counts)
- Editor integration (Neovim, Emacs, Helix)

**Migration Path**: BridgeAdapter users can keep their configuration if/when CLI wiring is added.

---

## Troubleshooting

### Perl::LanguageServer Not Found (BridgeAdapter only)

**Symptom**: Error message "Failed to spawn Perl::LanguageServer DAP process" when starting debugger.

**Solution**:
1. Verify installation:
   ```bash
   perl -e "use Perl::LanguageServer::DebuggerInterface; print qq{OK\n};"
   ```

2. If module not found, install:
   ```bash
   cpanm Perl::LanguageServer
   ```

3. Check CPAN installation path is in `@INC`:
   ```bash
   perl -V
   ```

### Perl Binary Not Found on PATH

**Symptom**: Error "perl binary not found on PATH" when launching debugger.

**Solution**:
1. Verify perl is installed:
   ```bash
   which perl  # Unix/macOS
   where perl  # Windows
   ```

2. Add perl to PATH or specify absolute path in `launch.json`:
   ```json
   {
     "perlPath": "/usr/local/bin/perl"  // Use actual path from 'which perl'
   }
   ```

### Breakpoints Not Hitting

**Symptom**: Breakpoints shown as gray circles, not red dots. Debugger doesn't stop.

**Common Causes**:
1. **Wrong file path**: Ensure `program` in `launch.json` matches the file with breakpoints
2. **Syntax errors**: Fix Perl syntax errors that prevent script from running
3. **Unverified breakpoints**: Perl::LanguageServer may reject breakpoints in invalid locations (comments, blank lines)

**Solution**:
- Set breakpoints on executable Perl statements (not comments or blank lines)
- Check Debug Console for error messages
- Try `"stopOnEntry": true` to verify debugger starts

### Path Issues on WSL

**Symptom**: "Program file does not exist" error when debugging on WSL.

**Solution**:
1. Use WSL-style paths in `launch.json`:
   ```json
   {
     "program": "${workspaceFolder}/script.pl",  // Correct
     "program": "C:\\Users\\Name\\script.pl"     // Wrong - use WSL path
   }
   ```

2. Let the adapter normalize paths automatically
3. Verify file exists in WSL:
   ```bash
   ls -l /mnt/c/Users/Name/workspace/script.pl
   ```

### Environment Variables Not Working

**Symptom**: Script doesn't see environment variables set in `launch.json`.

**Solution**:
1. Verify syntax in `launch.json`:
   ```json
   {
     "env": {
       "DEBUG": "1",           // Correct
       "LOG_LEVEL": "debug"    // Correct
     }
   }
   ```

2. Use shell environment variables:
   ```json
   {
     "env": {
       "API_KEY": "${env:API_KEY}"  // Reads from shell
     }
   }
   ```

3. Check environment in Debug Console:
   ```perl
   # In Debug Console, evaluate:
   $ENV{DEBUG}
   ```

### Slow Debugger Startup

**Symptom**: Debugging takes >5 seconds to start.

**Common Causes**:
- Large Perl modules with heavy initialization
- Slow filesystem (network drives, WSL)
- Many `@INC` directories to scan

**Solution**:
1. Reduce `includePaths` to only necessary directories
2. Use local filesystem instead of network drives
3. Optimize module loading in your Perl code

### Debugger Crashes or Hangs

**Symptom**: Debugger stops responding or crashes VS Code.

**Solution**:
1. Check Debug Console for error messages
2. Restart VS Code: `Ctrl+Shift+P` → "Developer: Reload Window"
3. Verify Perl script runs without debugger:
   ```bash
   perl script.pl
   ```

4. Report issue with logs:
   - Debug Console output
   - VS Code version (`Help` → `About`)
   - Perl version (`perl --version`)
   - Operating system

---

## Getting Help

- **Documentation**: See [DAP Implementation Specification](DAP_IMPLEMENTATION_SPECIFICATION.md) for technical details
- **Security**: See [DAP Security Specification](DAP_SECURITY_SPECIFICATION.md) for security considerations
- **Architecture**: See [Crate Architecture Guide](CRATE_ARCHITECTURE_GUIDE.md) for DAP crate design
- **Issues**: Report bugs at [GitHub Issues](https://github.com/EffortlessMetrics/perl-lsp/issues)

---

**Version History**:
- **0.9.x** (2025-10-04): Phase 1 bridge implementation with Perl::LanguageServer DAP support
