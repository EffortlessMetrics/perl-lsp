# VS Code Extension — Local Development Guide

This guide covers building, testing, and iterating on the extension locally without publishing to the Marketplace.

## Prerequisites

- Node.js 26.x (CI pins 26.5.0) and npm 11.18.0
- VS Code
- A built `perllsp` binary (see [Building the server](#building-the-server))

## Setup

```bash
cd vscode-extension
npm run doctor
npm ci
```

The extension uses npm and `package-lock.json` as its only package authority.
`npm run doctor` enforces the Node floor and the exact `packageManager` value
declared in `package.json`; run it before installing dependencies so an
unsupported environment fails before native packages are installed.

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
npm run typecheck   # Type-check only (tsc --noEmit) — TypeScript 7 is the sole type-check authority
npm run typecheck:all # Check source, unit tests, integration, published smoke, and scripts
npm run compile     # Single build (Rolldown bundles out/extension.js — does NOT type-check)
npm run sample:published:local # Repeat exact-source VSIX smoke and write p50/p95 receipt summary
npm run watch       # Rebuild out/extension.js on every file change (use during active development)
npm run watch:types # Optional companion: live tsc --noEmit type-check loop in a separate terminal
```

The shared configuration enables `noUncheckedIndexedAccess`,
`exactOptionalPropertyTypes`, and `noImplicitOverride` as blocking compiler
options. All source, test, integration, published-smoke, and script authority
configurations reject these forms of type drift directly through
`npm run typecheck:all`.

The shared TypeScript configuration also enables `noImplicitOverride` as a
blocking check. All source, test, integration, published-smoke, and script
authority configurations are clean under this policy, so it does not need a
debt baseline.

`npm run sample:published:local` runs the exact-source local VSIX smoke three
times by default, stores each receipt in a separate sample directory, and
writes the combined p50/p95 summary. Set `PERL_LSP_VSCODE_SAMPLE_RUNS` or pass
`--runs N` for a different sample count; the command still requires the same
current-source server variables as `npm run test:published:local`.

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

To keep Rust build output outside the repository worktree, set
`CARGO_TARGET_DIR` before running the verification:

```bash
CARGO_TARGET_DIR=/tmp/perl-lsp-vsix-target npm run verify:marketplace
```

To test the generated VSIX in a clean VS Code profile:

```bash
PERL_LSP_PUBLISHED_EXTENSION_SOURCE=vsix \
PERL_LSP_PUBLISHED_VSIX_PATH="$PWD/perl-lsp-rs-<version>.vsix" \
PERL_LSP_PUBLISHED_EXTENSION_VERSION=<version> \
PERL_LSP_REQUIRE_STRUCTURED_COMMANDS=1 \
PERL_LSP_SMOKE_RECEIPTS_DIR=/tmp/perl-lsp-vsix-smoke-receipts \
npm run test:published
```

The published smoke expects matching GitHub release assets for the requested
server version. For install-plumbing-only checks against an unreleased extension
version, set `PERL_LSP_PUBLISHED_BINARY_VERSION` to a released server version
and keep the claim limited to VSIX install behavior.

The `.vsix` file can be installed directly in VS Code via **Extensions → Install from VSIX**.

## Common tasks

| Task                          | Command                           |
| ----------------------------- | --------------------------------- |
| Compile TypeScript            | `npm run compile`                 |
| Watch mode                    | `npm run watch`                   |
| Run unit tests                | `npm test`                        |
| Lint                          | `npm run lint`                    |
| Build `.vsix` package         | `npm run package`                 |
| Check VSIX inventory baseline | `npm run check:package-inventory` |
| Full marketplace verification | `npm run verify:marketplace`      |

## Extension entry point

The main extension code lives in `src/extension.ts`. Key files:

| File                  | Purpose                                      |
| --------------------- | -------------------------------------------- |
| `src/extension.ts`    | Activation, server lifecycle                 |
| `src/downloader.ts`   | Auto-download logic for the `perllsp` binary |
| `src/healthWidget.ts` | Status bar health indicator                  |
| `src/onboarding.ts`   | First-run setup flow                         |
| `src/debugAdapter.ts` | DAP debug adapter                            |

## Pointing the extension at a different server version

Set `perl-lsp.serverPath` to any `perllsp` binary. This is the fastest way to test a specific build without reinstalling the extension:

```json
// .vscode/settings.json in your test workspace
{
  "perl-lsp.serverPath": "/absolute/path/to/perllsp"
}
```

Unset or remove `perl-lsp.serverPath` to revert to the auto-downloaded release binary.
