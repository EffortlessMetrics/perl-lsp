# perl-parser-core

Recursive descent parser engine. Most parser fixes happen here.

## Test Pattern
- Add tests in a NEW file under `tests/` (e.g., `tests/fix_undef_list.rs`), not in cpan_pattern_tests.rs
- This prevents merge conflicts when multiple agents add tests simultaneously
- Test template:
  ```rust
  mod cpan_test_helpers;
  use cpan_test_helpers::*;

  #[test]
  fn test_<description>() {
      let source = r#"<perl code>"#;
      assert_clean_parse(source);
  }
  ```
- Shared helpers live in `tests/cpan_test_helpers/mod.rs` (provides `parse`, `assert_clean_parse`, `top_level_kinds`)

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
