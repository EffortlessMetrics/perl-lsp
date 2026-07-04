# perl-tree-sitter-compat

**Tree-sitter-compatible output over the native Perl parser** — an *adapter*,
not a re-implementation. It projects the native recursive-descent AST into
tree-sitter's shapes so editors and tooling built for the tree-sitter ecosystem
can consume the native parser's output without maintaining a separate grammar.

See [PLSP-ADR-0006](../../docs/adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md)
(PR 9) and [NATIVE_STACK_POLICY.md](../../docs/reference/NATIVE_STACK_POLICY.md).

```rust
use perl_tree_sitter_compat::{parse_to_tree, to_sexp, to_sexp_pretty, highlights};

let tree = parse_to_tree("package App;\nsub run { 1 }\n1;\n")?;

// tree-sitter S-expression (matches Node::to_sexp() shape):
println!("{}", to_sexp(&tree));          // (program (package) (subroutine ...) ...)
println!("{}", to_sexp_pretty(&tree));   // indented, multi-line

// syntax-highlight captures:
for h in highlights(&tree) {
    println!("{}..{} @{}", h.start_byte, h.end_byte, h.capture);
}
# Ok::<(), perl_tree_sitter_compat::TreeError>(())
```

## What it provides

- **`TsNode`** — a named node with `kind` (snake_cased from the native
  `NodeKind`), `start_byte`/`end_byte`, `start_point`/`end_point` (0-based
  row/column, column in UTF-8 bytes, matching tree-sitter's `Point`), and named
  `children`.
- **`to_sexp` / `to_sexp_pretty`** — named-node S-expression rendering (no field
  labels or anonymous nodes — see the `sexp` module docs), so tree-sitter test
  corpora that assert on named-node S-expressions can run against the native
  parser.
- **`highlights` / `capture_for`** — a node-granular highlight capture map
  (`keyword`, `function`, `variable`, `string`, `number`, …).

## Layering

Depends only on `perl-parser-core` (the leaf parser) and `perl-workspace-core`
(the LSP-free substrate, for its UTF-8 line index) — never the editor runtime.

## Scope (`publish = false`)

First slice per ADR-0006 PR 9. The native AST exposes only **named** nodes, so
anonymous token/punctuation nodes are not surfaced (a documented difference from
a full tree-sitter grammar). Token-precise highlighting and
locals/scopes/injection capture queries (POD/regex/heredoc injections) are
documented follow-ups.
