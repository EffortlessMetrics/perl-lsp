# LSP Test Suite

This directory contains comprehensive tests for the Perl LSP server implementation.

## Test Organization

### Core LSP Tests
- `lsp_comprehensive_e2e_test.rs` - End-to-end tests for all LSP features
- `lsp_user_story_tests.rs` - Real-world user scenarios
- `lsp_features_snapshot_test.rs` - Feature compliance tracking
- `lsp_feature_gating_test.rs` - Ensures disabled features are properly gated

### Feature-Specific Tests
- `lsp_completion_tests.rs` - Code completion
- `lsp_hover_tests.rs` - Hover information
- `lsp_signature_tests.rs` - Signature help
- `lsp_diagnostics_tests.rs` - Error and warning detection
- `lsp_workspace_symbols_tests.rs` - Workspace-wide symbol search
- `lsp_rename_tests.rs` - Symbol renaming
- `lsp_code_actions_tests.rs` - Quick fixes and refactorings
- `lsp_semantic_tokens_tests.rs` - Semantic highlighting
- `lsp_inlay_hints_tests.rs` - Inline parameter hints
- `lsp_document_links_tests.rs` - Module navigation
- `lsp_selection_range_tests.rs` - Smart selection
- `lsp_on_type_formatting_tests.rs` - Auto-formatting

### Stress & Performance Tests
- `lsp_stress_tests.rs` - Resource exhaustion tests
- `lsp_memory_pressure.rs` - Memory limits testing
- `lsp_cancellation_test.rs` - Request cancellation

## Running Tests

### Basic Test Execution
```bash
# Run all LSP tests
cargo test -p perl-parser lsp

# Run specific test file
cargo test -p perl-parser --test lsp_completion_tests

# Run with output
cargo test -p perl-parser lsp -- --nocapture
```

### Snapshot Tests (Deterministic Environment)
For reproducible snapshot tests across different environments:

```bash
# Update snapshots with deterministic environment
LC_ALL=C.UTF-8 INSTA_UPDATE=auto cargo test -p perl-parser --test lsp_features_snapshot_test

# Review snapshot changes
cargo insta review

# Accept all snapshot changes
cargo insta accept
```

**Important**: Always use `LC_ALL=C.UTF-8` when updating snapshots to ensure:
- Consistent sort order across systems
- Deterministic output formatting
- CI/CD compatibility

### Stress Tests (Configurable)
```bash
# Run stress tests with custom iterations
PERL_LSP_STRESS_ITERS=1000 cargo test -p perl-parser --test lsp_stress_tests -- --ignored

# Run memory tests with custom scale
PERL_LSP_MEMORY_SCALE=10 cargo test -p perl-parser --test lsp_memory_pressure -- --ignored

# Run in CI with lower limits
PERL_LSP_STRESS_ITERS=100 PERL_LSP_MEMORY_SCALE=1 cargo test -- --ignored
```

## Test Infrastructure

### Support Modules
- `support/lsp_harness.rs` - LSP server test harness
- `support/client_caps.rs` - Shared client capabilities
- `support/mod.rs` - Assertion helpers and utilities

### Client Capabilities
All tests should use the shared client capabilities for consistency:

```rust
use support::client_caps;

// Get full capabilities (all features enabled)
let caps = client_caps::full();

// Get minimal capabilities
let caps = client_caps::minimal();

// Get specific features
let caps = client_caps::with_features(&["completion", "hover"]);
```

## CI Integration

### GitHub Actions Workflow
```yaml
- name: Run LSP tests
  env:
    LC_ALL: C.UTF-8  # Deterministic snapshots
    PERL_LSP_STRESS_ITERS: 100  # Reduced for CI
    PERL_LSP_MEMORY_SCALE: 1
  run: |
    cargo test -p perl-parser
    cargo test -p perl-parser -- --ignored  # Stress tests
```

### Snapshot Verification
```bash
# CI should verify snapshots haven't changed
LC_ALL=C.UTF-8 cargo test -p perl-parser --test lsp_features_snapshot_test
git diff --exit-code crates/perl-parser/tests/snapshots/
```

## Adding New Tests

1. **Create test file** in appropriate category
2. **Use shared helpers** from `support/` modules
3. **Add to feature catalog** if testing new LSP feature
4. **Update snapshots** if feature changes capabilities
5. **Document environment variables** if test is configurable

## Test Coverage

Current coverage as of v0.8.5:
- ✅ 530+ unit tests
- ✅ 33 comprehensive E2E tests
- ✅ 100% advertised feature coverage
- ✅ All user stories passing
- ✅ Stress tests validate graceful degradation