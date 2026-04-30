# Project Structure

## Repository Layout

```
perl-lsp/
├── crates/                    # All Rust crates (workspace members)
│   ├── perllsp/               # CLI binary entry point
│   ├── perl-lsp-rs/           # LSP server binary and host
│   ├── perl-lsp-rs-core/      # Core LSP server logic (providers, transport, config, governance)
│   ├── perl-lsp-perltidy/     # PerlTidy integration
│   ├── perl-lsp-ux-tests/     # LSP UX/integration tests
│   ├── perl-dap/              # Debug Adapter Protocol server
│   ├── perl-parser/           # Main recursive-descent parser (v3)
│   ├── perl-parser-core/      # Shared parser infrastructure
│   ├── perl-parser-bench/     # Parser benchmarks
│   ├── perl-parser-pest/      # Legacy PEG parser (compatibility)
│   ├── perl-lexer/            # Context-aware tokenizer
│   ├── perl-ast/              # AST node types
│   ├── perl-ast-v2/           # AST v2 node types
│   ├── perl-token/            # Token types
│   ├── perl-pragma/           # Pragma handling
│   ├── perl-regex/            # Regex safety and complexity analysis
│   ├── perl-semantic-analyzer/# Semantic analysis and symbol resolution
│   ├── perl-semantic-facts/   # Semantic fact types
│   ├── perl-workspace-index/  # Cross-file indexing and lookup
│   ├── perl-refactoring/      # Refactoring operations
│   ├── perl-incremental-parsing/ # Incremental parsing support
│   ├── perl-diagnostics/      # Diagnostic types
│   ├── perl-dead-code/        # Dead code detection
│   ├── perl-module/           # Module resolution (unified facade)
│   ├── perl-symbol/           # Symbol types
│   ├── perl-uri/              # URI handling
│   ├── perl-pod/              # POD documentation parsing
│   ├── perl-line-index/       # Line/column index utilities
│   ├── perl-position-tracking/# Source position tracking
│   ├── perl-subprocess-runtime/# Subprocess management
│   ├── perl-corpus/           # Test corpus utilities
│   ├── perl-ci-hygiene/       # CI hygiene checks
│   ├── perl-tdd-support/      # TDD test helpers
│   ├── perl-test-must/        # must/must_some test assertions
│   ├── perl-test-generators/  # Test data generators
│   ├── tree-sitter-perl-c/    # Tree-sitter C bindings for Perl
│   └── tree-sitter-perl-rs/   # Tree-sitter Rust facade
├── vscode-extension/          # VS Code extension (TypeScript)
├── xtask/                     # Custom cargo xtask commands
├── tests/                     # Integration tests
├── test_corpus/               # Perl test files for parser validation
├── benchmarks/                # Benchmark data and configs
├── fuzz/                      # Fuzz testing targets (cargo-fuzz)
├── docs/                      # Documentation
│   ├── project/               # Project status, roadmap, CI docs
│   ├── reference/             # Reference guides (commands, config, architecture)
│   ├── how-to/                # How-to guides (editor setup, troubleshooting)
│   ├── tutorials/             # Getting started tutorials
│   └── articles/              # Design articles and patterns
├── scripts/                   # Shell scripts for CI and tooling
├── schemas/                   # JSON schemas
├── hooks/                     # Git hooks (pre-push)
├── .ci/                       # CI configuration, baselines, policies
├── .justfiles/                # Additional justfile includes
├── Cargo.toml                 # Workspace root manifest
├── justfile                   # Task runner recipes
├── flake.nix                  # Nix flake for reproducible dev env
├── features.toml              # LSP capability catalog
├── AGENTS.md                  # AI implementation agent instructions
├── CLAUDE.md                  # Orchestrator agent instructions
└── CONTRIBUTING.md            # Human contributor guide
```

## Crate Architecture

Crates are organized in a tiered dependency hierarchy:

- **Tier 1 (Leaf)**: `perl-token`, `perl-position-tracking`, `perl-regex`, `perl-pod`, `perl-subprocess-runtime`
- **Tier 2 (Core infra)**: `perl-ast`, `perl-lexer`, `perl-pragma`, `perl-parser-core`, `perl-tdd-support`
- **Tier 3 (Analysis)**: `perl-parser`, `perl-semantic-analyzer`, `perl-workspace-index`, `perl-diagnostics`, `perl-module`
- **Tier 4 (LSP providers)**: `perl-lsp-rs-core` (consolidated — most `perl-lsp-*` satellites absorbed here)
- **Tier 5 (Application)**: `perl-lsp-rs`, `perllsp`, `perl-dap`

Many former satellite crates have been absorbed into `perl-lsp-rs-core` through consolidation waves (G1a, G1b, G2, G3). Comments in `Cargo.toml` document which crates were absorbed and where.

## Key Conventions
- New crates go under `crates/` following the naming convention of their family
- Add new crates to `workspace.members` in root `Cargo.toml`
- PRs should touch one concern only — no bundled unrelated changes
- Do not commit top-level `adr.md`, `specs.md`, or `task_list.md`
