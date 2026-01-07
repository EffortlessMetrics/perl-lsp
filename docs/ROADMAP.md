# Perl Parser Project - Roadmap

> **📌 CANONICAL**: This is the authoritative roadmap. See `CURRENT_STATUS.md` for verification protocol.
> **Stale roadmaps**: Archived at `docs/archive/roadmaps/`; retrieve from git history if needed.

> **Status (2026-01-07)**: Validation + packaging phase toward v0.9.0 ("semantic-ready") and v1.0.
>
> **Canonical receipt**: `nix develop -c just ci-gate` must be green before merging.
> **CI** is intentionally optional/opt-in; the repo is local-first by design.

---

## Truth Rules (read this first)

This roadmap describes goals, but any **status claim** must be backed by one of:
- `nix develop -c just ci-gate` output
- `bash scripts/ignored-test-count.sh` output
- A tracked feature matrix / snapshot test (e.g., GA-lock capabilities snapshot)

If a number is not backed by a receipt, it must be labeled **UNVERIFIED** or removed.

**Last verified**: 2026-01-07 – `ci-gate` green on Rust 1.89 MSRV (337 lib tests + 4 LSP semantic tests)

---

## Current State (v0.8.8 → v0.9.0)

| Component | Release Stance | Evidence | Notes |
|-----------|----------------|----------|-------|
| **perl-parser** (v3) ⭐ | Production | `just ci-gate` | Parser v3, statement tracker + heredocs in place |
| **perl-lexer** | Production | `just ci-gate` | Tokenization stable |
| **perl-corpus** | Production | `just ci-gate` | Regression corpus + mutation hardening inputs |
| **perl-lsp** | Production (advertised subset) | capability snapshots + targeted tests | Advertise only what's tested; keep GA-lock stable |
| **perl-dap** | Experimental (bridge mode) | manual smoke | Bridges to Perl::LanguageServer; not "full" native DAP |
| **perl-parser-pest** (v2) | Legacy | N/A | Optional crate; keep out of default gate |
| **Semantic Analyzer** | Phase 1 Complete | `just ci-gate` | 12/12 handlers; lexical scoping + textDocument/definition |

*Only features that pass `ci-gate` or have targeted integration tests count as "Production".*

---

## 📦 Component Status (v0.8.8)

### Published Crates
| Crate | Version | Status | Purpose |
|-------|---------|--------|----------|
| **perl-parser** | v0.8.8 | ✅ Production | Main parser library |
| **perl-lsp** | v0.8.8 | ✅ Production | Production LSP server (~91% functional) |
| **perl-lexer** | v0.8.8 | ✅ Production | Context-aware tokenizer |
| **perl-corpus** | v0.8.8 | ✅ Production | Test corpus (141 edge cases) |
| **perl-dap** | v0.1.0 | ✅ Phase 1 | Debug Adapter Protocol (bridge mode) |
| **perl-parser-pest** | v0.8.8 | ⚠️ Legacy/Experimental | Pest-based parser (maintained) |

### Component Integration
- **perl-parser + perl-lexer**: Fully integrated for v3 parser
- **perl-corpus**: Used by all parsers for validation
- **perl-lsp binary**: Ships with perl-parser crate
- **Tree-sitter compatibility**: All parsers output compatible AST

## ✅ What We've Achieved

### Production Components
- **perl-parser**: Native recursive descent parser + LSP server ⭐ **MAIN CRATE**
- **perl-lexer**: Context-aware tokenizer with mode-based lexing
- **perl-corpus**: Test corpus with 141 edge cases
- **perl-parser-pest**: Legacy Pest parser (feature complete within Pest limits)
- **perl-lsp**: Production LSP server with ~93% LSP 3.18 compliance

### Key Accomplishments (v0.8.5)
- ✅ **100% edge case coverage** (141/141 tests passing)
- ✅ **World-class performance** (4-19x faster than C)
- ✅ **LSP 3.18 Compliance** (75% of all LSP features, 100% of advertised features working)

<!-- BEGIN: COMPLIANCE_TABLE -->
| Area | Implemented | Total | Coverage |
|------|-------------|-------|----------|
| text_document | 23 | 26 | 88% |
| workspace | 5 | 7 | 71% |
| window | 3 | 4 | 75% |
| notebook | 0 | 2 | 0% |
| debug | 0 | 2 | 0% |
| **Overall** | **31** | **41** | **75%** |
<!-- END: COMPLIANCE_TABLE -->
  - **Text Document Features** (90% complete)
    - ✅ Diagnostics (push & pull models)
    - ✅ Completion with 150+ built-ins
    - ✅ Hover with documentation
    - ✅ Signature help with parameters
    - ✅ Go to definition/declaration
    - ✅ Find references (workspace-wide)
    - ✅ Document symbols (hierarchical)
    - ✅ Code actions & quick fixes
    - ✅ Code lens with reference counts
    - ✅ Document formatting (Perl::Tidy)
    - ✅ Range formatting
    - ✅ On-type formatting
    - ✅ Rename symbol (cross-file)
    - ✅ Document links
    - ✅ Folding ranges
    - ✅ Selection ranges
    - ✅ Semantic tokens
    - ✅ Inlay hints
    - ✅ Type hierarchy (v0.8.5)
    - ✅ Call hierarchy
    - ✅ Linked editing ranges (v0.8.6)
    - ✅ Type definition (v0.8.6 GA - multi-file support)
    - ✅ Implementation (v0.8.6 GA - workspace index)
    - ✅ Inline completions (v0.8.6 preview - via experimental)
    - ✅ Inline values (v0.8.6 - debug context)
    - ✅ Document color (v0.8.6 - hex detection)
    - ✅ Color presentation (v0.8.6)
    - ✅ Prepare rename (v0.8.6)
  - **Workspace Features** (70% complete)
    - ✅ Workspace symbols
    - ✅ Workspace diagnostics (pull)
    - ✅ File operations
    - ✅ Execute command
    - ✅ Workspace folders
    - ⛴ Workspace edit
    - ✅ Moniker (v0.8.6 - stable identifiers)
  - **Window Features** (60% complete)
    - ✅ Progress reporting
    - ✅ Log messages
    - ✅ Show message
    - ⛴ Work done progress
  - **Notebook Support** (0% - planned)
    - ⛴ Notebook document sync
    - ⛴ Notebook cell execution
- ✅ **Performance optimizations**
  - ✅ AST caching with LRU eviction
  - ✅ Symbol indexing with trie structure
  - ✅ Content hashing for change detection
  - ✅ Parallel file processing
- ✅ **Zero C dependencies** (v2 and v3)
- ✅ **Comprehensive documentation**
- ✅ **Distribution scaffolding ready** (internal binaries; public packages TBD)

---

## 🎯 Current Phase: Validation & De-Risking (December 2025)

**Status**: ~85-90% "fully working" for core goal: "Perl parser + LSP that actually works"

### ✅ Recent Completions

1. **Statement Tracker & Heredocs** - ✅ **100% COMPLETE** (2025-11-20)
   - `HeredocContext`, `BlockBoundary`, `BlockType` fully implemented
   - `StatementTracker` threaded through parser pipeline
   - AST integration (F1-F6 + edge cases) validated

2. **Semantic Analyzer Phase 1** - ✅ **COMPLETE** (2025-11-20)
   - 12/12 critical node handlers implemented
   - `SemanticModel` stable API wrapper
   - LSP `textDocument/definition` integrated

3. **Band 1: Semantic Stack Validation** - ✅ **COMPLETE** (2025-12-27)
   - `just ci-gate` verified on Rust 1.89 MSRV
   - 337 library tests passing (perl-parser: 279, perl-dap: 37, perl-corpus: 12, perl-lexer: 9)
   - 4/4 LSP semantic definition tests passing
   - Format, clippy, and policy checks all green

### 📋 Path to "Fully Working" v1.0

**Band 2: Reduce Ignored Tests** (1-2 weeks part-time) - 🟢 **IN PROGRESS**
- [x] Inventory ignored tests by file and reason (**done**: `docs/ci/IGNORED_TESTS_INDEX.md`)
- [x] Fix TestContext wrapper (params: `None` → `json!(null)`, add `initialize_with()`)
- [x] Apply "flip strategy" to protocol violations: 26 → 4 ignores (**-22**)
- [x] Sweep window progress tests: 21 → 0 ignores (**-21**)
- [x] Sweep unhappy paths tests: 9 → 1 ignores (**-8**)
- [x] Feature-gate `lsp_advanced_features_test.rs` (23 tests behind `lsp-extras`)
- [x] Complete sweep on all high-confidence files ✅
- **Current**: BUG=0, MANUAL=1 (run `bash scripts/ignored-test-count.sh` for live counts)
- **Target**: ✅ Achieved - only MANUAL utility test remains ignored

**Band 3: Tag v0.9 Semantic-Ready** (1-2 weeks)
- [ ] Align README/status docs with semantic LSP capabilities
- [ ] Tag `v0.9.0-semantic-lsp-ready` milestone
- [ ] Update CHANGELOG with semantic analyzer + LSP definition features
- **Target**: Externally-consumable "it just works" release

### 🚧 Known Constraints
- **Ignored tests**: BUG=0 achieved; only MANUAL utility tests remain (see `scripts/.ignored-baseline`)
- **CI Pipeline**: Issue #211 blocks merge-blocking gates (#210)
- **Semantic Phase 2/3**: Advanced features deferred (closures, multi-file, imports)

---

## Next Releases

### v0.9.0: "Semantic-Ready" Milestone (2-5 focused PRs)

**Goal**: A release that external users can try without reading internal docs.

**Deliverables**:

1. **Docs truth pass**
   - README + CURRENT_STATUS + ROADMAP agree on what's real and what's aspirational
   - DAP language corrected to "bridge mode"
   - Performance claims moved behind published results ✅

2. **Release artifacts**
   - Confirm `cargo install --path crates/perl-lsp` works cleanly
   - Release notes that match *advertised* capabilities

3. **Capability contracts**
   - Keep GA-lock snapshot stable
   - Any new capability is either behind feature flag or not advertised

**Exit criteria**:
- `nix develop -c just ci-gate` green on MSRV
- `bash scripts/ignored-test-count.sh` shows BUG=0, MANUAL≤1
- Release notes generated, tag cut

### v1.0.0: "Boring Promises" (sequence after v0.9.0)

**Goal**: Freeze the surfaces you're willing to support.

**Deliverables**:

1. **Stability statement**
   - What "GA-lock" means (capabilities + wire protocol invariants)
   - Versioning rules for changes

2. **Packaging stance**
   - What you ship (binaries? crates? both?)
   - Minimum supported platforms (explicit)

3. **Benchmark publication**
   - One canonical benchmark run committed
   - Remove "UNVERIFIED" tags where you now have receipts

**Exit criteria**:
- Capability snapshot + docs aligned
- Benchmarks published
- Upgrade notes exist from v0.8.x → v1.0

---

## 🚀 Historical Roadmap (February 2025)

### ✅ Q1 2025: Polish & Distribution - **COMPLETED**
**Status**: All goals achieved ahead of schedule!

#### Accomplished (v0.7.2-v0.7.5)
- ✅ **Parser Improvements**
  - ✅ Fixed operator precedence for word operators
  - ✅ Fixed division operator parsing
  - ✅ Added 150+ built-in function signatures
  - ✅ 100% edge case coverage (141/141)
- ✅ **LSP Excellence**
  - ✅ Multi-file support with cross-file references
  - ✅ Performance optimization (AST caching, symbol indexing)
  - ✅ Workspace-wide symbol search with fuzzy matching
  - ✅ 25+ IDE features implemented
- ✅ **Distribution Ready**
  - ✅ Crates.io metadata configured
  - ✅ Homebrew formula template created
  - ✅ Debian/RPM build scripts ready
  - ✅ VSCode extension v0.6.0 configured

### 🎯 Q2 2025: LSP 3.18 Full Compliance & Community Adoption
**Goal**: Achieve 100% LSP 3.18 compliance and launch to the Perl community

#### Immediate Priority (August-September) - LSP 3.18 Full Compliance
- [ ] **LSP 3.18 Remaining Features** (30% to go)
  - [x] Type Definition Provider (v0.8.6 preview)
  - [x] Implementation Provider (v0.8.6 preview)
  - [ ] Notebook Document support (LSP 3.17)
  - [ ] Inline Completions (LSP 3.18)
  - [ ] Inline Values for debugging
  - [ ] Moniker support for cross-project navigation
  - [ ] Linked Editing Ranges
  - [ ] Full Semantic Tokens modifiers
- [ ] **Parser v3 Enhancements**
  - [ ] Incremental parsing optimization
  - [ ] Streaming parser for large files
  - [ ] Error recovery improvements
- [ ] **Corpus Expansion**
  - [ ] Add CPAN top 100 modules test suite
  - [ ] Property-based testing for edge cases
  - [ ] Fuzzing harness integration
- [ ] **Easy Installation** (public) — *in progress*
  - [ ] One-liner installer script with checksums
  - [ ] Homebrew formula for macOS
  - [ ] Public pre-built binaries for all platforms
  - [ ] Smart PATH detection and shell config
  - [x] Internal pre-built binaries for testing
- [x] **LSP 3.18 Compliance** ✅ **IN PROGRESS (70% complete)**
  - [x] Pull Diagnostics support
  - [x] Type Hierarchy implementation
  - [x] Type Definition provider (v0.8.6 preview)
  - [x] Implementation provider (v0.8.6 preview)
  - [x] Typed ServerCapabilities
  - [x] Enhanced cancellation handling
  - [ ] Notebook support
  - [ ] Inline completions
- [ ] **VSCode Extension v0.9.0 (LSP 3.18)**
  - [ ] Update to LSP 3.18 client
  - [ ] Add notebook support UI
  - [ ] Implement inline completions
  - [ ] Add debugging visualizations
  - [ ] Publish to marketplace with auto-updates
- [ ] **Package Managers**
  - [ ] Homebrew formula with SHA256 verification
  - [ ] Debian package (.deb) with GPG signing
  - [ ] RPM package with signing
  - [ ] AUR package for Arch Linux
- [x] **True Incremental Parsing** ✅ **COMPLETED - 200x BETTER THAN TARGET!**
  - [x] Integrate IncrementalDocument with LSP
  - [x] Implement subtree reuse optimization
  - [x] Achieve <1ms incremental updates (0.005ms achieved!)
- [x] **Workspace-wide Refactoring** ✅ **COMPLETED**
  - [x] WorkspaceIndex for cross-file symbol tracking
  - [x] Multi-file rename refactoring
  - [x] Extract module/package refactoring
- [x] **Dead Code Detection** ✅ **COMPLETED**
  - [x] Find unused code across workspace
  - [x] Detect unreachable code
  - [x] Cross-file dependency analysis
- [x] **Type Inference Foundation** ✅ **COMPLETED**
  - [x] Basic type system implementation
  - [x] Built-in function signatures
  - [x] Type-based completions
- [x] **Code Quality Tools** ✅ **COMPLETED**
  - [x] Perl::Critic integration with built-in analyzer
  - [x] External perlcritic tool support with fallback
  - [x] Perltidy integration for formatting
  - [x] Quick fixes for violations (add strict/warnings)
  - [x] Import optimization across workspace
  - [x] Dead code detection
- [x] **TDD Workflow Support** ✅ **COMPLETED**
  - [x] Test generation for subroutines (Test::More, Test2::V0)
  - [x] Red-green-refactor cycle management
  - [x] Refactoring suggestions with complexity analysis
  - [x] Coverage tracking and diagnostics
- [ ] **Documentation & Tutorials**
  - [ ] Video tutorial series
  - [ ] Migration guide from other Perl tools
  - [ ] API documentation for extension developers

#### Community Building (March)
- [ ] **Editor Plugins**
  - [ ] Neovim plugin with Lua config
  - [ ] Emacs MELPA package
  - [ ] Sublime Text package
  - [ ] IntelliJ IDEA plugin
- [ ] **Community Engagement**
  - [ ] Present at Perl conference/meetup
  - [ ] Blog post series on implementation
  - [ ] Discord/Slack community setup

**Impact**: Widespread adoption in Perl community

### Q3 2025: v0.9.0 - Full LSP 3.18 Compliance
**Goal**: 100% LSP 3.18 feature compliance

#### LSP 3.18 Complete Implementation
- [ ] **Document Synchronization**
  - [ ] Notebook Document Synchronization
  - [ ] Incremental text document sync optimization
  - [ ] Pull-based document sync
- [ ] **Advanced Language Features**
  - [x] Type Definition provider (v0.8.6 preview)
  - [x] Implementation provider (v0.8.6 preview)
  - [ ] Inline Completions provider
  - [ ] Implementation provider enhancements (multi-file)
  - [ ] Declaration provider improvements
- [ ] **Debugging Support**
  - [ ] Inline Values provider
  - [ ] Evaluate request handler
  - [ ] Debug adapter protocol integration
- [ ] **Cross-Project Features**
  - [ ] Moniker provider for cross-repo navigation
  - [ ] Linked Editing Ranges
  - [ ] Workspace Diagnostics pull model
- [ ] **Enhanced Semantic Tokens**
  - [ ] All standard token types
  - [ ] All standard token modifiers
  - [ ] Delta semantic tokens
  - [ ] Range semantic tokens

**Impact**: AI-augmented Perl development

### Q4 2025: v0.10.0 - Perl 7 & AI Integration
**Goal**: Future-proof for next-generation Perl with AI capabilities

#### Modern Perl Support
- [ ] **Perl 7 Readiness**
  - [ ] New syntax constructs
  - [ ] Strict by default handling
  - [ ] Modern::Perl compatibility
  - [ ] Feature bundle support
- [ ] **Advanced Type System**
  - [ ] Optional type annotations
  - [ ] Generic types support
  - [ ] Type checking at edit-time
  - [ ] Auto-generate type stubs
- [ ] **Async/Await Support**
  - [ ] Coroutine syntax
  - [ ] Promise/Future integration
  - [ ] Async debugging support

#### Cross-Language Features
- [ ] **Polyglot Support**
  - [ ] Inline C/C++ parsing
  - [ ] XS file support
  - [ ] SQL in strings
  - [ ] HTML/XML templates
- [ ] **WASM Compilation**
  - [ ] Compile parser to WASM
  - [ ] Browser-based IDE support
  - [ ] Cloud IDE integration

**Impact**: Future-proof Perl tooling

### 2026: v1.0.0 - Industry Standard
**Goal**: The definitive Perl development platform

#### Enterprise Excellence
- [ ] **Certification & Compliance**
  - [ ] ISO/IEC 25010 quality certification
  - [ ] SBOM generation for supply chain
  - [ ] SOC 2 compliance features
  - [ ] GDPR-compliant telemetry
- [ ] **Enterprise Integration**
  - [ ] Active Directory/LDAP support
  - [ ] Corporate proxy handling
  - [ ] Air-gapped installation support
  - [ ] Custom telemetry endpoints

#### Platform Features
- [ ] **Cloud-Native Support**
  - [ ] Kubernetes operator
  - [ ] Docker official image
  - [ ] Cloud IDE plugins (Gitpod, CodeSpaces)
  - [ ] Remote development protocol
- [ ] **CI/CD Integration**
  - [ ] GitHub Actions marketplace
  - [ ] GitLab CI templates
  - [ ] Jenkins plugin
  - [ ] Azure DevOps extension

**Impact**: Industry-standard enterprise solution

---

## 🎯 Long-term Vision (2026+)

### Advanced Capabilities
- **Symbolic Execution** - Deep program analysis
- **Polyglot Parsing** - Embedded SQL, HTML, JS
- **Visual Debugging** - AST visualization
- **AI Pair Programming** - Context-aware coding

### Ecosystem Leadership
- **CPAN Integration** - Parse all CPAN modules
- **Documentation Generation** - Auto-generate POD
- **Test Generation** - Create tests from code
- **Cross-language Bridge** - Perl ↔ Other languages

---

## 📊 Success Metrics

### Current Achievements (January 2026)
- ✅ **Quality Excellence** (verified by `just ci-gate`)
  - 337 library tests passing (100% pass rate)
  - 4/4 LSP semantic definition tests passing
  - BUG=0, MANUAL=1 ignored tests (run `bash scripts/ignored-test-count.sh`)
  - Zero clippy warnings, consistent formatting
- ✅ **Technical Milestones**
  - Thread-safe architecture
  - Semantic Analyzer Phase 1 complete (12/12 handlers)
  - Statement tracker + heredoc support 100% implemented
  - Local-first development workflow (`nix develop -c just ci-gate`)
- ⚠️ **Performance** (UNVERIFIED - awaiting published benchmarks)
  - Benchmark framework exists but results not yet published
  - See `benchmarks/BENCHMARK_FRAMEWORK.md` for methodology

### 2027 Target Metrics
- **Adoption Goals**
  - 1K+ VSCode extension installs
  - 0.1K+ crates.io downloads
  - 50+ GitHub stars
  - 5+ contributors
  - 1+ enterprise deployments
- **Performance Targets**
  - <10ms incremental parsing
  - <1000ms for 100K LOC files
  - <5000MB memory for 1M LOC
- **Community Growth**
  - 1+ editor integrations
  - 10+ community plugins
  - 1+ Discord members

---

## Benchmarks

**Framework exists; results are not yet published as canonical numbers.**

Until benchmark outputs are committed under `benchmarks/results/`, we do not state performance claims in this roadmap.

To publish:
1. Run benchmark harness: `cargo bench -p perl-parser`
2. Commit `benchmarks/results/<date>-<machine>.json`
3. Update `benchmarks/BENCHMARK_FRAMEWORK.md` with machine + command line

*LSP 3.18 compliance percentage based on implemented and fully functional LSP protocol features. The server advertises only capabilities that are production-ready. See [LSP_FEATURES.md](LSP_FEATURES.md) for detailed feature matrix.*

---

## 🛠 How to Get Started

### For Users
```bash
# Build from source (recommended right now)
cargo build -p perl-parser --bin perl-lsp --release

# Or install locally from this workspace
cargo install --path crates/perl-parser --bin perl-lsp

# (If/when v0.8.2 is published from the repo)
# cargo install --git https://github.com/EffortlessSteven/tree-sitter-perl --tag v0.8.2 perl-parser --bin perl-lsp

# Use in your editor
./target/release/perl-lsp --stdio
```

**Run Perl::Critic via LSP**
```json
// LSP command (editor invokes this)
workspace/executeCommand: {
  "command": "perl.runCritic",
  "arguments": ["file:///path/to/file.pl"]
}
```

### For Contributors
1. **Pick an area**: LSP features, performance, docs
2. **Check issues**: Look for "good first issue"
3. **Join Discord**: Get help from community
4. **Submit PR**: We review quickly

### Priority Areas
1. **VSCode extension** (High impact)
2. **Code formatting** (Most requested)
3. **Performance tests** (Prevent regression)
4. **Documentation** (Always needed)

---

## 📅 Release Schedule

### Released
- **v0.7.2** - January 2025 - Parser fixes, built-in signatures
- **v0.7.3** - February 2025 - Enterprise LSP, distribution ready
- **v0.7.5** - February 2025 - Release automation, CI/CD, enhanced type inference
- **v0.8.0** - February 2025 - Production-hardened position helpers
- **v0.8.2** - February 2025 - Document links, selection ranges, on-type formatting
- **v0.8.3** - August 2025 - Perl::Critic integration, UTF-16 fallbacks, workspace scanning
- **v0.8.5** - August 2025 - GA: LSP 3.18 partial compliance, pull diagnostics, type hierarchy

### Upcoming
- **v0.9.0** - Q1 2026 - Full LSP 3.18 compliance (100%), semantic analyzer Phase 2/3
- **v0.10.0** - Q2 2026 - Perl 7 support, AI integration
- **v1.0.0** - Q4 2026 - Industry standard platform

---

## 🔗 Resources

- **[Architecture Overview](ARCHITECTURE_OVERVIEW.md)** - Technical details and system design
- **[Crate Architecture Guide](CRATE_ARCHITECTURE_GUIDE.md)** - Component relationships
- **[LSP Implementation Guide](LSP_IMPLEMENTATION_GUIDE.md)** - LSP server architecture
- **[LSP Features](LSP_FEATURES.md)** - Complete feature matrix
- **[Current Status](CURRENT_STATUS.md)** - Real-time project health
- **[Contributing Guide](../CONTRIBUTING.md)** - How to help

---

*The future of Perl tooling is here. Join us in building it!*

**Last Updated**: 2026-01-06
