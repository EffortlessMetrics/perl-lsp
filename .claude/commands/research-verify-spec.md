---
description: Research verifier step 3 — verify LSP/DAP protocol claims via web search of official specs
user-invocable: false
---

# Research: Verify Spec

Verify all LSP/DAP claims extracted in step 1 against the official protocol
specifications.

## Authoritative Sources

- **LSP 3.17**: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/
- **LSP current**: https://microsoft.github.io/language-server-protocol/specifications/specification-current/
- **DAP**: https://microsoft.github.io/debug-adapter-protocol/specification
- **LSP capabilities**: https://microsoft.github.io/language-server-protocol/specifications/lsp/3.17/specification/#capabilities

## Steps

For each LSP/DAP claim from step 1:

1. **Search the spec** using WebSearch:
   - `site:microsoft.github.io language-server-protocol <type or method>`
   - `LSP specification <method name> <field name>`
   - `DAP specification <type> <field>`

2. **Check the spec page directly** via WebFetch if search results are ambiguous.
   Key spec URL patterns:
   - `#<method>` (e.g., `#textDocument_completion`)
   - `#<typeName>` (e.g., `#completionItem`)
   - Use browser Ctrl+F equivalent by searching for the exact field name.

3. **Version gating**: LSP has changed significantly across versions. When a
   claim mentions a version, verify the spec for that version, not just
   the current one.

## Common errors to catch

- Snippet support in insertText → available since LSP 2.x (insertTextFormat field)
- `textDocument/publishDiagnostics` is server-to-client → TRUE (not request)
- Semantic tokens → introduced in LSP 3.16
- inlayHints → introduced in LSP 3.17
- WorkspaceEdit supports `documentChanges` → since LSP 3.13
- `CompletionItem.labelDetails` → since LSP 3.17
- DAP `StackFrame.source` → optional in spec (not required)

## Output

For each LSP/DAP claim:
```
L1: "<claim>"
  STATUS: VERIFIED | FALSE | UNVERIFIED
  FINDING: <1-2 sentences — what the spec actually says>
  SOURCE: <URL with anchor> — <section name>
  VERSION: <"since LSP 3.X" or "current spec" or "not in spec">
```

If a claim cannot be found in the spec:
```
L1: "<claim>"
  STATUS: UNVERIFIED
  FINDING: Not found in LSP 3.17 or DAP spec pages. May be implementation-defined.
  SOURCE: NONE
```
