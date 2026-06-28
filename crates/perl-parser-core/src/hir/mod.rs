//! High-level IR lowered from the parser AST.
//!
//! HIR is the first compiler-substrate layer above raw parser nodes. It keeps
//! stable language constructs, parser anchors, source ranges, and scope graph
//! proof data without changing LSP provider behavior.

mod body;
pub mod disposition;
mod lower;
mod model;

pub use body::{
    AccessMode, Arena, AssignMode, BinaryOp, BodyOwner, BodyOwnerKind, BodySourceMap,
    DeclStorageClass, HirBlock, HirBlockId, HirBody, HirBodyId, HirExpr, HirExprId, HirStmt,
    HirStmtId, HirVariable, Sigil, UnaryMode, VariableKind, lower_body,
};
pub use lower::lower_ast;
pub use model::{
    AstAnchor, BarewordExpr, BarewordFact, BarewordRole, BarewordTable, Binding, BindingReference,
    BlockShell, BranchKeyword, BranchShell, COMPILE_EFFECT_MODEL_VERSION, CallExpr, CallForm,
    CompileConfidence, CompileDirective, CompileDirectiveAction, CompileDirectiveKind,
    CompileEffect, CompileEffectFactKind, CompileEffectKind, CompileEffectSourceKind,
    CompileEnvironment, CompileEnvironmentBoundary, CompileEnvironmentBoundaryKind, CompilePhase,
    CompilePhaseBlock, CompileProvenance, ControlTransfer, ControlTransferKind, DynamicBoundary,
    DynamicBoundaryKind, ExportDeclaration, ExportDeclarationKind, FrameworkAdapterKind,
    FrameworkAdapterRegistry, FrameworkDynamicBoundaryFact, FrameworkExportedSymbolFact,
    FrameworkExportedSymbolKind, FrameworkFactGraph, GlobSlot, GlobSlotKind, GlobSlotSource,
    HIR_BODY_MODEL_VERSION, HirBindingId, HirFile, HirId, HirItem, HirKind, HirScopeId,
    IncRootAction, IncRootFact, IncRootKind, IndirectCallExpr, InheritanceSource, LiteralExpr,
    LiteralKind, LoopKind, LoopShell, MethodCallExpr, MethodDecl, ModuleRequest, ModuleRequestKind,
    ModuleResolutionCacheInvalidation, ModuleResolutionCacheKey, ModuleResolutionCacheRootKey,
    ModuleResolutionCandidate, ModuleResolutionCandidatePathState, ModuleResolutionCandidateRoot,
    ModuleResolutionCandidateStatus, ModuleResolutionRoot, ModuleResolutionStatus, PackageDecl,
    PackageInheritanceEdge, PackageStash, PragmaArgumentKind, PragmaEffect, PragmaStateFact,
    PrototypeFact, PrototypeTable, RecoveryConfidence, RequireDecl, ScopeFrame, ScopeGraph,
    ScopeKind, StashConfidence, StashDynamicBoundary, StashDynamicBoundaryKind, StashGraph,
    StashProvenance, StatementModifierKind, StatementModifierShell, StorageClass, SubDecl, UseDecl,
    VariableBinding, VariableDecl,
};
