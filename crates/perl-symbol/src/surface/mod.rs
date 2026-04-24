//! Projection layer: derives stable, reusable symbol-bearing views from the Perl AST.
//!
//! This module sits at the seam between the *syntax model* (`perl-ast`) and *IDE
//! features* (semantic analyzer, workspace index, navigation, rename, workspace
//! symbols, call hierarchy).  It converts raw AST nodes into well-typed
//! projection structs that consumers can work with without re-implementing
//! per-node pattern matching.
//!
//! # Design goals
//!
//! - **No `perl-parser-core`** dependency — depends only on `perl-ast` and
//!   the sibling `types` module inside this crate.
//! - **Single extraction pass** — `extract_symbol_decls` walks the entire tree
//!   once and returns all declaration sites.
//! - **Phased scope** — ships `SymbolDecl` and a narrow phase-1 `SymbolRef`;
//!   richer `SymbolRef` kinds and `CallSite` will follow in later phases.
//!
//! # Quick start
//!
//! ```rust,ignore
//! use perl_symbol::surface::extract_symbol_decls;
//!
//! // `ast` is a `perl_ast::Node` produced by the parser
//! let decls = extract_symbol_decls(&ast, Some("MyPackage"));
//! for d in &decls {
//!     println!("{} {:?} @ {:?}", d.qualified_name, d.kind, d.full_span);
//! }
//! ```

pub mod decl;
pub mod r#ref;

pub use decl::{SymbolDecl, extract_symbol_decls};
pub use r#ref::{SymbolRef, extract_symbol_refs};
