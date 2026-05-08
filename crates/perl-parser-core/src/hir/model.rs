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
    /// Scope context placeholder for later scope-graph work.
    pub scope_context: Option<HirId>,
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
