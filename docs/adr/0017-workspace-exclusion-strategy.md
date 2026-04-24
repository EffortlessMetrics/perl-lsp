# ADR-0017: Workspace Exclusion Strategy

**Status**: Accepted
**Date**: 2025-02-15
**Decision Makers**: Perl LSP Architecture Team
**Related**: [ARCHITECTURE_OVERVIEW.md](../reference/ARCHITECTURE_OVERVIEW.md), [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md)

## Context

The Perl LSP workspace contains 80+ crates with diverse dependencies and build requirements. Some crates require system-level C dependencies (libclang, bindgen) that create platform-specific build challenges. This creates several problems:

1. **Platform Fragility**: Builds fail on systems without C toolchains
2. **CI Instability**: Cross-platform CI runners have varying system dependency availability
3. **User Installation Friction**: Published crates should install cleanly without system prerequisites
4. **Development vs Production Tension**: Internal tooling needs differ from published crate requirements

### Problem Statement

The tree-sitter integration crates require C dependencies:
- `tree-sitter-perl-c`: Requires libclang-dev
- `tree-sitter-perl-rs`: Requires bindgen for C interop
- `tree-sitter-perl/`: Original C implementation with libclang dependency

These dependencies create build failures for users who only need the pure Rust parser and LSP server.

## Decision

**We implement a production-focused exclusion strategy that removes crates with C dependencies from the main workspace build, prioritizing published crate reliability over comprehensive internal tooling.**

### Excluded Crates

| Crate | Exclusion Reason |
|-------|------------------|
| `tree-sitter-perl-c` | libclang-dev dependency |
| `tree-sitter-perl-rs` | bindgen dependency |
| `tree-sitter-perl/` | libclang dependency |
| Legacy tooling | Internal development only |

### Implementation

The workspace configuration excludes these crates from default builds:

```toml
# Cargo.toml workspace configuration
[workspace]
members = [
    "crates/perl-parser",
    "crates/perl-lsp-rs",
    "crates/perl-dap",
    "crates/perl-lexer",
    # ... other pure-Rust crates
]
# Excluded: tree-sitter-perl-c, tree-sitter-perl-rs
```

### Architectural Benefits

1. **Platform Independence**: No C toolchain requirements for standard builds
2. **CI Stability**: Consistent build behavior across Windows, macOS, Linux
3. **Production Focus**: Testing only published crate surface area
4. **Dependency Safety**: Avoid system-specific build failures
5. **Clean Installation**: Users can `cargo install perllsp` without prerequisites

## Consequences

### Positive

- **Reliable Cross-Platform Builds**: Pure Rust builds work everywhere
- **Faster CI Pipelines**: No C compilation overhead
- **Reduced Support Burden**: No system dependency troubleshooting
- **Clean Published Experience**: `cargo install` just works
- **Simplified Dependency Tree**: No transitive C dependency issues

### Negative

- **Reduced Workspace Coverage**: Tree-sitter crates not tested in main CI
- **Manual Benchmarking**: C parser benchmarks require separate environment
- **Development Overhead**: Tree-sitter work requires explicit workspace opt-in
- **Feature Parity Tracking**: Must maintain awareness of excluded capabilities

### Mitigations

- Separate benchmark infrastructure for tree-sitter comparison
- Documentation clearly indicates excluded crates
- Benchmark scripts handle workspace exclusion gracefully

## References

- [ARCHITECTURE_OVERVIEW.md](../reference/ARCHITECTURE_OVERVIEW.md) - Workspace configuration strategy
- [CRATE_ARCHITECTURE_GUIDE.md](../reference/CRATE_ARCHITECTURE_GUIDE.md) - Excluded crates section
- [WORKSPACE_TEST_REPORT.md](../project/WORKSPACE_TEST_REPORT.md) - Current workspace status
