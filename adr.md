# ADR-XXXX: Document Links for Module::Runtime use_module/require_module

## Status
Proposed

## Context

The GitHub issue #3415 reports that `Module::Runtime`'s `use_module()` and `require_module()` functions are not recognized as navigable module references in the LSP. Users expect to click on `use_module('Some::Module')` and jump to that module, similar to how `use Some::Module;` works.

Investigation revealed that:

1. **Semantic analysis** (`perl-semantic-analyzer`) already handles `use_module`/`require_module` via the `module_runtime_alias` function — go-to-definition works.
2. **Completion provider** (`perl-lsp-completion`) already handles these via the same `module_runtime_alias` pattern — completions work.
3. **Document links** (`perl-lsp-rs-core/src/providers/document_links/mod.rs`) does NOT handle these — no clickable links.

The `parse_module_import_head` function in `perl-module` intentionally rejects function-call forms (verified by test `rejects_function_call_use_module`), because it is designed for `use`/`require` **statements**, not function calls. The function-call forms must be handled separately in document-links, following the same text-based pattern as the existing inline `require "path"` detection (lines 61–87).

## Decision

Add text-based detection of `use_module('...')` and `require_module('...')` function calls to the `compute_links` function in `perl-lsp-rs-core/src/providers/document_links/mod.rs`.

The implementation will:
- Detect `use_module(...)`, `require_module(...)`, `Module::Runtime::use_module(...)`, and `Module::Runtime::require_module(...)` patterns
- Extract only static string literal arguments (single or double quoted)
- Exclude matches within Perl comments (`# ...`)
- Emit module-type document links with `type: "module"` for the extracted module name

## Consequences

### Benefits
- Users can navigate to modules loaded via `use_module`/`require_module` by clicking on them
- Follows existing architectural pattern — the inline `require "path"` detection is the exact precedent
- Low risk — no new dependencies, one crate modified, follows established code patterns
- Orthogonal to semantic analysis — document-links and semantic analysis serve different purposes

### Tradeoffs
- Text-based detection cannot resolve dynamic module names (`use_module($variable)`) — these are correctly left unlinked
- Commented-out `use_module` calls will be correctly excluded (comment context detection)
- Does not modify `perl-module` crate — which is correct, since `parse_module_import_head` should not handle function-call forms

### Risks
1. **False positives**: A string containing `use_module` but not actually calling the function could be matched — mitigated by requiring the pattern to be `use_module(` with a valid string literal following
2. **Performance**: Adding another per-line pattern — mitigated by O(n) with small constant factor; document-links scan is already per-line

## Alternatives Considered

### Alternative 1: Extend `perl-module` crate to handle function-call forms
**Rejected because**: The existing test `rejects_function_call_use_module` explicitly documents that `parse_module_import_head` should NOT handle function-call forms. This is correct — `parse_module_import_head` parses `use`/`require` **statements**, not function calls. Mixing concerns would violate single-responsibility principle.

### Alternative 2: Reuse `module_runtime_alias` from semantic analyzer in document-links
**Rejected because**: The `module_runtime_alias` function operates on an AST with full semantic context. The document-links crate intentionally maintains a lightweight, dependency-free design to serve the LSP protocol. Introducing AST-dependent logic into document-links would violate the architectural separation.

### Alternative 3: Leave as-is (won't fix)
**Rejected because**: The issue identifies a real, bounded usability gap. The fix is low-risk and follows established patterns.

## Related Issues
- GitHub Issue #3415: Import/Export Gap: Module::Runtime support missing
- Issue #3409 (Exporter support) — related to import tracking
