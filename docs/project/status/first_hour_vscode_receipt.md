# First-Hour VS Code Receipt

Issue: #3102
Follow-up fix: #3159

Result: completed. A real VS Code extension-host run against Perl-Critic reached startup, completion, hover, definition, references, and diagnostics after extension activation was decoupled from the LSP startup wait.

## Environment

- OS: Windows x64
- VS Code: 1.126.0
- Extension: `EffortlessMetrics.perl-lsp-rs` 0.16.0
- Server: `perl-lsp 0.17.0`, git tag `c04e06b8c`
- Workspace: `Perl-Critic/Perl-Critic` at `c437d55`
- Workspace size sampled: 442 files, 261 Perl files

## Measured Moments

| Moment | Result |
| --- | --- |
| Startup | Extension activation completed in 300 ms |
| Command registration | 38 ms |
| Health check | OK in 5,258 ms |
| Indexing announcement | Not observable from the VS Code extension-host public API |
| First completion | Immediate: OK, 123 ms, 5 items; after 30s: OK, 3 ms, 5 items |
| Hover | Immediate: OK, 34 ms, 1 item; after 30s: OK, 3 ms, 1 item |
| Definition | Immediate: OK, 5 ms, 1 location; after 30s: OK, 3 ms, 1 location |
| References | Immediate: OK, 5 ms, 1 location; after 30s: OK, 2 ms, 1 location |
| Diagnostics | Immediate: 1 diagnostic; syntax-probe file: 2 diagnostics |
| Failures | None in the completed receipt |

## Commands

```powershell
npm test -- --runTestsByPath src/test/activationStartup.test.ts
npm run compile
npx tsc -p ./tsconfig.integration.json
```

The completed receipt run used:

```powershell
$env:PERL_LSP_FIRST_HOUR_ONLY='1'
$env:PERL_LSP_FIRST_HOUR_RECEIPT='1'
$env:PERL_LSP_EXTENSION_TEST_SKIP_STARTUP='0'
$env:PERL_LSP_FIRST_HOUR_MODULE='Perl::Critic'
$env:PERL_LSP_SMOKE_SOURCE_LABEL='first-hour-3159-after-retry'
node ./out/test/integration/runTest.js
```

One prior attempt with `PERL_LSP_SMOKE_SOURCE_LABEL='first-hour-3159-after'` exited before the test body completed and logged `Error mutex already exists`; no receipt was written. After stopping the specific orphaned VS Code test crashpad process, the retry completed.

## Interpretation

The original #3102 receipt correctly exposed a first-minute blocker: activation timed out after 90 seconds before provider checks could run. #3159 changes the extension startup contract so activation registers UI/commands and returns promptly while the language client starts in the background.

The first-hour harness now reaches provider moments on the 261-Perl-file workspace. The status bar/output-channel indexing announcement is still not observable through this automated extension-host API, so that remains a limitation of the receipt rather than a claim that the status text is visible.

The machine-readable receipt is checked in at `docs/project/status/first_hour_vscode_receipt.json`.
