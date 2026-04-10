# Perl-LSP PR Follow-Up: Completion Checklist

## PR Merge Status

| PR | Title | State | Merge Commit | Merged At |
|----|-------|-------|--------------|-----------|
| [#3340](https://github.com/EffortlessMetrics/perl-lsp/pull/3340) | fix(analysis): align pragma and scope diagnostics (#3339) | **MERGED** | e0120bf61fc1aecaf1ce01ecb3141f5bc78262e2 | 2026-04-10T00:51:55Z |
| [#3341](https://github.com/EffortlessMetrics/perl-lsp/pull/3341) | docs(config): clarify includePaths semantics (#3337) | **MERGED** | f1323f123c8610f21f68ba979654a5325a8d77a7 | 2026-04-10T00:51:47Z |

## PR #3340 Summary

**Scope:** Centralizes lexical `use VERSION` semantics in `perl-pragma` and reuses them across strict/warnings checks and version compatibility.

**Issues Closed:**
- #3339 - use v5.40; incorrectly triggers "add use strict; use warnings;" suggestions
- #3336 - open my $fh, ... triggers false "undefined variable" diagnostic (partial - follow-ups remain)
- #3338 - push @$arrayref does not mark my $arrayref as used (partial - follow-ups remain)

**Validation Commands (from PR):**
```bash
cargo fmt --all
cargo test -p perl-pragma
cargo test -p perl-semantic-analyzer --test scope_and_symbol_tests -- --nocapture
cargo test -p perl-lsp-diagnostics --test version_compat_tests --test diagnostic_integration_test -- --nocapture
cargo test -p perl-lsp-diagnostics --lib -- --nocapture
just pr-fast
```

## PR #3341 Summary

**Scope:** Clarifies the current `includePaths` contract instead of broadening resolver behavior implicitly.

**Issues Closed:**
- #3337 - Config/docs mismatch: includePaths workspace-relative vs 'appended to @INC'
- #3343 - Docs: includePaths description in CONFIG.md contradicts implementation

## Related Open Issues (Follow-Up Work)

### Direct Follow-Ups to PR #3340 (pragma/scope alignment)

| Issue | Title | Status | Relation to PR #3340 |
|-------|-------|--------|---------------------|
| [#3363](https://github.com/EffortlessMetrics/perl-lsp/issues/3363) | scope analyzer still misses code-ref and glob sigil deref forms | OPEN | Continuation of #3338 (sigil deref normalization) |
| [#3364](https://github.com/EffortlessMetrics/perl-lsp/issues/3364) | hash slice form %{...} still loses the base ref | OPEN | Related to #3338 (deref tracking) |
| [#3365](https://github.com/EffortlessMetrics/perl-lsp/issues/3365) | dynamic method deref $obj->${...} is misparsed | OPEN | Related to scope analysis gaps |
| [#3345](https://github.com/EffortlessMetrics/perl-lsp/issues/3345) | Missing builtin initializers: read, sysread, recv | OPEN | Continuation of #3336 (builtin declaration tracking) |
| [#3346](https://github.com/EffortlessMetrics/perl-lsp/issues/3346) | Missing builtin initializer: socketpair | OPEN | Continuation of #3336 |
| [#3347](https://github.com/EffortlessMetrics/perl-lsp/issues/3347) | Missing builtin initializers: dbmopen, shmread | OPEN | Continuation of #3336 |

### Pragma Tracking Gaps (related to #3339)

| Issue | Title | Status | Notes |
|-------|-------|--------|-------|
| [#3353](https://github.com/EffortlessMetrics/perl-lsp/issues/3353) | [Pragma] Version-specific feature tracking missing: v5.10, v5.14, v5.26 regex modifiers | OPEN | Extends version→pragma mapping |
| [#3367](https://github.com/EffortlessMetrics/perl-lsp/issues/3367) | Missing: use utf8 pragma tracking in PragmaTracker | OPEN | Enhancement |
| [#3368](https://github.com/EffortlessMetrics/perl-lsp/issues/3368) | Missing: use encoding pragma tracking in PragmaTracker | OPEN | Enhancement |
| [#3369](https://github.com/EffortlessMetrics/perl-lsp/issues/3369) | Missing: use feature 'unicode_strings' tracking | OPEN | Enhancement |
| [#3370](https://github.com/EffortlessMetrics/perl-lsp/issues/3370) | Missing: use locale pragma tracking in PragmaTracker | OPEN | Enhancement |
| [#3398](https://github.com/EffortlessMetrics/perl-lsp/issues/3398) | feat(pragma): Track 'use feature' pragma including 'switch' | OPEN | PL90X feature |
| [#3385](https://github.com/EffortlessMetrics/perl-lsp/issues/3385) | mro pragma not tracked - affects method resolution order | OPEN | Enhancement |

### Scope Analysis Gaps (related to #3340 work)

| Issue | Title | Status | Notes |
|-------|-------|--------|-------|
| [#3351](https://github.com/EffortlessMetrics/perl-lsp/issues/3351) | [Scope] Missing special variables: ${^MATCH}, ${^PREMATCH}, ${^POSTMATCH} | OPEN | 5.10+ /p modifier |
| [#3352](https://github.com/EffortlessMetrics/perl-lsp/issues/3352) | [Analysis] Capture variables ($1, $2, etc.) should track regex match context | OPEN | Enhancement |
| [#3354](https://github.com/EffortlessMetrics/perl-lsp/issues/3354) | [Scope] Missing regex position arrays: @- and @+ | OPEN | LAST_MATCH_START/END |
| [#3356](https://github.com/EffortlessMetrics/perl-lsp/issues/3356) | scope-analyzer: Package statement not handled in scope analysis traversal | OPEN | Bug |
| [#3358](https://github.com/EffortlessMetrics/perl-lsp/issues/3358) | strict-subs: Package-qualified function calls (Foo::bar()) not validated | OPEN | Bug |
| [#3360](https://github.com/EffortlessMetrics/perl-lsp/issues/3360) | [diagnostics] Perl 5.36+ signatures should auto-enable strict | OPEN | Related to version→pragma |
| [#3361](https://github.com/EffortlessMetrics/perl-lsp/issues/3361) | [semantic-analyzer] Signature parameters not added as symbols | OPEN | Bug |
| [#3362](https://github.com/EffortlessMetrics/perl-lsp/issues/3362) | [navigation] Go-to-declaration skips signature parameters | OPEN | Related to #3361 |
| [#3377](https://github.com/EffortlessMetrics/perl-lsp/issues/3377) | Scope analyzer does not isolate lexical scopes for BEGIN/CHECK/INIT/END/UNITCHECK blocks | OPEN | Bug |
| [#3378](https://github.com/EffortlessMetrics/perl-lsp/issues/3378) | ScopeAnalyzer does not bind catch (^B4e) variables in try/catch | OPEN | Bug |
| [#3379](https://github.com/EffortlessMetrics/perl-lsp/issues/3379) | Missing lint for global %SIG{__DIE__} and %SIG{__WARN__} handler assignment | OPEN | Enhancement |
| [#3380](https://github.com/EffortlessMetrics/perl-lsp/issues/3380) | Missing diagnostics for $@ / $EVAL_ERROR flow after eval and try | OPEN | Enhancement |

## Completion Checklist

### What's Done (via PR #3340 & #3341)

- [x] Centralize lexical `use VERSION` semantics in `perl-pragma`
- [x] Reuse version/pragma semantics across strict/warnings checks
- [x] Split scope analysis for undeclared variables vs barewords (strict vars/subs)
- [x] Normalize dereference/container use tracking for `@$arrayref`, `@{$arrayref}`, `$hashref->{k}`
- [x] Treat declaration-capable builtin handle arguments as consumed/initialized (`open my $fh, ...`)
- [x] Fix `use v5.40;` incorrectly triggering strict/warnings suggestions (#3339)
- [x] Clarify `includePaths` as workspace-relative search roots (#3341)
- [x] Make `useSystemInc` explicitly separate, opt-in system lookup path (#3341)
- [x] Tighten resolver/config comments and tests around workspace-bounded behavior (#3341)

### What's Pending (Follow-Up Issues)

**High Priority (Core functionality gaps):**

- [ ] Fix code-ref and glob sigil deref forms (`&$cb()`, `*$gref`) - [#3363](https://github.com/EffortlessMetrics/perl-lsp/issues/3363)
- [ ] Fix hash slice form losing base ref (`%$href{'a', 'b'}`) - [#3364](https://github.com/EffortlessMetrics/perl-lsp/issues/3364)
- [ ] Add position-aware builtin initializers (read, sysread, recv at position 1) - [#3345](https://github.com/EffortlessMetrics/perl-lsp/issues/3345)
- [ ] Add socketpair builtin initializer - [#3346](https://github.com/EffortlessMetrics/perl-lsp/issues/3346)
- [ ] Add dbmopen and shmread builtin initializers - [#3347](https://github.com/EffortlessMetrics/perl-lsp/issues/3347)
- [ ] Track version-specific features (given/when, isa, defer, builtin, etc.) - [#3353](https://github.com/EffortlessMetrics/perl-lsp/issues/3353)
- [ ] Add signature parameters as symbols in symbol table - [#3361](https://github.com/EffortlessMetrics/perl-lsp/issues/3361)
- [ ] Handle Package statement in scope analysis traversal - [#3356](https://github.com/EffortlessMetrics/perl-lsp/issues/3356)

**Medium Priority (Enhancement/Completeness):**

- [ ] Add utf8 pragma tracking - [#3367](https://github.com/EffortlessMetrics/perl-lsp/issues/3367)
- [ ] Add encoding pragma tracking - [#3368](https://github.com/EffortlessMetrics/perl-lsp/issues/3368)
- [ ] Add unicode_strings feature tracking - [#3369](https://github.com/EffortlessMetrics/perl-lsp/issues/3369)
- [ ] Add locale pragma tracking - [#3370](https://github.com/EffortlessMetrics/perl-lsp/issues/3370)
- [ ] Add 'use feature' tracking (switch, etc.) - [#3398](https://github.com/EffortlessMetrics/perl-lsp/issues/3398)
- [ ] Add mro pragma tracking - [#3385](https://github.com/EffortlessMetrics/perl-lsp/issues/3385)
- [ ] Add ${^MATCH}, ${^PREMATCH}, ${^POSTMATCH} special variables - [#3351](https://github.com/EffortlessMetrics/perl-lsp/issues/3351)
- [ ] Add regex position arrays @- and @+ - [#3354](https://github.com/EffortlessMetrics/perl-lsp/issues/3354)
- [ ] Track capture variables ($1, $2) with regex match context - [#3352](https://github.com/EffortlessMetrics/perl-lsp/issues/3352)
- [ ] Isolate lexical scopes for BEGIN/CHECK/INIT/END/UNITCHECK blocks - [#3377](https://github.com/EffortlessMetrics/perl-lsp/issues/3377)
- [ ] Bind catch variables in try/catch - [#3378](https://github.com/EffortlessMetrics/perl-lsp/issues/3378)

**Lower Priority (Nice to have):**

- [ ] Validate package-qualified function calls under strict-subs - [#3358](https://github.com/EffortlessMetrics/perl-lsp/issues/3358)
- [ ] Auto-enable strict for Perl 5.36+ signatures - [#3360](https://github.com/EffortlessMetrics/perl-lsp/issues/3360)
- [ ] Navigate to signature parameters (go-to-declaration) - [#3362](https://github.com/EffortlessMetrics/perl-lsp/issues/3362)
- [ ] Fix dynamic method deref misparsing - [#3365](https://github.com/EffortlessMetrics/perl-lsp/issues/3365)
- [ ] Add lint for global %SIG{__DIE__} and %SIG{__WARN__} - [#3379](https://github.com/EffortlessMetrics/perl-lsp/issues/3379)
- [ ] Add diagnostics for $@ flow after eval/try - [#3380](https://github.com/EffortlessMetrics/perl-lsp/issues/3380)

## Recommended Next Steps

1. **Immediate (this week):** Address the 4 builtin initializer issues ([#3345](https://github.com/EffortlessMetrics/perl-lsp/issues/3345), [#3346](https://github.com/EffortlessMetrics/perl-lsp/issues/3346), [#3347](https://github.com/EffortlessMetrics/perl-lsp/issues/3347)) - these are small, mechanical additions following the pattern established in #3340.

2. **Short-term (next sprint):** 
   - Fix the remaining sigil deref gaps ([#3363](https://github.com/EffortlessMetrics/perl-lsp/issues/3363), [#3364](https://github.com/EffortlessMetrics/perl-lsp/issues/3364))
   - Add signature parameter tracking ([#3361](https://github.com/EffortlessMetrics/perl-lsp/issues/3361)) - this is user-visible for modern Perl
   - Handle Package statement scope ([#3356](https://github.com/EffortlessMetrics/perl-lsp/issues/3356))

3. **Medium-term:**
   - Implement version-specific feature tracking ([#3353](https://github.com/EffortlessMetrics/perl-lsp/issues/3353)) - larger effort, affects multiple version-gated constructs
   - Add pragma tracking for utf8, encoding, locale ([#3367](https://github.com/EffortlessMetrics/perl-lsp/issues/3367), [#3368](https://github.com/EffortlessMetrics/perl-lsp/issues/3368), [#3370](https://github.com/EffortlessMetrics/perl-lsp/issues/3370))

## Risk Assessment

| Risk | Level | Mitigation |
|------|-------|------------|
| #3340 follow-ups remain open | Medium | Prioritize builtin initializers (#3345-3347) as they complete the partial work |
| Version→feature mapping incomplete | Medium | Document known gaps; user impact is version-gated syntax linting only |
| Scope analysis edge cases accumulating | Medium | Consider a "scope analysis hardening" sprint after core gaps fixed |

---
*Generated: 2026-04-09*  
*Source: EffortlessMetrics/perl-lsp PRs #3340, #3341 and related issues*
