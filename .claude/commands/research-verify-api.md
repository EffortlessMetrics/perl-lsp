---
description: Research verifier step 4 — verify crate API claims via docs.rs search and codebase grep
user-invocable: false
---

# Research: Verify API

Verify all CRATE API claims extracted in step 1. Use docs.rs for published
crates and grep/read for internal crates.

## Steps

For each CRATE API claim from step 1:

1. **Classify the crate:**
   - Published (tokio, serde, lsp-types, tower-lsp, etc.) → use docs.rs
   - Internal (perl-parser, perl-lsp, perl-lexer, etc.) → grep source

2. **For published crates** — search docs.rs:
   ```bash
   # Use the mcp__docs-rs tools when available, or WebFetch:
   # https://docs.rs/<crate>/<version>/<crate>/fn.<function>.html
   # https://docs.rs/<crate>/latest/<crate>/struct.<Struct>.html
   ```
   Example: `https://docs.rs/tokio/latest/tokio/time/fn.timeout.html`

3. **For internal crates** — grep the workspace source:
   ```bash
   grep -r "fn <function_name>" crates/<crate>/src/
   grep -r "pub fn <function_name>" crates/
   grep -rn "struct <TypeName>" crates/<crate>/src/
   ```

4. **For function existence claims**: Check exact name. Common false positives:
   - Function exists under a different name (e.g., `recover_or_error` not `expect_or_recover`)
   - Function is private (no `pub`)
   - Function exists but in a different module/crate

5. **For behavioral claims** (e.g., "tokio::timeout works with sync code"):
   - Read the docs.rs page for the function
   - Check if the function signature requires `async` or `impl Future`
   - Note whether the claim about behavior matches the documented contract

## Common errors to catch

- `parse_method_call()` in perl-parser → grep often shows it doesn't exist with that exact name
- Internal function names mangled (scout guessed the name)
- `tokio::time::timeout` → requires `async fn`; can't be used with sync traits
- `lsp_types::CompletionItem` fields → check actual struct fields on docs.rs

## Output

For each CRATE API claim:
```
A1: "<claim>"
  STATUS: VERIFIED | FALSE | UNVERIFIED
  FINDING: <1-2 sentences — what was found>
  SOURCE: <URL or "grep: crates/X/src/Y.rs:NN">
  NOTE: <any caveats — private, different name, different module>
```

Example verdicts:
```
A1: "expect_or_recover() exists in perl-parser-core"
  STATUS: FALSE
  FINDING: grep finds no function named expect_or_recover in perl-parser-core. Closest match: recover_or_skip in crates/perl-parser-core/src/recovery.rs:42
  SOURCE: grep: crates/perl-parser-core/src/

A2: "tokio::time::timeout works with sync code"
  STATUS: FALSE
  FINDING: docs.rs shows timeout() requires impl Future — cannot wrap sync functions directly.
  SOURCE: https://docs.rs/tokio/latest/tokio/time/fn.timeout.html
```
