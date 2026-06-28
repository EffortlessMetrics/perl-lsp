# Acceptance Criteria for #1664: state variable without initializer misreport fix

## §Behavior

| Input | Condition | Expected Result |
|-------|-----------|-----------------|
| `state $x;` in a sub | Variable declared but no explicit initializer | No UninitializedVariable warning when $x is used |
| `state $x; print $x;` | state var used after declaration (no initializer) | No UninitializedVariable warning (value is undef, which is defined) |
| `my $y;` in a sub | Variable declared but no explicit initializer | UninitializedVariable warning when $y is used (true uninitialized) |
| `state $x = 42;` | state var with explicit initializer | No UninitializedVariable warning (already initialized) |
| `state $x; $x = 42;` | state var assigned before use | No UninitializedVariable warning |

## §Hazards

| Hazard Class | Surface | Risk | Mitigation |
|--------------|---------|------|-----------|
| ANALYZER-1: Scope state corruption on declarator check | `declarations.rs:29` initialization flag | If initialized check breaks for other declarators (my, our, local), scope analysis fails for all declarations | Verify is_initialized behavior unchanged for my/our/local via regression tests |
| ANALYZER-2: Off-by-one or dropped initialization marker | `declarations.rs:29` boolean assignment | If the fix is incomplete (e.g., only checks `state` in one branch), state vars may still warn in some contexts | Verify both initial declaration AND use paths; test nested scopes, multiple state vars |
| ANALYZER-3: Uninitialized check path consumes stale data | `mod.rs:1062-1069` use phase | If use-path ignores is_initialized flag or uses wrong scope entry, fix doesn't propagate to warnings | Test that ScopeAnalyzer's use_variable_parts_in_context correctly retrieves is_initialized from scope |
| ANALYZER-4: State behavior regression across block scopes | `declarations.rs` + `mod.rs` scope entry creation | If state vars are reinitialized on each scope entry (e.g., nested blocks), fix breaks state semantics | Verify state persists across function calls; test nested blocks do NOT reset state |
| ANALYZER-5: My/Our/Local declarators regress | `declarations.rs:29` all branches | If the fix inadvertently changes is_initialized for other declarators, scope analysis breaks for them | Regression tests: my without init SHOULD warn, our should not warn (package global), local special vars skipped |
| ANALYZER-6: Edge case: state in different package scopes | Multiple scopes in same file | state vars in different packages should be independent; ensure no cross-package corruption | Test: two subs in different packages with `state $x;` should be independent |

## §Contracts

| Contract | Touched | Requirement | How Verified |
|----------|---------|-------------|--------------|
| **PARSER_CONTRACTS.md: VariableDeclaration node structure** | No | Parser must emit NodeKind::VariableDeclaration with declarator="state" and initializer=None for bare state vars | Already verified: parser correctly provides declarator and initializer to handler |
| **Scope analyzer: is_initialized flag semantics** | Yes | is_initialized must be set to true for state vars even without explicit initializer; this flag feeds use-phase warning decision | Test accepts this contract by checking that scope entry has is_initialized=true after state declaration |
| **LSP protocol: DiagnosticKind::UninitializedVariable must not appear for state** | No | Protocol unchanged; only internal issue generation changes | No protocol change required |
| **Semantic analyzer public API** | No | No new types, functions, or ID spaces | No API surface change |

## §API-Shape

**New API elements**: None

**Modified elements**: 
- `is_initialized` logic in `declarations.rs:29` (internal implementation detail, no public change)

**ID-space changes**: None

**Dup-risk grep**: Search for other uses of `is_initialized` assignment:
- `crates/perl-semantic-analyzer/src/analysis/scope_analyzer/declarations.rs:29` (TARGET)
- `crates/perl-semantic-analyzer/src/analysis/scope_analyzer/mod.rs` — uses is_initialized but does not assign (read-only use-phase)

**Caller count**: Internal to scope analysis; no external callers affected

## §Test-Grid

### Positive Cases (should NOT warn)

| Test Name | Input Code | Assertion | Invariant |
|-----------|-----------|-----------|-----------|
| `state_without_initializer_no_warning` | `state $x; print $x;` | No UninitializedVariable issue for $x | state is marked initialized at declaration time |
| `state_with_explicit_initializer_no_warning` | `state $x = 42; print $x;` | No UninitializedVariable issue for $x | state with initializer remains unchanged |
| `state_in_nested_scope_no_warning` | `sub f { { state $x; print $x; } }` | No UninitializedVariable issue for $x in nested block | state persists across nested scopes |
| `multiple_state_vars_independent` | `sub f { state $x; state $y; print $x, $y; }` | No UninitializedVariable for either $x or $y | each state var independently marked initialized |

### Negative Cases (SHOULD warn — regression checks)

| Test Name | Input Code | Assertion | Invariant |
|-----------|-----------|-----------|-----------|
| `my_without_initializer_warns` | `my $y; print $y;` | UninitializedVariable issue for $y | my declarator unchanged (still uninitialized without init) |
| `my_in_strict_warns` | `use strict; my $y; print $y;` | UninitializedVariable issue for $y | my behavior unaffected by state fix |
| `local_without_initializer_warns` | `local $z; print $z;` | UninitializedVariable issue for $z | local treated as uninitialized (standard behavior) |

### Adversarial Cases

| Test Name | Input Code | Assertion | Invariant |
|-----------|-----------|-----------|-----------|
| `state_multiple_calls_persists` | `sub f { state $x; $x++; } f(); f(); assert $x == 2` | No uninitialized warning on either call | state persistence not broken by initialization flag |
| `state_in_loop_persists` | `for (1..3) { state $x; $x++; } assert $x == 3` | No uninitialized warning | state persists across loop iterations |
| `state_and_my_same_name` | `sub f { state $x; my $x; print $x; }` | my shadows state; my still warns if uninitialized | shadowing logic unaffected |

### State-Transition Cases

| Test Name | Input Code | Assertion | Invariant |
|-----------|-----------|-----------|-----------|
| `state_declared_unused` | `state $x;` (declared but never used) | UnusedVariable warning (not eliminated by initialization fix) | initialization != usage tracking |
| `state_declared_initialized_unused` | `state $x = 1;` (declared, initialized, never used) | UnusedVariable warning | same as above |
| `state_assigned_after_declaration` | `state $x; $x = 42; print $x;` | No uninitialized warning (assignment counts as initialization) | declaration-time initialization correct; assignment handling unchanged |

## §Blast-Radius

**Consumers of scope_analyzer**:
- `crates/perl-lsp-rs/src/handlers/code_lens.rs` — uses scope analysis for diagnostics (will now correctly NOT report uninitialized state)
- `crates/perl-lsp-rs/src/handlers/text_document.rs` — publishes diagnostics (fewer UninitializedVariable issues for state vars)
- `crates/perl-semantic-analyzer/src/analysis/` modules — internal uses of ScopeAnalyzer

**Downstream effects**:
- LSP clients will see fewer false UninitializedVariable diagnostics for state variables (user-visible improvement, not regression)
- No API breaking changes
- No new error types or protocol changes

**Must-not-touch boundaries**:
- Parser layer (NodeKind, declarator values) — read-only from scope analyzer
- Public API of perl-semantic-analyzer — only internal is_initialized logic changes
- Type inference or other analysis modules — scope analysis is independent
