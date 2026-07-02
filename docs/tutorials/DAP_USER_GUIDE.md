# DAP User Guide: Debugging Perl with VS Code

**Status**: Native `perl-dap` CLI for launch, attach, stepping, stack frames,
variables, evaluate, and breakpoint validation.

**Dependency note**: Native `perl-dap` requires a local Perl interpreter for
debug sessions. Its Rust parser-backed runtime is compiled into the shipped
binary; users do not install parser crates separately.

This guide covers the native debugger path shipped with `perl-lsp`.

## Prerequisites

Before debugging Perl code, make sure you have:

1. Perl 5.10 or newer available on `PATH`.
2. VS Code with the Perl LSP extension installed.
3. The `perl-dap` binary from the Perl LSP release package.

Check the interpreter with:

```bash
perl --version
```

## Launch A Script

Create `.vscode/launch.json` in your workspace:

```json
{
  "version": "0.2.0",
  "configurations": [
    {
      "type": "perl",
      "request": "launch",
      "name": "Launch current Perl file",
      "program": "${file}",
      "perlPath": "perl",
      "args": [],
      "includePaths": ["${workspaceFolder}/lib"],
      "cwd": "${workspaceFolder}",
      "env": {}
    }
  ]
}
```

Set breakpoints in a `.pl`, `.pm`, or `.t` file, choose the configuration, and
start debugging from VS Code.

## Attach Over TCP

Use socket mode when an editor or tool needs a TCP DAP endpoint:

```bash
perl-dap --socket --port 13603
```

Then configure the client to attach to `127.0.0.1:13603`.

## Include Paths

Use `includePaths` to add project library roots to `@INC`:

```json
"includePaths": [
  "${workspaceFolder}/lib",
  "${workspaceFolder}/local/lib/perl5"
]
```

## Environment Variables

Use `env` for debug-session environment overrides:

```json
"env": {
  "PERL5LIB": "${workspaceFolder}/lib",
  "APP_ENV": "development"
}
```

## Common Problems

### Perl Interpreter Not Found

If launch fails because Perl cannot be found, set `perlPath` to an absolute
interpreter path:

```json
"perlPath": "/usr/bin/perl"
```

On Windows, this may look like:

```json
"perlPath": "C:\\Strawberry\\perl\\bin\\perl.exe"
```

### Program Path Not Found

Make sure `program` points at a real script file. `${file}` is usually the
right value when debugging the active editor file.

### Breakpoint Not Verified

Breakpoints are validated against source locations. Move the breakpoint to an
executable Perl statement if it lands on a comment, blank line, POD block, or
other non-executable region.

## Command Reference

Run native DAP over stdio:

```bash
perl-dap --stdio
```

Run native DAP over a TCP socket:

```bash
perl-dap --socket --port 13603
```

Print CLI help:

```bash
perl-dap --help
```

## Native Stack Policy

The shipped debugger path is native. External Perl debugger backends are not
required for normal operation. Compatibility and migration notes, when needed,
belong in reference documentation rather than this first-mile guide.
