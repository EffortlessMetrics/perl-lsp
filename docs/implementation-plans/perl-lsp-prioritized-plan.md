# Perl-LSP Prioritized Implementation Plan

**Generated:** 2026-04-09  
**Issues Analyzed:** 103 open issues across 4 waves  
**Focus Areas:** builtins, version→feature, regex/special vars, subroutines, packages, unicode/encoding, control flow, sigils, phasers, error handling, AUTOLOAD/UNIVERSAL, attributes, constants, XS/C, Moose/MOP, smartmatch

---

## Executive Summary: Top 10 Fixes by Impact/Effort Ratio

| Rank | Issue | Title | Severity | Effort | Impact | Why It Matters |
|------|-------|-------|----------|--------|--------|----------------|
| 1 | #3338 | push @$arrayref does not mark my $arrayref as used | P0 | Small | **Critical** | Core scope analysis bug affecting common Perl idiom; false positives on valid code |
| 2 | #3361 | Signature parameters not added as symbols in symbol table | P0 | Small | **Critical** | Blocks #3362 (go-to-declaration); affects modern Perl 5.36+ signatures |
| 3 | #3356 | Package statement not handled in scope analysis traversal | P0 | Medium | **High** | Blocks #3358 (strict-subs for package-qualified calls); fundamental scope gap |
| 4 | #3351 | Missing special variables: ${^MATCH}, ${^PREMATCH}, ${^POSTMATCH} | P0 | Small | **High** | Perl 5.10+ regex features; false "used but not declared" warnings |
| 5 | #3344 | Missing version→feature mapping: given/when | P1 | Small | **High** | Perl 5.10+ syntax; missing from version_compat lint |
| 6 | #3378 | ScopeAnalyzer does not bind catch (^B4e) variables in try/catch | P0 | Small | **High** | Modern error handling; common Perl 5.34+ pattern |
| 7 | #3383 | UNIVERSAL methods (can, isa, DOES, VERSION) not recognized | P0 | Small | **High** | Core OO methods available on all objects |
| 8 | #3386 | AUTOLOAD methods not recognized as valid method definitions | P0 | Small | **High** | Common Perl pattern for dynamic method resolution |
| 9 | #3377 | Scope analyzer does not isolate lexical scopes for BEGIN/CHECK/INIT/END/UNITCHECK | P1 | Medium | **Medium** | Phaser blocks need special scope handling |
| 10 | #3347 | Missing builtin initializers: dbmopen, shmread | P1 | Small | **Medium** | Position-aware builtin handling (architectural improvement) |

**Key Architectural Wins:**
1. **BUILTIN_INITIALIZERS PHF Registry** (#3345, #3346, #3347) — Systematic handling of builtins with position-aware declaration capability
2. **Parser deref_base annotation** (#3338, #3363, #3364, #3365) — Fix sigil bridging for reference dereferencing
3. **EffectiveSemantics layer** (#3344, #3348, #3349, #3350, #3353, #3375) — Unified version/pragma/feature tracking
4. **Method resolution with mro pragma** (#3384, #3385) — Complete OO method navigation

---

## P0 Issues (Crash/Wrong Behavior — Fix First)

### Scope Analysis (P0)
| Issue | Title | Component | Blocks | Notes |
|-------|-------|-----------|--------|-------|
| #3338 | push @$arrayref does not mark my $arrayref as used | scope_analyzer | #3363, #3364, #3365 | Core sigil bridging bug |
| #3361 | Signature parameters not added as symbols in symbol table | semantic-analyzer | #3362 | Perl 5.36+ signatures |
| #3356 | Package statement not handled in scope analysis traversal | parser | #3358, #3400 | Fundamental scope gap |
| #3378 | ScopeAnalyzer does not bind catch (^B4e) variables in try/catch | scope_analyzer | #3380 | Modern error handling |
| #3363 | scope analyzer still misses code-ref and glob sigil deref forms | scope_analyzer | — | Follow-up to #3338 |
| #3364 | hash slice form %{...} still loses the base ref | scope_analyzer | — | Follow-up to #3338 |
| #3365 | dynamic method deref $obj->${...} is misparsed | parser | — | Follow-up to #3338 |

### Builtins/Special Variables (P0)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3351 | Missing special variables: ${^MATCH}, ${^PREMATCH}, ${^POSTMATCH} | builtins | Perl 5.10+ /p modifier |
| #3354 | Missing regex position arrays: @- and @+ | builtins | LAST_MATCH_START/END |
| #3383 | UNIVERSAL methods not recognized as always-available | semantic-analyzer | can, isa, DOES, VERSION |
| #3386 | AUTOLOAD methods not recognized as valid method definitions | semantic-analyzer | Common pattern |
| #3372 | Missing label resolution validation for loop control | semantic-analyzer | next/last/redo |
| #3373 | Missing label resolution validation for goto | semantic-analyzer | goto LABEL |

### Parser/Lexer (P0)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3366 | Missing: UTF-8 BOM handling in source files | lexer | File encoding |
| #3376 | Lexer: Complex string interpolation not fully handled | lexer | String parsing |

### Diagnostics (P0)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3360 | Perl 5.36+ signatures should auto-enable strict | diagnostics | Feature detection |
| #3375 | Missing version compatibility check for given/when (removed in Perl 5.42) | diagnostics | Deprecation warning |
| #3396 | Detect deprecated smartmatch operator (~~) | diagnostics | PL50X series |
| #3397 | Detect deprecated given/when syntax | diagnostics | PL50X series |

---

## P1 Issues (Feature Gap — Fix Second)

### Version→Feature Mapping (P1)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3344 | Missing version→feature mapping: given/when | version_compat | Perl 5.10+ |
| #3348 | Missing version→feature mapping: isa operator | version_compat | Perl 5.36+ |
| #3349 | Missing version→feature mapping: builtin functions | version_compat | Perl 5.40+ |
| #3350 | Missing version→feature mapping: defer block | version_compat | Perl 5.36+ |
| #3353 | Version-specific feature tracking missing: regex modifiers | version_compat | Perl 5.10, 5.14, 5.26 |

### Builtin Initializers (P1)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3345 | Missing builtin initializers: read, sysread, recv | scope_analyzer | Position-aware handling |
| #3346 | Missing builtin initializer: socketpair | scope_analyzer | Multi-output builtin |
| #3347 | Missing builtin initializers: dbmopen, shmread | scope_analyzer | Position-aware handling |

### Subroutines/Signatures (P1)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3355 | Prototype validation - invalid characters not detected | parser | Subroutine prototypes |
| #3357 | Attribute validation for subroutines | parser | sub :attr |
| #3359 | Signature parameter validation - ordering and rules | parser | Perl 5.36+ signatures |
| #3381 | Attributes after prototype ordering not supported | parser | proto-then-attr |

### OO/Method Resolution (P1)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3384 | SUPER method calls don't navigate to parent class implementations | navigation | #3385 |
| #3385 | mro pragma not tracked - affects method resolution order | semantic-analyzer | — |
| #3408 | Missing Moose method modifiers: override and augment | semantic-analyzer | #3411 |
| #3411 | Add MOP (Meta Object Protocol) support for Moose | semantic-analyzer | — |

### Phasers/Control Flow (P1)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3377 | Scope analyzer does not isolate lexical scopes for phaser blocks | scope_analyzer | — |
| #3374 | Missing unreachable code detection in continue blocks | diagnostics | #3377 |

### Unicode/Encoding (P1)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3367 | Missing: use utf8 pragma tracking in PragmaTracker | pragma | UTF-8 source |
| #3368 | Missing: use encoding pragma tracking in PragmaTracker | pragma | Legacy encoding |
| #3369 | Missing: use feature 'unicode_strings' tracking in PragmaTracker | pragma | Unicode semantics |
| #3370 | Missing: use locale pragma tracking in PragmaTracker | pragma | Locale |
| #3371 | Missing: utf8::encode/utf8::decode function handling | semantic-analyzer | UTF-8 functions |

### POD/Documentation (P1)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3400 | POD entity encoding E<> not decoded in hover text | docs | #3356 |
| #3405 | Add POD coverage lint for exported subroutines without documentation | diagnostics | #3400 |
| #3407 | POD inside subroutine bodies not associated with subroutine hover | docs | — |

---

## P2 Issues (Nice to Have — Fix When Convenient)

### XS/C Integration (P2)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3399 | No C language injection for Inline::C heredocs | parser | #3404 |
| #3401 | No builtin signatures for XSLoader, DynaLoader, or bootstrap | semantic-analyzer | #3404 |
| #3402 | No type inference for FFI::Platypus patterns | semantic-analyzer | #3404 |
| #3403 | No enhanced folding for Inline::C blocks | folding | #3399 |
| #3404 | No XS typemap syntax support in grammar | parser | — |

### Import/Export Systems (P2)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3409 | Exporter 'import' pattern not analyzed for symbol resolution | semantic-analyzer | #3416 |
| #3413 | Sub::Exporter support missing | semantic-analyzer | #3416 |
| #3414 | Import::Into support missing | semantic-analyzer | #3416 |
| #3415 | Module::Runtime (use_module, require_module) support missing | semantic-analyzer | #3416 |
| #3416 | Cross-module export symbol table not built | semantic-analyzer | — |

### OO Frameworks (P2)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3410 | Add Class::Tiny framework support | semantic-analyzer | #3411 |

### Constants (P2)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3391 | use constant hash-ref syntax not fully extracted | semantic-analyzer | — |
| #3392 | Const::Fast read-only variables not tracked | semantic-analyzer | — |
| #3393 | Readonly module patterns not supported | semantic-analyzer | — |
| #3394 | Compile-time constant folding not implemented | semantic-analyzer | — |
| #3395 | Feature 'const_attr' not supported | semantic-analyzer | Perl 5.22+ |

### Attributes (P2)
| Issue | Title | Component | Notes |
|-------|-------|-----------|-------|
| #3387 | Missing :locked attribute documentation and support | documentation | — |
| #3388 | No support for Attribute::Handlers custom attribute patterns | semantic-analyzer | — |

### Error Handling/Signals (P2)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3379 | Missing lint for global %SIG{__DIE__} and %SIG{__WARN__} handler assignment | diagnostics | — |
| #3380 | Missing diagnostics for $@ / $EVAL_ERROR flow after eval and try | diagnostics | #3378 |

### Documentation/POD (P2)
| Issue | Title | Component | Depends On |
|-------|-------|-----------|------------|
| #3406 | POD L<> links should be clickable and support goto-definition | navigation | #3407 |

---

## Infrastructure/Test Quality Issues

These are from the comprehensive test quality audit (#3237) and infrastructure improvements:

| Issue | Title | Category | Priority |
|-------|-------|----------|----------|
| #3258 | Replace panic! in match-arm catches with assert_matches | Test Quality | P1 |
| #3261 | Add perl-tdd-support dep to crates with test unwrap | Test Quality | P1 |
| #3262 | Link or resolve unlinked TODOs in test code | Test Quality | P2 |
| #3263 | Audit println and eprintln in tests | Test Quality | P2 |
| #3259 | Audit no-assertion test functions | Test Quality | P2 |
| #3260 | Remove hardcoded absolute paths in test fixtures | Test Quality | P2 |
| #3250 | Archive dead tree-sitter harness crates | Infrastructure | P1 |
| #3233 | Per-crate README completeness audit | Documentation | P2 |
| #3231 | Cargo-semver-checks findings vs v0.12.1 baseline | Infrastructure | P1 |
| #3227 | Docs directory audit (21 drift findings) | Documentation | P2 |
| #3220 | 1527 missing-docs warnings across 10 publishable crates | Documentation | P2 |

---

## Dependency Graph (Mermaid Format)

```mermaid
flowchart TD
    subgraph Core_Scope["Core Scope Analysis"]
        A[#3338: Sigil bridging bug] --> B[#3363: Code-ref deref]
        A --> C[#3364: Hash slice deref]
        A --> D[#3365: Method deref]
        E[#3356: Package statement] --> F[#3358: Package-qualified calls]
        E --> G[#3400: POD encoding]
        H[#3377: Phaser scope isolation] --> I[#3372: Loop label resolution]
        H --> J[#3373: Goto label resolution]
        H --> K[#3374: Unreachable continue blocks]
    end

    subgraph Signatures["Signatures (Perl 5.36+)"]
        L[#3359: Signature validation] --> M[#3360: Auto-enable strict]
        L --> N[#3355: Prototype validation]
        L --> O[#3357: Attribute validation]
        L --> P[#3381: Attr-after-proto]
        Q[#3361: Signature symbols] --> R[#3362: Go-to-declaration]
    end

    subgraph Regex["Regex/Special Variables"]
        S[#3351: ${^MATCH} etc] --> T[#3352: Capture context tracking]
        S --> U[#3354: @- and @+]
        S --> V[#3353: Regex modifiers]
    end

    subgraph OO_Methods["OO/Method Resolution"]
        W[#3385: mro pragma] --> X[#3384: SUPER navigation]
        Y[#3383: UNIVERSAL methods]
        Z[#3386: AUTOLOAD]
    end

    subgraph Version_Feature["Version→Feature Mapping"]
        AA[#3344: given/when] --> AB[#3397: given/when deprecation]
        AA --> AC[#3375: given/when removed 5.42]
        AD[#3348: isa operator]
        AE[#3349: builtin functions]
        AF[#3350: defer block]
    end

    subgraph Builtins["Builtin Initializers"]
        AG[#3345: read/sysread/recv]
        AH[#3346: socketpair]
        AI[#3347: dbmopen/shmread]
    end

    subgraph Error_Handling["Error Handling"]
        AJ[#3378: Catch variable binding] --> AK[#3380: $@ diagnostics]
    end

    subgraph XS_C["XS/C Integration"]
        AL[#3404: XS typemap] --> AM[#3399: Inline::C injection]
        AL --> AN[#3401: XSLoader signatures]
        AL --> AO[#3402: FFI::Platypus]
        AM --> AP[#3403: Inline::C folding]
    end

    subgraph Import_Export["Import/Export Systems"]
        AQ[#3416: Cross-module symbol table] --> AR[#3409: Exporter]
        AQ --> AS[#3413: Sub::Exporter]
        AQ --> AT[#3414: Import::Into]
        AQ --> AU[#3415: Module::Runtime]
    end

    subgraph Moose_MOP["Moose/MOP"]
        AV[#3411: MOP support] --> AW[#3408: Moose modifiers]
        AV --> AX[#3410: Class::Tiny]
    end

    subgraph Test_Quality["Test Quality (from #3237)"]
        AY[#3258: panic! → assert_matches]
        AZ[#3261: Add perl-tdd-support] --> BA[#3259: No-assertion tests]
        AZ --> BB[#3263: println audit]
        AY --> BC[#3262: Link TODOs]
    end
```

---

## Recommended Execution Order

### Phase 1: Foundation (Weeks 1-2)
**Goal:** Fix core scope analysis bugs that cause false positives

1. **#3338** — push @$arrayref false positive (Small, unblocks 3 follow-ups)
2. **#3356** — Package statement scope handling (Medium, unblocks strict-subs)
3. **#3361** — Signature parameters in symbol table (Small, Perl 5.36+ support)
4. **#3351** — Missing special variables ${^MATCH} etc (Small, Perl 5.10+ features)
5. **#3378** — Catch variable binding in try/catch (Small, modern error handling)

### Phase 2: Builtins & Version Features (Weeks 3-4)
**Goal:** Implement BUILTIN_INITIALIZERS registry and version→feature mapping

6. **#3345, #3346, #3347** — Position-aware builtin initializers (Small-Medium)
7. **#3344, #3348, #3349, #3350** — Version→feature mappings (Small)
8. **#3359** — Signature parameter validation (Medium, unblocks related features)
9. **#3354** — Regex position arrays @- and @+ (Small)

### Phase 3: OO & Method Resolution (Weeks 5-6)
**Goal:** Complete OO method navigation

10. **#3383** — UNIVERSAL methods recognition (Small)
11. **#3385** — mro pragma tracking (Medium, unblocks SUPER navigation)
12. **#3384** — SUPER method call navigation (Medium)
13. **#3386** — AUTOLOAD method recognition (Small)

### Phase 4: Diagnostics & Polish (Weeks 7-8)
**Goal:** Add deprecation warnings and improve diagnostics

14. **#3396, #3397** — Deprecated smartmatch and given/when (Small)
15. **#3375** — given/when removed in Perl 5.42 warning (Small)
16. **#3367-#3371** — UTF-8/encoding pragma tracking (Small)
17. **#3400, #3405-#3407** — POD improvements (Small-Medium)

### Phase 5: Infrastructure & Test Quality (Ongoing)
**Goal:** Clean up technical debt

18. **#3250** — Archive dead tree-sitter crates (Medium)
19. **#3258, #3261** — Test quality improvements (Medium)
20. **#3231, #3233, #3227** — Documentation and semver audits (Medium)

### Phase 6: Advanced Features (Post-v0.13.0)
**Goal:** XS/C integration and advanced OO frameworks

21. **#3404, #3399, #3401-#3403** — XS/C support (Medium-Large)
22. **#3411, #3408, #3410** — MOP/Moose support (Large)
23. **#3416, #3409, #3413-#3415** — Import/Export systems (Large)

---

## Estimated Effort by Category

| Category | Issues | Estimated Effort | Key Dependencies |
|----------|--------|-------------------|------------------|
| Scope Analysis (sigil bridging) | 4 | 3-4 days | #3338 is prerequisite |
| Scope Analysis (package/phasers) | 4 | 4-5 days | #3356, #3377 are foundational |
| Builtins/Special Variables | 5 | 3-4 days | BUILTIN_INITIALIZERS registry |
| Version→Feature Mapping | 5 | 2-3 days | EffectiveSemantics layer |
| Signatures/Subroutines | 5 | 4-5 days | #3359 unblocks others |
| OO/Method Resolution | 4 | 4-5 days | #3385 required for SUPER |
| Unicode/Encoding | 5 | 2-3 days | PragmaTracker additions |
| Diagnostics/Deprecations | 4 | 2-3 days | — |
| POD/Documentation | 4 | 2-3 days | #3400 unblocks coverage |
| Test Quality | 6 | 5-7 days | #3258, #3261 are prerequisites |
| Infrastructure | 5 | 4-5 days | Archive, semver, docs |
| XS/C Integration | 5 | 5-7 days | Grammar changes needed |
| Import/Export | 5 | 5-7 days | #3416 is foundational |
| Constants/Attributes | 5 | 3-4 days | — |
| **Total** | **69** | **~46-60 days** | Parallel work possible |

---

## Notes on Architecture

### BUILTIN_INITIALIZERS PHF Registry
The recommended approach for builtins that auto-declare variables:

```rust
static BUILTIN_INITIALIZERS: phf::Map<&'static str, &'static [usize]> = phf_map! {
    // Position 0 initializers (handle/socket)
    "open" => &[0],
    "opendir" => &[0],
    "sysopen" => &[0],
    "pipe" => &[0, 1],
    "socket" => &[0],
    "socketpair" => &[0, 1],
    "accept" => &[0],
    // Position 1 initializers (scalar buffer)
    "read" => &[1],
    "sysread" => &[1],
    "recv" => &[1],
    // Position 0 initializers (hash/buffer)
    "dbmopen" => &[0],
    "shmread" => &[1],
};
```

### EffectiveSemantics Layer
A unified layer for tracking:
- Perl version requirements (`use v5.36;`)
- Feature pragmas (`use feature 'signatures';`)
- Regular pragmas (`use strict; use warnings;`)
- Regex modifiers (`/p`, `/a`, `/u`, `/l`, `/d`)

This resolves the scattered version→feature mapping issues (#3344, #3348, #3349, #3350, #3353, #3375).

### Parser deref_base Annotation
For sigil bridging (#3338 and follow-ups), the parser should annotate dereference operations with the base variable reference so the scope analyzer can properly track usage across sigil buckets ($ → @, $ → %, etc.).

---

## Completion Unlocks

| When These Are Done | These Become Possible |
|--------------------|----------------------|
| #3338, #3363-#3365 | Complete sigil bridging; no more false positives on dereferencing |
| #3356 | strict-subs for package-qualified calls; POD encoding fixes |
| #3361 | Full signature support; go-to-declaration for parameters |
| #3359 | Prototype and attribute validation; signature strict mode |
| #3385 | Complete method resolution with C3/MRO; SUPER navigation |
| #3411 | Moose method modifiers; Class::Tiny support |
| #3416 | Complete import/export analysis; cross-module symbol resolution |
| #3250 | Clean workspace without dead tree-sitter crates |
| #3231 | Semver compliance for all published crates |

---

*Generated by subagent analysis. This plan should be reviewed and adjusted based on changing priorities and new findings.*
