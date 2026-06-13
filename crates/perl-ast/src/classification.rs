//! Static classification metadata for [`NodeKind`] variants.
//!
//! This module answers the question **"what kind of node is this?"** at the
//! variant level, without looking at the node's position in the tree or at
//! the source text it covers. Consumers that need positional facts (is this
//! inside a heredoc body? inside POD? after `__DATA__`?) must layer those
//! checks on top using their own positional knowledge.
//!
//! # Design contract
//!
//! - **Variant-level only.** Every flag is determined by the `NodeKind`
//!   discriminant alone. The same `NodeKind::Heredoc { .. }` always returns
//!   the same flags regardless of where it appears in the AST.
//!
//! - **No wildcard arms.** Both `category()` and `flags()` use exhaustive
//!   `match self { ... }` expressions with no `_ =>` catch-all arm. Adding a
//!   new `NodeKind` variant is a compile error until the match is extended.
//!   This is the **drift guard**: consumers that pattern-match `NodeKind` can
//!   rely on classification staying current.
//!
//! - **`safe_for_breakpoint` semantics.** This flag answers the question
//!   *"can a breakpoint ever be set on this kind of node?"* at the variant
//!   level. A `true` value must be AND-ed by the DAP/LSP consumer with
//!   positional checks (e.g., is the cursor inside a heredoc body? Is the
//!   line inside a POD block?). The flag never means "always stop here" —
//!   it means "this kind of node is a valid candidate for breakpoint
//!   placement, pending positional validation."
//!
//! - **Invariant.** `recovery_artifact == true` implies
//!   `safe_for_breakpoint == false`. This is enforced by the table and
//!   verified by [`NodeKindFlags::validate`].
//!
//! # Usage
//!
//! ```rust
//! use perl_ast::NodeKind;
//! use perl_ast::classification::NodeKindCategory;
//!
//! let kind = NodeKind::FunctionCall { name: "print".to_string(), args: vec![] };
//! assert_eq!(kind.category(), NodeKindCategory::Expression);
//! assert!(kind.is_executable());
//! assert!(kind.safe_for_breakpoint());
//! ```

use crate::ast::NodeKind;

/// High-level semantic category for a [`NodeKind`] variant.
///
/// Each variant belongs to exactly one category. The category reflects the
/// dominant role of the construct in the Perl language, not its syntactic
/// surface form.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NodeKindCategory {
    /// The root program node.
    Program,
    /// A statement (flow control, returns, loop control).
    Statement,
    /// An expression (has a value; may have side effects).
    Expression,
    /// A declaration (introduces a name into some scope or symbol table).
    Declaration,
    /// A scoping construct that directly wraps a block (Block only).
    Scope,
    /// A pure literal value with no side effects.
    Literal,
    /// An operator or punctuation node (Ellipsis).
    Operator,
    /// A comment or documentation node (reserved for future use).
    CommentDoc,
    /// A synthetic recovery node produced by error recovery.
    Recovery,
    /// An uncategorized or unknown node (reserved for future use).
    Unknown,
}

/// Boolean flags describing the semantic role of a [`NodeKind`] variant.
///
/// All flags are determined statically from the variant discriminant alone.
/// See the module-level documentation for the precise semantics of each flag
/// and the invariants between them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NodeKindFlags {
    /// Code that can/will execute (methods, ops, side effects).
    ///
    /// Opposite of pure literals and declaration-only nodes.
    /// Conservative default: `false`. If wrong, breakpoints fail silently.
    pub executable: bool,

    /// Creates a new lexical scope (my vars, blocks, subs, class, package).
    ///
    /// Triggers scope analyzer stack push. Conservative default: `false`.
    /// If wrong, scope-bleeding bugs may result.
    pub introduces_scope: bool,

    /// Binds a name (`my $x`, `sub foo`, `use Module`, `package Foo`).
    ///
    /// Triggers symbol table insertion. Conservative default: `false`.
    /// Missing declarations lose symbols but do not crash.
    pub declares_symbol: bool,

    /// References a name (`$x`, `foo()`, `@arr`).
    ///
    /// Triggers use-def chain lookups. Conservative default: `false`.
    /// Missing refs lose cross-references but are safe.
    pub references_symbol: bool,

    /// Has `Vec` or `Box` children worth walking during AST traversal.
    ///
    /// Speeds up traversal filters that can skip leaf nodes. Conservative
    /// default: `false`. If wrong, traversal misses nodes but does not crash.
    pub contains_children: bool,

    /// Synthetic node produced by error recovery (`Error`, `Missing*`, `UnknownRest`).
    ///
    /// **Critical**: recovery nodes must NEVER be `safe_for_breakpoint`.
    /// Conservative default: `false`.
    pub recovery_artifact: bool,

    /// This kind of node can host a debugger breakpoint.
    ///
    /// **Static, variant-level.** Must be AND-ed with positional checks by
    /// the consumer. `true` means "this kind is a valid breakpoint candidate";
    /// it does not mean "always stop here".
    ///
    /// **Invariant**: if `recovery_artifact` is `true`, this must be `false`.
    /// Conservative default: `false`. Only `true` if proven safe.
    pub safe_for_breakpoint: bool,
}

impl NodeKindFlags {
    /// Verify the invariant: `recovery_artifact && safe_for_breakpoint` is forbidden.
    ///
    /// Returns `Ok(())` when the flags are internally consistent, or an error
    /// string describing the violation.
    pub fn validate(&self) -> Result<(), String> {
        if self.recovery_artifact && self.safe_for_breakpoint {
            Err("recovery_artifact and safe_for_breakpoint must not both be true".into())
        } else {
            Ok(())
        }
    }
}

// ────────────────────────────────────────────────────────────────────────────
// Helper macro for concise flag construction
// ────────────────────────────────────────────────────────────────────────────

macro_rules! flags {
    (
        exec=$exec:expr,
        scope=$scope:expr,
        decl=$decl:expr,
        refs=$refs:expr,
        children=$children:expr,
        recovery=$recovery:expr,
        bp=$bp:expr
    ) => {
        NodeKindFlags {
            executable: $exec,
            introduces_scope: $scope,
            declares_symbol: $decl,
            references_symbol: $refs,
            contains_children: $children,
            recovery_artifact: $recovery,
            safe_for_breakpoint: $bp,
        }
    };
}

impl NodeKind {
    /// Return the high-level [`NodeKindCategory`] for this variant.
    ///
    /// The match is exhaustive with no wildcard arm — adding a new
    /// `NodeKind` variant is a compile error until this function is updated.
    pub fn category(&self) -> NodeKindCategory {
        match self {
            NodeKind::Program { .. } => NodeKindCategory::Program,

            NodeKind::ExpressionStatement { .. }
            | NodeKind::Defer { .. }
            | NodeKind::Try { .. }
            | NodeKind::If { .. }
            | NodeKind::LabeledStatement { .. }
            | NodeKind::While { .. }
            | NodeKind::For { .. }
            | NodeKind::Foreach { .. }
            | NodeKind::Given { .. }
            | NodeKind::When { .. }
            | NodeKind::Default { .. }
            | NodeKind::StatementModifier { .. }
            | NodeKind::Return { .. }
            | NodeKind::LoopControl { .. }
            | NodeKind::Goto { .. } => NodeKindCategory::Statement,

            NodeKind::Variable { .. }
            | NodeKind::VariableWithAttributes { .. }
            | NodeKind::Assignment { .. }
            | NodeKind::Binary { .. }
            | NodeKind::Ternary { .. }
            | NodeKind::Unary { .. }
            | NodeKind::Diamond
            | NodeKind::Readline { .. }
            | NodeKind::Glob { .. }
            | NodeKind::Typeglob { .. }
            | NodeKind::ArrayLiteral { .. }
            | NodeKind::HashLiteral { .. }
            | NodeKind::Eval { .. }
            | NodeKind::Do { .. }
            | NodeKind::Tie { .. }
            | NodeKind::Untie { .. }
            | NodeKind::MethodCall { .. }
            | NodeKind::FunctionCall { .. }
            | NodeKind::IndirectCall { .. }
            | NodeKind::Match { .. }
            | NodeKind::Substitution { .. }
            | NodeKind::Transliteration { .. }
            | NodeKind::Identifier { .. } => NodeKindCategory::Expression,

            NodeKind::VariableDeclaration { .. }
            | NodeKind::VariableListDeclaration { .. }
            | NodeKind::Subroutine { .. }
            | NodeKind::Prototype { .. }
            | NodeKind::Signature { .. }
            | NodeKind::MandatoryParameter { .. }
            | NodeKind::OptionalParameter { .. }
            | NodeKind::SlurpyParameter { .. }
            | NodeKind::NamedParameter { .. }
            | NodeKind::Method { .. }
            | NodeKind::Package { .. }
            | NodeKind::Use { .. }
            | NodeKind::No { .. }
            | NodeKind::PhaseBlock { .. }
            | NodeKind::DataSection { .. }
            | NodeKind::Class { .. }
            | NodeKind::Format { .. } => NodeKindCategory::Declaration,

            NodeKind::Block { .. } => NodeKindCategory::Scope,

            NodeKind::Number { .. }
            | NodeKind::String { .. }
            | NodeKind::Heredoc { .. }
            | NodeKind::Regex { .. }
            | NodeKind::Undef => NodeKindCategory::Literal,

            NodeKind::Ellipsis => NodeKindCategory::Operator,

            NodeKind::Error { .. }
            | NodeKind::MissingExpression
            | NodeKind::MissingStatement
            | NodeKind::MissingIdentifier
            | NodeKind::MissingBlock
            | NodeKind::UnknownRest => NodeKindCategory::Recovery,
        }
    }

    /// Return the full [`NodeKindFlags`] for this variant.
    ///
    /// The match is exhaustive with no wildcard arm — adding a new
    /// `NodeKind` variant is a compile error until this function is updated.
    ///
    /// See [`NodeKindFlags`] for the precise semantics of each flag.
    ///
    /// # Invariant
    ///
    /// The returned flags always satisfy `flags.validate().is_ok()`.
    pub fn flags(&self) -> NodeKindFlags {
        //  Columns:       exec   scope  decl   refs   children  recovery  bp
        match self {
            NodeKind::Program { .. } => flags!(
                exec = false,
                scope = true,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::ExpressionStatement { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::VariableDeclaration { .. } => flags!(
                exec = true,
                scope = true,
                decl = true,
                refs = false,
                children = false,
                recovery = false,
                bp = true
            ),
            NodeKind::VariableListDeclaration { .. } => flags!(
                exec = true,
                scope = true,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Variable { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = true,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::VariableWithAttributes { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::Assignment { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Binary { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Ternary { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Unary { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Diamond => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = true
            ),
            NodeKind::Ellipsis => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Undef => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Readline { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Glob { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Typeglob { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = true,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Number { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::String { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::Heredoc { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::ArrayLiteral { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::HashLiteral { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::Block { .. } => flags!(
                exec = true,
                scope = true,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Eval { .. } => flags!(
                exec = true,
                scope = true,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Do { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Defer { .. } => flags!(
                exec = true,
                scope = true,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Try { .. } => flags!(
                exec = true,
                scope = true,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::If { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::LabeledStatement { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::While { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Tie { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Untie { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = false,
                recovery = false,
                bp = true
            ),
            NodeKind::For { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Foreach { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Given { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::When { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Default { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::StatementModifier { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Subroutine { .. } => flags!(
                exec = false,
                scope = true,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Prototype { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Signature { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::MandatoryParameter { .. } => flags!(
                exec = false,
                scope = false,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::OptionalParameter { .. } => flags!(
                exec = false,
                scope = false,
                decl = true,
                refs = true,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::SlurpyParameter { .. } => flags!(
                exec = false,
                scope = false,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::NamedParameter { .. } => flags!(
                exec = false,
                scope = false,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = false
            ),
            NodeKind::Method { .. } => flags!(
                exec = false,
                scope = true,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Return { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::LoopControl { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = true
            ),
            NodeKind::Goto { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::MethodCall { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::FunctionCall { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::IndirectCall { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Regex { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Match { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Substitution { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Transliteration { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = true,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Package { .. } => flags!(
                exec = true,
                scope = true,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Use { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::No { .. } => flags!(
                exec = true,
                scope = false,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::PhaseBlock { .. } => flags!(
                exec = true,
                scope = true,
                decl = false,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::DataSection { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Class { .. } => flags!(
                exec = false,
                scope = true,
                decl = true,
                refs = false,
                children = true,
                recovery = false,
                bp = true
            ),
            NodeKind::Format { .. } => flags!(
                exec = false,
                scope = false,
                decl = true,
                refs = false,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Identifier { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = true,
                children = false,
                recovery = false,
                bp = false
            ),
            NodeKind::Error { .. } => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = true,
                bp = false
            ),
            NodeKind::MissingExpression => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = true,
                bp = false
            ),
            NodeKind::MissingStatement => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = true,
                bp = false
            ),
            NodeKind::MissingIdentifier => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = true,
                bp = false
            ),
            NodeKind::MissingBlock => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = true,
                bp = false
            ),
            NodeKind::UnknownRest => flags!(
                exec = false,
                scope = false,
                decl = false,
                refs = false,
                children = false,
                recovery = true,
                bp = false
            ),
        }
    }

    // ────────────────────────────────────────────────────────────────────────
    // Convenience flag accessors
    // ────────────────────────────────────────────────────────────────────────

    /// Returns `true` if this node kind represents executable code.
    ///
    /// See [`NodeKindFlags::executable`].
    #[inline]
    pub fn is_executable(&self) -> bool {
        self.flags().executable
    }

    /// Returns `true` if this node kind introduces a new lexical scope.
    ///
    /// See [`NodeKindFlags::introduces_scope`].
    #[inline]
    pub fn introduces_scope(&self) -> bool {
        self.flags().introduces_scope
    }

    /// Returns `true` if this node kind declares a symbol into a scope or
    /// symbol table.
    ///
    /// See [`NodeKindFlags::declares_symbol`].
    #[inline]
    pub fn declares_symbol(&self) -> bool {
        self.flags().declares_symbol
    }

    /// Returns `true` if this node kind references a symbol.
    ///
    /// See [`NodeKindFlags::references_symbol`].
    #[inline]
    pub fn references_symbol(&self) -> bool {
        self.flags().references_symbol
    }

    /// Returns `true` if this node kind can host a debugger breakpoint.
    ///
    /// This is a **variant-level** flag. Consumers must AND it with positional
    /// checks (heredoc body interior, POD block, `__DATA__` section, etc.)
    /// before accepting a breakpoint request.
    ///
    /// See [`NodeKindFlags::safe_for_breakpoint`].
    #[inline]
    pub fn safe_for_breakpoint(&self) -> bool {
        self.flags().safe_for_breakpoint
    }

    /// Returns `true` if this node kind is a synthetic recovery artifact.
    ///
    /// Recovery nodes should never be offered to editor features such as
    /// hover, go-to-definition, or breakpoint placement.
    ///
    /// See [`NodeKindFlags::recovery_artifact`].
    #[inline]
    pub fn is_recovery(&self) -> bool {
        self.flags().recovery_artifact
    }
}

// ─────────────────────────────────────────────────────────────────────────────
// Inline lib tests — counted by `--lib` profdata for Codecov patch coverage.
//
// These assert real behaviour (not padding). The integration contract tests in
// `tests/classification_tests.rs` remain the primary specification suite; this
// module ensures every production line in `classification.rs` is reachable
// under `cargo llvm-cov -p perl-ast --lib`.
// ─────────────────────────────────────────────────────────────────────────────
#[cfg(test)]
mod tests {
    use super::NodeKindCategory;
    use super::NodeKindFlags;
    use crate::ast::{Node, NodeKind};
    use perl_position_tracking::SourceLocation;

    fn loc() -> SourceLocation {
        SourceLocation::new(0, 1)
    }

    fn leaf() -> Node {
        Node::new(NodeKind::Identifier { name: "x".to_string() }, loc())
    }

    fn block_node() -> Node {
        Node::new(NodeKind::Block { statements: vec![] }, loc())
    }

    /// One representative of every `NodeKind` variant so every match arm in
    /// `category()` and `flags()` is exercised under `--lib` profdata.
    fn all_variants() -> Vec<NodeKind> {
        vec![
            NodeKind::Program { statements: vec![] },
            NodeKind::ExpressionStatement { expression: Box::new(leaf()) },
            NodeKind::VariableDeclaration {
                declarator: "my".to_string(),
                variable: Box::new(leaf()),
                attributes: vec![],
                initializer: None,
            },
            NodeKind::VariableListDeclaration {
                declarator: "my".to_string(),
                variables: vec![],
                attributes: vec![],
                initializer: None,
            },
            NodeKind::Variable { sigil: "$".to_string(), name: "x".to_string() },
            NodeKind::VariableWithAttributes { variable: Box::new(leaf()), attributes: vec![] },
            NodeKind::Assignment {
                lhs: Box::new(leaf()),
                rhs: Box::new(leaf()),
                op: "=".to_string(),
            },
            NodeKind::Binary {
                op: "+".to_string(),
                left: Box::new(leaf()),
                right: Box::new(leaf()),
            },
            NodeKind::Ternary {
                condition: Box::new(leaf()),
                then_expr: Box::new(leaf()),
                else_expr: Box::new(leaf()),
            },
            NodeKind::Unary { op: "-".to_string(), operand: Box::new(leaf()) },
            NodeKind::Diamond,
            NodeKind::Ellipsis,
            NodeKind::Undef,
            NodeKind::Readline { filehandle: None },
            NodeKind::Glob { pattern: "*.pl".to_string() },
            NodeKind::Typeglob { name: "foo".to_string() },
            NodeKind::Number { value: "42".to_string() },
            NodeKind::String { value: "hello".to_string(), interpolated: false },
            NodeKind::Heredoc {
                delimiter: "EOF".to_string(),
                content: "body".to_string(),
                interpolated: false,
                indented: false,
                command: false,
                body_span: None,
            },
            NodeKind::ArrayLiteral { elements: vec![] },
            NodeKind::HashLiteral { pairs: vec![] },
            NodeKind::Block { statements: vec![] },
            NodeKind::Eval { block: Box::new(block_node()) },
            NodeKind::Do { block: Box::new(block_node()) },
            NodeKind::Defer { block: Box::new(block_node()) },
            NodeKind::Try {
                body: Box::new(block_node()),
                catch_blocks: vec![],
                finally_block: None,
            },
            NodeKind::If {
                condition: Box::new(leaf()),
                then_branch: Box::new(block_node()),
                elsif_branches: vec![],
                else_branch: None,
                keyword: None,
            },
            NodeKind::LabeledStatement {
                label: "OUTER".to_string(),
                statement: Box::new(Node::new(
                    NodeKind::LoopControl { op: "next".to_string(), label: None },
                    loc(),
                )),
            },
            NodeKind::While {
                condition: Box::new(leaf()),
                body: Box::new(block_node()),
                continue_block: None,
                keyword: None,
            },
            NodeKind::Tie { variable: Box::new(leaf()), package: Box::new(leaf()), args: vec![] },
            NodeKind::Untie { variable: Box::new(leaf()) },
            NodeKind::For {
                init: None,
                condition: None,
                update: None,
                body: Box::new(block_node()),
                continue_block: None,
            },
            NodeKind::Foreach {
                variable: Box::new(leaf()),
                list: Box::new(leaf()),
                body: Box::new(block_node()),
                continue_block: None,
            },
            NodeKind::Given { expr: Box::new(leaf()), body: Box::new(block_node()) },
            NodeKind::When { condition: Box::new(leaf()), body: Box::new(block_node()) },
            NodeKind::Default { body: Box::new(block_node()) },
            NodeKind::StatementModifier {
                statement: Box::new(leaf()),
                modifier: "if".to_string(),
                condition: Box::new(leaf()),
            },
            NodeKind::Subroutine {
                name: Some("foo".to_string()),
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block_node()),
            },
            NodeKind::Prototype { content: "$@".to_string() },
            NodeKind::Signature { parameters: vec![] },
            NodeKind::MandatoryParameter { variable: Box::new(leaf()) },
            NodeKind::OptionalParameter {
                variable: Box::new(leaf()),
                default_value: Box::new(leaf()),
            },
            NodeKind::SlurpyParameter { variable: Box::new(leaf()) },
            NodeKind::NamedParameter { variable: Box::new(leaf()) },
            NodeKind::Method {
                name: "bar".to_string(),
                signature: None,
                attributes: vec![],
                body: Box::new(block_node()),
            },
            NodeKind::Return { value: None },
            NodeKind::LoopControl { op: "next".to_string(), label: None },
            NodeKind::Goto { target: Box::new(leaf()) },
            NodeKind::MethodCall {
                object: Box::new(leaf()),
                method: "foo".to_string(),
                args: vec![],
            },
            NodeKind::FunctionCall { name: "print".to_string(), args: vec![] },
            NodeKind::IndirectCall {
                method: "new".to_string(),
                object: Box::new(leaf()),
                args: vec![],
            },
            NodeKind::Regex {
                pattern: "foo".to_string(),
                replacement: None,
                modifiers: "".to_string(),
                has_embedded_code: false,
            },
            NodeKind::Match {
                expr: Box::new(leaf()),
                pattern: "foo".to_string(),
                modifiers: "".to_string(),
                has_embedded_code: false,
                negated: false,
            },
            NodeKind::Substitution {
                expr: Box::new(leaf()),
                pattern: "foo".to_string(),
                replacement: "bar".to_string(),
                modifiers: "".to_string(),
                has_embedded_code: false,
                negated: false,
            },
            NodeKind::Transliteration {
                expr: Box::new(leaf()),
                search: "a".to_string(),
                replace: "b".to_string(),
                modifiers: "".to_string(),
                negated: false,
            },
            NodeKind::Package { name: "Foo".to_string(), name_span: loc(), block: None },
            NodeKind::Use { module: "strict".to_string(), args: vec![], has_filter_risk: false },
            NodeKind::No { module: "strict".to_string(), args: vec![], has_filter_risk: false },
            NodeKind::PhaseBlock {
                phase: "BEGIN".to_string(),
                phase_span: None,
                block: Box::new(block_node()),
            },
            NodeKind::DataSection { marker: "__DATA__".to_string(), body: None },
            NodeKind::Class {
                name: "Foo".to_string(),
                parents: vec![],
                body: Box::new(block_node()),
            },
            NodeKind::Format { name: "STDOUT".to_string(), body: "".to_string() },
            NodeKind::Identifier { name: "foo".to_string() },
            NodeKind::Error {
                message: "oops".to_string(),
                expected: vec![],
                found: None,
                partial: None,
            },
            NodeKind::MissingExpression,
            NodeKind::MissingStatement,
            NodeKind::MissingIdentifier,
            NodeKind::MissingBlock,
            NodeKind::UnknownRest,
        ]
    }

    // ── Test 1: every variant produces a category and flags that pass validate()

    #[test]
    fn all_variants_return_category_and_valid_flags() {
        for kind in all_variants() {
            let _cat = kind.category();
            let flags = kind.flags();
            assert!(
                flags.validate().is_ok(),
                "variant {} failed validate(): {:?}",
                kind.kind_name(),
                flags.validate()
            );
        }
    }

    // ── Test 2: category() spot-checks for every NodeKindCategory value ───────

    #[test]
    fn category_spot_checks() {
        assert_eq!(NodeKind::Program { statements: vec![] }.category(), NodeKindCategory::Program);
        assert_eq!(
            NodeKind::If {
                condition: Box::new(leaf()),
                then_branch: Box::new(block_node()),
                elsif_branches: vec![],
                else_branch: None,
                keyword: None,
            }
            .category(),
            NodeKindCategory::Statement
        );
        assert_eq!(
            NodeKind::FunctionCall { name: "print".to_string(), args: vec![] }.category(),
            NodeKindCategory::Expression
        );
        assert_eq!(
            NodeKind::Subroutine {
                name: Some("foo".to_string()),
                name_span: None,
                prototype: None,
                signature: None,
                attributes: vec![],
                body: Box::new(block_node()),
            }
            .category(),
            NodeKindCategory::Declaration
        );
        assert_eq!(NodeKind::Block { statements: vec![] }.category(), NodeKindCategory::Scope);
        assert_eq!(
            NodeKind::Number { value: "1".to_string() }.category(),
            NodeKindCategory::Literal
        );
        assert_eq!(NodeKind::Ellipsis.category(), NodeKindCategory::Operator);
        assert_eq!(NodeKind::MissingExpression.category(), NodeKindCategory::Recovery);
        assert_eq!(NodeKind::UnknownRest.category(), NodeKindCategory::Recovery);
    }

    // ── Test 3: recovery_artifact implies !safe_for_breakpoint (invariant) ────

    #[test]
    fn recovery_artifact_implies_not_safe_for_breakpoint() {
        for kind in all_variants() {
            let flags = kind.flags();
            if flags.recovery_artifact {
                assert!(
                    !flags.safe_for_breakpoint,
                    "variant {} has recovery_artifact=true but safe_for_breakpoint=true",
                    kind.kind_name()
                );
            }
        }
    }

    // ── Test 4: convenience accessors are consistent with flags() ─────────────

    #[test]
    fn convenience_accessors_match_flags() {
        for kind in all_variants() {
            let flags = kind.flags();
            assert_eq!(
                kind.is_executable(),
                flags.executable,
                "{}: is_executable() != flags.executable",
                kind.kind_name()
            );
            assert_eq!(
                kind.introduces_scope(),
                flags.introduces_scope,
                "{}: introduces_scope() != flags.introduces_scope",
                kind.kind_name()
            );
            assert_eq!(
                kind.declares_symbol(),
                flags.declares_symbol,
                "{}: declares_symbol() != flags.declares_symbol",
                kind.kind_name()
            );
            assert_eq!(
                kind.references_symbol(),
                flags.references_symbol,
                "{}: references_symbol() != flags.references_symbol",
                kind.kind_name()
            );
            assert_eq!(
                kind.safe_for_breakpoint(),
                flags.safe_for_breakpoint,
                "{}: safe_for_breakpoint() != flags.safe_for_breakpoint",
                kind.kind_name()
            );
            assert_eq!(
                kind.is_recovery(),
                flags.recovery_artifact,
                "{}: is_recovery() != flags.recovery_artifact",
                kind.kind_name()
            );
        }
    }

    // ── Test 5: specific expected flag values for representative variants ──────

    #[test]
    fn flag_values_for_representative_variants() {
        // FunctionCall: executable, references symbols, safe for breakpoint
        let fc = NodeKind::FunctionCall { name: "say".to_string(), args: vec![] };
        assert!(fc.is_executable());
        assert!(fc.references_symbol());
        assert!(fc.safe_for_breakpoint());
        assert!(!fc.introduces_scope());
        assert!(!fc.declares_symbol());
        assert!(!fc.is_recovery());

        // VariableDeclaration: executable, introduces scope, declares symbol
        let vd = NodeKind::VariableDeclaration {
            declarator: "my".to_string(),
            variable: Box::new(leaf()),
            attributes: vec![],
            initializer: None,
        };
        assert!(vd.is_executable());
        assert!(vd.introduces_scope());
        assert!(vd.declares_symbol());
        assert!(vd.safe_for_breakpoint());

        // Number literal: none of the executable/scope/decl/refs/bp flags set
        let num = NodeKind::Number { value: "0".to_string() };
        assert!(!num.is_executable());
        assert!(!num.introduces_scope());
        assert!(!num.declares_symbol());
        assert!(!num.references_symbol());
        assert!(!num.safe_for_breakpoint());
        assert!(!num.is_recovery());

        // Error: recovery_artifact, never safe for breakpoint
        let err = NodeKind::Error {
            message: "bad".to_string(),
            expected: vec![],
            found: None,
            partial: None,
        };
        assert!(err.is_recovery());
        assert!(!err.safe_for_breakpoint());

        // All five Missing*/UnknownRest variants are recovery artifacts
        for recovery_kind in [
            NodeKind::MissingExpression,
            NodeKind::MissingStatement,
            NodeKind::MissingIdentifier,
            NodeKind::MissingBlock,
            NodeKind::UnknownRest,
        ] {
            assert!(
                recovery_kind.is_recovery(),
                "{} should be recovery",
                recovery_kind.kind_name()
            );
            assert!(
                !recovery_kind.safe_for_breakpoint(),
                "{} should not be safe_for_breakpoint",
                recovery_kind.kind_name()
            );
        }

        // Block: introduces scope, executable, and safe for breakpoint
        let blk = NodeKind::Block { statements: vec![] };
        assert!(blk.introduces_scope());
        assert!(blk.is_executable());
        assert!(blk.safe_for_breakpoint());
    }

    // ── Test 6: NodeKindFlags::validate() accepts valid, rejects invalid ───────

    #[test]
    fn flags_validate_rejects_recovery_with_breakpoint() {
        let good = NodeKindFlags {
            executable: false,
            introduces_scope: false,
            declares_symbol: false,
            references_symbol: false,
            contains_children: false,
            recovery_artifact: true,
            safe_for_breakpoint: false,
        };
        assert!(good.validate().is_ok());

        let bad = NodeKindFlags {
            executable: false,
            introduces_scope: false,
            declares_symbol: false,
            references_symbol: false,
            contains_children: false,
            recovery_artifact: true,
            safe_for_breakpoint: true, // INVALID: recovery AND breakpoint
        };
        assert!(bad.validate().is_err());
    }
}
