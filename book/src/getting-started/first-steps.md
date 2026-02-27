# Project Orientation

> For the documentation hub, see [README.md](README.md). This page provides project orientation for active contributors.

> **SNAPSHOT DISCLAIMER**: Orientation-only. For live status and metrics, see `docs/CURRENT_STATUS.md` and GitHub milestones/issues.

Welcome to the perl-lsp project! This guide will get you up to speed quickly.

## 📍 You Are Here

**Project Status**: v0.9.1 close-out receipts captured; v0.9.x hardening underway
**Open Issues**: See GitHub milestones/issues for live counts

## 🎯 5-Minute Orientation

### What Is This Project?

perl-lsp is a comprehensive Perl parsing + LSP/DAP ecosystem:
- Fast native Rust parser with near-complete Perl 5 coverage
- LSP server with broad feature support (tracked in `features.toml`)
- DAP support with native preview adapter + BridgeAdapter compatibility path
- Quality gates: tests, fuzzing/mutation hardening, missing_docs enforcement (see `CURRENT_STATUS.md`)

### Current Focus

**Now (post v0.9.1 close-out)**
- Keep close-out receipts green (`just ci-gate`, targeted state-machine tests, benchmark checks)
- Publish benchmark outputs under `benchmarks/results/`

**Next (v0.10.0)**
- Moo/Moose semantic depth improvements
- Native DAP enhancements (variables/evaluate)
- Stability goal refinement for v0.15.0 contract

**Later (targeting v0.15.0 for Stability Contract)**
- Formal API stability and locked wire protocol
- Full LSP 3.18 compliance
- Package manager distribution

See [ROADMAP.md](ROADMAP.md) for milestones and exit criteria.

## 📚 Essential Documents (Read These First)

### Status & Planning
1. **[Current Status](CURRENT_STATUS.md)** ⭐ **START HERE** - Computed metrics + receipts
2. **[Roadmap](ROADMAP.md)** - Plans, exit criteria, and deferrals
3. **[Milestones](MILESTONES.md)** - GitHub milestone mapping
4. **[Docs Index](INDEX.md)** - Routes to the right doc fast
5. **[TODO Backlog](TODO.md)** - Actionable tasks + missing features
6. **[LSP Missing Features](LSP_MISSING_FEATURES_REPORT.md)** - Non-advertised capabilities (derived from `features.toml`)

### Development
5. **[CLAUDE.md](../CLAUDE.md)** - Project guidance for AI assistants
6. **[CONTRIBUTING.md](../CONTRIBUTING.md)** - How to contribute
7. **[COMMANDS_REFERENCE.md](COMMANDS_REFERENCE.md)** - Build/test commands

## 🚨 What Needs Attention RIGHT NOW

### Now (as of 2026-02-16)
1. 🟡 **Benchmark publication** - commit canonical benchmark outputs under `benchmarks/results/`
2. 🟡 **v0.9.x packaging stance** - finalize supported platforms and shipping model
3. 🟡 **Upgrade notes polish** - ensure v0.8.x → v0.9.x path is explicit
4. 📌 **Expanded backlog** - see `docs/TODO.md` + `docs/LSP_MISSING_FEATURES_REPORT.md`

### Next
1. **v0.15.0 readiness** - stability contract, packaging stance, benchmark receipts
2. **Merge gates** - #210 after CI pipeline cleanup (#211)

### Critical Blockers / Constraints
- **Issue #211**: CI Pipeline cleanup blocks merge gates (#210)

## 🏗️ Project Structure

```
perl-lsp/
├── crates/
│   ├── perl-parser/      ⭐ Main crate - Parser
│   ├── perl-lsp/          LSP server binary + LSP logic
│   ├── perl-dap/          Debug Adapter Protocol (native preview + bridge fallback)
│   ├── perl-lexer/        Context-aware tokenizer
│   ├── perl-corpus/       Test corpus (see CURRENT_STATUS for counts)
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

- Check the active milestone and the `good first issue` / `help wanted` labels
- Near-term work: benchmark publication + v0.9.x packaging/readiness (see ROADMAP)
- Larger efforts: see ROADMAP.md and `phase:*` labels
- See [CONTRIBUTING.md](../CONTRIBUTING.md) for workflow details

## 📊 Quality Metrics

All metrics are computed and published in [CURRENT_STATUS.md](CURRENT_STATUS.md).
Run `just status-check` for live numbers.

## 🔍 Understanding the Codebase

### Parser Architecture
- **v3 Native Parser** ⭐ RECOMMENDED: near-complete Perl 5 coverage with strong performance (see CURRENT_STATUS)
- **v2 Pest Parser**: Legacy but stable; maintained for compatibility
- **Incremental Parsing**: Sub-millisecond updates with high node reuse (see CURRENT_STATUS)

### LSP Components
- **Providers**: completion, hover, diagnostics, references, etc.
- **Workspace Index**: Dual indexing for qualified + bare symbol forms
- **Threading**: Adaptive threading to stabilize CI environments
- **Cancellation**: Enhanced system (PR #165)

### Key Innovations
- **Dual Indexing** (PR #122): Functions indexed as both `Package::function` and `function`
- **Adaptive Threading** (PR #140): Thread-aware timeout scaling for CI
- **API Documentation** (PR #160/SPEC-149): `#![warn(missing_docs)]` enforcement
- **Mutation Testing** (PR #153): Comprehensive mutation hardening suite

## 🎓 Learning Path

### Day 1: Orientation
1. Read this document
2. Read [CURRENT_STATUS.md](CURRENT_STATUS.md)
3. Read [ROADMAP.md](ROADMAP.md)
4. Clone repo and run tests

### Day 2: Deep Dive
1. Read [CLAUDE.md](../CLAUDE.md)
2. Read [ARCHITECTURE_OVERVIEW.md](ARCHITECTURE_OVERVIEW.md)
3. Read [LSP_IMPLEMENTATION_GUIDE.md](LSP_IMPLEMENTATION_GUIDE.md)
4. Explore codebase structure + docs index

### Day 3: First Contribution
1. Pick an issue from the active milestone or `good first issue`
2. Read the issue’s research comment (if present)
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

See [ROADMAP.md](ROADMAP.md) for current exit criteria and release gates.

## 📈 Project Health Indicators

See [CURRENT_STATUS.md](CURRENT_STATUS.md) for computed health signals and receipts.

## 🚀 Let's Build Together!

The perl-lsp project is in active development with a clear path toward a stable v0.15.0 release. Your contributions will help make Perl development smoother across editors.

**Pick an issue, dive in, and let's ship this! 🎉**

---

*This guide is kept up-to-date as the project evolves. Last updated: 2026-02-17*

*For detailed status, see: [CURRENT_STATUS.md](CURRENT_STATUS.md)*
