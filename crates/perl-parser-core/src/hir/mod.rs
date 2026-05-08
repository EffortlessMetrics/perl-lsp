//! High-level IR lowered from the parser AST.
//!
//! HIR is the first compiler-substrate layer above raw parser nodes. It keeps
//! stable language constructs, parser anchors, source ranges, and conservative
//! context placeholders without changing LSP provider behavior.

mod lower;
mod model;

pub use lower::lower_ast;
pub use model::{
    AstAnchor, BarewordExpr, BlockShell, CallExpr, CallForm, DynamicBoundary, DynamicBoundaryKind,
    HirFile, HirId, HirItem, HirKind, IndirectCallExpr, LiteralExpr, LiteralKind, MethodCallExpr,
    MethodDecl, PackageDecl, RecoveryConfidence, RequireDecl, SubDecl, UseDecl, VariableBinding,
    VariableDecl,
};
