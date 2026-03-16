# perl-parser-core

Recursive descent parser engine. Most parser fixes happen here.

## Test Pattern
- Add tests in a NEW file under `tests/` (e.g., `tests/fix_undef_list.rs`), not in cpan_pattern_tests.rs
- This prevents merge conflicts when multiple agents add tests simultaneously
- Test template:
  ```rust
  use perl_parser_core::parse;

  #[test]
  fn test_<description>() -> Result<(), Box<dyn std::error::Error>> {
      let source = r#"<perl code>"#;
      let result = parse(source);
      assert!(result.errors.is_empty(), "Errors: {:?}", result.errors);
      Ok(())
  }
  ```

## Verify
```bash
cargo fmt --all
cargo clippy -p perl-parser-core --tests
cargo test -p perl-parser-core
```

## Key Files
- `src/engine/parser/` — main parsing logic
- `src/engine/parser/expressions.rs` — expression parsing
- `src/engine/parser/statements.rs` — statement parsing
- `src/engine/parser/declarations.rs` — use/my/sub declarations
- `src/engine/parser/control_flow.rs` — if/while/for/etc
