//! High-level IR lowered from the parser AST.
//!
//! HIR is the first compiler-substrate layer above raw parser nodes. It keeps
//! stable language constructs, parser anchors, source ranges, and conservative
//! context placeholders without changing LSP provider behavior.

mod lower;
mod model;

pub use lower::lower_ast;
pub use model::{
    AstAnchor, HirFile, HirId, HirItem, HirKind, MethodDecl, PackageDecl, RecoveryConfidence,
    RequireDecl, SubDecl, UseDecl, VariableBinding, VariableDecl,
};
