# IntelliJ IDEA Setup

Use this guide to wire `perllsp` into IntelliJ IDEA when you are not using VS
Code.

## Prerequisites

- IntelliJ IDEA installed
- `perllsp` available on `PATH`
- the project opened from its workspace root

Validate the binary before configuring IntelliJ:

```bash
perllsp --version
perllsp --health
```

## Option A: LSP Support plugin (recommended)

If your IntelliJ installation includes an LSP client plugin, create a new LSP
server entry for Perl and point it at `perllsp --stdio`.

Use these values:

- **Name**: `perl-lsp`
- **Command**: `perllsp`
- **Arguments**: `--stdio`
- **File types / patterns**: Perl files (`*.pl`, `*.pm`, `*.t`, `*.pod`)
- **Working directory**: `$ProjectFileDir$`

After saving the server entry, reopen a Perl file and check the IntelliJ Event
Log or LSP tool window for a successful initialize handshake.

## Option B: Built-in Perl tooling + external checks

If your IDE flavor does not expose LSP settings, keep IntelliJ for editing and
run `perllsp` checks from the terminal while using the built-in Perl plugin for
syntax highlighting/navigation.

Useful commands:

```bash
perllsp --health
just dev-watch-tests
```

## Optional: file watcher for fast feedback

Create a File Watcher or External Tool that runs the repository test loop on
save:

- **Program**: `just`
- **Arguments**: `dev-watch-tests`
- **Working directory**: `$ProjectFileDir$`

This gives IntelliJ users a continuous feedback loop similar to the VS Code
task setup.

## Troubleshooting

- If IntelliJ reports that `perllsp` is missing, use an absolute path to the
  binary in the server command.
- If the server starts but no diagnostics appear, verify the project root in
  the server entry is `$ProjectFileDir$`.
- If startup fails silently, run `perllsp --health` in a terminal from the same
  project directory to confirm runtime dependencies.
