# ADR-2026-0417: Add ParseResult and parse_perl_summary to tree-sitter-perl-c

## Status

Proposed

## Context

Issue #4369 requests adding a `ParseResult` struct and a `parse_perl_summary()`
convenience function to the `tree-sitter-perl-c` crate. Currently,
`parse_perl_code()` and `parse_perl_file()` return raw `tree_sitter::Tree` objects,
forcing all callers to import the `tree-sitter` crate to do anything useful (e.g.,
check `has_error()`, call `to_sexp()`, count nodes).

The proposed `ParseResult` struct bundles `has_errors`, `root_kind`,
`node_kind_count`, `sexp`, and a `tree: tree_sitter::Tree` escape hatch. This is
a thin ergonomic layer over the existing API.

### ROADMAP Conflict

The `crates/tree-sitter-perl-c/ROADMAP.md` explicitly states:

> New high-level parse APIs (out of scope — use `perl-parser` for that).

The maintainer vision agent assessed this issue as **misaligned**, citing:

1. Direct contradiction of the crate's stated role as a "conventional
   C/tree-sitter reference implementation" for compatibility testing and benchmarking.
2. ROADMAP explicitly blocking "New high-level parse APIs".
3. `node_kind_count` field name is misleading — it returns a grammar-level constant,
   not a per-tree node count.

The plan-reviewer assessed the issue as **feasible with modifications**, noting:
- The change is a thin ergonomic wrapper, not a new parsing engine.
- The `tree` field provides an escape hatch for advanced users.
- An explicit ROADMAP exception is required before proceeding.

## Decision

We will add `ParseResult` and `parse_perl_summary()` to `tree-sitter-perl-c` as
specified in issue #4369, subject to the following conditions:

1. **ROADMAP.md is updated first** — the "Not planned" designation for
   "New high-level parse APIs" will be revised to allow thin ergonomic wrappers
   that expose pre-computed tree-sitter fields, as distinct from new parsing engines.
2. **`node_kind_count` is renamed** — the field will be renamed to
   `grammar_node_kind_count` to accurately reflect that it returns a grammar-level
   constant (the total number of distinct node kinds in the grammar), not a
   per-tree node count.
3. **No new dependencies** — the implementation uses only existing imports.
4. **Backward compatibility** — `parse_perl_code()` and `parse_perl_file()` signatures
   are unchanged.

## Alternatives Considered

### 1. Direct issue author to perl-parser (vision agent recommendation)
The ergonomic friction is real, but `perl-parser` is the appropriate venue for
friendly Rust APIs. However, `tree-sitter-perl-c` is a published crate on crates.io
with direct callers; leaving those callers without a convenient summary API forces
them to either import `tree-sitter` directly or switch parsers. This alternative
was rejected because the caller base for `tree-sitter-perl-c` deserves ergonomic
relief without being forced to switch parsers.

### 2. Do nothing
Reject the issue and close it. This leaves existing callers with the current
friction. Rejected because the ergonomic argument is valid and the change is
low-effort and backward-compatible.

### 3. Add to a new wrapper crate
Create a separate `tree-sitter-perl-c-derive` or `perl-parse-summary` crate.
Rejected as over-engineering for a single struct and function.

## Consequences

### Benefits
- Callers no longer need to import `tree_sitter` directly for common operations.
- Pre-computed `sexp` and `has_errors` fields reduce boilerplate at every call site.
- The `tree` escape hatch preserves access to the full tree-sitter API for advanced
  users.
- Fully backward-compatible — no existing APIs change.

### Risks
- **ROADMAP conflict** — this change directly contradicts the current ROADMAP
  designation. Proceeding requires an explicit exception/waiver recorded in this
  ADR and a ROADMAP update.
- **`node_kind_count` semantics** — the field name is genuinely misleading. The
  rename to `grammar_node_kind_count` mitigates this.
- **Scope creep** — adding ergonomic wrappers could open the door to further
  high-level APIs, undermining the crate's identity as a thin tree-sitter binding.
  The ROADMAP update should clarify that only pre-computed-field wrappers are
  allowed, not new parsing logic.
- **`sexp` allocation** — `root.to_sexp()` allocates on every call. This is a
  documented tradeoff; performance-sensitive callers can use `parse_perl_code`
  directly.

## Implementation Notes

- Single file change: `crates/tree-sitter-perl-c/src/lib.rs`
- Insert `ParseResult` struct after `get_scanner_config()` (~line 156)
- Insert `parse_perl_summary()` function after the struct
- Add 3 unit tests in the existing `#[cfg(test)] mod tests` block
- No changes to other files are required
