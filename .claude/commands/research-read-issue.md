---
description: Research verifier step 1 — read the scout issue and extract all factual claims for verification
user-invocable: false
---

# Research: Read Issue

Read the scout-filed issue and extract every factual claim that can be
independently verified against an external source (Perl docs, LSP spec,
crate API, internal function existence).

## Steps

1. Read the issue and its comments:

   ```bash
   gh issue view <number> --json title,body,labels,comments --jq '{title: .title, body: .body, labels: [.labels[].name], comments: [.comments[].body]}'
   ```

2. Extract factual claims by category:

   **Perl language claims** — assertions about Perl syntax, semantics, or version
   availability. Examples:
   - "Perl supports `if EXPR { }` without parens"
   - "Named captures available since Perl 5.32"
   - "The `say` builtin requires `use feature 'say'`"

   **LSP/DAP spec claims** — assertions about what the protocol requires,
   supports, or specifies. Examples:
   - "CompletionItem.insertText supports snippet syntax in LSP 3.0"
   - "textDocument/hover must return `null` if no info available"
   - "DAP `StackFrame` requires a `source` field"

   **Crate API claims** — assertions about function existence, signatures, or
   behavior in published or internal crates. Examples:
   - "expect_or_recover() exists in perl-parser-core"
   - "tokio::time::timeout works with sync traits"
   - "parse_method_call() is defined in perl-parser"

3. Skip claims that are:
   - Pure opinions or design preferences ("Option A is better because...")
   - Already sourced with a file:line in the issue
   - About the issue author's observations (e.g., "I found that X fails")

## Output

```
Claims extracted from issue #NNN:

PERL:
  P1: "<claim>" — needs: perlsyn / perlfunc / perlop / perlre / perlmod / version check
  P2: ...

LSP/DAP:
  L1: "<claim>" — needs: LSP spec section / DAP spec
  L2: ...

CRATE API:
  A1: "<claim>" — needs: grep <crate> / docs.rs <crate>
  A2: ...

SKIP (already sourced or opinion):
  - "<reason>"
```

If there are no verifiable claims, output:
```
No external factual claims found in issue #NNN. Nothing to verify.
Label research-reviewed anyway to unblock pipeline.
```
