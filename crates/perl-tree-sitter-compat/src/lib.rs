//! `perl-tree-sitter-compat` — tree-sitter-compatible output over the native
//! Perl parser.
//!
//! This is an **adapter, not a re-implementation**: it projects the native
//! recursive-descent parser's AST into tree-sitter's shapes — named nodes with
//! kinds, byte and point ranges, S-expression rendering, and highlight
//! captures — so editors and tooling built for the tree-sitter ecosystem can
//! consume the native parser's output without a separate grammar (see
//! [PLSP-ADR-0006](../../../docs/adr/PLSP-ADR-0006-perl-workspace-core-facts-substrate.md)
//! PR 9).
//!
//! It sits at the same layer as the other substrate consumers: it depends on
//! the leaf parser (`perl-parser-core`) and the LSP-free substrate
//! (`perl-workspace-core`, for its UTF-8 line index), never the editor runtime.
//!
//! # Quick start
//!
//! ```
//! use perl_tree_sitter_compat::{parse_to_tree, to_sexp, highlights};
//!
//! let tree = parse_to_tree("use strict;\nmy $x = 42;\n")?;
//! println!("{}", to_sexp(&tree));       // (program (use) ...)
//! for h in highlights(&tree) {
//!     println!("{}..{} @{}", h.start_byte, h.end_byte, h.capture);
//! }
//! # Ok::<(), perl_tree_sitter_compat::TreeError>(())
//! ```
//!
//! # Scope
//!
//! First slice: named-node tree + S-expression + a node-granular highlight
//! capture map. The native AST exposes only named nodes, so anonymous
//! token/punctuation nodes are not surfaced; token-precise highlighting and
//! locals/injection capture queries are documented follow-ups.

#![warn(missing_docs)]

pub mod convert;
pub mod highlight;
pub mod node;
pub mod sexp;

pub use convert::{TreeError, parse_to_tree, to_ts_node};
pub use highlight::{Highlight, capture_for, highlights};
pub use node::{TsNode, TsPoint, pascal_to_snake};
pub use sexp::{to_sexp, to_sexp_pretty};
