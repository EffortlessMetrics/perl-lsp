# ADR: Sub::Exporter Support Architecture

**Work Item**: work-9c61d264
**Issue**: [GitHub #3413 - Import/Export Gap: Sub::Exporter support missing](https://github.com/EffortlessMetrics/perl-lsp/issues/3413)

## Status

**Proposed** — Pending implementation team review.

## Context

The Perl LSP lacks support for Sub::Exporter, a widely-used Perl module that provides sophisticated export configuration via hashref-based APIs. Sub::Exporter is used by critical Perl ecosystem modules including Moose, Moo, Test::More, Dist::Zilla, and Catalyst components.

This gap causes broken goto-definition and completion for any Perl code using these modules. Users cannot navigate to definitions of symbols imported via Sub::Exporter configs, nor can they get autocompletion for available exports.

### Root Cause Analysis

The `perl-ast` crate's `NodeKind::Use` struct uses `args: Vec<String>` which flattens all import arguments into a flat list of strings:

```rust
Use {
    module: String,
    args: Vec<String>,       // Loses Sub::Exporter hash structure
    has_filter_risk: bool,
}
```

When Sub::Exporter patterns like `{ exports => [qw(foo bar)] }` are parsed, the structural information is lost — the hash becomes a sequence of tokens like `{`, `exports`, `=>`, `[`, `qw`, `(`, `foo`, `bar`, `)`, `}`.

### Architectural Constraints

1. **Tree-sitter is NOT part of the LSP's critical path.** The TECHNICAL_VISION.md (section 4.2, "Technology Radar") categorizes tree-sitter as **TRIAL** with note "Not on the LSP's critical path." The LSP uses `perl-parser-core` — a native Rust recursive descent parser.

2. **Backward compatibility required.** The project is approaching v1.0 stability contract. Changes to fundamental AST representations must not break existing consumers.

3. **Two distinct code paths exist:**
   - `collect_import_symbols()` — handles `use` statement args (flat `Vec<String>`)
   - `collect_node_import_symbols()` — handles `MethodCall` args like `Module->import(...)` (actual `Node` objects)

## Decision

We adopt a **hybrid approach** combining backward-compatible AST enhancement with targeted token re-parsing:

### Approach: Backward-Compatible AST Enhancement + Token Re-parsing

**Phase 1: Detection (Token Pattern Matching)**
- Detect Sub::Exporter patterns in `args: Vec<String>` by identifying hash-like structures
- Pattern: args starting with `{` followed by `exports` or `-setup` key
- Emit a marker/attribute indicating "complex exports" for Sub::Exporter modules

**Phase 2: Basic Symbol Extraction (Token Re-parsing)**
- When Sub::Exporter pattern detected, re-parse the token stream for `exports => [qw(...)]`
- Use the existing structured `HashLiteral` node type concept for token extraction
- Support the common patterns: simple exports array, group definitions

**Phase 3: Structured Args Field (Future Enhancement)**
- Add `structured_args: Option<Node>` to `NodeKind::Use` for future clean implementation
- This is backward compatible — existing code using `args: Vec<String>` continues to work
- Enables future removal of fragile token re-parsing once perl-ast consumers are updated

### What This Solves

1. **Detection**: Identifies Sub::Exporter modules without requiring tree-sitter integration
2. **Basic extraction**: Extracts symbols from `exports => [qw(foo bar)]` patterns
3. **Group support**: Resolves `:default`, `:all` tags from `groups => {...}` definitions
4. **Renaming support**: Handles `-as => 'renamed'` patterns for completion

### What This Does NOT Solve (Out of Scope)

- Coderef-based exporters (require runtime evaluation)
- Collector/generator patterns
- Sub::Exporter::Composable
- Full group resolution for all custom groups

## Alternatives Considered

### Alternative 1: Tree-sitter Integration
The original plan proposed using tree-sitter nodes because tree-sitter preserves structure.

**Rejected because**: Tree-sitter is not part of the LSP's critical path. Adding it as a Sub::Exporter-specific dependency would create architectural inconsistency and fragile external dependency. The README explicitly states tree-sitter-perl is "not on the LSP's critical path."

### Alternative 2: Pure Token Re-parsing (No AST Changes)
Re-parse `args: Vec<String>` back into structured form without any AST modifications.

**Rejected because**: Fragile and error-prone. Nested structures, escape sequences, and edge cases in token representation are difficult to handle correctly. Would create technical debt that is hard to maintain.

### Alternative 3: Hardcoded Module Export Lists
For known modules (Moose, Moo, etc.), use hardcoded export lists.

**Rejected because**: Does not solve the general problem — only works for explicitly known modules. High maintenance burden as CPAN modules evolve. Does not help users of arbitrary Sub::Exporter-using modules.

### Alternative 4: Full `NodeKind::Use` Refactor
Replace `args: Vec<String>` with `Option<Node>` for structured args throughout.

**Rejected because**: Breaking change that affects many consumers. Not backward compatible. Risk of breaking existing Exporter support. Better to add optional field for gradual migration.

## Consequences

### Positive Consequences

1. **Works within existing architecture** — Uses `perl-parser-core` and `perl-ast` without introducing external dependencies
2. **Backward compatible** — Existing `args: Vec<String>` remains; no breaking changes to API consumers
3. **No tree-sitter dependency** — Respects codebase's stated technology direction
4. **Enables future cleanup** — The optional `structured_args` field enables gradual migration without breaking existing code
5. **Supports critical CPAN ecosystem** — Enables proper LSP support for Moose, Moo, Test::More, Dist::Zilla, Catalyst

### Negative Consequences / Tradeoffs

1. **Token re-parsing is inherently fragile** — Limited to patterns that can be reliably detected and extracted from flat token strings
2. **Coderef exporters not supported** — Static analysis cannot determine coderef exports without runtime evaluation
3. **Initial scope limited** — Only common Sub::Exporter patterns (simple exports, basic groups, basic renaming)
4. **Multiple code paths affected** — Changes needed in `perl-semantic-analyzer`, `perl-lsp-completion`, potentially `perl-ast`
5. **Performance consideration** — Token re-parsing adds some overhead for Sub::Exporter detection

### Risk Mitigation

1. **Start with prototype** — Implement minimal proof-of-concept before committing to full implementation
2. **Comprehensive tests** — Add tests for both Sub::Exporter patterns and existing Exporter patterns
3. **Targeted changes** — Only affect Sub::Exporter cases; existing Exporter paths remain unchanged
4. **Documentation** — Clearly document supported vs. unsupported Sub::Exporter patterns

## Implementation Notes

### Code Path Clarification

For `use` statement completion/navigation:
- Entry point: `collect_import_symbols` with flat `Vec<String>` args
- Detection: Pattern match on tokens for `{`, `exports`, `=>`, `[`, `qw` sequence
- Extraction: Re-parse relevant tokens for symbol extraction

For `MethodCall` (e.g., `Module->import({ ... })`):
- Entry point: `collect_node_import_symbols` with actual `Node` objects
- Can be enhanced to handle `HashLiteral` nodes directly (this is the cleaner path)
- `collect_node_import_symbols` should be extended to handle `HashLiteral` variant

### Branch State

The work item specifies branch `feat/work-9c61d264/import/export-gap:-sub::exporter-support`. The repo is currently on a different branch. Implementation must create/switch to the specified feature branch before committing changes.