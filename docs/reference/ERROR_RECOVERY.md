# Parser Error Recovery and Resilience

**Audience**: Contributors adding parser features, users diagnosing unexpected diagnostics.

The parser is designed for an IDE environment where code is almost always in a partially-written state. Rather than failing on the first syntax error, it continues parsing and produces a partial AST so that all downstream LSP features — completion, go-to-definition, semantic tokens, diagnostics — can still operate on whatever parsed cleanly.

---

## Three Degradation Tiers

After every parse attempt the LSP server assigns the open document a `DegradationTier` based on the parse result. Features consult the tier before attempting operations that require a valid AST.

| Tier | Condition | Available features |
|------|-----------|-------------------|
| **Full** | Parse succeeded with zero errors | All features |
| **Partial** | Parse produced errors but also an AST | Best-effort completions, navigation, diagnostics from the valid portions |
| **Minimal** | Parse failed completely — no AST | Word completion, bracket matching, text-based symbol extraction only |

In code:

```rust
// crates/perl-lsp-rs/src/state/document.rs
pub enum DegradationTier {
    Minimal, // No AST
    Partial, // AST present, parse errors exist
    Full,    // AST present, no errors
}

impl DegradationTier {
    pub fn has_ast(self) -> bool {
        matches!(self, DegradationTier::Full | DegradationTier::Partial)
    }
    pub fn has_full_semantics(self) -> bool {
        matches!(self, DegradationTier::Full)
    }
}
```

Feature providers guard AST operations with `doc.degradation_tier.has_ast()` and reserve deep semantic work (unused-variable detection, type inference) for `has_full_semantics()`.

---

## What Gets Parsed in the Partial Tier

The parser never stops at the first error. Instead it follows three escalating strategies:

### 1. Expression-Level Recovery

When the right-hand side of an expression is missing the parser creates an `Error` node in the AST and continues to the next statement boundary.

```perl
my $x = ;   # Error node inserted for the missing RHS
print 1;    # Parsed cleanly — available to all features
```

The resulting AST has two top-level statements: an `Error` node for `my $x = ;` and a valid `ExpressionStatement` for `print 1;`.

### 2. Statement-Level Recovery (Synchronization)

After inserting an error node the parser advances forward — skipping tokens until it reaches a _synchronization point_:

- `;` (semicolon — statement boundary)
- `}` (closing brace — block boundary)
- A statement-opening keyword: `my`, `our`, `local`, `state`, `field`, `sub`, `if`, `unless`, `while`, `until`, `for`, `foreach`, `return`, `last`, `next`, `redo`, `goto`, `die`, `eval`, `do`

This guarantees the parser is back at a known-valid position before attempting the next statement, so a single bad statement does not cause a cascade of false errors.

### 3. Block-Level Recovery (Unclosed Delimiters)

Unclosed braces are handled independently from expression errors. When the parser reaches EOF while still inside a block it synthesizes the missing `}`, records the error, and returns the partial block. The result is a partial `Subroutine` or `If` node that contains whatever statements were parsed before EOF.

```perl
sub foo { my $x = 1;   # EOF here — no closing brace
```

The AST contains a `Subroutine` node for `foo` with `my $x = 1` inside its body. The unclosed-brace error is surfaced as a diagnostic; completion and navigation inside the body still work.

---

## Budget Limits

Recovery is bounded by a `ParseBudget` to prevent runaway parsing on adversarial or deeply malformed input.

| Parameter | Default | Description |
|-----------|---------|-------------|
| `max_errors` | 100 | Stop collecting diagnostics after this many errors |
| `max_depth` | 256 | Maximum block/expression nesting depth |
| `max_tokens_skipped` | 1000 | Tokens consumed by all recovery attempts combined |
| `max_recoveries` | 500 | Total number of recovery attempts per parse |

The LSP server uses `ParseBudget::for_ide()` (same as `default()`). The `strict()` preset lowers all limits for untrusted input: 10 errors, 64 depth, 100 tokens skipped, 50 recoveries.

A `BudgetTracker` accumulates consumption. When a limit is hit the parser returns immediately with a `terminated_early: true` flag in `ParseOutput`. Partially collected diagnostics and any AST nodes built so far are still surfaced to the user.

---

## ParseOutput: Structured Output

Parse results are returned as a `ParseOutput` rather than a plain `Result`. This ensures the caller always has access to both the AST and the error list, even when errors occurred.

```rust
pub struct ParseOutput {
    pub ast: Node,               // Always present, may contain Error nodes
    pub diagnostics: Vec<ParseError>,
    pub budget_usage: BudgetTracker,
    pub terminated_early: bool,  // true if budget was exhausted
}
```

The higher-level `parse_with_recovery()` method on `Parser` returns this type directly. The simpler `parse()` method returns `Result<Node, ParseError>` for callers that don't need granular diagnostic access.

---

## Error Node Structure

Every place in the AST where parsing failed is represented by a `NodeKind::Error` node. The node carries:

- `message` — human-readable description of what went wrong
- `expected` — list of token types that were valid at that position
- `partial` — optionally, the partially-built child node (e.g. a `Subroutine` with a missing closing brace)

The `partial` field is what enables downstream analysis to still extract the subroutine name, parameter list, and body statements even when the declaration is incomplete.

---

## Diagnostic Categories

The `ErrorClassifier` in `perl-error` maps raw `ParseError` variants to one of 15 named categories for display in the editor:

| Category | Typical cause |
|----------|--------------|
| `UnclosedString` | Unmatched `"` or `'` |
| `UnclosedRegex` | Regex delimiter not closed |
| `UnclosedBlock` | Missing `}` for `sub`, `if`, `while`, etc. |
| `UnclosedParenthesis` | Unmatched `(` |
| `UnclosedBracket` | Unmatched `[` |
| `UnclosedBrace` | Unmatched `{` in hash or regex context |
| `MissingSemicolon` | Statement not terminated with `;` |
| `MissingOperator` | Two operands with no operator between them |
| `MissingOperand` | Operator with no right-hand side |
| `UnterminatedHeredoc` | Heredoc terminator absent or indented wrong |
| `InvalidVariableName` | Sigil not followed by a valid identifier |
| `InvalidSubroutineName` | Bare `sub` name violates identifier rules |
| `UnexpectedToken` | Parser expected X but found Y |
| `UnexpectedEof` | Input ended while inside an open construct |
| `InvalidSyntax` | General fallback when no category matches |

Each category has a user-facing message, an optional suggestion (e.g. "Add a closing brace `}` to complete the code block"), and an optional explanation for less common errors.

---

## Recovery in Practice

### Example: missing expression

```perl
my $a = ;      # Error 1
print 'valid'; # Valid
my $b = ;      # Error 2
my $c = 10;    # Valid
my $d = ;      # Error 3
```

Parse result: 5 statements in the AST — 3 `Error` nodes interleaved with 2 valid statements. All 3 diagnostics appear in the editor. Completion on `$c` and `$a` in later lines still works because those variables are indexed from the partial AST.

### Example: unclosed subroutine

```perl
sub foo { }
sub bar { my $x = 1;
```

Parse result: 2 top-level statements. `foo` is a clean `Subroutine` node. `bar` is a `Subroutine` node (or an `Error` node with `bar` as the `partial`) containing `my $x = 1`. One diagnostic is emitted for the unclosed block. Go-to-definition for `bar` and completion of `$x` inside `bar`'s body both work.

### Example: deeply nested errors

```perl
if ($a) {
    if ($b) {
        if ($c) {
            my $x = ;        # expression error
        }
    }
}
```

The expression error at depth 3 is contained to that one statement. All three `if` blocks parse cleanly and are fully available for navigation.

---

## For Contributors

When adding a new parsing rule:

1. Return `Err(ParseError)` from the parsing function — never panic.
2. The caller will call `context.recover_with_node(error)` which:
   - Adds the error to the diagnostic list.
   - Creates an `Error` node at the current source range.
   - Synchronizes forward to the next statement boundary.
3. If the construct is partially built (e.g. a `Subroutine` with name and parameters but no body), pass it as the `partial` argument to `create_error_node` so downstream analysis can still use it.
4. Always check `budget.depth_would_exceed()` before recursing into a new nesting level. Return `Err(ParseError::RecursionLimit)` if the limit is reached.

The recovery tests in `crates/perl-parser-core/src/engine/parser/error_recovery_tests.rs` and `unclosed_block_recovery_tests.rs` are the canonical examples of how these invariants are exercised.
