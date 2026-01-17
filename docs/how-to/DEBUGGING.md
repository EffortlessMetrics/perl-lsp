# Perl Debugging Support

The Perl Language Server now includes full debugging support through the Debug Adapter Protocol (DAP), enabling step-through debugging in VSCode and other DAP-compatible editors.

## Features

### Core Debugging
- **Breakpoints**: Set breakpoints in your Perl code
- **Step Controls**: Step over, step into, step out
- **Variable Inspection**: View local variables and their values
- **Call Stack**: Navigate through the call stack
- **Watch Expressions**: Evaluate Perl expressions during debugging
- **Conditional Breakpoints**: Break only when conditions are met

### Test Debugging
- Debug individual test functions
- Debug entire test files
- Integrated with Test Explorer
- TAP output support during debugging

## Installation

### 1. Install the Debug Adapter
```bash
# Build and install the debug adapter
cargo install --path crates/perl-parser --bin perl-dap
```

### 2. Configure VSCode
The Perl Language Server extension automatically detects and uses the debug adapter.

## Usage

### Quick Start
1. Open a Perl file in VSCode
2. Set breakpoints by clicking in the gutter
3. Press F5 or use Run → Start Debugging
4. Select "Perl: Launch Script" configuration

### Debug Configurations

#### Basic Script Debugging
```json
{
    "type": "perl",
    "request": "launch",
    "name": "Launch Perl Script",
    "program": "${file}",
    "stopOnEntry": true,
    "args": []
}
```

#### Test File Debugging
```json
{
    "type": "perl",
    "request": "launch",
    "name": "Debug Perl Test",
    "program": "${file}",
    "stopOnEntry": false,
    "args": [],
    "env": {
        "PERL_TEST_HARNESS_DUMP_TAP": "1"
    }
}
```

#### Custom Working Directory
```json
{
    "type": "perl",
    "request": "launch",
    "name": "Launch with Custom CWD",
    "program": "${file}",
    "cwd": "${workspaceFolder}/scripts",
    "args": ["--verbose"]
}
```

### Debugging from Test Explorer
1. Open the Testing panel in VSCode
2. Navigate to a test
3. Right-click and select "Debug Test"
4. Or use the debug icon next to the test

## Debug Commands

### Execution Control
- **Continue** (F5): Resume execution
- **Step Over** (F10): Execute current line
- **Step Into** (F11): Step into subroutines
- **Step Out** (Shift+F11): Step out of current subroutine
- **Restart** (Ctrl+Shift+F5): Restart debugging session
- **Stop** (Shift+F5): Stop debugging

### Breakpoints
- Click in the gutter to toggle breakpoints
- Right-click for conditional breakpoints
- Use the Breakpoints panel to manage all breakpoints

### Variables
- View local variables in the Variables panel
- Hover over variables in the editor to see values
- Add watch expressions in the Watch panel

## Configuration Options

### launch.json Properties

| Property | Type | Description | Default |
|----------|------|-------------|---------|
| `program` | string | Path to Perl script | `${file}` |
| `args` | array | Command line arguments | `[]` |
| `stopOnEntry` | boolean | Stop at first line | `false` |
| `cwd` | string | Working directory | `${workspaceFolder}` |
| `env` | object | Environment variables | `{}` |
| `perlPath` | string | Path to Perl interpreter | `perl` |

## Troubleshooting

### Workspace Build Issues (v0.8.8+)

#### "Cannot find parser.c" or "libclang not found"
The workspace uses an exclusion strategy to avoid these system dependency issues:

```bash
# ✅ This should work (workspace tests only production crates)
cargo test

# ❌ This may fail if you try to build excluded crates directly
cargo build -p tree-sitter-perl-c
```

**Solution**: The workspace is configured to exclude problematic crates. Use the standard workspace commands:

```bash
# Build only production crates
cargo build

# Test only production crates  
cargo test

# Check workspace configuration
cargo check
```

#### Feature Conflicts Between Crates
If you see feature resolution errors:

```bash
# ✅ Use workspace-level commands
cargo test

# ❌ Avoid direct crate builds that may conflict
cargo test -p example-crate-with-conflicts
```

**Reference**: See [WORKSPACE_TEST_REPORT.md](../WORKSPACE_TEST_REPORT.md) for current workspace status.

### Debug adapter not found
```bash
# Verify installation
which perl-dap

# Reinstall if needed
cargo install --path crates/perl-parser --bin perl-dap --force
```

### Breakpoints not working
1. Ensure the file is saved
2. Check that perl-dap is running
3. Verify Perl syntax is correct

### Variables not showing
- Some optimized variables may not be visible
- Use `my` declarations for better debugging experience

## Architecture

The debugging system consists of:

1. **Debug Adapter (perl-dap)**: Rust implementation of DAP protocol
2. **Perl Debugger Integration**: Interfaces with `perl -d`
3. **VSCode Extension**: Provides UI integration
4. **Test Integration**: Connects with Test Explorer

## Limitations

- Remote debugging not yet supported
- Attach to process not implemented
- Some Perl internals may not be inspectable

## Future Enhancements

- [ ] Remote debugging support
- [ ] Attach to running Perl process
- [ ] Data structure visualization
- [ ] Performance profiling integration
- [ ] Multi-threaded debugging support