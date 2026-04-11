# Issue #3539 Re-Scope: Coroutine Support Must Be Split Into Core-Syntax Tracking vs CPAN API UX

**Original Issue Title:** "Add LSP support for Perl coroutines"

**Re-Scope Title:** "Investigate coroutine support scope: core syntax status vs CPAN coroutine APIs"

**Issue Number:** #3539

---

## Decision Summary

Do **not** implement core-language `coro sub` / `yield` parser support as part of #3539 right now.

The issue should be split into:

1. **Core syntax status tracking** (deferred until upstream Perl documents a released syntax contract)
2. **CPAN coroutine UX support** (deliver user-facing value now without grammar risk)

---

## Evidence Snapshot (as of 2026-04-11)

### Core Perl status does not provide a stable parser target

- The official experimental feature list for Perl 5.42.2 does not list coroutines.
- The Perl core tracker still has a coroutines request open (`[feature] Coroutines in Perl`, opened 2020).

Implication: there is no authoritative released core syntax contract to implement in parser/AST today.

### CPAN already exposes coroutine-style APIs with different surface area

- `Coro` APIs center around library constructs such as `async { ... }`, `schedule`, `cede`, and object methods.
- This is not equivalent to introducing parser keywords like `coro` and `yield`.

Implication: immediate IDE value should target library-aware completion/hover support, not grammar changes.

---

## Scope Correction

The original issue mixes three separate concerns that should not be implemented as one task:

1. **Hypothetical core syntax work** (`coro` keyword, `yield`, AST semantics)
2. **Version/status claims** about experimental support that are inconsistent
3. **CPAN library ergonomics** (`Coro` API discoverability and method help)

These need separate acceptance criteria, risk profiles, and implementation paths.

---

## Recommended Successor Issues

### 1) Core coroutine syntax tracker (deferred)

**Question:** Is there a documented, released, core Perl coroutine syntax that perl-lsp should parse?

**Definition of ready:**

- Upstream perldoc or equivalent official docs define syntax and feature gating.
- Version gates and warning/experimental behavior are explicit.
- Parser contract is stable enough for AST and diagnostics.

Until then: no lexer/parser keyword additions for `coro`/`yield`.

### 2) CPAN coroutine API support (deliverable now)

**Scope:** LSP UX support for CPAN coroutine ecosystems, starting with `Coro`.

**Potential deliverables:**

- Hover docs for common `Coro` symbols and lifecycle methods.
- Completion for object methods such as `resume`, `is_suspended`, and `cancel`.
- Lightweight semantic inference when `use Coro;` + `async { ... }` patterns are present.

**Explicit non-goals:**

- No lexer changes.
- No parser grammar changes.
- No synthetic core-language keyword modeling.

---

## Implementation Options and Risk

### Option A — Defer #3539 (recommended default)

- Convert #3539 into a scoped investigation/tracker issue.
- Record defer rationale and upstream references.
- Open successor issues for CPAN support and future core tracking.

**Risk:** Low.

### Option B — Build CPAN coroutine UX support only

- Ship hover/completion support for `Coro` surfaces.
- Keep parser/AST untouched.

**Risk:** Moderate but contained.

### Option C — Parser groundwork without fictional syntax

- Refactor extensibility points for sub modifiers / control-flow keywords without enabling coroutine syntax.

**Risk:** Medium and speculative.

### Option D — Full `coro sub` + `yield` syntax support now

- Not recommended without upstream syntax contract.

**Risk:** High (grammar churn, semantic ambiguity, rework probability).

---

## Proposed Near-Term Plan

1. Re-title/reframe #3539 around scope investigation and decision recording.
2. Add a decision comment documenting defer of core syntax.
3. Open a dedicated issue for CPAN `Coro` hover/completion support.
4. Implement only the CPAN issue in code, with parser grammar unchanged.

---

## Acceptance Criteria for the Re-Scope

- [ ] #3539 no longer claims shipped core coroutine syntax without authoritative upstream docs.
- [ ] Core syntax work is explicitly deferred pending upstream contract.
- [ ] A CPAN-focused coroutine UX issue is created with concrete LSP acceptance criteria.
- [ ] Any future parser work references explicit upstream syntax documentation first.

---

## Last Updated

- 2026-04-11: Initial re-scope documentation for issue #3539.
