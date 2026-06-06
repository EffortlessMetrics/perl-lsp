# Release UX Smoke Fixtures

This directory contains fixture-only workspaces for local release user testing.
The fixtures are not a runtime harness yet. They define the user scenarios,
source files, expected editor behavior, and the LSP requests a future
`cargo xtask lsp-ux-smoke` command should drive.

## Fixture Index

| Fixture | User scenario | Primary proof target |
|---|---|---|
| `minimal_script` | Open a single ordinary Perl script | quiet startup, diagnostics, symbols |
| `lib_project` | Open a project with workspace `lib/` modules | effective `@INC`, definition, completion |
| `local_lib_project` | Open a project with `local/lib/perl5` dependencies | local dependency resolution |
| `crlf_links` | Open a Windows-style CRLF file with local quoted links | document-link range correctness |
| `diagnostics_quickfix` | Open code with undefined loop-control labels | PL410 diagnostics and safe quick fixes |
| `perldoc_links` | Open code using core/library modules | hover and perldoc/document-link behavior |

Each fixture contains:

- `README.md` for the user scenario and claim boundary
- source files arranged like a small Perl workspace
- `requests.json` for the LSP methods and position markers a harness should
  drive
- `expected.json` for response-shape expectations and negative cases

## Common Smoke Sequence

Each fixture should support this sequence when the harness is added:

```text
initialize
initialized
textDocument/didOpen
diagnostics receipt
textDocument/documentSymbol
textDocument/definition
textDocument/completion
textDocument/codeAction
textDocument/documentLink
textDocument/hover
shutdown
exit
```

Individual fixture READMEs mark requests that are expected to return empty
results. Empty is acceptable when it is quiet, deterministic, and documented.

## Claim Boundary

These fixtures define release-smoke scenarios only. They do not prove runtime
behavior until a harness consumes them and writes receipts.
