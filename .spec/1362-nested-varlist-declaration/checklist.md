# Implementation Checklist: #1362 — Nested variable list declarations

## Change order (compiles at each step)

### Step 1: Modify parse_variable_list_item to handle nested lists with multiple items
- **File:** `crates/perl-parser-core/src/engine/parser/variables.rs`
- **Change:** Replace the recursive single-item parse in the `LeftParen` branch with a loop that parses comma-separated items until `RightParen`
- **Details:** Current code (lines 163-167):
  ```rust
  Some(TokenKind::LeftParen) => {
      self.consume_token()?;
      let item = self.parse_variable_list_item()?;
      self.expect_closing_delimiter(TokenKind::RightParen)?;
      Ok(item)
  }
  ```
  Should become:
  ```rust
  Some(TokenKind::LeftParen) => {
      self.consume_token()?;
      let mut items = Vec::new();
      while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
          items.push(self.parse_variable_list_item()?);
          if self.peek_kind() == Some(TokenKind::Comma) {
              self.consume_token()?;
          } else if self.peek_kind() != Some(TokenKind::RightParen) {
              return Err(ParseError::syntax(
                  "Expected comma or closing parenthesis in nested variable list",
                  self.current_position(),
              ));
          }
      }
      self.expect_closing_delimiter(TokenKind::RightParen)?;
      
      // Wrap multiple items in a NestedVariableList node
      if items.len() == 1 {
          Ok(items.into_iter().next().unwrap())
      } else {
          let start = /* ... */;
          let end = /* ... */;
          Ok(Node::new(
              NodeKind::NestedVariableList { items },
              SourceLocation { start, end },
          ))
      }
  }
  ```
- **Depends on:** None
- **Verify:** `cargo check -p perl-parser-core`

### Step 2: Add NestedVariableList AST node kind
- **File:** `crates/perl-ast/src/node_kind.rs` (or wherever NodeKind is defined)
- **Change:** Add a new enum variant to `NodeKind`
- **Details:** 
  ```rust
  /// Nested variable list in destructuring: my ($a, ($b, $c)) represents a NestedVariableList
  NestedVariableList { items: Vec<Node> },
  ```
- **Depends on:** Step 1 compilation check
- **Verify:** `cargo check -p perl-parser-core`

### Step 3: Update NodeKind matching in consumers
- **File:** `crates/perl-parser-core/src/engine/parser/expressions/calls.rs` (line ~557 area)
- **Change:** Verify that the call to `parse_variable_list_item()` in this function also benefits from the fix without requiring changes (it should; the function is shared)
- **Details:** This is a validation check — no code changes needed if the function is properly shared
- **Depends on:** Step 2
- **Verify:** `cargo check -p perl-parser-core`

### Step 4: Final verification
- **Verify:** 
  ```bash
  cargo test -p perl-parser-core nested_varlist
  cargo test -p perl-parser-core
  cargo xtask fmt
  cargo clippy -p perl-parser-core
  ```

## Callers and consumers

- `parse_variable_list_item()` is called from:
  - `crates/perl-parser-core/src/engine/parser/variables.rs:16` — in `parse_variable_declaration()`
  - `crates/perl-parser-core/src/engine/parser/variables.rs:165` — recursive call within itself
  - `crates/perl-parser-core/src/engine/parser/expressions/calls.rs:557` — in list context parsing

## Scope boundary

Files IN scope:
- `crates/perl-parser-core/src/engine/parser/variables.rs` — main fix
- `crates/perl-ast/src/node_kind.rs` (or AST definition location) — new enum variant

Files OUT of scope (consumers that may need no changes):
- `crates/perl-parser-core/src/engine/parser/expressions/calls.rs` — shared function fix applies automatically
- `crates/perl-lsp-rs/` — LSP layer should work automatically with new AST shape
- `crates/perl-semantic-analyzer/` — semantic analysis should handle new node automatically
- `crates/perl-workspace/` — workspace indexing should handle new node automatically

## Flags for builder

1. **AST node location**: Verify exact location of `NodeKind` enum definition (assumed `crates/perl-ast/src/node_kind.rs` but may be elsewhere)
2. **Single vs multiple items wrapping**: When nested list has exactly 1 item, we return that item directly (no wrapper node) for backward compatibility with existing AST shape
3. **Location tracking**: Must capture accurate `start` and `end` positions for the nested list node using `self.current_position()` before consuming `(` and updating after consuming `)`
4. **Error messages**: Changed error message to specifically mention "nested variable list" for clarity
5. **NodeKind::NestedVariableList consumers**: May need to add handling in:
   - LSP symbol extraction (if it walks NodeKind variants)
   - Semantic analyzer (if it analyzes variable scopes)
   - Any other place that does exhaustive matching on NodeKind (will be caught by compiler)
