# VS Code Extension — Local Development Guide

This guide covers building, testing, and iterating on the extension locally without publishing to the Marketplace.

## Prerequisites

- Node.js 18+ and npm
- VS Code
- A built `perllsp` binary (see [Building the server](#building-the-server))

## Setup

```bash
cd vscode-extension
npm install
```

## Building the server

The extension downloads a pre-built `perllsp` binary on first use. For local development, build it from source and point the extension at it:

```bash
# From the repo root:
cargo build -p perl-lsp-rs --release
# Binary lands at: target/release/perllsp (or perllsp.exe on Windows)
```

Then in VS Code settings, set:

```json
"perl-lsp.serverPath": "/path/to/perl-lsp/target/release/perllsp"
```

This bypasses the auto-download and uses your local build.

## Compile the extension

```bash
npm run compile     # Single build
npm run watch       # Rebuild on every file change (use during active development)
```

## Run and test in VS Code

1. Open the `vscode-extension/` folder in VS Code.
2. Press **F5** — this opens an Extension Development Host window with your local build loaded.
3. Open any `.pl` or `.pm` file in the host window and verify the server starts (check the Output panel → "Perl LSP").

To reload after code changes: **Ctrl+Shift+P** → "Developer: Reload Window" in the host window.

## Run the test suite

```bash
npm test            # Jest unit tests (no VS Code required)
npm run test:ci     # Same with coverage report
```

## Lint

```bash
npm run lint
```

## Test the bundled extension end-to-end

This packages the extension and verifies it can compile, bundle the server binary, and produce a valid `.vsix`:

```bash
npm run verify:marketplace
```

The `.vsix` file can be installed directly in VS Code via **Extensions → Install from VSIX**.

## Common tasks

| Task | Command |
|------|---------|
| Compile TypeScript | `npm run compile` |
| Watch mode | `npm run watch` |
| Run unit tests | `npm test` |
| Lint | `npm run lint` |
| Build `.vsix` package | `npm run package` |
| Full marketplace verification | `npm run verify:marketplace` |

## Extension entry point

The main extension code lives in `src/extension.ts`. Key files:

| File | Purpose |
|------|---------|
| `src/extension.ts` | Activation, server lifecycle |
| `src/downloader.ts` | Auto-download logic for the `perllsp` binary |
| `src/healthWidget.ts` | Status bar health indicator |
| `src/onboarding.ts` | First-run setup flow |
| `src/debugAdapter.ts` | DAP debug adapter |

## Pointing the extension at a different server version

Set `perl-lsp.serverPath` to any `perllsp` binary. This is the fastest way to test a specific build without reinstalling the extension:

```json
// .vscode/settings.json in your test workspace
{
  "perl-lsp.serverPath": "/absolute/path/to/perllsp"
}
```

Unset or remove `perl-lsp.serverPath` to revert to the auto-downloaded release binary.
