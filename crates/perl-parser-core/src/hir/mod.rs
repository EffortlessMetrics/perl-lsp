//! High-level IR lowered from the parser AST.
//!
//! HIR is the first compiler-substrate layer above raw parser nodes. It keeps
//! stable language constructs, parser anchors, source ranges, and scope graph
//! proof data without changing LSP provider behavior.

mod lower;
mod model;

pub use lower::lower_ast;
pub use model::{
    AstAnchor, BarewordExpr, Binding, BindingReference, BlockShell, CallExpr, CallForm,
    DynamicBoundary, DynamicBoundaryKind, GlobSlot, GlobSlotKind, GlobSlotSource, HirBindingId,
    HirFile, HirId, HirItem, HirKind, HirScopeId, IndirectCallExpr, InheritanceSource, LiteralExpr,
    LiteralKind, MethodCallExpr, MethodDecl, PackageDecl, PackageInheritanceEdge, PackageStash,
    RecoveryConfidence, RequireDecl, ScopeFrame, ScopeGraph, ScopeKind, StashConfidence,
    StashDynamicBoundary, StashDynamicBoundaryKind, StashGraph, StashProvenance, StorageClass,
    SubDecl, UseDecl, VariableBinding, VariableDecl,
};
