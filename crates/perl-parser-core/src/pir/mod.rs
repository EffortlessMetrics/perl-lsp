//! PIR v0: a source-anchored tooling intermediate representation.
//!
//! PIR is lowered from [`HirFile`](crate::hir::HirFile). It models data access,
//! calls, and control flow for editor tooling and static analysis without
//! executing Perl. PIR v0 is a compiler-substrate data layer only: it does not
//! replace HIR, does not promote provider behavior, does not claim determinism,
//! and does not run real Perl.
//!
//! See [`PLSP-SPEC-0025`](../../../../docs/specs/PLSP-SPEC-0025-pir-v0.md) for
//! the authoritative contract.
//!
//! # Quick start
//!
//! ```rust
//! use perl_parser_core::Parser;
//! use perl_parser_core::hir::lower_ast;
//! use perl_parser_core::pir::lower_hir;
//!
//! let mut parser = Parser::new("my $x = foo();");
//! let output = parser.parse_with_recovery();
//! let hir = lower_ast(&output.ast);
//! let pir = lower_hir(&hir);
//!
//! // Every source-derived node preserves a source anchor.
//! assert!(pir.nodes.iter().all(|n| n.source_anchor.is_anchored()));
//! // Provider behavior never changes from lowering alone.
//! assert!(!pir.receipt.provider_behavior_changed);
//! ```

mod lower;
mod model;

pub use lower::{lower_hir, lower_hir_with_identity};
pub use model::{
    LexicalName, PIR_RECEIPT_VERSION, PirAnchorCoverage, PirAnchorKind, PirCallee, PirContext,
    PirDynamicBoundaryKind, PirEdge, PirEdgeKind, PirGraph, PirId, PirLoweringMode, PirMethod,
    PirNode, PirOperation, PirReceipt, PirReceiver, PirSourceAnchor, SymbolName,
};
