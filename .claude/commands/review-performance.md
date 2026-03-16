---
description: Review code for performance issues — allocations, clone patterns, hot paths, algorithmic complexity
argument-hint: "<PR-number-or-branch>"
---

# Performance Review

Review PR **$ARGUMENTS** through a performance lens.

## Checklist

- [ ] No unnecessary `.clone()` on Copy types
- [ ] Strings: `.push(char)` not `.push_str("x")` for single chars
- [ ] Collections: `or_default()` not `or_insert_with(Vec::new)`
- [ ] Prefer `.first()` over `.get(0)`
- [ ] No O(n^2) where O(n) or O(n log n) is possible
- [ ] HashMap/HashSet for frequent lookups (not linear search)
- [ ] Avoid repeated regex compilation — compile once, reuse
- [ ] String building: use `String::with_capacity()` for known sizes
- [ ] Avoid allocating in hot loops

## Parser-Specific Checks

- Token creation should be allocation-light
- AST nodes: minimize boxing where possible
- Lexer state transitions should be O(1)

## Process

1. Get changed files: `gh pr diff $ARGUMENTS --stat`
2. Review each changed file against the checklist
3. Focus on hot paths — parser engine, lexer, LSP request handlers
4. Report findings with line references and suggested fixes
