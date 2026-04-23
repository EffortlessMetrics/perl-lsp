# Continuous Testing

Use this guide when you want tests to rerun automatically while you edit. The
repo already has the pieces you need:

- `bacon.toml` for interactive watch loops
- `just dev-watch*` wrappers around the common local loops
- `cargo nextest` for faster targeted test execution

## Recommended Loops

Start with the repo-native watch commands:

```bash
just dev-watch
just dev-watch-clippy
just dev-watch-tests
```

Those recipes use `bacon.toml` when `bacon` is installed, and they fall back
cleanly when it is not.

If you want a narrower test runner loop, use `nextest` directly:

```bash
cargo nextest run --profile local-fast --workspace
```

You can combine that with `cargo-watch` if you prefer a shell-driven file
watcher:

```bash
cargo watch -s "cargo nextest run --profile local-fast --workspace"
```

## VS Code Example

The repository keeps a reusable task file at
[docs/examples/vscode/tasks.json](../examples/vscode/tasks.json). Copy it into
your workspace `.vscode/tasks.json` and adjust the command you want on save.

The example includes two tasks:

- run the bacon-backed watch loop through `just dev-watch-tests`
- run the `cargo nextest` local-fast profile directly

## IntelliJ Rust Plugin

IntelliJ and the Rust plugin do not need a perl-lsp-specific integration point
for this workflow. Configure an external tool or file watcher that runs one of
the commands above.

Use `just dev-watch-tests` if you want the repo defaults, or run
`cargo nextest run --profile local-fast --workspace` when you want the smaller
test loop.

## When To Use Each Tool

- Use `bacon` when you want a fast feedback loop while editing.
- Use `nextest` when you want a quicker targeted test run than `cargo test`.
- Use `cargo-watch` when your editor or shell workflow already expects a
  watch-and-rerun command.
