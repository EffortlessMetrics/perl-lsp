# perl-lsp Documentation

Use this directory as the short docs front door. It tells you where to go next
without making you learn the workspace layout first. For the full Diataxis-style
map of the docs tree, use [INDEX.md](INDEX.md).

## Diataxis in This Repository

When adding or moving docs, choose the content type first, then the file:

| Content intent | Place it under | Writing focus |
| --- | --- | --- |
| Teach by doing | `docs/tutorials/` | step-by-step learning journey |
| Solve a concrete task | `docs/how-to/` | shortest reliable path to an outcome |
| Describe the contract | `docs/reference/` | exact behavior, options, and constraints |
| Explain rationale | `docs/explanation/` | design tradeoffs and mental models |

If a doc starts mixing multiple intents, split it and cross-link the parts.

## Canonical Sources

| Topic | Source | Verified By |
| --- | --- | --- |
| Current release line | [`../Cargo.toml`](../Cargo.toml) | Workspace manifest |
| Metrics and receipts | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) | `just status-update` and `just status-check` |
| Roadmap and active milestone | [project/ROADMAP.md](project/ROADMAP.md) | Human review |
| Capability catalog | [`../features.toml`](../features.toml) | `just ci-gate` |
| Local validation flow | [project/CI_LOCAL_VALIDATION.md](project/CI_LOCAL_VALIDATION.md) | `just ci-gate` |

Rule: if a project metric appears outside [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), treat it as stale until reverified.

## Repository Map (Quick Orientation)

Use this map when you need to quickly find the right area of the workspace before
opening architecture deep dives.

| Path | What lives here | Start with |
| --- | --- | --- |
| `crates/perllsp/` | End-user binary entry point (`perllsp`) and CLI wiring | [`crates/perllsp/README.md`](../crates/perllsp/README.md) |
| `crates/perl-lsp-rs/` | Language server host/runtime wiring used by the binary | [`reference/ARCHITECTURE_OVERVIEW.md`](reference/ARCHITECTURE_OVERVIEW.md) |
| `crates/perl-dap/` | Debug Adapter Protocol server support | [`tutorials/DAP_USER_GUIDE.md`](tutorials/DAP_USER_GUIDE.md) |
| `crates/perl-parser/`, `crates/perl-lexer/`, `crates/perl-parser-core/` | Native parser stack (tokenization + parse engine) | [`reference/CRATE_ARCHITECTURE_GUIDE.md`](reference/CRATE_ARCHITECTURE_GUIDE.md) |
| `crates/perl-semantic-analyzer/` | Scope/symbol analysis over parsed ASTs | [`reference/ARCHITECTURE.md`](reference/ARCHITECTURE.md) |
| `crates/perl-workspace-index/` | Cross-file indexing, search, and rename plumbing | [`reference/CRATE_ARCHITECTURE_GUIDE.md`](reference/CRATE_ARCHITECTURE_GUIDE.md) |
| `crates/perl-lsp-rs-core/` | Shared LSP protocol/provider modules and compatibility surfaces | [`reference/LSP_FEATURES.md`](reference/LSP_FEATURES.md) |
| `crates/tree-sitter-perl-c/`, `crates/tree-sitter-perl-rs/` | Tree-sitter compatibility/interoperability layers | [`reference/MODERN_ARCHITECTURE.md`](reference/MODERN_ARCHITECTURE.md) |
| `docs/` | Tutorials/how-to/reference/project status docs | [`INDEX.md`](INDEX.md) |
| `xtask/` | Project automation tasks (`cargo xtask ...`) | [`reference/COMMANDS_REFERENCE.md`](reference/COMMANDS_REFERENCE.md) |
| `test_corpus/` | Golden fixtures and parser confidence corpus | [`project/CURRENT_STATUS.md`](project/CURRENT_STATUS.md) |

## Common Routes

| If you need to... | Read this |
| --- | --- |
| get working fast | [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md) |
| set up continuous testing | [how-to/CONTINUOUS_TESTING.md](how-to/CONTINUOUS_TESTING.md) |
| set up pre-commit hooks | [how-to/PRE_COMMIT.md](how-to/PRE_COMMIT.md) |
| install or upgrade | [how-to/INSTALLATION.md](how-to/INSTALLATION.md), [how-to/UPGRADING.md](how-to/UPGRADING.md) |
| set up `perllsp` in GitHub Actions | [how-to/GITHUB_ACTIONS.md](how-to/GITHUB_ACTIONS.md) |
| configure an editor | [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md) |
| troubleshoot a broken setup | [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md) |
| enforce public API docs coverage in CI | [reference/MISSING_DOCUMENTATION_GUIDE.md](reference/MISSING_DOCUMENTATION_GUIDE.md) |
| learn API docs writing standards | [reference/API_DOCUMENTATION_STANDARDS.md](reference/API_DOCUMENTATION_STANDARDS.md) |
| choose the right Diátaxis doc type before writing | [reference/DOCUMENTATION_GUIDE.md](reference/DOCUMENTATION_GUIDE.md) |
| tune performance or threading | [how-to/PERFORMANCE_TUNING.md](how-to/PERFORMANCE_TUNING.md), [how-to/THREADING_CONFIGURATION_GUIDE.md](how-to/THREADING_CONFIGURATION_GUIDE.md) |
| work with DAP workflows | [tutorials/DAP_USER_GUIDE.md](tutorials/DAP_USER_GUIDE.md) |
| understand project architecture | [reference/ARCHITECTURE_OVERVIEW.md](reference/ARCHITECTURE_OVERVIEW.md), [reference/CRATE_ARCHITECTURE_GUIDE.md](reference/CRATE_ARCHITECTURE_GUIDE.md) |
| check known limitations and parser support | [reference/KNOWN_LIMITATIONS.md](reference/KNOWN_LIMITATIONS.md), [reference/PARSER_FEATURE_MATRIX.md](reference/PARSER_FEATURE_MATRIX.md) |
| see what is true now | [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md) |
| see the current release plan | [project/ROADMAP.md](project/ROADMAP.md) |
| inspect the workflow UX scorecard contract | [project/metrics/WORKFLOW_SCORECARDS.md](project/metrics/WORKFLOW_SCORECARDS.md), [reference/UX_TESTING.md](reference/UX_TESTING.md) |
| work on the codebase | [../CONTRIBUTING.md](../CONTRIBUTING.md) |
| browse the full docs map | [INDEX.md](INDEX.md) |
| classify or author docs by Diataxis type | [reference/DIATAXIS_GUIDE.md](reference/DIATAXIS_GUIDE.md) |

## Docs by Type

- Tutorials: [tutorials/GETTING_STARTED.md](tutorials/GETTING_STARTED.md), [tutorials/LSP_DEVELOPMENT_GUIDE.md](tutorials/LSP_DEVELOPMENT_GUIDE.md), [tutorials/DAP_USER_GUIDE.md](tutorials/DAP_USER_GUIDE.md), [tutorials/COMPREHENSIVE_TESTING_GUIDE.md](tutorials/COMPREHENSIVE_TESTING_GUIDE.md)
- How-to: [how-to/INSTALLATION.md](how-to/INSTALLATION.md), [how-to/GITHUB_ACTIONS.md](how-to/GITHUB_ACTIONS.md), [how-to/EDITOR_SETUP.md](how-to/EDITOR_SETUP.md), [how-to/TROUBLESHOOTING.md](how-to/TROUBLESHOOTING.md), [how-to/CONTINUOUS_TESTING.md](how-to/CONTINUOUS_TESTING.md), [how-to/UPGRADING.md](how-to/UPGRADING.md), [how-to/PRE_COMMIT.md](how-to/PRE_COMMIT.md), [how-to/PERFORMANCE_TUNING.md](how-to/PERFORMANCE_TUNING.md), [how-to/THREADING_CONFIGURATION_GUIDE.md](how-to/THREADING_CONFIGURATION_GUIDE.md), [how-to/SECURITY_DEVELOPMENT_GUIDE.md](how-to/SECURITY_DEVELOPMENT_GUIDE.md)
- Reference: [reference/COMMANDS_REFERENCE.md](reference/COMMANDS_REFERENCE.md), [reference/CONFIG.md](reference/CONFIG.md), [reference/LSP_FEATURES.md](reference/LSP_FEATURES.md), [reference/ARCHITECTURE_OVERVIEW.md](reference/ARCHITECTURE_OVERVIEW.md), [reference/CRATE_ARCHITECTURE_GUIDE.md](reference/CRATE_ARCHITECTURE_GUIDE.md), [reference/KNOWN_LIMITATIONS.md](reference/KNOWN_LIMITATIONS.md), [reference/PARSER_FEATURE_MATRIX.md](reference/PARSER_FEATURE_MATRIX.md), [reference/MISSING_DOCUMENTATION_GUIDE.md](reference/MISSING_DOCUMENTATION_GUIDE.md), [reference/API_DOCUMENTATION_STANDARDS.md](reference/API_DOCUMENTATION_STANDARDS.md), [reference/DIATAXIS_GUIDE.md](reference/DIATAXIS_GUIDE.md), [reference/DOCUMENTATION_GUIDE.md](reference/DOCUMENTATION_GUIDE.md), [reference/FAQ.md](reference/FAQ.md)
- Project, specs, and explanations: [INDEX.md](INDEX.md), [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), [project/ROADMAP.md](project/ROADMAP.md), [project/CI.md](project/CI.md), [project/FEATURE_GOVERNANCE.md](project/FEATURE_GOVERNANCE.md), [explanation/LSP_DOCUMENTATION.md](explanation/LSP_DOCUMENTATION.md)

## Maintenance

```bash
nix develop -c just ci-gate
just status-update
just status-check
```

- Put computed metrics in [project/CURRENT_STATUS.md](project/CURRENT_STATUS.md), not scattered through the docs tree.
- Update [project/ROADMAP.md](project/ROADMAP.md) when the active milestone or release framing changes.
- Keep top-level summary docs short and link back to the canonical project docs.
- Keep each doc in the correct Diataxis category; prefer cross-links over hybrid docs that try to do everything.
