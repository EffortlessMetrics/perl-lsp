# NOW / NEXT / LATER

> **Last Updated**: 2026-03-13
> **Current Version**: v0.11.0 (Release Orchestration)
> **Status**: Active Development

---

## Legend

| Status | Meaning |
|--------|---------|
| 🟢 Active | Currently in progress |
| 🟡 Planned | Ready to start, waiting for capacity |
| 🔵 Future | Strategic item for future consideration |
| ⬜ Blocked | Dependencies not yet resolved |
| ✅ Done | Completed |

---

## NOW — Current Quarter (Q1/Q2 2026)

*Immediate priorities actively being worked on*

### Release Stabilization

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| v0.11.0 post-release monitoring | 🟢 Active | Release Team | Zero P0 regressions for 2 weeks | None |
| crates.io publishing validation | 🟢 Active | Release Team | All 80+ crates published successfully | None |
| Extension marketplace stability | 🟢 Active | DX Team | VS Code extension installs without errors | None |

### v0.12.0 — Advanced Semantic Engine

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Moo attribute resolution | 🟢 Active | Parser Team | 100% of test corpus passes | None |
| Moose meta-protocol support | 🟡 Planned | Parser Team | Core attributes resolved | Moo completion |
| Cross-file type inference | 🟡 Planned | Semantic Team | `use parent`/`use base` resolution | None |
| Export list parsing | 🟡 Planned | Semantic Team | Exporter + Sub::Exporter support | None |

### Community & Documentation

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| GitHub issue triage workflow | 🟢 Active | Community | 24h response time on new issues | None |
| API documentation completion | 🟡 Planned | Docs Team | 100% public API documented | None |
| Getting Started guide improvements | 🟡 Planned | Docs Team | New user onboarding <15 min | None |
| Community feedback integration | 🟢 Active | All Teams | Feedback incorporated into backlog | None |

### Quality & Security

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Security audit follow-ups | 🟢 Active | Security | All findings addressed | None |
| Mutation score improvement | 🟡 Planned | QA Team | 87% → 90% mutation score (target ratcheted) | None |
| CI pipeline optimization | 🟢 Active | DevOps | Build time <15 min | None |

### Architecture Decision Records (ADRs) — Critical

*High-priority ADRs identified from codebase investigations*

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Security ADRs (6 total) | 🟢 Active | Security Team | Path normalization, timeout policy, file size limits, UTF-16 fix, dependency management, unsafe code budget documented | None |
| Parser/Lexer ADRs (6 total) | 🟢 Active | Parser Team | include! macro, FIFO heredoc, hash vs block, recursion depth, AST v2, statement modifiers documented | None |
| LSP Server ADRs (10 total) | 🟡 Planned | LSP Team | Dual representation, lifecycle routing, deadline enforcement, and 7 others documented | None |

### Known Limitations Documentation

*Document known issues and limitations discovered during investigations*

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| DAP parser hang workaround | 🟢 Active | DAP Team | GitHub issue created, workaround documented in ADR | None |
| `use lib` limitation | 🟡 Planned | Module Team | Limitation documented with workaround guidance | None |
| Lexer mode transitions | 🟡 Planned | Parser Team | Mode transition diagram and edge cases documented | None |

---

## NEXT — Next Quarter (Q2/Q3 2026)

*Things that will be tackled after NOW items are complete*

### v0.13.0 — Diagnostic Hardening

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| `use strict` emulation | 🔵 Future | Semantic Team | Variable declaration checking | v0.12.0 complete |
| `use warnings` emulation | 🔵 Future | Semantic Team | Common warning categories | v0.12.0 complete |
| Dead code detection | 🔵 Future | Semantic Team | Unreachable code, unused variables | v0.12.0 complete |
| Deprecation warnings | 🔵 Future | Parser Team | smartmatch, indirect objects, bareword filehandles | v0.12.0 complete |

### Performance Optimization

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Incremental parse optimization | 🔵 Future | Parser Team | <1ms for single-line changes | None |
| Workspace indexing speedup | 🔵 Future | Workspace Team | <300µs workspace index | None |
| Memory efficiency audit | 🔵 Future | All Teams | Reduced per-document footprint | None |
| Completion response time | 🔵 Future | LSP Team | <30ms p95 response | None |

### Distribution Expansion

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Homebrew formula | 🔵 Future | Release Team | `brew install perl-lsp` works | v0.12.0 stable |
| Scoop manifest (Windows) | 🔵 Future | Release Team | `scoop install perl-lsp` works | v0.12.0 stable |
| Chocolatey package | 🔵 Future | Release Team | `choco install perl-lsp` works | v0.12.0 stable |
| Linux packages (deb, rpm, AUR) | 🔵 Future | Release Team | Native package managers supported | v0.13.0 stable |

### Enhanced Refactoring

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Safe rename across workspace | 🔵 Future | LSP Team | No cross-package collisions | v0.12.0 complete |
| Extract Method refactoring | 🔵 Future | LSP Team | Semantics preserved | v0.12.0 complete |
| Extract Variable refactoring | 🔵 Future | LSP Team | Single-expression extraction | v0.12.0 complete |

### Architecture Decision Records (ADRs) — Complete Coverage

*Remaining ADRs from codebase investigations*

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| DAP ADRs (5 total) | 🟡 Planned | DAP Team | Bridge vs native, timeout values, parser hang workaround, marker framing, poison-safe mutex documented | None |
| Module Resolution ADRs (3 total) | 🟡 Planned | Module Team | Module resolution architecture, legacy separator handling, static vs dynamic resolution documented | None |
| Test Infrastructure ADRs (5 total) | 🟡 Planned | QA Team | Sentinel values, mutation hardening, zero quarantine, proptest regression, BDD grid documented | None |
| CI/CD ADRs (4 total) | 🟡 Planned | DevOps | Receipt-based gates, topological sort, four-tier CI, multi-platform Docker documented | None |

### Documentation Guides

*Missing guides identified during investigations*

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Testing Guide | 🟡 Planned | QA Team | Comprehensive testing guide covering mutation testing, proptest, BDD patterns | None |
| CI/CD Architecture Guide | 🟡 Planned | DevOps | Complete CI/CD architecture documentation with receipt system explanation | None |
| Performance Targets Documentation | 🟡 Planned | Performance Team | Document all performance SLOs and measurement methodology | None |
| Quote Operator Parsing Guide | 🟡 Planned | Parser Team | Document quote operator parsing edge cases and resolution | None |

---

## LATER — Future Quarters (Q4 2026+)

*Strategic initiatives for future consideration*

### v0.14.0 — Complete Refactoring Suite

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Inline Method refactoring | 🔵 Future | LSP Team | Single-use targets only | v0.13.0 complete |
| Inline Variable refactoring | 🔵 Future | LSP Team | Safe inline with validation | v0.13.0 complete |
| Modern Perl syntax conversion | 🔵 Future | Parser Team | Signature syntax conversion | v0.13.0 complete |
| Native DAP implementation | 🔵 Future | DAP Team | Bridge replacement complete | v0.13.0 complete |

### v0.15.0 — The Stability Contract

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Semantic versioning enforcement | 🔵 Future | Release Team | No breaking changes without deprecation | v0.14.0 complete |
| Protocol capability locking | 🔵 Future | LSP Team | LSP capabilities contract-locked | v0.14.0 complete |
| Platform certification | 🔵 Future | DevOps | CI on all tier-1 platforms | v0.14.0 complete |
| Formal deprecation policy | 🔵 Future | Governance | N-2 release support minimum | v0.14.0 complete |

### v0.16.0 — Performance Hardening

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| P99 latency optimization | 🔵 Future | Performance Team | <100ms for all LSP operations | v0.15.0 complete |
| Memory optimization | 🔵 Future | Core Team | <500MB for 1000-file workspaces | v0.15.0 complete |
| Incremental parsing optimization | 🔵 Future | Parser Team | <2ms for multi-line changes | v0.15.0 complete |
| Workspace indexing speedup | 🔵 Future | Workspace Team | <5s initial index for large projects | v0.15.0 complete |

### v0.17.0 — Security Hardening

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Security audit remediation | 🔵 Future | Security Team | All findings addressed | v0.16.0 complete |
| Input validation coverage | 🔵 Future | Security Team | 100% external inputs validated | v0.16.0 complete |
| Security documentation | 🔵 Future | Docs Team | Threat model, security policy, ADRs | v0.16.0 complete |
| Vulnerability scanning automation | 🔵 Future | DevOps | Automated CI pipeline with alerts | v0.16.0 complete |

### v0.18.0 — API Stabilization

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Public API freeze | 🔵 Future | Architecture | No breaking changes to stable APIs | v0.17.0 complete |
| Deprecation warnings | 🔵 Future | Release Team | All unstable APIs clearly marked | v0.17.0 complete |
| Migration guides | 🔵 Future | Docs Team | Comprehensive v0.x → v1.0 guides | v0.17.0 complete |
| Stability markers | 🔵 Future | Docs Team | Stability markers on all public items | v0.17.0 complete |

### v0.19.0 — Documentation Complete

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| ADR completion | 🔵 Future | Architecture | 30+ architecture decisions documented | v0.18.0 complete |
| User guides | 🔵 Future | Docs Team | Complete guides for all personas | v0.18.0 complete |
| API documentation | 🔵 Future | Docs Team | 100% public API documented with examples | v0.18.0 complete |
| Video tutorials | 🔵 Future | Community | Onboarding video series | v0.18.0 complete |

### v0.20.0 — Release Candidate

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Feature freeze | 🔵 Future | Release Team | No new features, bug fixes only | v0.19.0 complete |
| Final bug fixes | 🔵 Future | All Teams | Zero P0/P1 issues | v0.19.0 complete |
| Performance validation | 🔵 Future | Performance Team | All SLOs validated at scale | v0.19.0 complete |
| Early adopter feedback | 🔵 Future | Community | Feedback incorporated | v0.19.0 complete |

### v1.0.0 — General Availability

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Zero P0/P1 issues | 🔵 Future | All Teams | Production-ready stability | v0.20.0 complete |
| LSP 3.18 full compliance | 🔵 Future | LSP Team | 100% specification coverage | v0.20.0 complete |
| CPAN metadata integration | 🔵 Future | Ecosystem Team | Dependency resolution from META.json | v0.20.0 complete |
| LTS commitment | 🔵 Future | Governance | 2 years support guaranteed | v0.20.0 complete |

### Perl 7 Preparation

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Perl 7 syntax preview mode | 🔵 Future | Parser Team | Experimental flag for new syntax | v1.0.0 complete |
| Feature flag infrastructure | 🔵 Future | Parser Team | Runtime syntax mode selection | v1.0.0 complete |
| Backward compatibility testing | 🔵 Future | QA Team | Perl 5 code works in Perl 7 mode | Perl 7 spec stable |

### Advanced Type Inference

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Type signature parsing | 🔵 Future | Parser Team | Support for Zydeco, Types::Standard | v1.0.0 complete |
| Cross-file type propagation | 🔵 Future | Semantic Team | Types flow across module boundaries | Type signature parsing |
| Type-aware completion | 🔵 Future | LSP Team | Completion filtered by expected type | Cross-file type propagation |

### Plugin Architecture

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Plugin API design | 🔵 Future | Architecture | Stable extension points defined | v1.0.0 complete |
| Dynamic loader | 🔵 Future | Core Team | Runtime plugin loading | Plugin API design |
| Third-party integration | 🔵 Future | Community | 3+ community plugins available | Dynamic loader |

### Enterprise Features

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Multi-root workspace support | 🔵 Future | Workspace Team | Monorepo support | v1.0.0 complete |
| Language server clustering | 🔵 Future | Performance Team | Horizontal scaling for large repos | Multi-root workspace |
| Integrated performance profiling | 🔵 Future | DevTools Team | Built-in profiling dashboard | v1.0.0 complete |
| AI-assisted code generation | 🔵 Future | AI Team | LSP integration for AI tools | v1.0.0 complete |

### Community Ecosystem

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| JetBrains IDE plugin | 🔵 Future | Community | IntelliJ/PyCharm support | v1.0.0 complete |
| Raku syntax exploration | 🔵 Future | Parser Team | Compatibility layer prototype | v1.1.0+ planning |
| CPAN index integration | 🔵 Future | Ecosystem Team | Offline CPAN index for module info | v1.0.0 complete |
| Dist::Zilla integration | 🔵 Future | Ecosystem Team | Build system awareness | CPAN index integration |

### Advanced Documentation

*Strategic documentation initiatives from investigation findings*

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| Threat Model Documentation | 🔵 Future | Security Team | Complete threat model with attack vectors and mitigations | Security ADRs complete |
| Performance Contracts | 🔵 Future | Performance Team | Formal performance SLOs with measurement methodology | Performance targets documented |
| ADR Index & Cross-References | 🔵 Future | Architecture | Searchable ADR index with dependency tracking | All ADRs complete |
| Decision Log Automation | 🔵 Future | DevOps | Automated ADR generation from PR templates | ADR index complete |

### Tooling Improvements

*Future tooling enhancements from investigation insights*

| Item | Status | Owner | Success Criteria | Dependencies |
|------|--------|-------|------------------|--------------|
| ADR Linting & Validation | 🔵 Future | DevOps | CI checks for ADR completeness and format | ADR templates defined |
| Documentation Coverage Metrics | 🔵 Future | Docs Team | Automated tracking of documentation coverage | None |
| Investigation-to-ADR Pipeline | 🔵 Future | Architecture | Streamlined process from investigation findings to ADRs | None |

---

## Dependency Graph

```mermaid
graph TD
    subgraph NOW
        A1[v0.11.0 Stabilization] --> A2[v0.12.0 Semantic Engine]
        A2 --> A3[Moo/Moose Attributes]
        A2 --> A4[Cross-file Types]
        A2 --> A5[Export Lists]
        A6[Critical ADRs] --> A7[Security ADRs]
        A6 --> A8[Parser ADRs]
        A9[Known Limitations] --> A10[DAP Hang Workaround]
    end

    subgraph NEXT
        A2 --> B1[v0.13.0 Diagnostic Hardening]
        B1 --> B2[Strict/Warnings Emulation]
        B1 --> B3[Dead Code Detection]
        A2 --> B4[Performance Optimization]
        A2 --> B5[Distribution Expansion]
        B6[Complete ADR Coverage] --> B7[DAP ADRs]
        B6 --> B8[Module Resolution ADRs]
        B6 --> B9[Test Infrastructure ADRs]
        B6 --> B10[CI/CD ADRs]
        B11[Documentation Guides] --> B12[Testing Guide]
        B11 --> B13[CI/CD Architecture Guide]
    end

    subgraph LATER
        B1 --> C1[v0.14.0 Refactoring Suite]
        C1 --> C2[v0.15.0 Stability Contract]
        C2 --> D1[v0.16.0 Performance Hardening]
        D1 --> D2[v0.17.0 Security Hardening]
        D2 --> D3[v0.18.0 API Stabilization]
        D3 --> D4[v0.19.0 Documentation Complete]
        D4 --> D5[v0.20.0 Release Candidate]
        D5 --> C3[v1.0.0 GA Release]
        C3 --> C4[Perl 7 Preparation]
        C3 --> C5[Plugin Architecture]
        C3 --> C6[Enterprise Features]
        C7[Advanced Documentation] --> C8[Threat Model]
        C7 --> C9[Performance Contracts]
        C7 --> C10[ADR Index]
        C11[Tooling Improvements] --> C12[ADR Linting]
        C11 --> C13[Doc Coverage Metrics]
    end
```

---

## Metrics Dashboard

### Current State (v0.11.0)

| Metric | Value | Target | Status |
|--------|-------|--------|--------|
| LSP User-Visible Coverage | 100% (53/53) | 100% | ✅ |
| Protocol Compliance | 100% (97/97) | 100% | ✅ |
| Perl 5 Syntax Coverage | ~100% | 100% | ✅ |
| Mutation Score | 87% | 90%+ | 🟡 In Progress |
| Incremental Parsing | <1ms | <5ms | ✅ |
| Reference Coverage | 98% | 95%+ | ✅ |
| Workspace Crates | 115+ | Scalable | ✅ |
| Documented ADRs | 0/39 | 39 | 🟡 In Progress |

### Target State (v1.0.0)

| Metric | Current | v0.15.0 | v0.18.0 | v0.20.0 | v1.0.0 |
|--------|---------|---------|---------|---------|--------|
| Mutation Score | 87% | 90% | 93% | 95% | 95%+ |
| Full Parse Time | ~200µs | <150µs | <120µs | <100µs | <100µs |
| Completion Response (p95) | <50ms | <50ms | <40ms | <30ms | <30ms |
| Completion Response (p99) | N/A | <100ms | <80ms | <60ms | <50ms |
| Workspace Index | ~370µs | <300µs | <275µs | <250µs | <250µs |
| ADR Coverage | 0% | 50% | 80% | 95% | 100% |
| Security Audit Findings | 0 P0/P1 | 0 P0/P1 | 0 P0/P1 | 0 P0/P1 | 0 P0/P1 |

---

## How to Use This Document

1. **NOW items** are actively being worked on. Check related issues and PRs for current status.
2. **NEXT items** are prioritized for the upcoming quarter. Begin planning and design.
3. **LATER items** are strategic. Research and prototype as capacity allows.
4. **Update frequency**: This document should be updated at the start of each quarter and when major milestones are reached.

---

## Related Documents

### Strategic Planning

- [ROADMAP.md](ROADMAP.md) — Full project roadmap with detailed milestones
- [TECHNICAL_VISION.md](TECHNICAL_VISION.md) — Long-term technical direction and principles behind priorities
- [STRATEGIC_DOCUMENTATION.md](docs/STRATEGIC_DOCUMENTATION.md) — Index of all strategic documents

### Project Status

- [CURRENT_STATUS.md](docs/project/CURRENT_STATUS.md) — Current metrics and project health
- [features.toml](features.toml) — Canonical capability definitions

### Contributing

- [CHANGELOG.md](CHANGELOG.md) — Version history and release notes
- [AGENTS.md](AGENTS.md) — Development guidelines and architecture
- [CONTRIBUTING.md](CONTRIBUTING.md) — How to contribute to the project
