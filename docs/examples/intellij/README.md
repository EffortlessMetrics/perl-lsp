# IntelliJ IDEA / RustRover Watch Loop Setup

This example gives IntelliJ-family IDE users a copy-paste setup for the same
continuous-testing loops documented in
[`docs/how-to/CONTINUOUS_TESTING.md`](../../how-to/CONTINUOUS_TESTING.md).

## Option A: External Tool (recommended)

1. Open **Settings > Tools > External Tools**.
2. Add a new tool with:
   - **Name:** `perl-lsp dev-watch-tests`
   - **Program:** `just`
   - **Arguments:** `dev-watch-tests`
   - **Working directory:** `$ProjectFileDir$`
3. Run it from **Tools > External Tools**.

This gives you the repo-default watcher loop and works on Linux/macOS/Windows.

## Option B: Cargo command (narrower loop)

If you want the smaller direct test loop, create a Cargo command run
configuration:

- **Command:** `nextest`
- **Arguments:** `run --profile local-fast --workspace`
- **Working directory:** `$ProjectFileDir$`

## Notes

- Keep the process running in a dedicated tool window while editing.
- Stop and rerun when switching between broad (`just dev-watch-tests`) and
  narrow (`cargo nextest`) loops.
