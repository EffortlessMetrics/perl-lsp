# 🚀 START HERE - perl-lsp Quick Start Guide

> **⚠️ SNAPSHOT DISCLAIMER**: Status snapshot as of 2025-11-12. For live status, treat GitHub issues & milestones as canonical. This guide provides point-in-time orientation; see GitHub for current progress.

Welcome to the perl-lsp project! This guide will get you up to speed quickly.

## 📍 You Are Here

**Project Status**: 70-75% MVP complete, production-ready foundations
**Timeline**: 2-3 weeks to MVP, 11-13 weeks to Production v1.0
**Open Issues**: 27 total (3 P0-CRITICAL, recently closed 3 resolved issues)

## 🎯 5-Minute Orientation

### What Is This Project?

perl-lsp is the most comprehensive Perl parsing and LSP solution available:
- **4-19x faster** parsing than legacy tools
- **91% LSP 3.17+ coverage** (targeting 93%+)
- **~100% Perl 5 syntax coverage** with world-class performance
- **Enterprise-grade quality**: 99.6% test pass rate, 87% mutation score

### Current Focus

**Sprint A** (Parser Foundation): 35-40% complete, Days 1-10 → 2-3 weeks
- Heredoc parsing edge cases
- Statement tracking for AST placement
- Test infrastructure stabilization

**Sprint B** (LSP Polish): Blocked, Days 11-19
- Semantic analyzer (43 NodeKind handlers)
- Document highlighting
- Workspace features
- Name span implementation

## 📚 Essential Documents (Read These First)

### Status & Planning
1. **[Current Status](CURRENT_STATUS.md)** ⭐ **START HERE** - Real-time project dashboard
2. **[Issue Status Report](ISSUE_STATUS_2025-11-12.md)** - All 30 issues analyzed
3. **[MVP Roadmap (#195)](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/195)** - 2-3 week path
4. **[Production Roadmap (#196)](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/196)** - 11-13 week plan

### Development
5. **[CLAUDE.md](../CLAUDE.md)** - Project guidance for AI assistants
6. **[CONTRIBUTING.md](../CONTRIBUTING.md)** - How to contribute
7. **[ROADMAP.md](ROADMAP.md)** - Long-term vision (2025-2026+)

## 🚨 What Needs Attention RIGHT NOW

### This Week (2025-11-12)
1. ✅ **Close 3 resolved issues**: #203, #202, #194
2. 🔴 **Statement tracker architecture session** (#182) - 2 hours, URGENT
3. 🔴 **Fix CI guardrail** (#198) - Monitor perl-lsp tests
4. 🟡 **Complete #183** - Heredoc declaration parser (Days 1-3)
5. 🟡 **Merge PR #214** - CI infrastructure hardening

### Critical Blockers
- **Issue #182**: Statement Tracker - Blocks 17 test re-enablement
- **Issue #211**: CI Pipeline - $720/year savings, blocks #210
- **Issue #210**: Merge Gates - 8-week implementation, needs #211 first

## 🏗️ Project Structure

```
perl-lsp/
├── crates/
│   ├── perl-parser/      ⭐ Main crate - Parser + LSP logic
│   ├── perl-lsp/          LSP server binary
│   ├── perl-dap/          Debug Adapter Protocol (Phase 1 complete)
│   ├── perl-lexer/        Context-aware tokenizer
│   ├── perl-corpus/       Test corpus (141 edge cases)
│   └── perl-parser-pest/  Legacy Pest parser
├── docs/                  📚 Comprehensive documentation
│   ├── CURRENT_STATUS.md  ⭐ Read this first!
│   ├── ISSUE_STATUS_*.md  Issue tracking
│   └── *.md               Technical guides
├── CLAUDE.md              Project guidance
└── CONTRIBUTING.md        How to help
```

## 🎬 Quick Commands

```bash
# Build everything
cargo build --workspace

# Run tests
cargo test

# Run LSP server
cargo run -p perl-lsp -- --stdio

# Check for issues
cargo clippy --workspace

# Format code
cargo fmt --all

# Build docs
cargo doc --no-deps --package perl-parser

# Run specific tests
cargo test -p perl-parser               # Parser tests
cargo test -p perl-lsp                  # LSP tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp  # With adaptive threading
```

## 💡 Where to Start Contributing

### Easy Wins (2-4 hours)
- **Issue #180**: Name spans (3 TODO items, 2-3 hours)
- **Issue #181**: LSP features (3 TODO items, 2-3 hours)
- **Issue #191**: Document highlighting (17-25 handlers, 2-3 hours)
- **Issue #200**: Flaky timeout fix (1 day)
- **Issue #201**: Mutation tests (1 day)

### Medium Tasks (1-2 weeks)
- **Issue #197**: Documentation (Phase 1, 484 → 350 violations)
- **Issue #208**: Batteries Included (4 phases, 3.5 weeks)
- **Issue #187**: Symbol extraction optimization (3 phases)

### Large Projects (3-8 weeks)
- **Issue #188**: Semantic analyzer (43 handlers, cornerstone of Sprint B)
- **Issue #179**: Refactoring features (8 TODO items, 3-4 weeks)
- **Issue #211**: CI pipeline cleanup (3 weeks, $720/year savings)
- **Issue #210**: Merge-blocking gates (8 weeks)

## 📊 Quality Metrics

| Metric | Current | Target | Status |
|--------|---------|--------|--------|
| Test Pass Rate | 99.6% | 100% | 🟢 Excellent |
| LSP Coverage | 91% | 93%+ | 🟡 Sprint B |
| Mutation Score | 87% | 87%+ | 🟢 Met |
| Ignored Tests | 779 | <100 | 🔴 Issue #198 |
| Doc Violations | 484 | 0 | 🟡 8-week plan |
| CI Automation | 40% | 100% | 🔴 Issue #211 |

## 🔍 Understanding the Codebase

### Parser Architecture
- **v3 Native Parser** ⭐ RECOMMENDED: ~100% coverage, 4-19x faster
- **v2 Pest Parser**: Legacy but stable, 99.996% coverage
- **Incremental Parsing**: <1ms updates (actual: 931ns!)

### LSP Components
- **Providers**: completion, hover, diagnostics, references, etc.
- **Workspace Index**: Dual indexing for 98% reference coverage
- **Threading**: Adaptive threading (PR #140, 5000x improvements)
- **Cancellation**: Enhanced system (PR #165)

### Key Innovations
- **Dual Indexing** (PR #122): Functions indexed as both `Package::function` and `function`
- **Adaptive Threading** (PR #140): Thread-aware timeout scaling for CI
- **API Documentation** (PR #160/SPEC-149): `#![warn(missing_docs)]` enforcement
- **Mutation Testing** (PR #153): 87% score with comprehensive hardening

## 🎓 Learning Path

### Day 1: Orientation
1. Read this document
2. Read [CURRENT_STATUS.md](CURRENT_STATUS.md)
3. Skim [Issue Status Report](ISSUE_STATUS_2025-11-12.md)
4. Clone repo and run tests

### Day 2: Deep Dive
1. Read [CLAUDE.md](../CLAUDE.md)
2. Study [Sprint A Meta (#212)](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/212)
3. Study [Sprint B Meta (#213)](https://github.com/EffortlessMetrics/tree-sitter-perl-rs/issues/213)
4. Explore codebase structure

### Day 3: First Contribution
1. Pick an issue from "Easy Wins" above
2. Read the comprehensive research comment on that issue
3. Ask questions in issue comments
4. Submit your first PR!

## 🤝 Getting Help

### Documentation
- **Technical questions**: Check [docs/](.) directory
- **Issue-specific**: Read the research comment on the issue
- **LSP features**: [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)
- **Testing**: [COMPREHENSIVE_TESTING_GUIDE.md](COMPREHENSIVE_TESTING_GUIDE.md)

### Communication
- **GitHub Issues**: For bugs, features, questions
- **Pull Requests**: For code contributions
- **Issue Comments**: For collaboration and clarification

## 🎯 Success Criteria

### MVP (2-3 Weeks)
- [ ] Sprint A complete (parser foundation)
- [ ] Sprint B complete (LSP polish)
- [ ] 93%+ LSP coverage
- [ ] <100 ignored tests
- [ ] Zero critical bugs

### Production v1.0 (11-13 Weeks)
- [ ] All P0/P1 issues resolved
- [ ] Merge-blocking gates operational
- [ ] Batteries included UX
- [ ] Core documentation complete
- [ ] Test infrastructure stable
- [ ] CI cost optimized

## 📈 Project Health Indicators

**🟢 GREEN** (Excellent):
- Parser coverage, performance, test pass rate, mutation score
- LSP foundation, response times, feature breadth
- Security (UTF-16 fixes, path validation)
- Documentation infrastructure (PR #160)

**🟡 YELLOW** (In Progress):
- Sprint A completion (35-40%)
- LSP coverage (91% → 93%+)
- Documentation content (484 violations)

**🔴 RED** (Needs Attention):
- 779 ignored tests (87% BrokenPipe errors)
- CI/CD automation (40%)
- Statement tracker architecture (undefined)

## 🚀 Let's Build Together!

The perl-lsp project is in excellent shape with clear paths to both MVP and Production v1.0. Your contributions will help make Perl development as smooth as TypeScript/Rust!

**Pick an issue, dive in, and let's ship this! 🎉**

---

*This guide is kept up-to-date as the project evolves. Last updated: 2025-11-12*

*For detailed status, see: [CURRENT_STATUS.md](CURRENT_STATUS.md)*
