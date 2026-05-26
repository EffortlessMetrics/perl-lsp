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
//! - **Scoped rollout** — ships `SymbolDecl` and a phase-1 `SymbolRef`
//!   extractor. `CallSite` remains a later phase.
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
/// Adapters that translate `surface` declarations and references into the
/// `perl-semantic-facts` graph model (anchors, entities, occurrences, edges).
pub mod facts;
pub mod r#ref;

pub use decl::{SymbolDecl, extract_symbol_decls};
pub use facts::{
    SymbolDeclSemanticFacts, SymbolRefSemanticFacts, UnsupportedDeclFact,
    symbol_decls_to_semantic_facts, symbol_refs_to_semantic_facts,
};
pub use r#ref::{SymbolRef, SymbolRefKind, extract_symbol_refs};
