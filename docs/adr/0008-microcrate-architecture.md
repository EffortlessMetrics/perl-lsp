# ADR-0008: Microcrate Architecture (Single Responsibility Principle)

**Status**: Accepted
**Date**: 2025-01-15
**Decision Makers**: Perl LSP Architecture Team
**Related**: [AGENTS.md](../../AGENTS.md), [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md)

## Context

The Perl LSP project has grown into an 80+ crate Rust workspace, raising questions about the granularity of crate organization. Traditional Rust projects typically use fewer, larger crates with internal module separation. This workspace takes a different approach with many small, focused crates organized into families:

| Family | Count | Purpose |
|--------|-------|---------|
| `perl-module-*` | ~13 | Module resolution microcrates |
| `perl-lsp-*` | ~21 | LSP feature providers |
| `perl-lsp-feature-*` | ~7 | Feature governance subsystem |
| `perl-dap-*` | ~4 | Debug adapter components |
| `perl-ts-*` | ~5 | Tree-sitter integration |
| `perl-workspace-*` | ~4 | Workspace discovery and indexing |

### Problem Statement

1. **Compilation Efficiency**: How to maximize parallel compilation across the codebase?
2. **Dependency Management**: How to prevent circular dependencies and maintain clear boundaries?
3. **Code Organization**: How to make it easy for contributors to find relevant code?
4. **Release Flexibility**: How to enable independent versioning of components?

## Decision

**We adopt a microcrate architecture where each crate has a single, well-defined responsibility following the Single Responsibility Principle (SRP).**

### Core Principles

1. **One Job Per Crate**: Each crate implements one cohesive concern
   - `perl-token` - Token types and lexing infrastructure
   - `perl-ast` - Abstract syntax tree definitions
   - `perl-quote` - Quote-like operator handling
   - `perl-regex` - Regular expression parsing
   - `perl-heredoc` - Heredoc processing

2. **Clear Dependency Direction**: Dependencies flow inward toward core crates
   - LSP providers depend on parser crates
   - Parser crates depend on core infrastructure
   - Core crates have minimal dependencies

3. **Family Grouping**: Related crates share a common prefix for discoverability
   - `perl-lsp-*` for all LSP-specific functionality
   - `perl-module-*` for module resolution chain
   - `perl-dap-*` for debug adapter components

4. **Independent Versioning**: Crates can be released independently when changes are localized

### Crate Categories

**Core Leaf Crates** (~30 crates):
- Token, AST, quote, regex, heredoc, error handling
- Zero or minimal internal dependencies
- Stable APIs, rarely change

**Integration Crates** (~20 crates):
- Parser, lexer, semantic analyzer
- Combine core crates into usable units
- Moderate dependency complexity

**Feature Crates** (~30 crates):
- LSP providers, DAP components
- User-facing functionality
- Higher dependency count, more frequent changes

## Alternatives Considered

### Option 1: Monolithic Crate Architecture
**Description**: Consolidate into 5-10 larger crates with internal modules

**Pros**:
- Simpler `Cargo.toml` management
- Fewer crate boundaries to consider
- Easier cross-module refactoring

**Cons**:
- Reduced parallel compilation opportunities
- Higher risk of circular dependencies
- All-or-nothing releases
- Harder to reason about impact of changes

**Decision**: Rejected - does not scale for 80+ functional areas

### Option 2: Hybrid Architecture
**Description**: Use medium-sized crates (15-20) with some internal modules

**Pros**:
- Balance between granularity and simplicity
- Moderate parallel compilation

**Cons**:
- Inconsistent boundaries
- Ambiguous module vs crate decisions
- Still risks circular dependencies

**Decision**: Rejected - creates ambiguity in organization

## Consequences

### Positive

1. **Compilation Parallelism**: 
   - Cargo can compile independent crates simultaneously
   - Clean builds utilize all available CPU cores effectively
   - Incremental builds only recompile affected crates

2. **Clear Boundaries**:
   - Each crate has a single, documented purpose
   - Contributors can quickly locate relevant code
   - API surface is explicit and minimal

3. **Dependency Clarity**:
   - `Cargo.toml` explicitly shows all dependencies
   - Circular dependencies are caught at crate level
   - Dependency graphs are meaningful and auditable

4. **Release Flexibility**:
   - Bug fixes can target specific crates
   - Semver compliance is easier to verify per crate
   - Users can depend on only what they need

5. **Testing Isolation**:
   - Unit tests are naturally scoped
   - Integration tests clearly span crate boundaries
   - Test parallelization is more effective

### Negative

1. **Workspace Complexity**:
   - 80+ `Cargo.toml` files to maintain
   - Workspace coordination requires discipline
   - IDE indexing can be slower initially

2. **Crate Management Overhead**:
   - Version synchronization across related crates
   - Changelog maintenance across crates
   - Release process touches multiple crates

3. **Learning Curve**:
   - New contributors need to understand crate layout
   - Finding the right crate for a change requires knowledge
   - Documentation must cover crate organization

### Mitigations

1. **Documentation**: 
   - [AGENTS.md](../../AGENTS.md) provides crate family overview
   - Each crate has README with purpose and API summary

2. **Tooling**:
   - Workspace-level `Cargo.toml` manages shared dependencies
   - `cargo-semver-checks` ensures API compatibility
   - CI validates cross-crate consistency

3. **Conventions**:
   - Consistent naming scheme (`perl-<family>-<feature>`)
   - Standard error handling patterns
   - Shared test utilities

## References

- [Cargo Workspace Documentation](https://doc.rust-lang.org/cargo/reference/workspaces.html)
- [Rust API Guidelines](https://rust-lang.github.io/api-guidelines/)
- [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md)
