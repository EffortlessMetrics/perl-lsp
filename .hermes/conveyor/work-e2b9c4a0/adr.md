# ADR-041: Loop Control Label Resolution Validation

## Status
Proposed

## Context

The Perl LSP does not validate label resolution for loop control statements (`next`, `last`, `redo`). When these statements are used with a label (e.g., `next OUTER;`), the parser correctly handles the syntax, but no semantic validation occurs to check if the referenced label actually exists in the file. This results in a runtime error in Perl that could be caught at analysis time.

**Example:**
```perl
# No label "OUTER" is defined
next OUTER;  # Should emit a diagnostic, but currently does not
```

### Existing Infrastructure

1. **Labels ARE tracked in the symbol table** — `SymbolExtractor` already adds `SymbolKind::Label` entries for `LabeledStatement` nodes at `crates/perl-semantic-analyzer/src/analysis/symbol.rs:782-796`.

2. **Goto label validation already exists** — `crates/perl-lsp-diagnostics/src/lints/goto_label.rs` provides a complete pattern: walk AST → find `NodeKind::Goto` → check label exists in symbol table → emit diagnostic if not found.

3. **`NodeKind::LoopControl` exists** — The AST has `NodeKind::LoopControl { op, label }` where `label: Option<String>` holds the label name.

4. **Diagnostic code PL409 (GotoUndefinedLabel)** already exists and could theoretically be reused.

## Decision

Implement label resolution validation for `next`/`last`/`redo` loop control statements as a **new lint** (`check_loop_labels`) in `perl-lsp-diagnostics`, following the established pattern from `goto_label.rs`.

**Key decisions:**

1. **Create new lint file `loop_label.rs`** — Not added to the existing `goto_label.rs` because:
   - Loop control (`next`/`last`/`redo`) is semantically distinct from `goto`
   - A separate lint allows independent control (disable, configure severity) in the future
   - Keeps the door open for future loop-specific validations (e.g., "label does not refer to a loop")

2. **Create new diagnostic code `LoopLabelUndefined` (PL410)** — Do NOT reuse `GotoUndefinedLabel` (PL409) because:
   - The diagnostic message says "Goto label 'X' is not defined" which would be factually wrong for `next OUTER;`
   - Users would be confused seeing "Goto" in a diagnostic for a `next` statement
   - Separate code allows independent configuration and future extensibility

3. **Make `has_label` helper `pub(crate)`** in `goto_label.rs` — To allow reuse by `loop_label.rs` without code duplication. The helper is a simple symbol table lookup that both lints need.

4. **Call `check_loop_labels` in `diagnostics.rs`** alongside `check_goto_labels` at line ~158.

## Consequences

### Benefits
- **Parity with goto validation**: Users get consistent label resolution diagnostics for all labeled control flow
- **Minimal scope**: Only `perl-lsp-diagnostics` changes; no parser or scope analyzer modifications needed
- **Established pattern**: Uses the same lint architecture already proven in `goto_label.rs`
- **User experience**: Error messages correctly reference `next`/`last`/`redo` rather than `goto`

### Tradeoffs
- **Code duplication risk**: Until `has_label` is made `pub(crate)`, the lint must either share the helper or duplicate logic
- **New diagnostic code**: PL410 must be registered in `perl-diagnostics` crate, adding to the code inventory
- **Future consideration**: Perl semantics require loop labels to target enclosing loops; future enhancement could validate this, but current scope limits to existence check only (matching goto behavior)

### Risks
- **Visibility change**: Making `has_label` `pub(crate)` is a minor API expansion that could affect future refactoring
- **Diagnostic proliferation**: Adding a new code increases the diagnostic catalog size; however, this is necessary for accurate user messaging

## Alternatives Considered

### Alternative 1: Reuse GotoUndefinedLabel (PL409)
- **Description**: Call `check_goto_labels` for both `Goto` and `LoopControl` nodes, reusing the same diagnostic code
- **Rejected because**: Message would read "Goto label 'OUTER' is not defined" for `next OUTER;` — factually incorrect and confusing to users
- **Risk**: User confusion; inability to independently configure/dismiss loop label diagnostics

### Alternative 2: Integrate into Scope Analyzer
- **Description**: Add loop label validation to `scope_analyzer.rs` alongside `UndeclaredVariable`
- **Rejected because**: Labels in Perl are file-scoped, not lexically-scoped like variables. The scope analyzer is designed for lexical scope tracking. Fitting labels into this model would require significant refactoring and conceptual mismatch.
- **Risk**: Higher complexity; scope analyzer is not designed for file-scoped symbols

### Alternative 3: Validate in Parser
- **Description**: Perform label resolution during parsing phase
- **Rejected because**: Parser operates on single statements without file-wide symbol context. The symbol table is built during semantic analysis, after parsing.
- **Risk**: Not architecturally sound; would require passing symbol context to parser

## References

- Issue: GitHub #3372
- Work Item: work-e2b9c4a0
- Related: `goto_label.rs` (existing pattern), `codes/mod.rs` (PL409 definition)
