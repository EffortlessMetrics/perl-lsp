# perl-lsp 0.13.0 Release Notes (DRAFT)

## Highlights

The 0.13.0 release represents the public alpha announcement milestone. Since
v0.12.1, the project shipped 71 commits across seven sub-milestones (v0.12.2
through v0.12.8) in a single high-throughput development session, delivering
major advances in refactoring, diagnostics, parser performance, distribution
packaging, and developer experience. LSP and DAP feature coverage reached 100% across
all 119 catalogued capabilities (88 LSP + 24 DAP + 7 extension features, corrected
from the prior 102 count in PR #4107 after a DAP catalog audit surfaced 14 uncatalogued
handlers).

## Features

### Refactoring Engine
- **Subroutine inlining** code action (#3040, PR #3083) -- inline a subroutine call at its use site; deep review caught and fixed 4 bugs before merge
- **Extract variable and subroutine** code actions (#3031, PR #3090) -- select an expression and extract it into a named variable or new subroutine

### Diagnostics
- **Dead code highlighting** with `DiagnosticTag::Unnecessary` (#2060, PR #3092) -- unused code now renders with a faded style in editors
- **Perlcritic integration hardened**: cached analyzer instance, walk-up `.perlcriticrc` discovery (#2018, PR #3097)
- **Strict/warnings diagnostics** catalogued in `features.toml` as PL100/PL101 (#3093, PR #3095)

### Semantic Analysis
- **Complete semantic framework coverage** for inheritance and exports (#3077, PR #3103)
- **Type inference in hover**: wired `TypeInferenceEngine` to show inferred types in hover tooltips (#2357, PR #3150)

### Parser
- **Context-sensitive quote-like operator parsing** (#3020, PR #3105) -- correct handling of `q{}`, `qq{}`, `qw{}`, and related operators in context

### Debug Adapter Protocol (DAP)
- **Cross-platform continue and interrupt** signal handling (#3028, PR #3117)
- **Stale attach mock stub removed** and tests updated (#3025, PR #3135)

### Editor Experience
- **Heredoc language injection** for SQL, HTML, and JSON embedded in Perl heredocs (#2059, PR #3134)
- **POD preview panel** -- `Perl: Preview POD` command renders POD documentation inline (#2062, PR #3131)
- **AST explorer debug panel** -- `perl/showAst` custom LSP handler for inspecting parse trees (#2065, PR #3124)

### Distribution & Packaging
- **Docker image** with perllsp + Perl runtime (#2083, PR #3113)
- **Linux/macOS installer script** (#2095, PR #3122)
- **Homebrew bump workflow** and install documentation (#2086, PR #3120)
- **Windows bump workflows** aligned with `perllsp` binary name (#2596, PR #3106)
- **Linux packaging templates** aligned with `perllsp` binary name (#3144)

## Bug Fixes

- fix(ci): skip CI gate on branch-deletion pushes (#3081, PR #3086)
- fix(ci): enforce Era 7 session learnings (#2660, PR #3088)
- fix(ci): fix pipeline-labels race condition on `reviewed-deep` (#3089, PR #3100)
- fix(ci): regenerate stale LSP capability snapshots (PR #3142, PR #3147)
- fix(ci): revert xtask optional deps that broke v2_parity gate (PR #3149)
- fix(ci): convert justfile recipes from `$$` escaping to shebang format (PR #3140)
- fix(perl-uri): move windows-only import into function scope (PR #3084)
- fix(error-handling): add trace logging for silent fallbacks (#3029, #3032, #3036, PR #3087)
- fix(tests): correct broken test imports and struct initializers (PR #3143, PR #3148)

## Performance

- **Incremental parser checkpoint recovery** (#2080, PR #3114) -- parser can now resume from checkpoints after partial edits
- **Token caching for incremental parsing** (#3021, PR #3116) -- avoids re-lexing unchanged regions
- **`Parser::from_tokens`** to complete the incremental pipeline (PR #3128) -- accepts pre-lexed tokens for zero-lex-cost re-parses
- **HashMap optimization for large-workspace startup** (#2078, PR #3112) -- faster workspace indexing via pre-sized maps
- **Memory profiling infrastructure** (#2085, PR #3125) -- tooling to measure and track allocator behavior
- **CPAN-scale benchmarks**: 10K files, 500K symbols (#1664, PR #3121, PR #3132) -- validation that the index scales
- **Benchmark-driven completion latency analysis** (#2077, PR #3104) -- instrumented completion provider for latency tracking

## Infrastructure

### CI & Quality Gates
- Version sync enforcement in merge gate (PR #3078)
- Benchmark regression alerts tightened (PR #3079)
- Branch coverage baseline surfaced and refreshed (PR #3080)
- Automated corpus ratchet after parser fix merges (#2026, PR #3110)
- 90% CPAN clean rate target documented (#3076, PR #3123)
- Clippy zero-warning enforcement across workspace (PR #3138)
- All 7 Tier 1 parser blockers confirmed fixed (PR #3085, PR #3096)

### Code Quality
- Removed 8 unused dependencies across 6 crates (PR #3146)
- Removed debug `println!` from library code (PR #3145)
- Deduplicated benchmark functions (PR #3138)
- VSCode extension ESLint config and floating promise fixes (#1910, PR #3111)

### Test Coverage
- Added missing tests for error builders, lexer modes, and error paths (#3024, #3030, #3039, PR #3091)

### Dependency Updates
- codecov/codecov-action 5 to 6 (PR #3066)
- toml 1.0.7 to 1.1.0 (PR #3071)
- insta 1.46.3 to 1.47.1 (PR #3070)
- uuid 1.22.0 to 1.23.0 (PR #3069)
- proptest updated (PR #3065)
- tar updated (PR #3064)
- actions/deploy-pages 4 to 5 (PR #3067)
- 2 additional dependency group updates (PR #3068)

## Documentation

- Problem-first README rewrite for the modern Perl tooling gap (PR #3119)
- End-to-end LSP feature development guide (#3027, PR #3115)
- Large-workspace testing and profiling guide (#3022, PR #3118, PR #3126)
- GIF recording guide and asset structure (#2336, PR #3130)
- Roadmap refresh reflecting all 0.12.x milestones complete (PR #3082, PR #3102, PR #3139)
- Economics/session analysis documents (PR #3098, PR #3107, PR #3109, PR #3133, PR #3141)

## Breaking Changes

None. This release is additive -- all existing APIs and LSP capabilities remain
stable.

## Resolved Issues

Over 60 issues were closed across the 0.12.2--0.12.8 sub-milestones, including:
#1664, #1910, #2018, #2026, #2059, #2060, #2062, #2065, #2077, #2078, #2080,
#2083, #2085, #2086, #2095, #2325, #2328, #2336, #2357, #2596, #2660, #3018,
#3020, #3021, #3022, #3024, #3025, #3027, #3028, #3029, #3030, #3031, #3032,
#3036, #3037, #3039, #3040, #3076, #3077, #3078, #3079, #3080, #3081, #3085,
#3089, #3093, #3096.

## Statistics

- 71 commits merged to master since v0.12.1
- 59+ PRs merged in the 0.12.x series
- 134 workspace crates, 119 catalogued capabilities at 100% coverage (88 LSP + 24 DAP + 7 extension)
- 8 Dependabot PRs for dependency freshness
