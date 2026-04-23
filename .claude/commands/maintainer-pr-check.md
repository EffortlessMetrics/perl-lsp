---
description: Maintainer vision (PR) step 2 — evaluate project fit, scope, patterns, quality
user-invocable: false
---

# Maintainer PR: Check

Evaluate whether the implementation fits the project's direction and quality bar.

## Checks

1. **Scope discipline** — does the diff match the issue spec? Extra files = scope drift.
2. **Pattern introduction** — new error type, test helper, config surface, CI gate? Is it justified?
3. **Complexity budget** — LOC vs. user value. 500 lines for 1% of users?
4. **Consistency** — follows the crate's existing conventions?
5. **Test quality** — not just "tests exist" but "tests verify the right thing at the right level"
6. **Documentation debt** — new public API documented? features.toml updated?
7. **Migration** — breaks existing users? Migration path documented?
