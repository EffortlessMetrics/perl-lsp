# Troubleshooting Guide

Use these checks when `perl-lsp` installs successfully but does not behave as
expected in your editor.

## Start Here

Run the health checks first:

```bash
which perl-lsp
perl-lsp --version
perl-lsp --health
```

If `perl-lsp` is missing, return to [INSTALLATION.md](INSTALLATION.md).
If the binary works but the editor still does not connect, return to
[EDITOR_SETUP.md](EDITOR_SETUP.md).

## Binary Not Found

If the shell says `perl-lsp: command not found`:

1. Check that Cargo installed the binary into `~/.cargo/bin`.
2. Add that directory to your `PATH` if needed.
3. Re-run `perl-lsp --version`.

On Windows, make sure the install location is on `PATH` and then start a new
terminal session.

## Editor Connects But Features Are Missing

If the server starts but you do not see diagnostics, completion, or hover:

1. Confirm the file is recognized as Perl.
2. Verify the editor is launching `perl-lsp --stdio`.
3. Check the editor's LSP output or log panel.
4. Open a small test file and compare behavior with a known-good example.

## Slow Or Memory-Heavy

If the server feels slow on a large workspace:

1. Reduce the workspace scope or exclude generated directories.
2. Turn off features you are not using.
3. Lower the workspace limits in your editor configuration.
4. Confirm you are not indexing a vendor directory by accident.

## Parsing Or Language Gaps

If a syntax construct is not handled well:

1. Check [KNOWN_LIMITATIONS.md](../reference/KNOWN_LIMITATIONS.md).
2. Reproduce the problem with the smallest possible Perl file.
3. File an issue with the reproduction and the exact `perl-lsp --version` output.

## Debug Adapter Problems

If `perl-dap` or a DAP bridge session is failing:

1. Confirm the debugger binary is installed.
2. Verify the Perl debugging runtime works from the shell.
3. See [DAP_USER_GUIDE.md](../tutorials/DAP_USER_GUIDE.md) for the intended setup.

## What To Include In A Bug Report

- `perl-lsp --version`
- your OS and editor
- the smallest reproducible Perl snippet
- relevant logs from the editor or shell

If the problem looks like a bug rather than a configuration issue, report it on
GitHub with that information.
