# Issue #1664: state variable without initializer falsely reported as uninitialized

## Problem Statement

The scope analyzer treats `state $var;` (without an explicit initializer) as uninitialized, and reports UninitializedVariable warnings when the variable is used. This is incorrect Perl semantics.

**Perl semantics**: `state` variables are implicitly initialized to `undef` on first call, making them safe to use without an explicit initializer.

**Contrast with `my`**:
- `my $x;` is truly uninitialized (no implicit initialization) — should warn
- `state $x;` is implicitly initialized to `undef` on first call — should NOT warn

This distinction is critical for Perl code that relies on state variables for persistent lexical storage.

## Verification

✓ **Perl documentation confirmed**: perlsub(1), "Persistent Private Variables" section confirms state initialization to undef
✓ **Code paths verified on current main**:
  - `crates/perl-semantic-analyzer/src/analysis/scope_analyzer/declarations.rs:28-29` — line 29 shows `is_initialized = initializer.is_some()`
  - `crates/perl-semantic-analyzer/src/analysis/scope_analyzer/mod.rs:1062-1069` — uninitialized reporting path confirmed

## Root Cause

In `declarations.rs:28-29`, the declaration handler treats all declarators uniformly:

```rust
let is_our = declarator == "our";
let is_initialized = initializer.is_some();  // True only if explicit initializer
```

This logic marks `state $x;` as uninitialized because there's no explicit initializer, even though Perl implicitly initializes `state` variables to `undef`.

## Solution

Modify the initialization check in `declarations.rs:29` to account for `state`'s implicit initialization:

```rust
// Current (incorrect):
let is_initialized = initializer.is_some();

// Fixed (correct):
let is_initialized = declarator == "state" || initializer.is_some();
```

Since `state` variables are guaranteed to be initialized to `undef` on first call, they should never trigger UninitializedVariable warnings.

## Alternatives Considered

1. **Track state-ness separately in Variable struct** — More explicit metadata, but overkill for this fix. The declarator-time check is sufficient.
2. **Special-case the uninitialized check** — Check `is_state` before reporting UninitializedVariable in the check phase. Rejected: better to mark as initialized upfront.
3. **New issue kind** (e.g., StateUninitializedVariable) — Never report it anyway; unnecessary complexity.

## Dependencies

No structural changes required. This is a single-line logic fix in the initialization check, no struct field additions.

## Related Issues

- **#1654** (broader state variable semantics) — addresses redeclaration errors, block-scoping, persistence tracking. Independent of this fix but in the same problem domain.
- **#1657, #1659, #1661** — other state variable semantics improvements

## Scope & Blast Radius

**Crate touched**: 1
- `perl-semantic-analyzer` (declarations.rs, tests)

**Risk**: Very low
- No public API changes
- No struct field additions
- Single-line logic change in declaration phase
- Clearly aligns with Perl semantics

**Test coverage**: 
- Positive case: `state $x;` without initializer should NOT warn
- Negative case: `my $y;` without initializer SHOULD warn (regression)
