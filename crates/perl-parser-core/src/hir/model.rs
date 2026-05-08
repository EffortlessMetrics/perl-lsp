//! HIR data model.

use crate::SourceLocation;

/// Stable identifier for a HIR item within one lowered file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct HirId {
    index: u32,
}

impl HirId {
    /// Create an identifier from a zero-based lowering index.
    #[inline]
    pub const fn from_index(index: u32) -> Self {
        Self { index }
    }

    /// Return the zero-based lowering index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Stable identifier for a HIR scope frame within one lowered file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct HirScopeId {
    index: u32,
}

impl HirScopeId {
    /// Create a scope identifier from a zero-based lowering index.
    #[inline]
    pub const fn from_index(index: u32) -> Self {
        Self { index }
    }

    /// Return the zero-based lowering index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Stable identifier for a HIR binding within one lowered file.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct HirBindingId {
    index: u32,
}

impl HirBindingId {
    /// Create a binding identifier from a zero-based lowering index.
    #[inline]
    pub const fn from_index(index: u32) -> Self {
        Self { index }
    }

    /// Return the zero-based lowering index.
    #[inline]
    pub const fn index(self) -> u32 {
        self.index
    }
}

/// Parser AST location that produced a HIR item.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct AstAnchor {
    /// Parser AST node kind name.
    pub node_kind: &'static str,
    /// Full AST node source range.
    pub range: SourceLocation,
    /// Precise name range when the AST exposes one.
    pub name_range: Option<SourceLocation>,
}

/// Recovery quality for a lowered HIR item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum RecoveryConfidence {
    /// Lowered from a normally parsed AST node.
    Parsed,
    /// Lowered from a parser recovery wrapper with a partial valid tree.
    Recovered,
    /// Lowered from a partially known or placeholder AST shape.
    Partial,
    /// Lowering could not classify recovery confidence yet.
    Unknown,
}

/// HIR for one parsed file.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct HirFile {
    /// Items lowered in stable depth-first source order.
    pub items: Vec<HirItem>,
    /// Scope and binding graph lowered beside HIR items.
    pub scope_graph: ScopeGraph,
}

impl HirFile {
    /// Return true when no HIR items were lowered.
    #[inline]
    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }
}

/// One lowered HIR item with common metadata required by compiler layers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct HirItem {
    /// Stable item id for this file.
    pub id: HirId,
    /// Lowered language construct.
    pub kind: HirKind,
    /// Source range for the construct.
    pub range: SourceLocation,
    /// Parser AST anchor for this item.
    pub anchor: AstAnchor,
    /// Recovery quality inherited from parser recovery.
    pub recovery_confidence: RecoveryConfidence,
    /// Package context known at lowering time.
    pub package_context: Option<String>,
    /// Scope context known at lowering time.
    pub scope_context: Option<HirScopeId>,
}

/// HIR-local scope graph for compiler-substrate proof.
///
/// The graph is intentionally parser-core-local. Later compiler fact export can
/// map these ids to `perl-semantic-facts` ids without changing provider
/// behavior in this first scope/pad slice.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct ScopeGraph {
    /// Scope frames in stable creation order.
    pub scopes: Vec<ScopeFrame>,
    /// Bindings in stable declaration order.
    pub bindings: Vec<Binding>,
    /// Variable references observed while lowering.
    pub references: Vec<BindingReference>,
}

impl ScopeGraph {
    /// Return the root file scope, when present.
    #[inline]
    pub fn root_scope(&self) -> Option<&ScopeFrame> {
        self.scopes.first()
    }
}

/// One lexical/package scope frame.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ScopeFrame {
    /// Stable scope id.
    pub id: HirScopeId,
    /// Parent scope id, absent for the file scope.
    pub parent: Option<HirScopeId>,
    /// Scope category.
    pub kind: ScopeKind,
    /// Source range covered by the scope.
    pub range: SourceLocation,
    /// Package context active for this scope, when known.
    pub package_context: Option<String>,
}

/// Scope frame category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum ScopeKind {
    /// Whole-file root scope.
    File,
    /// Package context scope.
    Package,
    /// Plain block scope.
    Block,
    /// Subroutine pad scope.
    Subroutine,
    /// Method pad scope.
    Method,
    /// Signature parameter scope.
    Signature,
    /// Legacy `format` declaration scope.
    Format,
    /// Dynamic/string eval scope boundary.
    EvalString,
    /// Compile-time phase block scope, such as `BEGIN`.
    PhaseBlock,
}

/// Compiler binding produced from a HIR declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Binding {
    /// Stable binding id.
    pub id: HirBindingId,
    /// Scope that owns this binding.
    pub scope_id: HirScopeId,
    /// Variable sigil.
    pub sigil: String,
    /// Variable name without sigil.
    pub name: String,
    /// Source range of the binding declaration token.
    pub range: SourceLocation,
    /// Storage class represented by the declaration.
    pub storage: StorageClass,
    /// Package context active for this binding, when known.
    pub package_context: Option<String>,
    /// HIR item that declared this binding.
    pub declaration_item: Option<HirId>,
    /// Earlier visible binding shadowed by this declaration, when known.
    pub shadows: Option<HirBindingId>,
}

/// Storage class represented by a binding.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum StorageClass {
    /// Lexical `my` variable.
    LexicalMy,
    /// Persistent lexical `state` variable.
    LexicalState,
    /// `our` package variable made lexically visible.
    PackageOur,
    /// `local` package variable localization.
    LocalizedPackage,
    /// Signature parameter binding.
    Parameter,
    /// Method invocant binding.
    MethodInvocant,
    /// Implicit lexical binding such as `$_`.
    Implicit,
    /// Package global observed without a lexical binding.
    PackageGlobal,
}

/// Variable reference and its lexical binding resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BindingReference {
    /// Scope containing the reference.
    pub scope_id: HirScopeId,
    /// Variable sigil.
    pub sigil: String,
    /// Variable name without sigil.
    pub name: String,
    /// Source range for the reference token.
    pub range: SourceLocation,
    /// Resolved binding, if one was visible in the scope chain.
    pub resolved_binding: Option<HirBindingId>,
}

/// First-slice HIR constructs.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum HirKind {
    /// `package Foo;` or block package declaration.
    PackageDecl(PackageDecl),
    /// `sub foo { ... }` declaration.
    SubDecl(SubDecl),
    /// `method foo { ... }` declaration.
    MethodDecl(MethodDecl),
    /// `use Module ...;` declaration.
    UseDecl(UseDecl),
    /// `require Module;` call recognized as a compile-time declaration shape.
    RequireDecl(RequireDecl),
    /// `my`, `our`, `state`, or `local` variable declaration.
    VariableDecl(VariableDecl),
    /// Function-like call expression shell.
    CallExpr(CallExpr),
    /// Method-call expression shell.
    MethodCallExpr(MethodCallExpr),
    /// Indirect-object method-call expression shell.
    IndirectCallExpr(IndirectCallExpr),
    /// Bareword expression shell.
    BarewordExpr(BarewordExpr),
    /// Literal expression shell.
    LiteralExpr(LiteralExpr),
    /// Block expression shell without scope construction.
    BlockShell(BlockShell),
    /// Unsupported or intentionally dynamic Perl boundary.
    DynamicBoundary(DynamicBoundary),
}

impl HirKind {
    /// Canonical names for all first-slice HIR construct variants.
    ///
    /// Metrics and status generators should use this list instead of keeping a
    /// separate copy of the current HIR surface.
    pub const ALL_KIND_NAMES: &[&'static str] = &[
        "BarewordExpr",
        "BlockShell",
        "CallExpr",
        "DynamicBoundary",
        "IndirectCallExpr",
        "LiteralExpr",
        "MethodCallExpr",
        "MethodDecl",
        "PackageDecl",
        "RequireDecl",
        "SubDecl",
        "UseDecl",
        "VariableDecl",
    ];
}

/// Package declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct PackageDecl {
    /// Package name.
    pub name: String,
    /// Precise package-name source range.
    pub name_range: SourceLocation,
    /// Whether this declaration owns an inline block.
    pub has_block: bool,
}

/// Subroutine declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct SubDecl {
    /// Subroutine name, absent for anonymous subs.
    pub name: Option<String>,
    /// Precise subroutine-name source range when available.
    pub name_range: Option<SourceLocation>,
    /// Whether the declaration has a prototype.
    pub has_prototype: bool,
    /// Whether the declaration has a signature.
    pub has_signature: bool,
    /// Number of parsed attributes.
    pub attribute_count: usize,
}

/// Method declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MethodDecl {
    /// Method name.
    pub name: String,
    /// Whether the declaration has a signature.
    pub has_signature: bool,
    /// Number of parsed attributes.
    pub attribute_count: usize,
}

/// Use declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct UseDecl {
    /// Module or pragma name.
    pub module: String,
    /// Parsed import arguments.
    pub args: Vec<String>,
    /// Whether the parser classified the module as a source-filter risk.
    pub has_filter_risk: bool,
}

/// Require declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct RequireDecl {
    /// Statically recognized require target when available.
    pub target: Option<String>,
    /// Number of parser arguments on the underlying function call.
    pub arg_count: usize,
}

/// Variable declaration HIR payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VariableDecl {
    /// Scope/storage declarator: `my`, `our`, `state`, or `local`.
    pub declarator: String,
    /// Variables statically visible in the declaration.
    pub variables: Vec<VariableBinding>,
    /// Number of parsed attributes on the declaration.
    pub attribute_count: usize,
    /// Whether the declaration has an initializer expression.
    pub has_initializer: bool,
    /// Whether this came from a list declaration.
    pub is_list: bool,
}

/// One variable binding named by a declaration.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VariableBinding {
    /// Variable sigil.
    pub sigil: String,
    /// Variable name without sigil.
    pub name: String,
    /// Source range for the variable token.
    pub range: SourceLocation,
}

/// Function-like call shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CallExpr {
    /// Callee name, or parser sentinel for dynamic call forms.
    pub name: String,
    /// Number of parsed arguments.
    pub arg_count: usize,
    /// Parser-observed call shape.
    pub form: CallForm,
}

/// Parser-observed call shape.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CallForm {
    /// A named function call such as `foo(...)`.
    NamedFunction,
    /// A coderef/dynamic callee call such as `$callback->(...)`.
    Coderef,
}

/// Method-call shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct MethodCallExpr {
    /// Method name.
    pub method: String,
    /// Number of parsed arguments.
    pub arg_count: usize,
    /// Parser AST kind for the receiver expression.
    pub object_kind: &'static str,
}

/// Indirect-object call shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct IndirectCallExpr {
    /// Method name.
    pub method: String,
    /// Number of parsed arguments.
    pub arg_count: usize,
    /// Parser AST kind for the receiver/class expression.
    pub object_kind: &'static str,
}

/// Bareword expression shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BarewordExpr {
    /// Bareword text as parsed.
    pub name: String,
}

/// Literal expression shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct LiteralExpr {
    /// Literal category.
    pub kind: LiteralKind,
    /// Preserved value for compact scalar literals.
    pub value: Option<String>,
    /// Whether the literal can interpolate variables.
    pub interpolated: Option<bool>,
    /// Element count for aggregate literals.
    pub element_count: Option<usize>,
    /// Pair count for hash literals.
    pub pair_count: Option<usize>,
}

/// Literal category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum LiteralKind {
    /// Numeric literal.
    Number,
    /// String literal.
    String,
    /// `undef`.
    Undef,
    /// Array/list literal.
    Array,
    /// Hash literal.
    Hash,
}

/// Block shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BlockShell {
    /// Number of parsed statements directly inside the block.
    pub statement_count: usize,
}

/// Dynamic-boundary shell payload.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DynamicBoundary {
    /// Boundary category.
    pub kind: DynamicBoundaryKind,
    /// Short human-readable reason for the boundary.
    pub reason: String,
}

/// Dynamic-boundary category.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DynamicBoundaryKind {
    /// Coderef/dynamic callee call through `->()`.
    CoderefCall,
    /// `eval` whose body is not a statically parsed block.
    EvalExpression,
    /// `do` whose body is not a statically parsed block.
    DoExpression,
}
