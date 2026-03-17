# Agent Catalog
This is the canonical tracked inventory for `.claude/agents/`. If a file lives in this directory, it is part of the tracked swarm surface and should be cataloged here or explicitly marked as compatibility material.
Every live agent file should encode four things directly in the prompt surface: local todo or task discipline, required startup slash entrypoints, context or scope boundaries, and clear flow integration (who spawns it, where it hands work next).
## Core Coordinators
| Agent | Owns | Routes to | First entrypoints | File |
| --- | --- | --- | --- | --- |
| `scout` | discovery lane | `builder` | /swarm-protocol, /coding-standards, /swarm-priorities | `scout.md` |
| `builder` | implementation lane | `reviewer` | /swarm-protocol, /coding-standards, /swarm-priorities | `builder.md` |
| `reviewer` | review lane | `ops or builder` | /swarm-protocol, /coding-standards | `reviewer.md` |
| `ops` | merge and queue lane | `fixer or validator` | /swarm-protocol, /green-merge | `ops.md` |
| `improver` | docs/tests/devex lane | `builder or reviewer` | /swarm-protocol, /coding-standards, /swarm-priorities | `improver.md` |

## Core Reusable Workers
| Agent | Usually spawned by | Hands off to | First entrypoints | File |
| --- | --- | --- | --- | --- |
| `bootstrapper` | `improver or lead` | `improver or reviewer` | /swarm-protocol | `bootstrapper.md` |
| `fixer` | `ops or reviewer` | `reviewer or ops` | /swarm-protocol, /coding-standards, /verify-build | `fixer.md` |
| `validator` | `ops` | `ops or fixer` | /swarm-protocol, validation command | `validator.md` |
| `pr-responder` | `reviewer or ops` | `reviewer` | /swarm-protocol, /coding-standards, /pr-ready | `pr-responder.md` |
| `research-web` | `scout or improver` | `scout or builder` | /swarm-protocol, /swarm-priorities | `research-web.md` |
| `research-docs` | `scout or improver` | `scout or builder` | /swarm-protocol, /swarm-priorities | `research-docs.md` |
| `research-verify` | `scout or improver` | `scout or builder` | /swarm-protocol, /swarm-priorities | `research-verify.md` |

## Specialist Workers
| Agent | Category | Usually spawned by | Hands off to | First entrypoints | File | Description |
| --- | --- | --- | --- | --- | --- | --- |
| `adr-writer` | `docs_devex` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /pr-create | `adr-writer.md` | Architecture Decision Record writer. Documents architectural choices with context, decision, and consequences. Reads recent PRs and code patterns to identify implicit decisions that need documentation. |
| `api-docs` | `docs_devex` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /pr-create | `api-docs.md` | API documentation — doc comments, doctests, module-level docs, and API reference. Ensures public items are documented and examples compile. |
| `baseline-ratchet` | `quality` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `baseline-ratchet.md` | Corpus and CPAN baseline ratchet. Runs sweep, compares against baseline, updates manifests when improved. Knows the sweep/ratchet workflow and manifest files. |
| `changelog-writer` | `docs_devex` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /pr-create | `changelog-writer.md` | CHANGELOG maintenance. Reads recent git history and merged PRs, adds entries in Keep a Changelog format. Groups by Added/Changed/Fixed/Removed. |
| `ci-gate` | `quality_ops` | `ops` | `fixer or reviewer` | /swarm-protocol, /coding-standards, /verify-build | `ci-gate.md` | Full CI gate execution. Knows gate tiers (pr-fast, ci-gate, ci-full), gate policy, and how to diagnose gate failures. |
| `coverage-filler` | `quality` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `coverage-filler.md` | Find and fill test coverage gaps. Identifies crates with low test counts relative to LOC, adds meaningful tests that exercise real behavior paths. |
| `dap-feature` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `dap-feature.md` | DAP feature implementation. Knows the DAP protocol, perl-dap crate structure, bridge mode architecture, and how the debug adapter communicates with Perl debugger. |
| `dap-test` | `quality` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `dap-test.md` | DAP (Debug Adapter Protocol) test coverage. Knows perl-dap-* crate structure, test gaps in perl-dap-value/shell/command-args/security, and DAP protocol test patterns. |
| `dead-code` | `docs_devex` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `dead-code.md` | Dead code detection and removal. Runs dead code analysis, identifies unreachable functions/types/modules, and safely removes them. |
| `dep-cleaner` | `docs_devex` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `dep-cleaner.md` | Unused dependency removal. Runs cargo machete, verifies each removal compiles, and cleans up Cargo.toml files. |
| `explore-codebase` | `explore` | `scout or improver` | `scout or builder` | /swarm-protocol, /swarm-priorities, /plan-fix | `explore-codebase.md` | Deep codebase exploration with perl-lsp context. Knows crate structure, tier dependencies, key paths, and where to find things. Use for understanding how modules work, tracing call chains, and answering architecture questions. |
| `explore-deps` | `explore` | `scout or improver` | `scout or builder` | /swarm-protocol, /swarm-priorities, /plan-fix | `explore-deps.md` | Dependency analysis. Checks for unused deps, security advisories, outdated versions, license compliance, and supply chain health. |
| `explore-issues` | `explore` | `scout` | `scout or builder` | /swarm-protocol, /swarm-priorities, /plan-fix | `explore-issues.md` | GitHub issue research and analysis. Reads issue details, linked PRs, comments, and labels. Knows key open issues and their context. |
| `flaky-fixer` | `quality` | `improver or fixer` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `flaky-fixer.md` | Diagnose and fix flaky tests. Reads debt-ledger.yaml for known flaky tests, runs them repeatedly to reproduce, diagnoses root cause (timing, ordering, resources), and fixes. |
| `friction-logger` | `docs_devex` | `improver` | `improver or reviewer` | /swarm-protocol, /coding-standards, /pr-create | `friction-logger.md` | Friction log maintenance. Tracks what trips up developers and agents — confusing errors, hard-to-find code, unclear APIs, missing docs, broken workflows. Creates actionable improvement items. |
| `fuzz-tester` | `quality` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `fuzz-tester.md` | Fuzz testing for parser and LSP components. Runs bounded fuzz campaigns, analyzes crashes, and creates regression tests. Knows fuzz target structure and cargo-fuzz workflow. |
| `lsp-feature` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `lsp-feature.md` | Full LSP feature implementation — provider + navigation + test. For implementing a complete new LSP feature or significantly improving an existing one. Knows the full stack from parser to LSP response. |
| `lsp-navigation` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `lsp-navigation.md` | Go-to-definition, references, workspace symbols, and cross-file navigation. Knows dual indexing architecture, perl-workspace-index, and navigation provider integration. |
| `lsp-provider` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `lsp-provider.md` | Implement and improve LSP feature providers — completion, hover, signature help, diagnostics, code actions. Knows provider trait patterns, perl-lsp-* crate structure, and features.toml. |
| `lsp-test` | `quality` | `improver or builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `lsp-test.md` | LSP integration tests. Knows threading constraints (RUST_TEST_THREADS=2), LSP protocol test patterns, and how to test provider responses end-to-end. |
| `module-resolution` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `module-resolution.md` | Module resolution — use/require handling, @INC search, module name→path mapping. Knows perl-module-* microcrates and module resolution pipeline. |
| `mutant-killer` | `quality` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `mutant-killer.md` | Kill mutation testing survivors. Runs cargo-mutants, identifies surviving mutations, and writes targeted tests that catch them. Focuses on boundary conditions, error paths, and return value checks. |
| `parser-corpus` | `implementation` | `builder or improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `parser-corpus.md` | Corpus sweep, error bucket analysis, and test fixture creation. Knows parser-corpus-baseline.json structure, cpan-corpus-manifest, and the sweep/ratchet workflow. |
| `parser-fix-constructs` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /parser-fix, /verify-build | `parser-fix-constructs.md` | Fix parsing of complex Perl constructs — heredocs, regex, quotes, formats, special variables, and context-sensitive syntax. Knows perl-quote, perl-heredoc, perl-regex crates and their integration with the lexer. |
| `parser-fix-engine` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /parser-fix, /verify-build | `parser-fix-engine.md` | Fix parser engine bugs in expressions, statements, declarations, and control flow. Knows perl-parser-core/src/engine/ structure, precedence climbing, and recursive descent patterns. TDD approach with crate-level verification. |
| `parser-lexer` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /parser-fix, /verify-build | `parser-lexer.md` | Lexer and tokenizer fixes and tests. Knows perl-lexer, perl-tokenizer, perl-token crates, context-aware tokenization, and the token pipeline. |
| `parser-test` | `quality` | `builder or improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `parser-test.md` | Add parser tests — unit tests for engine functions and integration tests for Perl constructs. Knows test patterns, corpus fixtures, and the parse→assert-no-errors pattern. |
| `refactoring` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `refactoring.md` | Refactoring operations — rename, extract function/module, inline, move. Knows perl-refactoring crate and LSP refactoring protocol. |
| `review-api` | `review` | `reviewer` | `builder or ops` | /swarm-protocol, /coding-standards, /pr-ready | `review-api.md` | API design review. Checks for ergonomic public APIs, proper error types, backwards compatibility, and SemVer compliance. |
| `review-performance` | `review` | `reviewer` | `builder or ops` | /swarm-protocol, /coding-standards, /pr-ready | `review-performance.md` | Performance-focused code review. Checks for unnecessary allocations, clone-heavy patterns, missing caches, hot path inefficiencies, and O(n²) algorithms. |
| `review-scope` | `review` | `reviewer` | `builder or ops` | /swarm-protocol, /coding-standards, /pr-ready | `review-scope.md` | Scope and focus review. Checks for scope creep, unrelated changes, oversized PRs, and file ownership violations. Ensures PRs do one thing well. |
| `review-security` | `review` | `reviewer` | `builder or ops` | /swarm-protocol, /coding-standards, /pr-ready | `review-security.md` | Security-focused code review. Checks for banned constructs, input validation, path traversal prevention, UTF-16/UTF-8 boundary safety, and supply chain issues. |
| `review-standards` | `review` | `reviewer` | `builder or ops` | /swarm-protocol, /coding-standards, /pr-ready | `review-standards.md` | Coding standards review. Checks for perl-lsp coding conventions, conventional commits, crate boundary violations, and project patterns. |
| `scout-dap` | `scout` | `scout` | `builder or issue queue` | /swarm-protocol, /swarm-priorities, /plan-fix, /scout-report | `scout-dap.md` | DAP-focused scout. Knows DAP crate test gaps, protocol compliance areas, and related issues (#420, #435). Read-only. |
| `scout-parser` | `scout` | `scout` | `builder or issue queue` | /swarm-protocol, /swarm-priorities, /plan-fix, /scout-report | `scout-parser.md` | Parser-focused scout. Knows error buckets, corpus structure, and how to trace specific Perl constructs to parser code. Read-only — returns SLICE definitions. |
| `scout-security` | `scout` | `scout` | `builder or issue queue` | /swarm-protocol, /swarm-priorities, /plan-fix, /scout-report | `scout-security.md` | Security-focused scout. Checks for banned constructs, unsafe blocks, dependency vulnerabilities, and supply chain issues. Read-only. |
| `security-audit` | `quality_ops` | `ops or improver` | `fixer or reviewer` | /swarm-protocol, /coding-standards, /verify-build | `security-audit.md` | Security audit and supply chain checks. Runs cargo audit, checks deny.toml policy, verifies SBOM generation, and identifies security advisories. |
| `semantic-analysis` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `semantic-analysis.md` | Semantic analysis — scope analysis, symbol resolution, type inference, import tracking. Knows perl-semantic-analyzer crate and its integration with parser and workspace index. |
| `test-quality` | `quality` | `improver` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `test-quality.md` | Improve test naming, assertions, structure, and patterns. Converts implementation-detail tests to behavior-specification tests. Ensures BDD coverage and proper test infrastructure usage. |
| `workspace-index` | `implementation` | `builder` | `reviewer` | /swarm-protocol, /coding-standards, /verify-build | `workspace-index.md` | Workspace indexing — dual indexing, cross-file symbol resolution, file discovery. Knows perl-workspace-index, perl-workspace-discover, and the qualified/bare name indexing pattern. |

## Compatibility / Donor Files

These remain tracked so older prompts and docs do not break during the transition, but they are not the active roster. New docs and orchestration should reference the canonical names first.

| Agent | File |
| --- | --- |
| `swarm-builder` | `swarm-builder.md` |
| `swarm-fixer` | `swarm-fixer.md` |
| `swarm-improver-devex` | `swarm-improver-devex.md` |
| `swarm-improver-docs` | `swarm-improver-docs.md` |
| `swarm-improver-infra` | `swarm-improver-infra.md` |
| `swarm-improver-tests` | `swarm-improver-tests.md` |
| `swarm-janitor` | `swarm-janitor.md` |
| `swarm-merger` | `swarm-merger.md` |
| `swarm-pr-responder` | `swarm-pr-responder.md` |
| `swarm-reviewer` | `swarm-reviewer.md` |
| `swarm-scout` | `swarm-scout.md` |
| `swarm-strategist` | `swarm-strategist.md` |
| `swarm-validator` | `swarm-validator.md` |
