# Competitive Analysis: Perl Language Servers in 2026

Perl is one of the few major programming languages where the majority of developers use no IDE tooling at all. The 2025 Perl IDE Survey (602 respondents) found that 78% of Perl developers do not use any language server integration. This is not for lack of trying — it reflects genuine limitations in the existing tools and the exceptional difficulty of parsing Perl statically.

This article examines each existing Perl language server honestly — their strengths, their fundamental limitations, and where perl-lsp fits in the ecosystem.

---

## The Landscape

Four tools dominate the Perl LSP space:

1. **PerlNavigator** (~53,000 VSCode installs as of early 2026) — the most-installed Perl VS Code extension
2. **Perl::LanguageServer** (~293,000 VSCode installs as of early 2026) — the highest install count of any Perl extension
3. **PLS** — a clean PPI-based Perl server with ~117 GitHub stars
4. **coc-perl** — a thin integration layer for Neovim users

Combined, these tools reach roughly 350,000 installs. But install counts include significant overlap (developers who tried multiple tools) and many installs represent inactive users who installed, found limitations, and stopped using them. The 78% figure is the honest accounting.

---

## PerlNavigator

**Author**: Brian Scannell (bscan)
**Architecture**: TypeScript LSP server that calls out to the Perl runtime
**VSCode installs**: ~53,000 (VSCode Marketplace, early 2026 — point-in-time, subject to change)

PerlNavigator's distinguishing characteristic is that it delegates syntax checking to `perl -c`. This means it leverages Perl itself for validation — when PerlNavigator says your code has a syntax error, it is running the same check Perl would run. This is both its strength and its central limitation.

**What it does well**: PerlNavigator is actively maintained and has the best `perlcritic` and `perltidy` integration of any Perl language server. Its navigation features work well for conventional Perl code. For developers with a working Perl installation and all relevant modules available, it provides a solid experience.

**Where it falls short**: The `perl -c` strategy requires executing `BEGIN` blocks. This is a security concern when analyzing untrusted code — third-party PRs, unfamiliar modules, codebases from a new client. It also means PerlNavigator cannot work on code whose dependencies are not installed. Try opening a module that `use`s a module you haven't installed; PerlNavigator will fail to check it. On Windows, several features degrade due to Perl runtime differences.

Navigation in PerlNavigator is regex-based rather than AST-based. For simple subroutine definitions, this works fine. For complex constructs — method generation, mixin patterns, dynamically-created symbols — regex-based navigation misses what it cannot see textually.

---

## Perl::LanguageServer

**Author**: Gerald Richter
**Architecture**: Pure Perl LSP server using PPI
**VSCode installs**: ~293,000 (VSCode Marketplace, early 2026 — point-in-time, subject to change)

Perl::LanguageServer has the highest install count, built over years of being the only serious option for many users. It uses PPI (the Perl Parsing Interface), the most mature Perl static parser available.

**What it does well**: PPI is a genuine static parser — it produces a document model without executing code. Perl::LanguageServer provides workspace-aware symbol navigation and debugging support (via integration with `Devel::Perl5Db`). For developers already invested in the PPI ecosystem, it is a known quantity.

**Where it falls short**: PPI is pure Perl, and performance degrades on large files and large projects. More fundamentally, PPI deliberately avoids parsing certain constructs — it produces a "document" model, not a full AST. Features that require a complete AST (rename-across-files, call hierarchy, accurate type resolution) are either missing or inaccurate. The maintenance cadence has slowed, with long gaps between releases. No Windows support.

The core limitation is structural: PPI was designed to be "good enough for many purposes," not to produce the complete semantic model that modern IDE features require.

---

## PLS (Perl Language Server)

**Architecture**: Pure Perl, PPI-based
**Repository**: github.com/FractalBoy/perl-language-server

PLS is the cleanest of the Perl LSP implementations — well-structured code, modern Perl practices, PPI-based parsing. Its limitations are inherited from PPI rather than implementation quality: the same parser, the same gaps. PLS has a smaller contributor base and lower adoption than the two leaders, which means slower feature development.

---

## coc-perl

coc-perl is not an independent language server. It is an integration layer for Neovim users running `coc.nvim`, wrapping either PerlNavigator or Perl::LanguageServer. It inherits all limitations of whichever underlying server it wraps.

---

## The Common Thread

All three independent language servers share a fundamental characteristic: they require Perl to function.

PerlNavigator calls `perl -c` directly. Perl::LanguageServer is written in Perl and requires CPAN modules. PLS is the same. This requirement creates a class of problems that no amount of feature work can fully address:

- Installation is environment-dependent: the tool works if Perl is configured correctly
- Analysis is limited to code whose dependencies are present
- Security is compromised on untrusted code (any tool that runs Perl to analyze Perl can be exploited)
- Cross-platform support is complicated by Perl installation differences

The 78% adoption gap is largely explained by this friction. Developers who do not have a Perl installation on their development machine — front-end developers working on mixed codebases, container-based development environments, Windows developers — have no good option.

---

## Where perl-lsp Fits

perl-lsp occupies a different position in this landscape: zero-dependency static analysis.

**The parser does not call Perl.** It is a hand-written recursive descent parser in Rust (`crates/perl-parser-core/`) with a stateful lexer (`crates/perl-lexer/`) that handles Perl's context-sensitive grammar without executing any code. A single native binary handles parsing, analysis, and all 98 LSP and DAP features.

This has real implications:

**Installation is trivial.** `cargo install perllsp` or a pre-built binary download. No Perl, no CPAN modules, no environment configuration. It starts in milliseconds regardless of what Perl installation exists (or doesn't exist) on the machine.

**It is safe on untrusted code.** No `BEGIN` block execution, no module loading, no Perl process at all. You can open a PR from an unknown contributor and get diagnostics, navigation, and completions without executing anything from that codebase.

**It works on incomplete codebases.** Code with missing dependencies, broken CPAN installs, or partial implementations can still be analyzed. The parser works on the source text, not the runtime state.

**Cross-platform by construction.** A Rust binary compiled for Linux, macOS, and Windows behaves identically on all three platforms.

### Feature Coverage

perl-lsp implements 98 LSP and DAP features, tracked in `features.toml`. The feature table from the research comparison:

| Feature | perl-lsp | PerlNavigator | Perl::LanguageServer | PLS |
|---------|----------|---------------|---------------------|-----|
| Go to Definition | Yes | Yes | Yes | Yes |
| Find References | Yes | Partial | Yes | Partial |
| Completion | Yes (workspace-aware) | Yes | Yes | Yes |
| Hover | Yes (rich) | Yes | Basic | Basic |
| Diagnostics | Yes (static) | Yes (perl -c) | Yes (perl -c) | Yes |
| Rename | Yes | Partial | Partial | No |
| Code Actions | Yes | Partial | No | No |
| Call Hierarchy | Yes | No | No | No |
| Type Hierarchy | Yes | No | No | No |
| Semantic Tokens | Yes | No | No | No |
| Signature Help | Yes | No | No | No |
| Inlay Hints | Yes | No | No | No |
| Code Lens | Yes | No | No | No |
| Integrated Debugging (DAP) | Yes | No | Via external | No |

### The Trade-Off

perl-lsp's static parser is honest about its limitations. A static parser cannot be 100% correct for Perl — source filters rewrite code before the parser sees it, `use constant` changes parse state at compile time, and some runtime-modified symbol tables are fundamentally opaque to static analysis.

perl-lsp tests against 4,355 real CPAN modules in CI, with a ratchet gate that only allows the parse rate to increase. The current clean-parse rate is 85.7% and rising. The remaining gap is largely:

- Source-filtered code (2-3% of CPAN, incompatible with static analysis by definition)
- Complex `use` effects that modify parser state at runtime
- Deeply nested constructs with genuine grammatical ambiguity

PerlNavigator achieves higher accuracy for syntax validation by running `perl -c` — which sees exactly what Perl sees. The trade-off is real: dynamic accuracy requires executing code, which requires a working Perl installation and introduces security constraints. Static analysis achieves broad coverage without those requirements.

For the majority of IDE interactions — completion, navigation, hover, diagnostics on common patterns — the difference in practice is small. The 15% gap consists of constructs that static analysis cannot reliably handle, not everyday code.

---

## Summary

| Dimension | perl-lsp | PerlNavigator | Perl::LanguageServer |
|-----------|----------|---------------|---------------------|
| Requires Perl runtime | No | Yes | Yes |
| Code execution risk | None | Yes (perl -c) | No (PPI) |
| Windows support | Native | Limited | No |
| Accuracy model | Static (85.7% CPAN) | Dynamic (perl -c) | Static (PPI) |
| Feature breadth | 98 LSP/DAP | ~30 effective | ~20 effective |
| Installation | Single binary | CPAN + Perl runtime | CPAN + Perl runtime |
| Maturity | Public alpha (0.12.0) | Production | Production |
| CPAN corpus validation | CI-gated, ratchet-only | None | None |

perl-lsp is new. PerlNavigator and Perl::LanguageServer have years of production use and established user bases that perl-lsp does not yet have.

What perl-lsp offers is a different architecture with different trade-offs: zero dependencies, safe analysis of untrusted code, and a feature set that goes beyond what is possible when the parser is a regex engine or a call to `perl -c`.

The 78% of Perl developers who currently use no LSP tooling are not unserved by lack of choice — they are unserved by friction. perl-lsp is designed to remove that friction.

---

*Metrics verified against `docs/project/PUBLICATION_FACTS_LEDGER.md`. Feature count (98) verified against `features.toml`. Install counts from VSCode Marketplace (early 2026), subject to change — refresh before publication. The 78% survey figure is attributed to the 2025 Perl IDE Survey (602 respondents); primary source not independently linked.*
