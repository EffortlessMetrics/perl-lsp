# Competitive Landscape: Perl LSP Tools (2026)

*Explore agent research report. Data current as of March 2026.*

---

## Summary

The Perl LSP ecosystem has three meaningful players, all in different states of activity. Zero new competitors entered the market in 2025-2026. perl-lsp is the only Rust-based alternative.

---

## Player Analysis

### PerlNavigator

- **Installs**: 53,314 (VS Code Marketplace)
- **Version**: v0.8.20
- **Status**: Actively maintained — last release February 2026
- **Stack**: TypeScript/Node.js wrapper around `perlcritic`, `perltidy`, and external tools
- **Approach**: Delegates to existing CPAN tools rather than implementing a native parser

### Perl::LanguageServer

- **Installs**: 293,154 (VS Code Marketplace — highest install count in the ecosystem)
- **Version**: Last known release December 2023
- **Status**: STALE — no releases in 27+ months as of March 2026
- **Stack**: Perl-based LSP server
- **Approach**: Full LSP server written in Perl; stagnation suggests maintenance burden

### PLS (Perl Language Server)

- **Installs**: 9,709 (VS Code Marketplace)
- **Version**: Last release August 2021
- **Status**: Server inactive — over four years without a release
- **Stack**: Perl-based
- **Approach**: Largely abandoned; install count reflects historical adoption

### coc-perl

- **Status**: Wrapper only — not a standalone LSP server
- **Approach**: Delegates to one of the above servers; no independent implementation

---

## Market Gaps

| Gap | Evidence |
|-----|----------|
| No rename support | None of the active tools advertise cross-file rename |
| No inlay hints | Not present in PerlNavigator feature list |
| No code actions | None of the tools implement code action providers |
| No native parser | All tools wrap external processes; none parse Perl natively |
| Stagnant leader | Perl::LanguageServer has 5.5x more installs than PerlNavigator but is unmaintained |

---

## perl-lsp Differentiation

- Only Rust-based Perl LSP implementation
- Native recursive-descent parser (v3) — no subprocess delegation
- Zero CPAN runtime dependencies for the LSP server itself
- Active swarm development model: 100+ agents, 90%+ CPAN corpus coverage
- Features none of the incumbents have: rename, inlay hints, code actions, DAP

---

## Competitive Opportunity

The high install count of the stale Perl::LanguageServer (293K) represents a large user base with no upgrade path from their current server. PerlNavigator is actively maintained but architecturally limited to what `perlcritic` and `perltidy` expose. The market has an open lane for a native, feature-complete alternative.

No new Perl LSP competitors emerged in 2025 or early 2026.

---

*Source: Explore agent findings, March 2026. VS Code Marketplace install counts captured at time of research.*
