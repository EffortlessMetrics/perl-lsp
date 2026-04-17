# Project Orientation

> For the documentation hub, see [README.md](../README.md). This page provides project orientation for active contributors.

> **SNAPSHOT DISCLAIMER**: Orientation-only. For live status and metrics, see `docs/project/CURRENT_STATUS.md` and GitHub milestones/issues.

Welcome to the perl-lsp project! This guide will get you up to speed quickly.

## 📍 You Are Here

**Project Status**: v0.12.4 current; v0.12.x stabilization underway
**Open Issues**: See GitHub milestones/issues for live counts

## 🎯 5-Minute Orientation

### What Is This Project?

perl-lsp is a comprehensive Perl parsing + LSP/DAP ecosystem:
- Fast native Rust parser with near-complete Perl 5 coverage
- LSP server with broad feature support (tracked in `features.toml`)
- DAP support with native preview adapter + BridgeAdapter compatibility path
- Quality gates: tests, fuzzing/mutation hardening, missing_docs enforcement (see `CURRENT_STATUS.md`)

### Current Focus

**Now (post v0.12.x close-out)**
- Keep close-out receipts green (`just ci-gate`, targeted state-machine tests, benchmark checks)
- Publish benchmark outputs under `benchmarks/results/`

**Next (v0.13.x)**
- Stability statement + packaging stance
- Benchmark publication with receipts
- Upgrade notes from v0.8.x → v0.9.x

**Later (post v0.9.x)**
- DAP preview hardening (runtime variable/evaluate depth + packaging)
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
6. **[LSP Missing Features](../reference/LSP_MISSING_FEATURES_REPORT.md)** - Non-advertised capabilities (derived from `features.toml`)

### Development
5. **[CLAUDE.md](../CLAUDE.md)** - Project guidance for AI assistants
6. **[CONTRIBUTING.md](../CONTRIBUTING.md)** - How to contribute
7. **[COMMANDS_REFERENCE.md](../reference/COMMANDS_REFERENCE.md)** - Build/test commands

## 🚨 What Needs Attention RIGHT NOW

### Now (as of 2026-02-16)
1. 🟡 **Benchmark publication** - commit canonical benchmark outputs under `benchmarks/results/`
2. 🟡 **v0.9.x packaging stance** - finalize supported platforms and shipping model
3. 🟡 **Upgrade notes polish** - ensure v0.8.x → v0.9.x path is explicit
4. 📌 **Expanded backlog** - see `docs/TODO.md` + `docs/reference/LSP_MISSING_FEATURES_REPORT.md`

### Next
1. **v0.9.x readiness** - stability statement, packaging stance, benchmark receipts, upgrade notes
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
cargo run -p perl-lsp-rs -- --stdio

# Check for issues
cargo clippy --workspace

# Format code
cargo fmt --all

# Build docs
cargo doc --no-deps --package perl-parser

# Run specific tests
cargo test -p perl-parser               # Parser tests
cargo test -p perl-lsp-rs                  # LSP tests
RUST_TEST_THREADS=2 cargo test -p perl-lsp-rs  # With adaptive threading
```

## 💡 Where to Start Contributing

- Check the active milestone and the `good first issue` / `help wanted` labels
- Near-term work: benchmark publication + v0.9.x packaging/readiness (see ROADMAP)
- Larger efforts: v0.9.x milestone and `phase:*` labels
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
2. Read [ARCHITECTURE_OVERVIEW.md](../reference/ARCHITECTURE_OVERVIEW.md)
3. Read [LSP_IMPLEMENTATION_GUIDE.md](../reference/LSP_IMPLEMENTATION_GUIDE.md)
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
- **LSP features**: [LSP_IMPLEMENTATION_GUIDE.md](../reference/LSP_IMPLEMENTATION_GUIDE.md)
- **Testing**: [COMPREHENSIVE_TESTING_GUIDE.md](../tutorials/COMPREHENSIVE_TESTING_GUIDE.md)

### Communication
- **GitHub Issues**: For bugs, features, questions
- **Pull Requests**: For code contributions
- **Issue Comments**: For collaboration and clarification

## 🎯 Success Criteria

See [ROADMAP.md](ROADMAP.md) for current exit criteria and release gates.

## 📈 Project Health Indicators

See [CURRENT_STATUS.md](CURRENT_STATUS.md) for computed health signals and receipts.

## 🚀 Let's Build Together!

The perl-lsp project has clear paths forward. Your contributions will help make Perl development smoother across editors.

**Pick an issue, dive in, and let's ship this! 🎉**

---

*This guide is kept up-to-date as the project evolves. Last updated: 2026-02-17*

*For detailed status, see: [CURRENT_STATUS.md](CURRENT_STATUS.md)*
