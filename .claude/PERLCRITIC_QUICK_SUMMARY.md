# Perlcritic Integration Quick Summary

## What's Already Built

| Component | Status | Location |
|-----------|--------|----------|
| Output parser | ✅ Complete | `crates/perl-lsp-critic-parser/` |
| Analyzer (external subprocess) | ✅ Complete | `crates/perl-lsp-tooling/src/perl_critic/analyzer.rs` |
| Built-in analyzer | ✅ Complete | `crates/perl-lsp-tooling/src/perl_critic/built_in.rs` |
| Config types | ✅ Complete | `crates/perl-lsp-tooling/src/perl_critic/types.rs` |
| Quick fixes (partial) | ✅ Partial | `crates/perl-lsp-tooling/src/perl_critic/quick_fix.rs` |
| **Execute command** | ✅ Works | `crates/perl-lsp/src/execute_command/provider.rs:120-221` |
| **Diagnostics (built-in only)** | ✅ Works | `crates/perl-lsp/src/runtime/diagnostics.rs:63-90` |

## What's Missing

| Feature | Impact | Effort |
|---------|--------|--------|
| **External perlcritic in diagnostics** | HIGH (main gap) | 1-2 sprints |
| Runtime config (enable/disable) | MEDIUM | 0.5 sprints |
| `.perlcriticrc` discovery | MEDIUM | 0.5 sprints |
| Debouncing + async | LOW (optimization) | 1 sprint |
| Quick fix code actions | MEDIUM | 1 sprint |

## The One Thing That's Missing

**External perlcritic violations don't appear in the editor diagnostics.**

Current flow:
```
parse_diagnostics() → built-in analyzer violations → show to user
```

Missing flow:
```
parse_diagnostics() → external perlcritic violations → show to user
```

The infrastructure exists; it's just not wired in.

## Builder Task: Wire External Perlcritic (Phase 1)

**Goal**: Make this work:
1. User enables `perl.critic.enabled: true` in settings
2. Opens a Perl file
3. Sees perlcritic violations in diagnostics (red squiggles)
4. Hovers for message + policy

**Changes Required**:
1. Add toggle config to `LspServer`
2. In `publish_diagnostics()` (after line 90), call `CriticAnalyzer::analyze_file()`
3. Convert violations to internal `Diagnostic` structs
4. Add to diagnostics vector before publishing
5. Tests: mock subprocess responses

**Files to Touch**:
- `crates/perl-lsp/src/runtime/diagnostics.rs` (main change)
- `crates/perl-lsp/src/runtime/config.rs` (add settings)
- Tests

**Effort**: 8-13 story points (1-2 sprints)

## Code Pattern to Copy

Existing: Built-in violations (lines 66-90 in diagnostics.rs)
```rust
let violations = built_in_analyzer.analyze(ast, &doc.text);
for violation in violations {
    let internal_severity = /* map severity */;
    diagnostics.push(InternalDiagnostic {
        range: (violation.range.start.byte, violation.range.end.byte),
        severity: internal_severity,
        code: Some(violation.policy),
        message: violation.description,
        /* ... */
    });
}
```

Your job: Do the same thing but with `CriticAnalyzer::analyze_file()` instead.

## Design Decision: Sync or Async?

**Recommendation**: Sync for Phase 1
- Typical file: 50-100ms analysis time
- Pull diagnostics model can handle this
- Simpler implementation (fewer threads/channels)
- Add async + debounce in Phase 3 if needed

## Graceful Degradation

Must handle:
- ✅ perlcritic not installed → show only built-in
- ✅ perlcritic hangs → don't block LSP, show built-in
- ✅ Bad config → use defaults
- ✅ Analysis fails → silently skip external, keep built-in

Pattern: Try external, catch Err, proceed to built-in.

## Key Files to Read First

1. `crates/perl-lsp-tooling/src/perl_critic/analyzer.rs` (CriticAnalyzer API)
2. `crates/perl-lsp/src/runtime/diagnostics.rs:54-90` (where to add code)
3. `crates/perl-lsp-tooling/src/perl_critic/types.rs` (Violation struct)
