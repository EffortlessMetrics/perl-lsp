---
description: TDD parser fix (failing test → fix → verify → PR)
argument-hint: "<bug description> e.g. '$Pkg::Var[0] not parsed as subscript'"
---

# Parser Fix (TDD)

Fix a parser bug using test-driven development. Bug: **$ARGUMENTS**

## Launch an agent in a worktree to do this work:

Use the Agent tool with `isolation: "worktree"` and `mode: "auto"` to:

### 1. Find the root cause
- Search `crates/perl-parser-core/src/engine/parser/` for relevant parsing logic
- Key files: `variables.rs`, `statements.rs`, `expressions/postfix.rs`, `expressions/precedence.rs`, `declarations.rs`
- Understand WHY the construct fails to parse

### 2. Write failing tests FIRST
- Add tests in the appropriate test file under `crates/perl-parser-core/`
- Each test should parse a Perl snippet and assert no ERROR nodes:
```rust
#[test]
fn test_description() -> Result<()> {
    let source = r#"<perl code>"#;
    let mut parser = Parser::new(source);
    let ast = parser.parse()?;
    // Assert no errors
    Ok(())
}
```

### 3. Implement minimal fix
- Change as little code as possible
- Follow coding standards: NO `unwrap()`, `expect()`, `panic!()`, `todo!()` in production code

### 4. Verify
```bash
cargo xtask fmt
cargo clippy -p perl-parser-core --lib
cargo test -p perl-parser-core
cargo test -p perl-parser
```

### 5. Create PR
- Branch, commit, push, `gh pr create`
- Title: `fix(parser): <description>`
- Return PR URL

## Coding Standards Reminder
- No fatal constructs (`unwrap`, `expect`, `panic!`, `todo!`, `unimplemented!`)
- Use `?`, `.ok_or_else()`, pattern matching
- In tests: `Result<()>` return types
- `cargo fmt` + `cargo clippy` must be clean
