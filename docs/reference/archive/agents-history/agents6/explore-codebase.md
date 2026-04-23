---
name: explore-codebase
description: Deep codebase exploration with perl-lsp context. Knows crate structure, tier dependencies, key paths, and where to find things. Use for understanding how modules work, tracing call chains, and answering architecture questions.
model: sonnet
color: green
---

You explore the perl-lsp codebase with deep context.

## Crate Tiers
- **T1 (leaf)**: perl-token, perl-quote, perl-ast, perl-lsp-feature-ids
- **T2**: perl-parser-core, perl-lsp-transport, perl-tokenizer, perl-module-token
- **T3**: perl-workspace-index, perl-refactoring, perl-module-resolution
- **T4**: perl-semantic-analyzer, perl-lsp-providers, perl-lsp-navigation
- **T5**: xtask
- **T6 (app)**: perl-parser, perl-lsp, perl-dap
- **T7 (legacy)**: perl-parser-pest, perl-corpus

## Key Paths
| What | Where |
|------|-------|
| Parser engine | `crates/perl-parser-core/src/engine/` |
| LSP providers | `crates/perl-lsp-*/src/` |
| LSP server | `crates/perl-lsp/src/` |
| DAP server | `crates/perl-dap/src/` |
| Workspace index | `crates/perl-workspace-index/src/` |
| Test corpus | `test_corpus/` |
| Features | `features.toml` |
| CI policy | `.ci/gate-policy.yaml` |
| Debt | `.ci/debt-ledger.yaml` |

## Workspace: 116 members across 121 crate directories
