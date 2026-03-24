# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

- No unreleased changes have been recorded yet.

## [0.12.0] - 2026-03-24

This section covers 583 commits across ~90 PRs since the 0.12.0 finalization
(2026-03-20), spanning Era 7 sessions 1-4. Key themes: parser corpus push toward
100% CPAN, Moose/Moo class intelligence, diagnostic pipeline wiring, VS Code UX
polish, performance hardening, and swarm infrastructure maturation.

### Added

#### Parser
- **Error Recovery Substrate**: New `RecoverySite` and `RecoveryKind` infrastructure for structured recovery from parse failures (#2843, #2868)
- **Parser Corpus Push**: Addressed top CPAN corpus error buckets — `and`+comma, `+` prototype, unclosed-paren identifier, typeglob punctuation `*<` `*>` `*(` `*)`, s///e replacement, x-operator precedence, C-style for loops, keyword hash keys in `use` imports (#2149, #2189, #2254, #2261, #2395, #2461, #2625, #2684, #2703, #2731, #2732, #2755, #2826, #2829, #2834-#2835, #2856, #2859)
- **Parser Disambiguation**: Phase-block keywords as statement labels, anonymous sub with attributes, `bless []` as function call, regex-op names as hash subscript keys, leading `::` qualifier in subroutine names (#2601, #2710, #2754)

#### LSP Features
- **Per-Feature Disable**: `initializationOptions.disabledFeatures` allows users to opt out of individual LSP features (#2170, #2766)
- **Editor-Agnostic Project Config**: `.perl-lsp.toml` configuration file for project-level settings, recognized by all editors (#2053, #2805)
- **Semantic Token Delta Encoding**: LSP 3.16 delta encoding for semantic tokens reduces bandwidth on edits (#2743)
- **Document Ranges Formatting**: Advertise `documentRangesFormattingProvider` to close LSP 3.18 gap
- **Workspace Symbol Resolve**: Emit documentation and fix `containerName` in `workspace/symbol/resolve` (#2100, #2798)
- **Diagnostic Pull Path**: Wire diagnostic enrichment through pull diagnostic runtime path (#2102, #2795)
- **Diagnostic Data Field**: Populate `Diagnostic.data` field for client-side diagnostic actions (#2592)
- **Progress Notifications**: `$/progress` notifications during workspace indexing (#2317, #2356)
- **Parse Error Telemetry**: Opt-in parse error telemetry wired into diagnostic pipeline (#2740)
- **Startup Diagnostic**: Structured startup message to stderr for editor log integration (#2054, #2807)
- **On-Type Formatting**: Advertise `\n` as formatting trigger character (#2746, #2779)
- **POD Symbols in Outline**: `=head` POD sections appear in document symbol outline (#2614)
- **Signature Help Retrigger**: Added retrigger characters for complex Perl call signatures
- **InlayHint Resolve**: Wire `inlayHint/resolve` `labelDetails.location` for click-to-definition
- **Commit Characters**: Add `commit_characters` to `CompletionItem` for both handlers (#2597)
- **Binary File Guard**: Prevent parser from processing binary files in `didOpen`/`didChange` (#2107, #2764)
- **CRLF Guard**: Add CRLF line-ending guard in `position_to_offset_rope` (#2108, #2736)

#### Moose/Moo Intelligence
- **Class Model Wired**: `ClassModel` detection wired into `SemanticAnalyzer` (#1661, #2741)
- **Goto-Definition for Methods**: `$self->` and `$class->` method calls resolve via workspace index (#2536, #2831, #2858)
- **Type Hierarchy**: C3 MRO linearization for Moose inheritance chains; `extends`/`with` wired into `TypeHierarchyProvider` (#2720); cross-file supertypes/subtypes via workspace scan (#2738)
- **Role Composition Hover**: Hover card for `with`-applied roles showing consumed methods (#2325, #2745)
- **Method Modifier Hover**: Dedicated hover card for `before`/`after`/`around` modifiers (#2744)
- **Attribute Introspection**: `builder`, `coerce`, `predicate`, `clearer`, `trigger` in attribute hover (#2366)

#### Hover
- **Phase Blocks**: BEGIN/END/INIT/CHECK/UNITCHECK get hover documentation and diagnostics (#2623)
- **Special Variables**: Educational hover tooltips for Perl special variables (`$!`, `$@`, `$/`, etc.) (#2262, #2347)
- **Tied Variables**: Show tied class and tie magic method docs on hover (#2609)
- **Context-Sensitive Builtins**: Scalar/list context docs for 11 context-sensitive builtins (#2630)
- **Perl Built-in Examples**: Phase 1 hover docs with capture vars, autoflush, MetaCPAN links, examples (#2831, #2839)
- **Subroutine Complexity**: Complexity indicator shown in sub hover card
- **POD on Use Statements**: Show POD documentation when hovering on `use Module` (#2304)
- **Type Inference Display**: Wire `TypeInferenceEngine` to hover for inferred variable type (#2726)
- **Regex Pattern Explainer**: Explain regex patterns in human-readable format (#2048)
- **die/warn Enrich**: Enrich `die`/`warn` hover with docs and add `croak` modernize action (#2606)

#### Completion
- **85 Missing Built-ins**: Add 85 missing Perl built-ins with documentation; fix LSP trigger characters (#2780, #2813)
- **Relevance Sorting**: Relevance-based completion ranking with docs on builtins/keywords (#2832, #2841)
- **Import-Aware Sort**: Workspace symbols sorted by import proximity (#2645)
- **Auto-Import on Selection**: Auto-insert `use Module` when selecting an unimported symbol (#2322)
- **Regex Operator Snippets**: 13 regex operator snippets and regex flag completions (#2607, #2635)
- **Named Capture Groups**: Named capture group completions after `(?<name>)` (#2635)
- **Template Toolkit**: Directive/filter snippet completions for `.tt` files (#2350, #2735)
- **Test::More Snippets**: `ok`/`is`/`like` function hover documentation in completions
- **Built-in Snippets**: Built-in Perl snippet completions for common idioms

#### Diagnostics
- **PL200/PL201/PL300 Wired**: Emit `use strict`, `use warnings`, `package` diagnostics through the pipeline (#2781, #2803)
- **PL405 Format Arity**: `printf`/`sprintf` format specifier arity lint (#2636)
- **PL701 Module Not Found**: `ModuleNotFound` lint using workspace resolver (#2619)
- **Perl Version Compatibility**: Warnings for syntax requiring specific Perl versions (#2050, #2739)
- **Security Lints**: Security-focused lints for `eval`, two-arg `open`, backtick patterns; wire into pipeline (#2724)
- **Unused Import Detection**: Detect and grey-out potentially unused imports
- **Heredoc Anti-Pattern**: Warning for heredoc anti-patterns (#2438)
- **Dead Code Detection**: Wire dead code analysis into diagnostic pipeline
- **Perl::Critic Integration**: Wire perlcritic with severity/profile config and byte-offset mapping (opt-in) (#2362)
- **Auto-Quote Suggestion**: Suggest quoting bareword warnings (#2365)
- **Suppress on Empty Files**: Suppress `strict`/`warnings` lint on empty files (#2112, #2792)
- **Context Hints**: Add `context_hint()` to `DiagnosticCode` with catalog surfacing

#### Navigation
- **Go-to-Test / Go-to-Implementation**: Navigate between test files and implementation (#2532)
- **use parent/use base**: Workspace rename respects `use parent`/`use base` dependency graph (#2747, #2782)
- **Module Resolution**: Parse `use lib` and `FindBin` for include path resolution (#1662, #2620)
- **Inline Rename Scope**: Route `all_occurrences` to workspace-wide lookup (#439, #2765)

#### VS Code Extension
- **Report Issue Command**: One-click issue reporting from the command palette (#2160, #2748)
- **Extension Auto-Update Check**: Notify users when a new version is available (#2165, #2796)
- **Special Variables Reference**: Command to open special variable reference panel (#2318, #2763)
- **Debugger Setup Wizard**: Interactive DAP configuration wizard (#2338, #2762)
- **Interactive Onboarding Walkthrough**: First-run walkthrough for new users (#2046)
- **Smart File Creation**: New Perl file command inserts package boilerplate
- **What's New Panel**: Show release notes webview on extension update
- **Status Bar Health Widget**: Show file count and error counts in status bar
- **Version in Status Bar**: Display `perl-lsp` version in status bar (#2340)
- **Formatting Error Toasts**: Surface formatting errors as toast notifications (#2111, #2793)
- **Wire 4 Unimplemented Commands**: `extractVariable`, `extractMethod`, `showRefactoringOptions`, `reportIssue` (#2849, #2854)
- **Refactoring Discoverability**: Keyboard shortcuts and menu entries for refactoring actions
- **Boilerplate Snippets**: Perl boilerplate snippet library (#2314)

#### Code Actions
- **Portable Shebang**: Quick-fix to suggest portable shebang line (#2255)
- **Modernization Suggestions**: Perl modernization code actions (say, given/when, etc.)
- **Bareword Filehandle Fix**: Quick-fix for `bareword-filehandle` and two-arg `open` patterns
- **Find Undefined Functions**: Wire `find_undefined_functions` to semantic analyzer (#2692, #2719)

#### Code Lens
- **Test File Lenses**: Detect test functions and provide `Run Test`/`Debug Test` lenses
- **Subtest Lenses**: Subtest detection with `perl.runSubtest` command (#2617, #2673)

#### Performance
- **SemanticAnalyzer LRU Cache**: Cache hover/definition lookups to avoid repeated analysis (#2074, #2806)
- **Asynchronous Workspace Indexing**: Non-blocking workspace scan (#2352)
- **Request Prioritization**: Cancel stale requests; prioritize interactive over background work
- **perltidy/perlcritic Timeout**: Thread-based subprocess timeout prevents editor hangs (#2616)
- **Parser Cancellation**: Cancellation token checks in parser hot path for responsive LSP (#2615)
- **Debounced Diagnostics**: Debounce diagnostic publication during rapid typing
- **SymbolIndex Queries**: Wire `SymbolIndex` into completion and workspace symbols for O(1) lookups (#2728)
- **Deferred Health Check**: Move VS Code health check from activation to first-error path (#2715)
- **Workspace-Index CPAN Tuning**: Tune index for CPAN-scale workspaces (#1664)

#### CLI
- **--check-project**: Parsability report for a directory tree (#2534)
- **--check-format json**: Machine-readable JSON output for CI integration (#2734)

#### Infrastructure
- **Blocker Ledger**: Track known blockers to prevent scout rediscovery (#2586)
- **Corpus Sweep Schema 1.2.0**: Add `files_by_bucket` to baseline schema (#2585)
- **17 New Error Buckets**: Decompose `unexpected_token_in_expr` catch-all (#2611, #2814)
- **Unwired Infrastructure Scanner**: Find built-but-not-wired crates automatically (#2667, #2827)
- **Snapshot Testing**: Systematize snapshot tests for AST and error messages (#2104, #2823)
- **nextest**: Enable cargo-nextest for `ci-test-lib` (#1909, #2804)

### Fixed

#### Parser
- `and` operator collecting comma-separated RHS expressions (#2856, #2859)
- `+` prototype character handling, fixes `unexpected_rbrace_expr` (#2835, #2838)
- Unclosed-paren identifier patterns (#2834, #2840)
- Typeglob punctuation variables `*<`, `*>`, `*(`, `*)` (#2755, #2826)
- Keyword hash keys and `q{}` in `use` imports (#2189, #2723)
- Keyword tokens as package names (#2150, #2706)
- C-style `foreach` loops (#2149, #2703)
- Leading `::` qualifier in subroutine names (#2710)
- `s///e` replacement with `/` inside string literals (#2395)
- x operator RHS precedence with `**` (#2625, #2684)
- Substitution/transliteration modifier parsing regression (#2732, #2776)
- Soft recovery for unclosed bracket in postfix expressions (#2148, #2688)
- `unexpected_token_in_expr` decomposed into 4 sub-patterns (#2731, #2767)
- Optional-arg unary builtins before binary operators (#2730, #2775)
- Top corpus error buckets — multiple CPAN patterns fixed (#2461, #2829)
- Sigil-peek heuristic for imported unary functions (#1943)
- `$self->` and `$this->` method resolution via workspace index (#2536)
- Regex-op names as hash subscript keys (#2754, #2789)
- Phase-block keywords as statement labels
- Strip `#` comments from `qw()` content before word-splitting (#2618)
- Fat-arrow pairs as ternary branch expressions (#2402, #2613)
- Word operators after `return`, loop control, indirect calls, paren lists
- Missing special variables `$>`, `$<`, `$)` and empty `while` condition
- Handle `my $var->{key} = ...` lvalue declaration
- `defined`/`ref` without arg when followed by word operators (#2626)
- `bless []` as function call, not subscript (#2387)
- File-test-no-operand and `CORE::` builtins in grep/map context (#2674)
- `ref ne/eq` and `or` comma-expr in `grep` blocks (#2388)
- v-strings in expression and comparison contexts
- Recover inline from missing semicolons in C-style for loops (#2593)
- Relex slash as regex at statement start after closing brace
- Replace restrictive import parser in `parse_no` with depth-tracking slurp

#### Lexer
- `after_var_subscript` flag clearing on `)` — fixes `if($var){m//}` (#2844, #2851)
- `hash_brace_depth` narrowed to `after_var_subscript` — fixes quote-op suppression in blocks (#2833, #2837)
- `try_heredoc` guarded against ExpectOperator mode — fixes bitshift-in-paren (#2750, #2769)
- Whitespace-separated quote delimiters restricted to paired chars (#2732 regression, #2815)

#### LSP Runtime
- ABBA lock ordering deadlock eliminated; lock contention reduced (#2712)
- Reentrant deadlock in `publish_diagnostics` fixed (#2646)
- Concurrent workspace indexing scan prevention (#2641, #2711)
- `ParentMap` safety improvements Phase 1 (#2810, #2819)
- Semantic token legend synchronized with capability advertisement (#2103, #2772)
- Advertise `TextDocumentSyncKind::Full` (not `Incremental`) to match actual behavior
- Depth underflow panic in linked-editing backward bracket scan (#2603)
- 5 advertised-but-unhandled commands wired in `execute-command` provider (#2691, #2717)

#### Code Actions
- 4 defects in extract variable/subroutine refactoring (#349, #2797)
- Diagnostics lint checks wired into `get_diagnostics()` pipeline (#2544)

#### Security
- macOS sandbox path injection and profile file-path bug (#2749, #2799)
- `strict`/`warnings` false positives suppressed on empty files (#2112, #2792)

#### Tests
- Replace `unwrap`/`expect` with `must`/`must_some` across workspace (#2649, #2721-#2727)
- Use AST walk instead of string matching in `assert_clean_parse` (#2553, #2559)
- Fix stale capability snapshots blocking CI (#2855)

### Changed

#### Refactoring
- **17 Built-but-Unwired Crates Wired**: Wire 17 LSP provider microcrates into the runtime (#2756, #2768)
- **7 Dead Navigation Functions Removed**: Delete prototype navigation functions from `perl-lsp` (#2713)
- **`semantic.rs` God File Split**: 3,256-line `semantic.rs` split into 6 focused sub-modules
- **Status Docs Modularized**: Monolithic `CURRENT_STATUS.md` split into `docs/project/status/*.md` subsystem files (#2801, #2830)
- **`perl-lsp-inline-completion` Wired**: Replace inline duplicate impl with microcrate (#2758, #2786)
- **Dead Code Cleanup**: Remove dead `perl-symbol-table` crate (#2714), dead `perl-source-editing` crate (#2699, #2716), dead `IndexAccessMode` lint suppressors (#2702, #2790)
- **`perl-test-must` Extracted**: New micro-crate extracted from `perl-tdd-support` (#2842, #2864)
- **Test Modules Extracted**: Inline test modules moved to `tests/` directories (#2462-#2466)
- **`logos` Lexer Archived**: Broken logos experiment archived; `chumsky` dependency removed (#2612)
- **ADR Renumbering**: Duplicate ADR-0035/0036 renumbered to ADR-0039/0040 (#2808, #2816)

#### CI
- Consolidate clippy passes; enable nextest for library tests (#1909, #2804)
- Pre-push hook updated to track test counts (#2128)

#### CPAN Corpus
- Corpus baseline ratcheted from 85.7% toward 90%+ through multiple parser fix waves

### Performance

- **SemanticAnalyzer LRU Cache**: Avoid repeated analysis on hover/definition (#2074, #2806)
- **Async Workspace Indexing**: Non-blocking index scan for large workspaces (#2352)
- **Parser Cancellation Token**: Check cancellation in parser hot path (#2615)
- **Debounced Diagnostics**: Debounce publication during rapid typing
- **SymbolIndex O(1) Queries**: Wire `SymbolIndex` for completion and workspace symbols (#2728)
- **perltidy/perlcritic Timeout**: Subprocess timeout prevents editor hangs (#2616)
- **CPAN-Scale Index Tuning**: Workspace index tuned for large CPAN workspaces (#1664)

## [0.12.0] - 2026-03-20

This release is the **public alpha launch** -- the first release intended for broad
external adoption. It spans 590+ commits and 200+ merged PRs, delivering major parser
improvements (83% to 85.7% CPAN corpus), new LSP features, distribution tooling, and
comprehensive documentation for first-time users.

### Added
- **Graceful Degradation**: Three-tier degradation for partial parse results -- full AST, partial recovery, and graceful fallback (#2219).
- **Large File Guard**: Skip parsing for oversized files to prevent editor hangs (#2163, #2229).
- **Hover: Module Path**: Show resolved module path on `use` statement hover (#2211).
- **DAP Inline Values**: Display variable values inline during debugging sessions (#2212).
- **Parser: Fix Suggestions**: Helpful fix suggestions in parser error messages (#2200).
- **Distribution: cargo-binstall**: Pre-built binary installation via `cargo binstall perl-lsp` (#2071, #2209).
- **Distribution: Man Page**: `man perl-lsp` page generated from CLI definition (#2210).
- **Distribution: PowerShell Completions**: Shell completion generation for PowerShell (#2075, #2214).
- **VS Code: Test Explorer**: Test explorer integration for Perl `.t` files (#2033).
- **VS Code: Extension Polish**: Marketplace metadata, test suite, and documentation for 0.12.0 (#2032, #2034).
- **First-Run Experience**: Getting-started guide and improved install verification for new users (#1658).
- **Completion: Import Lists**: `use Module qw(...)` triggers symbol completion from the target module (#1937).
- **Completion: Regex Literals**: Variable and function completions inside `/…/`, `m/…/`, `qr/…/` patterns (#1925).
- **Completion: Scope-Ranked Locals**: Local symbols ranked by scope distance for more relevant suggestions (#1983).
- **Completion: Qualified Variables**: Workspace-qualified variable completion for cross-package symbols (#1731).
- **Global Reference Index**: O(1) symbol lookups via a new global reference HashMap in workspace-index (#1934).
- **V-String Tokenization**: Version strings (`v5.36.0`) now tokenized as a dedicated token type (#1914).
- **DeadBranch Detection**: Dead code analysis detects constant-condition `if`/`unless` branches (#1596).
- **Class Field Declarations**: Parser supports Perl class field declaration syntax (#1808+).
- **CLI Flags**: `--check`, `--info`, `--completion` flags for scripting and editor integration (#1682).
- **Diagnostic Accessibility**: Improved error message quality and accessibility in diagnostics (#1672).
- **Async Runtime with Concurrent Dispatch**: Two-lane scheduler -- exclusive worker for mutations + 4-worker read pool for concurrent requests. `$/cancelRequest` processed inline (#1555).
- **Goto AST Node**: Dedicated `Goto` node with full `TokenKind::display_name` support (#1521).
- **Smarter Selection Range**: Expand/shrink selection chains with semantic awareness (#1545).
- **Cross-file Go-to-Definition**: Improved navigation for method calls and `use parent`/`base` statements (#1542, #1544).
- **Enhanced Diagnostics**: Added `suggestion` field to diagnostic messages (#1543).
- **Inlay Hints for Builtins**: Parameter names derived from builtin function signatures (#1541).
- **Semantic Tokens**: Comprehensive AST walker with new token types for broader coverage (#1540).
- **Cross-sigil Variable Highlighting**: Highlight `@foo`/`%foo` references when cursor is on `$foo` (#1538).
- **Extract Variable for Methods**: Code actions handle method calls and hash/array access (#1534).
- **Workspace Symbols Ranking**: Improved ranking algorithm with comprehensive tests (#1529).
- **Completion for Moo/Moose Accessors**: Show `isa` type in completion for accessor methods (#1525).
- **Signature Help Builtin Coverage**: Expanded coverage for common Perl builtins (#1532).
- **DAP Improvements**: POD detection, conditional expression validation in breakpoints (#1536), improved variable inspection rendering (#1535), hardened smoke tests and timeout handling (#1883).
- **Hover Enhancements**: Improved documentation quality in hover responses (#1537).
- **VS Code Extension**: Trace support, config change detection (#1876), `--health` binary validation before starting LSP client (#1598), Open VSX keywords and metadata (#1879), client refresh behavior fix.

### Fixed
- **Parser -- Control Flow**: Handle orphaned `else`/`elsif` and `unless`+`else`/`elsif` chains (#1981), allow bare `return` in ternary branches (#1727), handle statement-start nullary builtin precedence (#1724), recover bare list-operator calls in postfix args (#1989), handle `for`/`foreach` without explicit loop variable (#1700, #2040), handle ternary after named-unary operators (#2025).
- **Parser -- Expressions**: Accept fat arrows in expression contexts (#1985), support last-index deref `->$#*` in bracket expressions (#1988), slurp trailing operators after Number/String in `use` import values (#1980), handle `use constant NAME => expr` fully (#1577), handle complex expressions in parenthesized arguments (#1704, #2206), accept complex expressions in `use`/`no` import lists (#2184, #2221), audit fat arrow expressions (#1651, #2171), allow postfix operators after typeglob expressions (#2188, #2238).
- **Parser -- Disambiguation**: Disambiguate `field` keyword from bareword identifier (#1978), allow keyword barewords as subroutine names (#1986), allow keyword methods and trailing separators (#1993), recover field bareword calls in recovery parser, validate declaration attributes.
- **Parser -- Builtins**: `map`/`grep`/`sort` BLOCK LIST without trailing semicolon (#1623), `tie(VARIABLE, CLASS, LIST)` with parenthesized args (#1630), `defined`/`ref` at statement start (#1618), `push`/`pop` with postfix deref lvalue (#1619), nullary builtins in paren expressions (#1629), improve word operator handling for 43 CPAN files (#1922).
- **Parser -- OO**: Tighten qualified class-name and namespaced class parsing, statement modifiers after complex expressions (#1550), package-qualified variable subscripts (#1548).
- **Parser -- Misc**: Tighten deref parsing (#1884), transliteration delimiter parsing, operator strings in `use overload` (#1492), chained method calls after deref constructs (#1474), ternary then-branch assignments (#1516, #1518), guard semicolon break with `at_top_level` in `use` imports (#2237), add `RightBrace` to indirect call argument terminators (#2222, #2236).
- **Lexer**: Prevent prototype mode leak after `sub` keyword (#1906), disambiguate regex after bare builtins (#1965), recognize special punctuation variables `$~`, `$^`, `$=`, `$%`, `$;`, `$^W` etc. (#1615), disambiguate `$$var` scalar deref from `$$` PID variable (#1572), peek/reset state restoration, make regex parse budget reachable (#1455).
- **Completion**: Preserve regex interpolation completions (#1925).
- **Workspace Index**: Rebuild find-definition symbol cache after index updates (#1919).
- **LSP Runtime**: Preserve scheduler ordering and stabilize tests (#1882), close outbound sender before joining writer thread (#1593), stop advertising unsupported `debugTests` command (#1742).
- **Incremental Parsing**: Improved efficiency and fixed position underflow (#1539).
- **On-Type Formatting**: Heredoc suppression, string/comment-aware brace matching, correct trigger semantics (#1530).
- **Diagnostics**: Suppress strict/warnings false positives for OO frameworks (#1565).
- **DAP**: Fix socket default port, harden debugger smoke and timeout handling (#1883), prevent subtraction overflow in inline values (#1515).

### Changed
- **Microcrate Extractions**: 10 new microcrates extracted -- perl-dap-config, perl-dap-session-model, perl-ast-v2, perl-ts-statement-tracker, perl-lsp-type-hierarchy, perl-perltidy, perl-lsp-completion-filepath, perl-workspace-index-monitor, perl-lsp-code-lens, perl-lsp-document-highlight.
- **God File Splits**: `debug_adapter.rs` (6778 lines) split into focused domain modules (#1666, #1639, #2208), `lsp_comprehensive_3_17_test.rs` split into feature-specific test files (#1681), `cpan_pattern_tests.rs` split into 16 standalone test files (#1665), runtime `mod.rs` handler groups extracted (#1676).
- **Refactored Internals**: Execute command modules, code actions provider, perl critic tooling, centralized server startup logging (#1826).
- **Feature Gating**: Tightened LSP capability feature gating and feature profile normalization.
- **Native Debt Report**: `xtask` now has a native `debt-report` subcommand (#1528).
- **Devex Targeted Checks**: Converted from shell script to native Rust xtask subcommand (#1527).
- **CI**: Auto-fix formatting instead of failing on `cargo fmt` (#1625), pre-push hook to update test counts (#2128).
- **CPAN Corpus**: Baseline ratcheted from 83% to 85.7% (6081/7095 files) (#1892, #2233, #2244).
- **Documentation**: Comprehensive README for 0.12.0 (#2012), launch articles covering development story, reference implementation, AI-native operating model, and case studies (#2203, #2235, #2240, #2242, #2243), contributing guide update (#2010).

### Performance
- **find_definition**: Replaced O(n*m) scan with O(1) HashMap lookup in workspace-index (#1919).
- **LSP Async Scheduling**: Improved read scheduling for lower latency on concurrent requests (#1837).
- **Completion**: Reduced unnecessary clones in completion hot path (#2073, #2220).
- **Document Highlight**: Linear dedup for small highlight sets, eliminated clone (#1928).
- **Workspace Symbols**: Reduced allocations in workspace symbol search (#1908).
- **Performance Baselines**: Established 0.12.0 performance baselines (#1654, #2166).

## [0.11.0] - 2026-03-12

This release finalizes the 0.11.0 distribution pipeline across GitHub releases,
crates.io, and the VS Code extension so the workspace can ship from a single,
repeatable release flow.

### Added
- **Turnkey Release Orchestration**: A PR-driven release path now covers version
  bumping, changelog generation, tagging, GitHub release creation, crates.io
  publishing, extension publishing, and downstream package manager automation.
- **Topological crates.io Publishing**: Workspace publish automation computes
  dependency order from `cargo metadata` and publishes only the crates in the
  workspace allowlist.
- **Release Guardrails**: Release helper scripts now validate semver inputs and
  align manual operator flows with the automated `0.11.0` release path.

### Changed
- **Workspace Release Alignment**: Workspace packages, extension metadata, and
  release workflows now target `0.11.0`.
- **Release Tooling**: Legacy release helper scripts now delegate to the current
  GitHub workflow-based release flow instead of relying on stale one-off cargo
  publish steps and outdated examples.
- **Operator Documentation in Scripts**: Manual publish and smoke-test helpers
  now accept an explicit version argument and default to the matching `vX.Y.Z`
  release ref when dispatching workflows or validating published artifacts.

### Fixed
- **Stale Release Examples**: Removed hardcoded `0.8.3` release references from
  publish and smoke-test scripts that could misdirect manual release operations.
- **Publish Version Safety**: crates.io publishing now fails early when the
  workflow target version does not match the versions resolved for workspace
  crates scheduled for publication.

## [0.10.0] - 2026-02-28

A major release campaign spanning 60+ PRs (#845-#911) focused on build reliability,
security hardening, crates.io publishing readiness, documentation, and code quality.

### Added
- **Document Highlight for Modern Perl**: try/catch parameters, method/sub signatures, and string interpolation (#882, #896).
- **Feature Governance Microcrates**: Extracted feature governance into 9 dedicated crates for modularity (#848).
- **Module Infrastructure Crates**: Content-Length framing and LSP transport hardening (#857).
- **Context-Aware Status Menu**: Perl LSP status menu with workspace-aware states (#646).
- **InlineValues Lifecycle Coverage**: Test coverage for inlineValues support (#729).
- **Tie-Interface Corpus Tests**: New corpus test fixtures for Perl tie interface syntax (#900).
- **Public API Documentation**: Comprehensive rustdoc for `perl-parser` (#904) and leaf crates (#903).
- **Copilot Instructions**: `.github/copilot-instructions.md` for AI-assisted development (#886).
- **Merge-Gate Commit Status**: CI now publishes merge-gate status checks (#880).
- **Benchmark Test Enablement**: Previously-ignored workspace benchmark test enabled with real assertions (#908).

### Changed
- **Version Bump to 0.10.0**: All 80+ workspace crates, documentation, VS Code extension, and feature catalogs updated (77+ files) (#879, #884).
- **crates.io Publishing Readiness**: All crate metadata verified, publish-ignore lists normalized, crate badges added, publish allowlist expanded (#865, #867, #871, #897).
- **VS Code Extension Polish**: Marketplace readiness with packaging fixes, runtime deps, npm lockfile (#863, #866, #869, #906).
- **Documentation Overhaul**: CONTRIBUTING.md polished for public release (#909), README.md and ROADMAP.md updated (#888), FrameworkKind/FrameworkFlags docs (#887), cargo doc warnings resolved (#894).
- **features.toml**: Version bumped to 0.10.0 with 100% LSP coverage maintained (53/53 user-visible, 97/97 protocol).
- **LSP Harness**: Replaced sleep-poll with condvar+drain-bytes pattern for deterministic testing (#846).
- **xtask Gates**: Fail closed for required timeout/error statuses (#868).
- **Unused Dependencies Removed**: cargo-machete sweep across workspace (#895).
- **Debt Ledger Updated**: Refreshed after cleanup campaign (#898).
- **Stale Files Cleaned**: Removed stale tracked files, hardened .gitignore (#889).
- **Semver-Aware Benchmark Sorting**: Correct version comparison for baseline selection (#885).

### Fixed
- **Build**: Resolved 4 compilation errors in the release candidate build (#881).
- **Clippy**: Resolved warnings across all targets (#901).
- **Document Highlight Regressions**: Fixed test regressions from modern syntax support (#896).
- **LSP Error Logging**: Improved error logging in LSP providers (#905).
- **Unresolved Review Comments**: Addressed outstanding comments from PRs #881 and #882 (#892).
- **Version Drift**: Fixed remaining v0.9.x references in satellite files (#884).
- **Checksum Verification**: Hardened verification and stabilized incremental parsing CI (#858).
- **Installer Scripts**: Hardened for security and reliability (#910).
- **Refactoring Test Isolation**: Isolated `cleanup_no_backups` backup root (#864).
- **CI Receipt Parsing**: Aligned receipt parsing and serialized BDD tests (#845).
- **CI BDD Gate**: Added `--locked` flag and timing receipts (#847).
- **CI Docs Deploy**: Skip when GitHub Pages is disabled (#859).
- **Release Workflow**: Asset naming alignment across chain (#890, #902), concurrency groups (#890).
- **Release Tooling**: git-cliff installation fixes (#873, #874, #875), cargo-release installs (#876, #877), PR-driven 0.x.y flow (#872).
- **Publish Workflow**: Dry-run quoting fix (#870), `--no-verify` for dev-dep cycles (#867).

### Security
- **[HIGH] Path Traversal in DAP Launch**: Fixed path traversal vulnerability in debug adapter (#640).
- **[HIGH] Argument Injection in TestRunner**: Fixed argument injection vulnerability (#633).
- **[MEDIUM] Safe Evaluation Bypass**: Fixed bypass for iterator/IO operations (#647).
- **GitHub Actions Hardening**: SHA-pinned all workflow action references (#911).
- **Installer Hardening**: Hardened install scripts for security and reliability (#910).
- **VS Code Extension**: Pinned minimatch to 10.2.3 to remediate CVEs (#861).

### Performance
- **Symbol Extraction**: Optimized regex compilation for faster workspace indexing (#645).
- **Semantic Analyzer**: Eliminated deep cloning of AST nodes in subroutine analysis (#632).
- **Scope Analyzer**: Optimized unused parameter detection, fixed double reporting (#638).

### Infrastructure
- **Nightly CI Stabilization**: Fuzz harness panic hardening, coverage test resilience, clippy cleanup (#860).
- **Release Orchestration**: Turnkey PR-driven 0.x.y release workflow (#872).
- **Release Tool Installs**: Deterministic git-cliff and cargo-release installation (#873-#877).
- **crates.io Dry-Run**: Unblocked dry-run packaging for all workspace crates (#865).
- **Lockfile Maintenance**: Refreshed lockfile for CI deny checks, fuzz lockfile exclusion (#885).

### Dependencies
- `rand` 0.9.2 -> 0.10.0 (#855).
- `serial_test` 3.3.1 -> 3.4.0 (#854).
- `uuid` 1.20.0 -> 1.21.0 (#856).
- `toml` 0.9.12 -> 1.0.3 (#853).
- `aquasecurity/trivy-action` 0.34.0 -> 0.34.1 (#851).
- `@types/node` 25.1.0 -> 25.3.0 (#849).
- `@types/tar` 6.1.13 -> 7.0.87 (#850).
- Additional dependency group updates (#852).

## [0.9.1] - 2026-02-20

### Added
- **Initial Public Alpha Release**: Substantially complete feature set for early testing.
- **Enhanced LSP Features**: 99% coverage of LSP 3.18 methods (alpha-validated).
- **Complete Semantic Analyzer**: All NodeKind handlers implemented (Phases 1, 2, 3) with 100% AST node coverage.
- **Debug Adapter Protocol (DAP) Support**: Phase 1 bridge to Perl::LanguageServer.
- **Enhanced LSP Cancellation System**: Thread-safe infrastructure for minimal latency.
- **Advanced Code Actions**: AST-aware refactoring including extraction and import optimization.
- **Security Hardening**: UTF-16 boundary fixes and path traversal prevention.
- **Comprehensive API Documentation**: Infrastructure for documentation enforcement.
- **Optimized Test Suite**: 0.31s full test suite execution via adaptive threading.

### Changed
- **Project Origins Documented**: Origins in Q2 2025, forked July 15, 2025 from `tree-sitter-perl-better`.
- **Stability Roadmap Refined**: Formal Stability Contract (contract-locked APIs) pushed to v0.15.0.
- **MSRV Updated**: Minimum Supported Rust Version bumped to 1.92 (Rust 2024 edition).
- **Parser Architecture**: Native recursive descent parser as the primary implementation.

### Fixed
- **v0.9.1 close-out receipts captured**: Workspace index state-machine transitions and early-exit behavior verified.
- **Security boundary fixes**: Resolved multi-root workspace path traversal issues.

## [0.9.0] - 2026-01-18

### Added
- **Semantic Analyzer Phase 1**: 12/12 critical node handlers implemented.
- **LSP textDocument/definition Integration**: Semantic-aware definition resolution.
- **Enhanced Cross-File Navigation**: Dual indexing strategy for improved reference coverage.

### Changed
- **LSP Coverage**: Increased to 82% of trackable features.

## [0.8.8] - 2025-12-01

### Added
- **Initial Workspace Configuration Support**.
- **Enhanced Formatting Fallback**: Always-available capabilities with perltidy integration.

---

## Future Milestones

### Next Release
- Enhanced DAP native implementation (Phase 2).
- Semantic depth improvements for Moo/Moose.

### v0.15.0 (Stability Contract Milestone)
- **Formal Stability Contract**: Contract-locked APIs and wire protocol invariants.
- Full protocol compliance audit.
- Multi-release deprecation cycles.

---

## Version Support Policy (Alpha Phase)

During the alpha phase (pre-v0.15.0):
- **Current Alpha (0.x.y)**: Active development and bug fixes.
- **Breaking Changes**: Allowed in minor (0.x) releases.
- **Security**: Critical patches prioritized for the latest alpha version.
