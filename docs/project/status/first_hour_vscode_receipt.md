# First-Hour VS Code Receipt

Issue: #3102

Result: redirect. A real VS Code extension-host run against Perl-Critic did not reach provider checks because extension activation timed out after 90 seconds.

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
| Startup | Timeout after 90,010 ms during extension activation |
| Indexing announcement | Not observable because activation did not complete |
| First completion | Not reached |
| Hover | Not reached |
| Definition/references | Not reached |
| Diagnostics | Not reached |
| Failures | `activation_timeout`: extension activation timed out after 90000 ms |

## Commands

```powershell
npm run compile
npx tsc -p ./tsconfig.integration.json
cargo build -p perl-lsp-rs --bin perl-lsp --profile agent --locked -j 2
git clone --depth 1 https://github.com/Perl-Critic/Perl-Critic.git target/first-hour-workspaces/Perl-Critic
```

The receipt run used:

```powershell
$env:PERL_LSP_FIRST_HOUR_ONLY='1'
$env:PERL_LSP_FIRST_HOUR_RECEIPT='1'
$env:PERL_LSP_EXTENSION_TEST_SKIP_STARTUP='0'
$env:PERL_LSP_FIRST_HOUR_MODULE='Perl::Critic'
$env:PERL_LSP_SMOKE_SOURCE_LABEL='first-hour-3102-run3'
node ./out/test/integration/runTest.js
```

## Interpretation

This confirms the value of #3102: the real editor path surfaced a worse first-minute blocker than provider correctness. Before claiming completion, hover, definition, or diagnostics are honest in the first minute, extension activation needs to stop blocking for more than 90 seconds on a normal multi-file Perl workspace.

The machine-readable receipt is checked in at `docs/project/status/first_hour_vscode_receipt.json`.
