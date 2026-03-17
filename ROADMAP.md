# Perl LSP Roadmap

> **Current Version**: v0.12.0 (Public Alpha)
> **Last Updated**: 2026-03-17
> **Status**: Active Development

---

## Executive Summary

Perl LSP is a comprehensive Perl development ecosystem built in Rust, delivering modern IDE features to Perl developers through the Language Server Protocol (LSP) and Debug Adapter Protocol (DAP). The project aims to provide a zero-dependency, high-performance alternative to existing Perl language servers while maintaining100% protocol compliance and comprehensive Perl 5 syntax coverage.

### Vision

Enable every Perl developer to have a world-class development experience with intelligent code completion, real-time diagnostics, cross-file navigation, and integrated debugging—without requiring a Perl runtime for parsing or analysis.

### Current State Highlights

| Metric | Value | Target |
|--------|-------|--------|
| LSP User-Visible Coverage | 100% (53/53) | 100% |
| Protocol Compliance | 100% (97/97) | 100% |
| Perl 5 Syntax Coverage | ~100% | 100% |
| Parser Coverage | 72.4% system corpus | 90%+ CPAN top 1000 |
| Mutation Score | 87% | 90%+ |
| Incremental Parsing | <1ms | <5ms |
| Reference Coverage | 98% | 95%+ |
| Workspace Crates | 115+ | Scalable |

### Strategic Direction

The roadmap is organized around five strategic themes that guide development from Public Alpha through General Availability and beyond:

1. **Parser Excellence** — Complete Perl 5 coverage with Perl 7 readiness
2. **LSP Server Completeness** — Full protocol implementation with optimal performance
3. **Developer Experience** — Seamless editor integration and debugging
4. **Ecosystem Integration** — CPAN connectivity and community tooling
5. **Platform Support** — Universal OS and editor coverage

---

## Version Milestones

### v0.10.0 — Initial Public Alpha (Released 2026-02-28)

The foundation release establishing core capabilities and public visibility.

**Key Deliverables**:
- 60+ PRs focused on build reliability and security hardening
- crates.io publishing readiness for all public crates
- VS Code Marketplace extension packaging
- Comprehensive documentation overhaul
- Security fixes: path traversal, argument injection, safe evaluation bypass
- Feature governance microcrates (9 dedicated crates)
- Document highlight for modern Perl syntax (try/catch, signatures, interpolation)

**Success Criteria Met**:
- All 80+ workspace crates versioned and validated
- 100% LSP advertised user-visible coverage maintained
- CI merge-gate status checks operational

---

### v0.11.0 — Release Orchestration (Released 2026-03-12)

Turnkey release pipeline and distribution infrastructure.

**Key Deliverables**:
- PR-driven release orchestration (version bump, changelog, tagging)
- Topological crates.io publishing with dependency ordering
- Release guardrails with semver validation
- Workspace release alignment across packages and extension
- Operator documentation for manual release flows

**Success Criteria Met**:
- Single, repeatable release flow from PR to distribution
- crates.io publishing automation validated
- Extension marketplace integration stable

---

### v0.12.0 — Public Alpha Epic Sprint (Target: Q2 2026)

Make the alpha credible on real-world Perl while continuing the semantic framework work users expect. This milestone is the coordinated public-alpha push across parser quality, boundedness, semantic frameworks, and release readiness.

**Goals**:
- Commit the first CPAN full-corpus baseline and use it as the ratchet floor
- Parse 90%+ of the CPAN top 1000 corpus without `ERROR` nodes
- Keep internal parser torture coverage green (`edge_cases`, parser stress, hang-risk suites)
- Grow the common corpus manifest while preserving ratchet-only-forward parser quality
- Full Moo/Moose/Class::Accessor attribute resolution
- Cross-file type inference via `use parent`/`use base`
- Improved bareword disambiguation from export lists

**Epic Sprint Tracks**:
- Corpus and ratchets: system/common ratchets stay green, and the CPAN lane is bootstrapped with a committed baseline plus known-clean manifest
- Parser robustness: edge-case recovery, parser stress, and hang-risk boundedness
- Semantic frameworks: Moo/Moose/Class::Accessor, inheritance, export lists
- Release readiness: docs, packaging, and `ci-gate` receipts stay green

**Success Criteria**:
- [ ] CPAN baseline: `.ci/cpan-corpus-baseline.json` committed and usable as the ratchet floor
- [ ] CPAN corpus: 90%+ clean parse rate across the top-1000 corpus
- [ ] Internal robustness suites: edge-case, parser stress, and hang-risk suites all pass
- [ ] Parser safety: no hang, stack-overflow, or crash regressions in corpus/torture runs
- [ ] Common corpus: strict zero-error manifest only grows
- [ ] CPAN known-clean manifest: `.ci/cpan-corpus-manifest.txt` grows from zero without regressions
- [ ] Moo attribute resolution: 100% of maintained test corpus
- [ ] Moose meta-protocol support: core attributes
- [ ] Class::Accessor handling: common accessor declarations parse and resolve
- [ ] Cross-file inheritance resolution: `use parent`/`use base`
- [ ] Export list parsing for: Exporter, Sub::Exporter

---

### v0.13.0 — Diagnostic Hardening (Target: Q3 2026)

Parity with `perl -c` without execution security risks.

**Goals**:
- Strict mode and warnings pragma emulation
- Uninitialized variable detection across function boundaries
- Dead code elimination recommendations
- Syntax deprecation warnings (smartmatch, indirect notation)

**Success Criteria**:
- [ ] `use strict` emulation: variable declaration checking
- [ ] `use warnings` emulation: common warning categories
- [ ] Dead code detection: unreachable code, unused variables
- [ ] Deprecation warnings: smartmatch, indirect objects, bareword filehandles

---

### v0.14.0 — Complete Refactoring Suite (Target: Q4 2026)

Safe, reliable automated code modification.

**Goals**:
- Safe rename across entire workspaces with boundary detection
- Extract Method / Extract Variable refactorings
- Inline Method / Inline Variable refactorings
- Automated translation to modern Perl 5.38+ syntax

**Success Criteria**:
- [ ] Rename safety: no cross-package collisions
- [ ] Extract refactoring: preserve semantics
- [ ] Inline refactoring: single-use targets
- [ ] Modernization: signature syntax conversion

---

### v0.15.0 — The Stability Contract (Target: Q1 2027)

Enterprise-ready stability guarantees and API lockdown.

**Goals**:
- Semantic versioning applied to all public APIs
- Contract-locked LSP features with compatibility guarantees
- Formal deprecation policy (N-2 release support minimum)
- Certified support for Linux, macOS, and Windows

**Success Criteria**:
- [ ] API stability: no breaking changes without deprecation cycle
- [ ] Protocol invariants: LSP capabilities contract-locked
- [ ] Platform certification: CI on all tier-1 platforms
- [ ] Documentation: complete API reference with stability markers

---

### v0.16.0 — Performance Hardening (Target: Q2 2027)

Optimize for production workloads and large workspaces.

**Goals**:
- P99 latency < 100ms for all LSP operations
- Memory optimization for large workspaces (1000+ files)
- Incremental parsing optimization for complex files
- Workspace indexing performance improvements

**Success Criteria**:
- [ ] P99 latency: <100ms for completion, hover, definition
- [ ] Memory footprint: <500MB for 1000-file workspaces
- [ ] Incremental parsing: <2ms for multi-line changes
- [ ] Workspace index: <5s for initial index of large projects

---

### v0.17.0 — Security Hardening (Target: Q3 2027)

Complete security audit remediation and hardening.

**Goals**:
- Complete security audit remediation
- Enhanced input validation for all external inputs
- Security documentation complete
- Dependency vulnerability scanning automation

**Success Criteria**:
- [ ] Security audit: all findings remediated
- [ ] Input validation: 100% coverage of external inputs
- [ ] Security documentation: threat model, security policy, ADRs
- [ ] Vulnerability scanning: automated CI pipeline with alerts

---

### v0.18.0 — API Stabilization (Target: Q4 2027)

Public API freeze and migration preparation.

**Goals**:
- Public API freeze for all stable interfaces
- Deprecation warnings for unstable APIs
- Migration guides published for breaking changes
- API stability markers in documentation

**Success Criteria**:
- [ ] API freeze: no breaking changes to stable APIs
- [ ] Deprecation warnings: all unstable APIs clearly marked
- [ ] Migration guides: comprehensive guides for v0.x → v1.0
- [ ] Documentation: stability markers on all public items

---

### v0.19.0 — Documentation Complete (Target: Q1 2028)

Comprehensive documentation for all user personas.

**Goals**:
- All ADRs documented (30+ total)
- User guides complete for all features
- API documentation 100% coverage
- Video tutorials for common workflows

**Success Criteria**:
- [ ] ADRs: 30+ architecture decisions documented
- [ ] User guides: complete guides for beginners, advanced, enterprise
- [ ] API docs: 100% public API documented with examples
- [ ] Tutorials: video series for onboarding

---

### v0.20.0 — Release Candidate (Target: Q2 2028)

Final stabilization before general availability.

**Goals**:
- Feature freeze for v1.0.0
- Final bug fixes and polish
- Performance validation at scale
- Release candidate testing with early adopters

**Success Criteria**:
- [ ] Feature freeze: no new features, bug fixes only
- [ ] Bug count: zero P0/P1 issues
- [ ] Performance: all SLOs validated at scale
- [ ] Early adopter feedback: incorporated into final release

---

### v1.0.0 — General Availability (Target: Q3 2028)

Production-ready release for enterprise adoption.

**Goals**:
- Stable API guarantee with long-term support commitment
- Enterprise readiness certification
- Complete DAP native implementation
- Full CPAN integration

**Success Criteria**:
- [ ] Zero known P0/P1 issues
- [ ] 100% LSP 3.18 compliance
- [ ] Native DAP replacing bridge entirely
- [ ] CPAN metadata integration for dependencies
- [ ] LTS commitment: 2 years of support guaranteed

---

### v1.1.0+ — Post-GA Evolution (Target: Q4 2028+)

Continuous improvement and ecosystem expansion.

**Planned Enhancements**:
- Perl 7 syntax preview support
- Multi-root workspace support
- Language server clustering for monorepos
- Integrated performance profiling
- AI-assisted code generation integration

---

## Theme-Based Roadmap

### 1. Parser Excellence

Complete Perl 5 syntax coverage with forward compatibility for Perl 7.

#### Current Capabilities
- ~100% Perl 5 syntax coverage via recursive descent parser (v3)
- Heredoc support with93% edge case coverage
- Regex, quote, and substitution operator parsing
- Phase-aware BEGIN/END block parsing
- Error recovery with clear diagnostics

#### Roadmap

| Version | Milestone | Description |
|---------|-----------|-------------|
| v0.12.0 | Attribute Parsing | Full Moo/Moose attribute meta-protocol |
| v0.13.0 | Syntax Validation | `perl -c` parity without execution |
| v0.14.0 | Modern Syntax | Perl 5.38+ signature and feature support |
| v1.0.0 | Perl 7 Preview | Experimental Perl 7 syntax mode |
| v1.1.0+ | Raku Bridge | Explore Raku syntax compatibility layer |

#### Technical Targets
- Parsing performance: <200µs for typical files (2.5KB)
- Incremental updates: <1ms for single-line changes
- Error recovery: continue parsing after errors
- Memory efficiency: Arc<str> zero-copy string storage

---

### 2. LSP Server Completeness

Full LSP 3.18 protocol implementation with optimal performance.

#### Current Capabilities
- 100% user-visible feature coverage (53/53 advertised)
- 100% protocol compliance (97/97 features)
- Completion: 150+ built-in functions, modules, variables
- Navigation: go-to-definition, references, workspace symbols
- Refactoring: rename, code actions, formatting
- Diagnostics: real-time error detection

#### Roadmap

| Version | Milestone | Description |
|---------|-----------|-------------|
| v0.12.0 | Semantic Tokens | Enhanced syntax highlighting |
| v0.13.0 | Inlay Hints | Type annotations and parameter names |
| v0.14.0 | Call Hierarchy | Caller/callee visualization |
| v0.15.0 | Protocol Lock | Contract-locked capabilities |
| v1.0.0 | LSP 3.18 Full | Complete 3.18 specification |

#### Performance Targets
| Operation | Target | Current |
|-----------|--------|---------|
| Completion | <50ms p95 | <50ms |
| Hover | <30ms p95 | <30ms |
| Go-to-Definition | <50ms local, <200ms workspace | Met |
| Rename | <500ms workspace | Met |
| Diagnostics | <100ms on save | Met |

---

### 3. Developer Experience

Seamless editor integration and debugging across all major platforms.

#### Current Capabilities
- VS Code extension (Marketplace: `effortlesssteven.perl-lsp`)
- Neovim integration via nvim-lspconfig
- Emacs integration via lsp-mode/eglot
- DAP debugging with breakpoints, stepping, variable inspection
- TCP socket mode for editor flexibility

#### Roadmap

| Version | Milestone | Description |
|---------|-----------|-------------|
| v0.12.0 | Enhanced Debugging | Conditional breakpoints, logpoints |
| v0.13.0 | Multi-process Debug | Fork-aware debugging |
| v0.14.0 | Native DAP Complete | Bridge replacement |
| v0.15.0 | Debug Visualization | Complex data structure rendering |
| v1.0.0 | Debug Profiles | Launch configuration templates |

#### Editor Support Matrix

| Editor | Support Level | Installation |
|--------|---------------|--------------|
| VS Code | Primary | Marketplace extension |
| Neovim | Full | nvim-lspconfig |
| Emacs | Full | lsp-mode / eglot |
| Vim | Community | vim-lsp |
| Sublime Text | Community | LSP package |
| JetBrains | Planned | Plugin development |

---

### 4. Ecosystem Integration

CPAN connectivity and community tooling integration.

#### Current Capabilities
- Module resolution from `@INC` paths
- Core module recognition (150+ built-ins)
- Dual indexing: qualified and bare names
- Cross-file reference resolution

#### Roadmap

| Version | Milestone | Description |
|---------|-----------|-------------|
| v0.12.0 | Export Lists | Exporter/Sub::Exporter parsing |
| v0.13.0 | CPAN Metadata | Dependency resolution from META.json |
| v0.14.0 | cpanfile Support | Dependency-aware completion |
| v1.0.0 | CPAN Index | Offline CPAN index for module info |
| v1.1.0+ | Dist::Zilla | Build system integration |

#### Integration Targets
- Module installation suggestions from CPAN
- Version constraint validation
- Dependency update notifications
- Security advisory integration (CPANSA)

---

### 5. Platform Support

Universal OS and editor coverage with consistent behavior.

#### Current Capabilities
- Linux: Full support (primary development platform)
- macOS: Full support with CI validation
- Windows: Full support with PowerShell installer
- Installation: curl installer, PowerShell, cargo install

#### Roadmap

| Version | Milestone | Description |
|---------|-----------|-------------|
| v0.12.0 | Package Managers | Homebrew, Scoop, Chocolatey |
| v0.13.0 | Linux Packages | deb, rpm, AUR packages |
| v0.15.0 | Platform Certification | CI on all tier-1 platforms |
| v1.0.0 | Enterprise Packages | Official repository channels |

#### Distribution Channels

| Channel | Status | Target |
|---------|--------|--------|
| crates.io | Active | Primary |
| GitHub Releases | Active | Binary distribution |
| VS Code Marketplace | Active | Extension |
| Homebrew | Planned (v0.12.0) | macOS/Linux |
| Scoop | Planned (v0.12.0) | Windows |
| Chocolatey | Planned (v0.12.0) | Windows |
| APT | Planned (v0.13.0) | Debian/Ubuntu |
| RPM | Planned (v0.13.0) | Fedora/RHEL |
| AUR | Planned (v0.13.0) | Arch Linux |

---

## Timeline

### 2026 Quarterly Milestones

```
Q2 2026 (Apr-Jun)          Q3 2026 (Jul-Sep)          Q4 2026 (Oct-Dec)
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│ v0.12.0 Alpha       │    │ v0.13.0 Beta        │    │ v0.14.0 RC          │
├─────────────────────┤    ├─────────────────────┤    ├─────────────────────┤
│ • Moo/Moose attrs   │    │ • Strict/warnings   │    │ • Refactoring suite │
│ • Cross-file types  │    │ • Dead code detect  │    │ • Modern Perl conv  │
│ • Export list parse │    │ • Deprecation warn  │    │ • Extract/Inline    │
│ • Enhanced semtok   │    │ • Package managers  │    │ • Native DAP stable │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘
```

### 2027 Quarterly Milestones

```
Q1 2027 (Jan-Mar)          Q2 2027 (Apr-Jun)          Q3 2027 (Jul-Sep)
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│ v0.15.0 Stability   │    │ v0.16.0 Perf Harden │    │ v0.17.0 Sec Harden  │
├─────────────────────┤    ├─────────────────────┤    ├─────────────────────┤
│ • API stability     │    │ • P99 <100ms        │    │ • Security audit    │
│ • Protocol lock     │    │ • Memory optimize   │    │ • Input validation  │
│ • Platform certify  │    │ • Incremental opt   │    │ • Security docs     │
│ • Deprecation pol   │    │ • WS index perf     │    │ • Vuln scanning     │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘

Q4 2027 (Oct-Dec)
┌─────────────────────┐
│ v0.18.0 API Stable  │
├─────────────────────┤
│ • API freeze        │
│ • Deprecation warn  │
│ • Migration guides  │
│ • Stability markers │
└─────────────────────┘
```

### 2028 Quarterly Milestones

```
Q1 2028 (Jan-Mar)          Q2 2028 (Apr-Jun)          Q3 2028 (Jul-Sep)
┌─────────────────────┐    ┌─────────────────────┐    ┌─────────────────────┐
│ v0.19.0 Docs Done   │    │ v0.20.0 RC          │    │ v1.0.0 GA           │
├─────────────────────┤    ├─────────────────────┤    ├─────────────────────┤
│ • 30+ ADRs          │    │ • Feature freeze    │    │ • Production ready  │
│ • User guides       │    │ • Final bug fixes   │    │ • LTS commitment    │
│ • API docs 100%     │    │ • Perf validation   │    │ • Enterprise cert   │
│ • Video tutorials   │    │ • Early adopter FB  │    │ • Full CPAN integ   │
└─────────────────────┘    └─────────────────────┘    └─────────────────────┘

Q4 2028+
┌─────────────────────┐
│ v1.1.0+ Evolution   │
├─────────────────────┤
│ • Perl 7 preview    │
│ • Multi-root WS     │
│ • AI integration    │
│ • Raku exploration  │
└─────────────────────┘
```

---

## Success Metrics

### Quality Metrics

| Metric | Current | v0.15.0 | v0.18.0 | v0.20.0 | v1.0.0 |
|--------|---------|---------|---------|---------|--------|
| LSP Coverage | 100% | 100% | 100% | 100% | 100% |
| Protocol Compliance | 100% | 100% | 100% | 100% | 100% |
| Mutation Score | 87% | 90% | 93% | 95% | 95%+ |
| Parser Coverage | ~100% | 100% | 100% | 100% | 100% |
| Documentation Coverage | High | Complete | Complete | Complete | Complete |
| Security Audit Findings | 0 P0/P1 | 0 P0/P1 | 0 P0/P1 | 0 P0/P1 | 0 P0/P1 |

### Performance Metrics

| Metric | Current | v0.16.0 | v0.18.0 | v0.20.0 | v1.0.0 |
|--------|---------|---------|---------|---------|--------|
| Incremental Parse | <1ms | <1ms | <1ms | <1ms | <1ms |
| Full Parse (2.5KB) | ~200µs | <150µs | <120µs | <100µs | <100µs |
| Completion Response (p95) | <50ms | <50ms | <40ms | <30ms | <30ms |
| Completion Response (p99) | N/A | <100ms | <80ms | <60ms | <50ms |
| Workspace Index | ~370µs | <300µs | <275µs | <250µs | <250µs |
| Memory per Document | Low | Optimized | Optimized | Optimized | Optimized |
| Memory (1000-file WS) | N/A | <500MB | <400MB | <350MB | <300MB |

### Adoption Metrics

| Metric | Current | v0.15.0 | v0.18.0 | v0.20.0 | v1.0.0 |
|--------|---------|---------|---------|---------|--------|
| VS Code Installs | Growing | 1,000+ | 3,000+ | 5,000+ | 10,000+ |
| crates.io Downloads | Active | 5,000+ | 15,000+ | 30,000+ | 50,000+ |
| GitHub Stars | Growing | 500+ | 1,000+ | 1,500+ | 2,000+ |
| Community Contributors | Active | 20+ | 30+ | 40+ | 50+ |

---

## Risk Assessment

### Technical Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Perl Syntax Complexity** | Medium | High | Comprehensive corpus testing, community feedback loops |
| **Performance Regression** | Low | High | Continuous benchmarking, performance gates in CI |
| **LSP Protocol Changes** | Low | Medium | Version pinning, capability negotiation |
| **Platform-Specific Bugs** | Medium | Medium | Multi-platform CI, community testing |
| **Memory Leaks** | Low | High | Regular profiling, address sanitizer testing |

### Project Risks

| Risk | Likelihood | Impact | Mitigation |
|------|------------|--------|------------|
| **Maintainer Burnout** | Low | High | Documentation-first approach, community onboarding |
| **Scope Creep** | Medium | Medium | Strict milestone boundaries, feature requests triage |
| **Dependency Vulnerabilities** | Medium | High | Automated dependency scanning, minimal dependencies |
| **Community Adoption** | Medium | Medium | Active outreach, editor integration, documentation |

### Mitigation Strategies

1. **Technical Debt Prevention**
   - No `unwrap()`/`expect()` in production code
   - Mandatory code review for all changes
   - Mutation testing for critical paths
   - Comprehensive test coverage requirements

2. **Quality Assurance**
   - Local CI gate required before merge
   - Multi-platform CI validation
   - Performance regression detection
   - Security scanning in CI pipeline

3. **Community Engagement**
   - Clear contribution guidelines
   - Responsive issue triage
   - Regular roadmap updates
   - Public development visibility

---

## Dependencies

### External Dependencies

#### Runtime Dependencies
| Dependency | Purpose | Risk Level |
|------------|---------|------------|
| `ropey` | Text storage and manipulation | Low (stable) |
| `tokio` | Async runtime for LSP | Low (stable) |
| `serde` | Serialization | Low (stable) |
| `lsp-types` | LSP type definitions | Low (active) |
| `toml` | Configuration parsing | Low (stable) |

#### Build Dependencies
| Dependency | Purpose | Risk Level |
|------------|---------|------------|
| `cargo` | Build system | Low (stable) |
| `rustc` | Compiler | Low (stable) |
| `nextest` | Test runner | Low (active) |

#### Optional Dependencies
| Dependency | Purpose | Risk Level |
|------------|---------|------------|
| Perl runtime | DAP bridge mode | Low (optional) |
| Perl::Tidy | Formatting backend | Low (optional) |
| Perl::Critic | Linting backend | Low (optional) |

### Internal Dependencies

The workspace follows a layered architecture with clear dependency boundaries:

```
┌─────────────────────────────────────────────────────────────┐
│                    Application Layer                         │
│  perl-lsp (server)  │  perl-dap (debugger)  │  vscode-ext   │
├─────────────────────────────────────────────────────────────┤
│                    Feature Providers                         │
│  perl-lsp-completion  │  perl-lsp-hover  │  perl-lsp-def    │
│  perl-lsp-references  │  perl-lsp-rename  │  ... (41crates) │
├─────────────────────────────────────────────────────────────┤
│                    Core Analysis                             │
│  perl-parser  │  perl-semantic-analyzer  │  perl-lexer      │
├─────────────────────────────────────────────────────────────┤
│                    Infrastructure                            │
│  perl-parser-core  │  perl-ast  │  perl-token  │  perl-error │
└─────────────────────────────────────────────────────────────┘
```

### Dependency Risk Mitigation

1. **Minimal External Dependencies**: Prefer pure Rust implementations
2. **Version Pinning**: Lock file committed, MSRV enforced
3. **Security Scanning**: Automated vulnerability detection via `cargo audit`
4. **Alternative Evaluation**: Maintain awareness of alternative crates

---

## Contributing to the Roadmap

### How to Influence Priority

1. **Open an Issue**: Tag with `roadmap` or `enhancement`
2. **Discussion**: Use GitHub Discussions for feature requests
3. **Pull Requests**: Contributions aligned with roadmap priorities welcome
4. **Sponsorship**: Funded development can accelerate specific features

### Roadmap Review Cadence

- **Monthly**: Progress review against current milestone
- **Quarterly**: Roadmap adjustment based on feedback
- **Per-Release**: Milestone completion assessment

---

## Related Documentation

### Strategic Documents

| Document | Purpose |
|----------|---------|
| [NOW_NEXT_LATER.md](NOW_NEXT_LATER.md) | Tactical execution and current priorities |
| [TECHNICAL_VISION.md](TECHNICAL_VISION.md) | Long-term technical direction (3-5 years) |
| [STRATEGIC_DOCUMENTATION.md](docs/STRATEGIC_DOCUMENTATION.md) | Index of all strategic documents |

### Architecture Decisions

| Document | Purpose |
|----------|---------|
| [docs/adr/](docs/adr/) | Architecture Decision Records |
| [ADR-0008](docs/adr/0008-microcrate-architecture.md) | Microcrate architecture (SRP) |
| [ADR-0010](docs/adr/0010-incremental-parsing-architecture.md) | Incremental parsing design |

### Reference Documentation

| Document | Purpose |
|----------|---------|
| [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) | Computed metrics and project health |
| [features.toml](features.toml) | Canonical capability definitions |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Contribution guidelines |
| [CHANGELOG.md](CHANGELOG.md) | Release history |
| [AGENTS.md](AGENTS.md) | AI assistant development guide |
| [ARCHITECTURE_OVERVIEW.md](docs/reference/ARCHITECTURE_OVERVIEW.md) | System architecture |

---

*This roadmap is a living document. Last updated: 2026-03-13*
