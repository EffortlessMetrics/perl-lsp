# CLAUDE.md - perl-ts-partial-ast

## Crate Overview

- **Version**: 0.1.0 (publish = false)
- **Tier**: Tree-sitter microcrate (`perl-ts-*` family)
- **Purpose**: Extends the Perl AST with nodes for partial parses, anti-pattern
  detection, phase-aware heredoc handling, and tree-sitter compatibility.

## Commands

```bash
cargo build -p perl-ts-partial-ast
cargo test -p perl-ts-partial-ast
cargo clippy -p perl-ts-partial-ast
cargo doc -p perl-ts-partial-ast --no-deps
```

## Architecture

### Dependencies

- `perl-parser-pest` -- Pest grammar parser, provides `AstNode` and `PureRustPerlParser`
- `perl-ts-heredoc-analysis` -- Anti-pattern detector, dynamic delimiter recovery
- `regex` -- Phase block pattern matching (LazyLock compiled)
- `pest` -- Parser combinator types used in recovery
- `serde_json` -- JSON serialization for tree-sitter node output

### Key Types and Modules

| Module | Key Types | Purpose |
|--------|-----------|---------|
| `partial_parse_ast` | `ExtendedAstNode`, `ExtendedAstBuilder`, `DynamicPart`, `RuntimeContext`, `RecoveryState` | AST nodes for partial/problematic parses with diagnostic collection |
| `phase_aware_parser` | `PhaseAwareParser`, `PerlPhase`, `PhaseTransition`, `PhaseAction`, `DeferredHeredoc` | Tracks BEGIN/CHECK/INIT/END/eval/use phases; decides parse vs defer vs warn |
| `understanding_parser` | `UnderstandingParser`, `ParseResult` | Combines Pest parsing with anti-pattern detection and chunk-based error recovery |
| `tree_sitter_adapter` | `TreeSitterAdapter`, `TreeSitterOutput`, `TreeSitterNode`, `EdgeCaseNodeType` | Converts `ExtendedAstNode` to tree-sitter-compatible AST with separate diagnostics |
| `edge_case_handler` | `EdgeCaseHandler`, `EdgeCaseAnalysis`, `EdgeCaseConfig`, `RecommendedAction` | Unified interface orchestrating all subsystems; generates reports and recommendations |

### Data Flow

1. `EdgeCaseHandler::analyze()` orchestrates the pipeline:
   - Anti-pattern detection via `AntiPatternDetector`
   - Phase transition analysis via `PhaseAwareParser`
   - Dynamic delimiter scanning via `DynamicDelimiterRecovery`
   - Parsing with recovery via `UnderstandingParser`
   - Recommendation generation

2. `UnderstandingParser` attempts normal Pest parsing first; on failure it
   does chunk-based recovery, skipping anti-pattern regions and building
   `ExtendedAstNode::PartialParse` or `Unparseable` nodes.

3. `TreeSitterAdapter::convert_to_tree_sitter()` maps the extended AST into
   `TreeSitterNode` with proper error/missing flags and diagnostic codes
   (PERL101-PERL107).

## Usage

```rust
use perl_ts_partial_ast::edge_case_handler::{EdgeCaseHandler, EdgeCaseConfig};

let mut handler = EdgeCaseHandler::new(EdgeCaseConfig::default());
let analysis = handler.analyze(perl_source_code);
let report = handler.generate_report(&analysis);
```

## Important Notes

- All regex patterns use `LazyLock` for thread-safe one-time compilation.
- `ExtendedAstNode::to_sexp()` provides S-expression output compatible with
  tree-sitter's `--debug` format.
- Tests use `perl_tdd_support::must` helper instead of `unwrap()`.
- The crate has `publish = false` -- it is workspace-internal only.
