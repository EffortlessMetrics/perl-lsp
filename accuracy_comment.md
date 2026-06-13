## Accuracy Scout Pass ✓

All claims verified against origin/main (52451a28e).

| Claim | Status | Details |
|-------|--------|---------|
| **primary.rs:401** — has_embedded_code derived from pattern only, ignoring modifiers | ✓ VERIFIED | Line 401 calls `analyze_regex_body_for_ast(&pattern, token.start)` without consulting modifiers parsed at line 371. Commit a5ecdb253 (in origin/main) consolidated this logic but didn't add modifier checking. |
| **quotes.rs:268** — has_embedded_code from body only, ignoring modifiers at line 267 | ✓ VERIFIED | Line 268 calls `analyze_regex_body_for_ast(&content, start)` while modifiers are parsed at line 267 by `parse_quote_operator_substitution_modifiers()`. Modifiers available but unused. |
| **analyze_regex_body_for_ast** only scans for (?{...}) patterns | ✓ VERIFIED | Function in expressions/mod.rs:10 calls `validator.find_code_execution(pattern, start)` on the pattern only. Takes no modifiers parameter. |
| **e/ee modifiers recognized** in valid modifier list | ✓ VERIFIED | primary.rs:377 error message lists valid modifiers: "g, i, m, s, x, o, e, r" |
| **(?{...}) patterns DO set has_embedded_code:true** | ✓ VERIFIED | validator.find_code_execution() detects inline (?{...}) blocks per perl-regex-validator logic. |
| **AST doc comment** says "Whether the regex contains embedded code (?{...})" | ✓ VERIFIED | ast.rs:2050-2051 (Substitution.has_embedded_code field) has doc "Whether the regex contains embedded code (?{...})" |
| **No covering issue** #895–#965 | ✓ VERIFIED | Spot-checked recent issue range and swarm-merged issues; no issue covers s///e / has_embedded_code gap |
| **Already fixed?** | ✗ NOT FIXED | No commits since a5ecdb253 (2026-05-05) modify primary.rs:401 or quotes.rs:268 to check modifiers. Sync commit 6925335fa (2026-06-06) carried identical code. |

## Root Cause

The `has_embedded_code` flag is set by analyzing only the **pattern** string (via `analyze_regex_body_for_ast(&pattern, ...)`), which detects `(?{...})` inline code. The `e` and `ee` modifiers—which tell the replacement string to be eval'd as code—are parsed but never consulted when deciding whether `has_embedded_code` should be true.

## Impact

Security/diagnostic consumers reading the AST (e.g., eval-in-regex warnings, code-injection static analysis) silently miss `s/PAT/expr/e` and `s///ee` substitutions, which execute arbitrary Perl in the replacement.

## Suggested Fix

Two-site + doc update:
1. **primary.rs:401** — Pass modifiers to has_embedded_code calculation
2. **quotes.rs:268** — Pass modifiers to has_embedded_code calculation  
3. **ast.rs:2050** — Update doc comment to mention `e`/`ee` modifiers

## Verdict: ✓ Facts Verified (Ready for Plan Review)
