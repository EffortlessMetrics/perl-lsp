# ADR-0021: LSP Capability Contract Policy

**Status**: Accepted
**Date**: 2025-02-15
**Decision Makers**: Perl LSP Architecture Team
**Related**: [LSP_CAPABILITY_POLICY.md](../reference/LSP_CAPABILITY_POLICY.md), [features.toml](../../features.toml)

## Context

The LSP server implements 53+ distinct capabilities with varying maturity levels, performance characteristics, and stability guarantees. Without a formal contract policy:

1. **Advertised vs. Implemented Gap**: Capabilities advertised before fully tested
2. **Enterprise Risk**: Organizations enable features that aren't production-ready
3. **Release Management**: No clear distinction between GA and experimental features
4. **User Trust**: Broken features damage credibility

### Problem Statement

LSP clients rely on capability advertisements to determine available features. Advertising capabilities without comprehensive test coverage creates:
- False expectations of functionality
- Production incidents from unstable features
- Difficulty tracking feature maturity

## Decision

**We implement contract-driven capability advertisement: a capability is advertised only after its acceptance tests land. Conservative releases can use `lsp-ga-lock` feature to reduce the surface to proven GA core.**

### Capability Advertisement Contract

```markdown
**Contract-driven:** A capability is advertised only after its acceptance tests land.

- **Main branch:** full surface (only features with passing tests).
- **Conservative point release:** build with `--features lsp-ga-lock` to reduce the surface to the proven "GA core".
```

### Implementation Process

#### Adding a New Capability

1. **Implement feature** in `crates/perl-parser/src/*`
2. **Add acceptance tests** in `crates/perl-parser/tests/…`
3. **Flip the advertised bit** in `lsp_server.rs` **in the same PR**
4. **Update documentation**:
   - `LSP_ACTUAL_STATUS.md` (status/percent)
   - `README.md` (matrix row)
   - Contract tests (`lsp_capabilities_contract_full.rs`)

### CI Configuration

```yaml
# Default CI - full surface with passing tests
- name: Run workspace tests
  run: cargo test --workspace

# Conservative release - GA core only
- name: Run GA lock tests
  run: cargo test -p perl-parser --features lsp-ga-lock --test lsp_capabilities_contract_lock
```

### Capability Categories

| Category | Examples | Advertisement |
|----------|----------|---------------|
| **GA Core** | textDocument/completion, textDocument/definition | Always advertised |
| **GA Extended** | textDocument/references, textDocument/implementation | Main branch |
| **Beta** | textDocument/semanticTokens | Feature flag |
| **Experimental** | textDocument/inlayHint | Explicit opt-in |

### Feature Flag Architecture

```rust
// Feature flag for conservative releases
#[cfg(feature = "lsp-ga-lock")]
fn build_capabilities() -> ServerCapabilities {
    // GA core only - proven stable features
    ServerCapabilities {
        completion_provider: Some(...),
        definition_provider: Some(...),
        // ... only GA features
    }
}

#[cfg(not(feature = "lsp-ga-lock"))]
fn build_capabilities() -> ServerCapabilities {
    // Full surface - all features with passing tests
    ServerCapabilities {
        completion_provider: Some(...),
        definition_provider: Some(...),
        references_provider: Some(...),
        semantic_tokens_provider: Some(...),
        // ... all tested features
    }
}
```

## Consequences

### Positive

- **Trustworthy Advertisements**: Advertised capabilities have test coverage
- **Enterprise Control**: Conservative releases for risk-averse organizations
- **Clear Maturity Tracking**: Documentation reflects actual capability status
- **Release Flexibility**: Same codebase supports different release profiles
- **Quality Enforcement**: Tests must land before advertisement

### Negative

- **Conservative Rollout**: New features delayed until tests complete
- **Maintenance Overhead**: Two capability profiles to maintain
- **Coordination Required**: Feature and test must land together

### Mitigations

- Clear documentation of capability categories
- Automated checks for advertisement/test alignment
- Feature branch development for experimental features

## Release Modes

### Main Branch (Full Surface)

```bash
# Default build - all tested features
cargo build -p perl-lsp-rs
```

Advertises all capabilities with passing acceptance tests.

### Conservative Release (GA Core)

```bash
# Conservative build - proven stable only
cargo build -p perl-lsp-rs --features lsp-ga-lock
```

Advertises only GA core capabilities with extensive production validation.

## Compliance Tracking

| Metric | Target | Validation |
|--------|--------|------------|
| Advertised coverage | 100% tested | Contract tests |
| GA core stability | 100% passing | CI gate |
| Documentation sync | 100% aligned | Automated checks |

## References

- [LSP_CAPABILITY_POLICY.md](../reference/LSP_CAPABILITY_POLICY.md) - Policy specification
- [features.toml](../../features.toml) - Feature tracking
- [LSP_ACTUAL_STATUS.md](../project/LSP_ACTUAL_STATUS.md) - Current capability status
- [ADR-0016: Feature Governance](0016-feature-governance.md) - Related feature governance ADR
