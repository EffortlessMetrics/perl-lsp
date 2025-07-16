# tree-sitter-perl → Rust Conversion Roadmap

> **Objective**: Convert tree-sitter-perl from C/JS to pure Rust implementation while maintaining full compatibility and improving maintainability.

---

## 🎯 Project Overview

| Metric | Current | Target |
|--------|---------|--------|
| **Language** | C/JS | Rust |
| **Dependencies** | C toolchain + Node.js | Rust only |
| **Build Time** | Multi-step (C + JS) | Single `cargo build` |
| **Test Coverage** | Corpus tests only | Corpus + Rust unit tests |
| **Maintainability** | Mixed C/JS | Pure Rust |

---

## 📋 Phase 1: Foundation & Analysis

### 1.1 Project Setup & Inventory
- [x] Analyze current codebase structure
- [x] Identify C components for porting
- [x] Document current build process
- [ ] Create Rust project structure
- [ ] Set up development environment

**Deliverables**: Project structure, component inventory, development setup

### 1.2 Scanner Logic Analysis
- [ ] Deep dive into `src/scanner.c` (970 lines)
- [ ] Document all token types and state management
- [ ] Analyze heredoc, quoting, and Unicode logic
- [ ] Identify external dependencies and helpers

**Deliverables**: Scanner specification, state machine documentation

---

## 🔧 Phase 2: Core Porting

### 2.1 Scanner Port (High Priority)
- [ ] Create `src/scanner.rs` skeleton
- [ ] Port token type definitions
- [ ] Port state management (LexerState, TSPQuote, etc.)
- [ ] Port heredoc logic
- [ ] Port quote stack management
- [ ] Port Unicode identifier helpers
- [ ] Implement serialization/deserialization
- [ ] Add comprehensive unit tests

**Estimated Effort**: 3-5 days  
**Dependencies**: None  
**Deliverables**: `src/scanner.rs`, unit tests

### 2.2 Unicode & Helper Logic
- [ ] Port `src/tsp_unicode.h` to Rust
- [ ] Port `src/bsearch.h` utilities
- [ ] Evaluate `unicode-ident` crate integration
- [ ] Create `src/unicode.rs` module
- [ ] Add property-based tests for Unicode edge cases

**Estimated Effort**: 1-2 days  
**Dependencies**: 2.1  
**Deliverables**: `src/unicode.rs`, Unicode tests

### 2.3 Build System Migration
- [ ] Remove C build logic from `bindings/rust/build.rs`
- [ ] Update `Cargo.toml` to remove C dependencies
- [ ] Ensure pure Rust build process
- [ ] Test build in clean environment

**Estimated Effort**: 0.5 days  
**Dependencies**: 2.1, 2.2  
**Deliverables**: Updated build files

---

## 🔗 Phase 3: Integration & Binding

### 3.1 Rust Bindings Update
- [ ] Update `bindings/rust/lib.rs` to use Rust scanner
- [ ] Register scanner with generated parser
- [ ] Expose all required constants and queries
- [ ] Ensure API compatibility

**Estimated Effort**: 1 day  
**Dependencies**: 2.1, 2.2, 2.3  
**Deliverables**: Updated `lib.rs`

### 3.2 Grammar Integration
- [ ] Verify `grammar.js` compatibility with Rust scanner
- [ ] Test scanner integration with generated parser
- [ ] Ensure all external tokens are handled

**Estimated Effort**: 0.5 days  
**Dependencies**: 3.1  
**Deliverables**: Working grammar integration

---

## 🧪 Phase 4: Testing & Validation

### 4.1 Corpus Test Validation
- [ ] Run all `test/corpus/` tests with Rust scanner
- [ ] Fix any regressions or edge cases
- [ ] Ensure 100% test pass rate
- [ ] Document any test changes needed

**Estimated Effort**: 1-2 days  
**Dependencies**: 3.2  
**Deliverables**: All corpus tests passing

### 4.2 Rust Unit Test Suite
- [ ] Add comprehensive unit tests for scanner logic
- [ ] Add property-based tests for complex scenarios
- [ ] Test serialization/deserialization
- [ ] Test Unicode edge cases
- [ ] Achieve >90% code coverage

**Estimated Effort**: 2-3 days  
**Dependencies**: 4.1  
**Deliverables**: Comprehensive test suite

### 4.3 Integration Testing
- [ ] Test with Neovim tree-sitter integration
- [ ] Test with Emacs tree-sitter integration
- [ ] Test with other Tree-sitter consumers
- [ ] Performance benchmarking

**Estimated Effort**: 1 day  
**Dependencies**: 4.2  
**Deliverables**: Integration test results

---

## 📚 Phase 5: Documentation & Polish

### 5.1 Documentation Update
- [ ] Update `README.md` for Rust usage
- [ ] Document scanner API and usage
- [ ] Add development setup instructions
- [ ] Create migration guide for downstream users

**Estimated Effort**: 1 day  
**Dependencies**: 4.3  
**Deliverables**: Updated documentation

### 5.2 Code Quality & Linting
- [ ] Run `clippy` and fix all warnings
- [ ] Format code with `rustfmt`
- [ ] Add comprehensive doc comments
- [ ] Review for idiomatic Rust patterns

**Estimated Effort**: 0.5 days  
**Dependencies**: 5.1  
**Deliverables**: Clean, idiomatic Rust code

---

## 🚀 Phase 6: Release & Deployment

### 6.1 CI/CD Setup
- [ ] Update GitHub Actions for Rust builds
- [ ] Add Rust-specific CI checks
- [ ] Ensure all tests run in CI
- [ ] Add performance regression testing

**Estimated Effort**: 1 day  
**Dependencies**: 5.2  
**Deliverables**: CI/CD pipeline

### 6.2 Release Preparation
- [ ] Version bump and changelog
- [ ] Tag release
- [ ] Publish to crates.io
- [ ] Update downstream consumers

**Estimated Effort**: 0.5 days  
**Dependencies**: 6.1  
**Deliverables**: Released Rust-native version

---

## 📊 Progress Tracking

### Current Status
- **Phase**: 1.1 (Foundation & Analysis)
- **Completion**: 40% (Analysis complete, setup pending)
- **Next Milestone**: Scanner port completion

### Blockers & Risks
| Risk | Impact | Mitigation |
|------|--------|------------|
| Scanner logic complexity | High | Incremental port, extensive testing |
| Unicode edge cases | Medium | Use proven crates, property tests |
| Downstream breakage | Medium | Maintain API compatibility |
| Build system issues | Low | Remove all C dependencies |

### Success Metrics
- [ ] Zero C dependencies in final build
- [ ] 100% corpus test pass rate
- [ ] >90% Rust code coverage
- [ ] All downstream consumers working
- [ ] Build time improvement >50%

---

## 🛠 Development Guidelines

### Rust Standards
- Use Rust 2021 edition
- Follow `clippy` recommendations
- Comprehensive error handling
- Extensive documentation
- Property-based testing where appropriate

### Testing Strategy
- Unit tests for all scanner logic
- Property tests for Unicode/quoting
- Integration tests with corpus
- Performance benchmarks
- Downstream compatibility tests

### Code Organization
```
src/
├── scanner.rs      # Main scanner logic
├── unicode.rs      # Unicode helpers
├── types.rs        # Type definitions
└── tests/          # Unit tests
```

---

## 📅 Timeline

| Phase | Duration | Start | End |
|-------|----------|-------|-----|
| 1. Foundation | 1 week | TBD | TBD |
| 2. Core Porting | 1-2 weeks | TBD | TBD |
| 3. Integration | 1 week | TBD | TBD |
| 4. Testing | 1-2 weeks | TBD | TBD |
| 5. Documentation | 1 week | TBD | TBD |
| 6. Release | 1 week | TBD | TBD |

**Total Estimated Duration**: 6-8 weeks

---

## 🔄 Maintenance & Future

### Post-Release Tasks
- [ ] Monitor downstream adoption
- [ ] Address any compatibility issues
- [ ] Performance optimization
- [ ] Feature additions (if needed)

### Long-term Goals
- [ ] Rust-native grammar definition (if Tree-sitter supports)
- [ ] Enhanced error reporting
- [ ] Performance improvements
- [ ] Extended language support

---

*Last Updated: [Current Date]*  
*Next Review: [Weekly]* 