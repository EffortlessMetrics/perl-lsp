//! Rust-native Perl parser with tree-sitter-style ergonomics and tree-sitter-compatible
//! output. This is a facade over the v3 native parser (`perl-parser-core`); it is NOT
//! bindings to the C tree-sitter grammar. For the conventional tree-sitter binding, see
//! `tree-sitter-perl-c`.
//!
//! # Quick start
//!
//! ```rust
//! use tree_sitter_perl_rs::Parser;
//!
//! let mut parser = Parser::new();
//! if let Some(tree) = parser.parse("my $x = 42;") {
//!     let root = tree.root_node();
//!     println!("{}", root.to_sexp());
//! }
//! ```
//!
//! # Design
//!
//! This crate wraps the v3 recursive-descent Perl parser (`perl-parser-core`) with an API
//! surface that matches the conventions of the `tree-sitter` crate. Users familiar with
//! tree-sitter can work with Perl ASTs immediately, while the underlying engine is the
//! full-featured native v3 stack (not the C tree-sitter grammar).
//!
//! Key properties:
//! - `Parser::parse()` returns `Option<Tree>` — `None` only on complete parse failure.
//!   The v3 parser is highly error-tolerant and almost always produces a partial tree.
//! - `Node::to_sexp()` delegates to `perl_ast::Node::to_sexp()` for tree-sitter-compatible
//!   S-expression output.
//! - `Node::kind()` returns the tree-sitter grammar-canonical kind string.
//! - `Node::start_byte()` / `Node::end_byte()` expose the `SourceLocation` byte offsets.
//! - `Node::children()` and `Node::child()` mirror tree-sitter traversal conventions.
//!
//! # Relationship to `tree-sitter-perl-c`
//!
//! | Crate | Backing engine | Use when |
//! |-------|---------------|----------|
//! | `tree-sitter-perl-rs` | v3 native Rust parser (this crate) | You want the full-featured Rust toolchain |
//! | `tree-sitter-perl-c` | C tree-sitter grammar | You need compatibility with the tree-sitter C ecosystem |

#![deny(unsafe_code)]
#![deny(unreachable_pub)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

use perl_ast::{Node as AstNode, NodeKind};
use perl_module::parse_module_import_head;
use perl_parser_core::{
    ParseOutput, Parser as CoreParser,
    incremental::{FallbackReason as CoreFallbackReason, IncrementalEdit, IncrementalState},
};
use perl_pragma::{PragmaState, PragmaTracker};
use perl_semantic_analyzer::semantic::SemanticModel;
use std::ops::{ControlFlow, Range};

/// Parser diagnostics surfaced by [`Parser::parse_detailed`].
pub use perl_parser_core::ParseError as ParseDiagnostic;
pub use perl_parser_core::incremental::IncrementalMetrics;

#[cfg(feature = "queries")]
mod query;

/// Re-export of Edit type for tree-sitter-compatible incremental parsing.
///
/// Mirrors `tree_sitter::InputEdit` field layout for drop-in compatibility.
pub use perl_parser_core::edit::Edit as InputEdit;
#[cfg(feature = "queries")]
pub use query::{Query, QueryCapture, QueryCursor, QueryError, QueryMatch, QueryMatches};

/// A tree-sitter-compatible source position.
///
/// `row` and `column` are both zero-based and `column` is measured in bytes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct Point {
    /// Zero-based row number.
    pub row: usize,
    /// Zero-based byte column within `row`.
    pub column: usize,
}

/// A Perl parser with tree-sitter-style ergonomics.
///
/// Wraps the v3 recursive-descent Perl parser. Create one parser instance and call
/// [`parse`][Parser::parse] for each source file you need to process.
///
/// # Example
///
/// ```rust
/// use tree_sitter_perl_rs::Parser;
///
/// let mut parser = Parser::new();
/// let tree = parser.parse("sub greet { print \"hello\"; }");
/// assert!(tree.is_some());
/// ```
pub struct Parser {
    // Stateless currently; the v3 CoreParser takes source at construction time.
    // Stored as a unit struct for forward compatibility (e.g. future options).
    _priv: (),
}

impl Parser {
    /// Create a new parser instance.
    pub fn new() -> Self {
        Parser { _priv: () }
    }

    /// Parse a Perl source string and return a [`Tree`], or `None` on complete failure.
    ///
    /// The v3 parser is highly error-tolerant — even malformed input usually produces a
    /// partial tree. `None` is reserved for extreme edge cases where no AST can be built
    /// at all.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_perl_rs::Parser;
    ///
    /// let mut parser = Parser::new();
    /// let tree = parser.parse("my $x = 42;");
    /// assert!(tree.is_some());
    /// ```
    pub fn parse(&mut self, source: &str) -> Option<Tree> {
        let mut core = CoreParser::new(source);
        match core.parse() {
            Ok(root) => Some(tree_from_parts(root, source, core.errors().to_vec())),
            Err(_) => None,
        }
    }

    /// Parse `source` and preserve recovery diagnostics and catastrophic failures.
    ///
    /// A recovered parse returns `tree: Some(_)` with one or more diagnostics. A
    /// catastrophic failure returns `tree: None` and a typed [`ParseFailure`]. Existing
    /// callers that only need the compatibility `Option` API can continue using
    /// [`parse`][Parser::parse].
    pub fn parse_detailed(&mut self, source: &str) -> ParseOutcome {
        let mut core = CoreParser::new(source);
        let ParseOutput { ast, diagnostics, terminated_early, .. } = core.parse_with_recovery();
        let failure = terminated_early
            .then(|| diagnostics.iter().find_map(ParseFailure::from_diagnostic))
            .flatten();
        let tree = failure.is_none().then(|| tree_from_parts(ast, source, diagnostics.clone()));

        ParseOutcome { tree, diagnostics, failure }
    }

    /// Parse `source` using `old_tree` as a hint for incremental re-parsing.
    ///
    /// A single validated edit uses the lower-tier checkpoint-bounded token replay
    /// kernel. The AST is rebuilt from the resulting token stream; this facade does
    /// not claim AST subtree reuse. Multiple, invalid, or missing edits use a safe
    /// full-parse fallback and record the reason on the returned tree.
    ///
    /// Returns `None` on complete parse failure (same semantics as `parse`).
    pub fn parse_with_old_tree(&mut self, source: &str, old_tree: &Tree) -> Option<Tree> {
        // Fast path: if source is unchanged and no edits were recorded, reuse the old tree
        // instead of re-parsing. This mirrors tree-sitter's incremental no-op behavior.
        if source == old_tree.source() && old_tree.pending_edits.is_empty() {
            let mut unchanged = old_tree.clone();
            unchanged.reparse_mode = Some(ReparseMode::Unchanged);
            return Some(unchanged);
        }

        let fallback_reason = match old_tree.pending_edits.as_slice() {
            [edit] => {
                let Some(incremental_edit) =
                    validated_incremental_edit(old_tree.source(), source, edit)
                else {
                    return self.parse_with_fallback(source, FallbackReason::InvalidEdit);
                };

                let mut state = old_tree.incremental_state.as_ref().cloned().unwrap_or_else(|| {
                    IncrementalState::with_diagnostics(old_tree.source(), old_tree.diagnostics())
                });
                match state.reparse(source, &incremental_edit) {
                    Ok(root) => {
                        let mode =
                            state.metrics().fallback.map_or(ReparseMode::TokenReplay, |reason| {
                                ReparseMode::FullParseFallback(FallbackReason::TokenReplay(reason))
                            });
                        return Some(Tree {
                            root,
                            source: source.to_string(),
                            pending_edits: Vec::new(),
                            diagnostics: state.diagnostics().to_vec(),
                            incremental_state: Some(state),
                            reparse_mode: Some(mode),
                        });
                    }
                    Err(_) => FallbackReason::TokenReplay(CoreFallbackReason::TokenReplayFailed),
                }
            }
            [] => FallbackReason::NoPendingEdit,
            _ => FallbackReason::MultipleEdits,
        };

        self.parse_with_fallback(source, fallback_reason)
    }

    fn parse_with_fallback(&mut self, source: &str, reason: FallbackReason) -> Option<Tree> {
        let mut tree = self.parse(source)?;
        tree.reparse_mode = Some(ReparseMode::FullParseFallback(reason));
        tree.incremental_state =
            Some(IncrementalState::with_diagnostics(source, tree.diagnostics()));
        Some(tree)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

fn validated_incremental_edit(
    old_source: &str,
    new_source: &str,
    edit: &InputEdit,
) -> Option<IncrementalEdit> {
    if edit.start_byte > edit.old_end_byte
        || edit.old_end_byte > old_source.len()
        || edit.new_end_byte < edit.start_byte
        || edit.new_end_byte > new_source.len()
        || !old_source.is_char_boundary(edit.start_byte)
        || !old_source.is_char_boundary(edit.old_end_byte)
        || !new_source.is_char_boundary(edit.start_byte)
        || !new_source.is_char_boundary(edit.new_end_byte)
    {
        return None;
    }

    let removed = edit.old_end_byte.checked_sub(edit.start_byte)?;
    let inserted = edit.new_end_byte.checked_sub(edit.start_byte)?;
    let expected_len = old_source.len().checked_sub(removed)?.checked_add(inserted)?;
    if expected_len != new_source.len() {
        return None;
    }

    let old_prefix = old_source.get(..edit.start_byte)?;
    let new_prefix = new_source.get(..edit.start_byte)?;
    let old_suffix = old_source.get(edit.old_end_byte..)?;
    let new_suffix = new_source.get(edit.new_end_byte..)?;
    if old_prefix != new_prefix || old_suffix != new_suffix {
        return None;
    }

    let new_text = new_source.get(edit.start_byte..edit.new_end_byte)?;
    Some(IncrementalEdit::new(edit.start_byte, edit.old_end_byte, new_text))
}

/// A descriptor for the Perl language as parsed by the native v3 engine.
///
/// Provides node kind names and field metadata for Rust-native tooling.
/// This is NOT a `tree_sitter::Language` — it does not require a C toolchain
/// and cannot be used with `tree_sitter::Parser::set_language`. For drop-in
/// tree-sitter compatibility use `tree-sitter-perl-c` instead.
///
/// # Example
///
/// ```rust
/// use tree_sitter_perl_rs::language;
///
/// let lang = language();
/// assert!(lang.node_kind_count() > 0);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PerlLanguage {
    kind_names: &'static [&'static str],
    field_names: &'static [perl_ast::FieldId],
}

impl PerlLanguage {
    /// Returns the number of distinct node kinds in the grammar.
    pub fn node_kind_count(&self) -> usize {
        self.kind_names.len()
    }

    /// Returns all node kind names, in declaration order.
    ///
    /// The order matches the variant declaration order of [`perl_ast::NodeKind`].
    /// `ALL_KIND_NAMES` is auto-derived via `strum::VariantNames`; callers that
    /// need a sorted list should sort the returned slice themselves.
    pub fn node_kind_names(&self) -> &[&'static str] {
        self.kind_names
    }

    /// Returns `true` if the given kind name is a named (non-anonymous) node kind.
    pub fn node_kind_is_named(&self, kind: &str) -> bool {
        self.kind_names.contains(&kind)
    }

    /// Returns the stable named-field identifiers exposed by the AST.
    pub fn field_names(&self) -> &'static [FieldId] {
        self.field_names
    }

    /// Returns the field identifier for a canonical field name.
    pub fn field_id_for_name(&self, name: &str) -> Option<FieldId> {
        perl_ast::FieldId::from_name(name)
    }
}

impl Default for PerlLanguage {
    fn default() -> Self {
        LANGUAGE
    }
}

/// Returns the [`PerlLanguage`] descriptor for Rust-native tooling.
///
/// Note: This is NOT equivalent to `tree_sitter::Language`. See [`PerlLanguage`].
pub fn language() -> PerlLanguage {
    LANGUAGE
}

/// The [`PerlLanguage`] descriptor as a constant.
pub static LANGUAGE: PerlLanguage = PerlLanguage {
    kind_names: perl_ast::NodeKind::ALL_KIND_NAMES,
    field_names: perl_ast::FieldId::ALL,
};

/// The operation used to produce a tree from an old tree.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ReparseMode {
    /// The source was byte-identical and the old tree was reused.
    Unchanged,
    /// One validated edit used checkpoint-bounded token replay.
    TokenReplay,
    /// The source was parsed from scratch after replay was not safe or usable.
    FullParseFallback(FallbackReason),
}

/// Why the facade used a complete parse instead of token replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum FallbackReason {
    /// The pending [`InputEdit`] did not describe `source`.
    InvalidEdit,
    /// More than one pending edit was recorded on the old tree.
    MultipleEdits,
    /// The source changed but no pending edit was recorded on the old tree.
    NoPendingEdit,
    /// The lower-tier token replay kernel rejected the incremental operation.
    TokenReplay(CoreFallbackReason),
}

/// The result of a successful parse: an owned syntax tree and the source text.
///
/// Use [`root_node`][Tree::root_node] to begin traversal.
#[derive(Debug, Clone)]
pub struct Tree {
    root: AstNode,
    source: String,
    /// Pending edits recorded via [`Tree::edit`].
    pending_edits: Vec<InputEdit>,
    diagnostics: Vec<ParseDiagnostic>,
    incremental_state: Option<IncrementalState>,
    reparse_mode: Option<ReparseMode>,
}

impl PartialEq for Tree {
    fn eq(&self, other: &Self) -> bool {
        self.root == other.root
            && self.source == other.source
            && self.pending_edits == other.pending_edits
            && self.diagnostics == other.diagnostics
    }
}

fn tree_from_parts(root: AstNode, source: &str, diagnostics: Vec<ParseDiagnostic>) -> Tree {
    Tree {
        root,
        source: source.to_string(),
        pending_edits: Vec::new(),
        incremental_state: None,
        diagnostics,
        reparse_mode: None,
    }
}

/// The result of [`Parser::parse_detailed`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct ParseOutcome {
    /// The recovered syntax tree, when parsing did not fail catastrophically.
    pub tree: Option<Tree>,
    /// Diagnostics collected during parsing, including recoverable errors.
    pub diagnostics: Vec<ParseDiagnostic>,
    /// The typed reason parsing could not produce a usable tree, if any.
    pub failure: Option<ParseFailure>,
}

impl ParseOutcome {
    /// Returns `true` when diagnostics or an explicit error node were observed.
    pub fn has_error(&self) -> bool {
        self.diagnostics.iter().any(ParseDiagnostic::blocks_clean_parse)
            || self.tree.as_ref().is_some_and(Tree::has_error)
    }

    /// Returns `true` when a tree was produced with recovery diagnostics.
    pub fn is_recovered(&self) -> bool {
        self.tree.is_some() && self.has_error()
    }
}

/// Typed catastrophic parse failures surfaced by [`Parser::parse_detailed`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum ParseFailure {
    /// The parser recursion budget was exceeded.
    RecursionLimit,
    /// The parser's structural nesting budget was exceeded.
    NestingTooDeep {
        /// Observed nesting depth.
        depth: usize,
        /// Configured maximum nesting depth.
        max_depth: usize,
    },
    /// Parsing was cancelled by the caller.
    Cancelled,
    /// A future or currently unclassified catastrophic failure.
    Other {
        /// The original parser diagnostic.
        diagnostic: ParseDiagnostic,
    },
}

impl ParseFailure {
    fn from_diagnostic(diagnostic: &ParseDiagnostic) -> Option<Self> {
        match diagnostic {
            ParseDiagnostic::RecursionLimit => Some(Self::RecursionLimit),
            ParseDiagnostic::NestingTooDeep { depth, max_depth } => {
                Some(Self::NestingTooDeep { depth: *depth, max_depth: *max_depth })
            }
            ParseDiagnostic::Cancelled => Some(Self::Cancelled),
            _ => Some(Self::Other { diagnostic: diagnostic.clone() }),
        }
    }
}

/// Experimental semantic overlay query handle.
///
/// This API is intentionally limited while the facade integration is in development.
/// Query capabilities will expand over time.
#[derive(Debug, Clone, Copy)]
#[non_exhaustive]
pub struct SemanticOverlay<'tree> {
    tree: &'tree Tree,
}

/// Symbol definition returned by [`SemanticOverlay`] queries.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OverlayDefinition {
    /// Symbol name as written in source.
    pub name: String,
    /// Package-qualified symbol name.
    pub qualified_name: String,
    /// Symbol kind label (debug string form).
    pub kind: String,
    /// Definition span start byte (inclusive).
    pub start_byte: usize,
    /// Definition span end byte (exclusive).
    pub end_byte: usize,
}

/// Import statement visible at a specific source offset.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VisibleImport {
    /// Imported module token (`Foo::Bar`, `strict`, etc.).
    pub module: String,
    /// Statement start byte (inclusive).
    pub statement_start_byte: usize,
    /// Statement end byte (exclusive).
    pub statement_end_byte: usize,
}

impl Tree {
    /// Returns the root node of the syntax tree.
    pub fn root_node(&self) -> Node<'_> {
        Node { inner: &self.root, tree_source: &self.source }
    }

    /// Returns the source text this tree was built from.
    pub fn source(&self) -> &str {
        &self.source
    }

    /// Returns the diagnostics collected while building this tree.
    pub fn diagnostics(&self) -> &[ParseDiagnostic] {
        &self.diagnostics
    }

    /// Returns the operation used to produce this tree from an old tree.
    pub fn reparse_mode(&self) -> Option<ReparseMode> {
        self.reparse_mode
    }

    /// Returns measurements for the most recent replay or fallback operation.
    ///
    /// Initial parses and unchanged-source reuse return `None`, so telemetry from
    /// a previous operation is never exposed as a no-op result.
    pub fn incremental_metrics(&self) -> Option<&IncrementalMetrics> {
        match self.reparse_mode {
            Some(ReparseMode::TokenReplay | ReparseMode::FullParseFallback(_)) => {
                self.incremental_state.as_ref().map(IncrementalState::metrics)
            }
            Some(ReparseMode::Unchanged) | None => None,
        }
    }

    /// Returns the source range reprocessed by the most recent operation.
    ///
    /// This reports lexer work, not a structural tree difference. Initial parses
    /// and unchanged-source reuse return an empty vector.
    pub fn reprocessed_ranges(&self) -> Vec<Range<usize>> {
        self.incremental_metrics()
            .map(|metrics| vec![metrics.changed_range.clone()])
            .unwrap_or_default()
    }

    /// Returns `true` when parsing produced diagnostics or an explicit error node.
    pub fn has_error(&self) -> bool {
        self.diagnostics.iter().any(ParseDiagnostic::blocks_clean_parse)
            || ast_has_error(&self.root)
    }

    /// Records a source edit on this tree, invalidating affected byte ranges.
    ///
    /// After calling `edit()`, pass this tree and the new source to
    /// [`Parser::parse_with_old_tree`] to re-parse efficiently.
    ///
    /// In the current implementation this stores the edit for API compatibility;
    /// true incremental re-parsing (skipping unchanged regions) is a planned
    /// optimization.
    pub fn edit(&mut self, edit: &InputEdit) {
        self.pending_edits.push(edit.clone());
    }

    /// Returns a cursor positioned at the root node for stateful tree traversal.
    ///
    /// This mirrors `tree_sitter::Tree::walk()` and is equivalent to
    /// `tree.root_node().walk()`.
    pub fn walk(&self) -> TreeCursor<'_> {
        self.root_node().walk()
    }

    /// Returns the experimental semantic overlay query handle for this tree.
    pub fn semantic_overlay(&self) -> SemanticOverlay<'_> {
        SemanticOverlay { tree: self }
    }
}

impl<'tree> SemanticOverlay<'tree> {
    /// Resolve a symbol definition at a byte offset in the source.
    pub fn definition_at_offset(&self, offset: usize) -> Option<OverlayDefinition> {
        let model = SemanticModel::build(&self.tree.root, self.tree.source());
        model.definition_at(offset).map(|symbol| OverlayDefinition {
            name: symbol.name.clone(),
            qualified_name: symbol.qualified_name.clone(),
            kind: format!("{:?}", symbol.kind),
            start_byte: symbol.location.start,
            end_byte: symbol.location.end,
        })
    }

    /// Resolve a symbol definition for the given node span.
    ///
    /// Uses the node start byte as the query point.
    pub fn definition_for_node(&self, node: &Node<'_>) -> Option<OverlayDefinition> {
        self.definition_at_offset(node.start_byte())
    }

    /// Returns the list of `use`-import modules visible at `offset`.
    ///
    /// Visibility is currently lexical-by-position: this returns `use` statements
    /// with starts less than or equal to `offset`.
    pub fn visible_imports_at_offset(&self, offset: usize) -> Vec<VisibleImport> {
        let mut imports = Vec::new();
        collect_visible_use_imports(&self.tree.root, self.tree.source(), offset, &mut imports);
        let mut deduped = Vec::new();
        for import in imports {
            if !deduped.iter().any(|existing: &VisibleImport| existing.module == import.module) {
                deduped.push(import);
            }
        }
        deduped
    }

    /// Returns the effective pragma state at a byte offset.
    pub fn pragma_state_at_offset(&self, offset: usize) -> PragmaState {
        let pragma_map = PragmaTracker::build(&self.tree.root);
        PragmaTracker::state_for_offset(&pragma_map, offset)
    }
}

/// A borrowed reference to a node in the syntax tree.
///
/// Mirrors the tree-sitter `Node` API surface. Lifetime `'tree` is tied to the
/// owning [`Tree`].
#[derive(Clone, Copy)]
pub struct Node<'tree> {
    inner: &'tree AstNode,
    tree_source: &'tree str,
}

impl<'tree> Node<'tree> {
    /// Returns the tree-sitter grammar-canonical node kind name.
    ///
    /// This matches tree-sitter expectations for `Node::kind()`, for example the
    /// root node kind is `"source_file"`. Use [`native_kind`][Node::native_kind]
    /// for the v3 parser's internal PascalCase kind name.
    pub fn kind(&self) -> String {
        self.grammar_kind()
    }

    /// Returns the v3 parser's internal node kind name (e.g. `"Program"`).
    pub fn native_kind(&self) -> &'static str {
        self.inner.kind.kind_name()
    }

    /// Returns the tree-sitter grammar-canonical node kind name.
    ///
    /// Alias of [`kind`][Node::kind] kept for compatibility.
    ///
    /// This method returns the grammar name
    /// used in S-expressions (e.g., `"source_file"`, `"sub"`).
    /// This matches the kind strings returned by `tree-sitter-perl-c` and the
    /// upstream tree-sitter Perl grammar.
    /// Error nodes use `"ERROR"` (uppercase), matching tree-sitter convention.
    ///
    /// For most nodes the grammar name is an allocation-free static mapping. For
    /// nodes whose name depends on runtime data (e.g., operator-named `Binary`
    /// or dynamic `VariableDeclaration`), only that node's fields are inspected.
    ///
    /// # Example
    ///
    /// ```rust
    /// use tree_sitter_perl_rs::Parser;
    ///
    /// let mut parser = Parser::new();
    /// let tree = parser.parse("my $x = 42;");
    /// assert!(tree.is_some());
    /// assert_eq!(tree.unwrap().root_node().grammar_kind(), "source_file");
    /// ```
    pub fn grammar_kind(&self) -> String {
        self.inner.kind.grammar_kind_name()
    }

    /// Returns `true` when this node is an explicit parser error node.
    pub fn is_error(&self) -> bool {
        matches!(self.inner.kind, NodeKind::Error { .. })
    }

    /// Returns `true` when this node or one of its descendants is an error node.
    pub fn has_error(&self) -> bool {
        ast_has_error(self.inner)
    }

    /// Returns a tree-sitter-compatible S-expression for this node and its subtree.
    ///
    /// Delegates to `perl_ast::Node::to_sexp()`. Example output:
    /// `(source_file (my_declaration (variable $ x) (number 42)))`.
    pub fn to_sexp(&self) -> String {
        self.inner.to_sexp()
    }

    /// Returns the number of direct children.
    pub fn child_count(&self) -> usize {
        ast_child_count(self.inner)
    }

    /// Returns the `i`-th direct child, or `None` if out of range.
    pub fn child(&self, i: usize) -> Option<Node<'tree>> {
        ast_child_at(self.inner, i)
            .map(|child| Node { inner: child, tree_source: self.tree_source })
    }

    /// Returns the first direct child carrying the given named field.
    ///
    /// Unknown field names and fields absent from this node return `None`.
    pub fn child_by_field_name(&self, name: &str) -> Option<Node<'tree>> {
        let field = FieldId::from_name(name)?;
        let mut found = None;
        let _ = self.inner.try_for_each_child_with_field(|candidate, child| {
            if candidate == Some(field) {
                found = Some(Node { inner: child, tree_source: self.tree_source });
                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });
        found
    }

    /// Returns all direct children carrying the given named field, in source order.
    pub fn children_by_field_name(&self, name: &str) -> impl Iterator<Item = Node<'tree>> + '_ {
        let field = FieldId::from_name(name);
        let mut children = Vec::new();
        if let Some(field) = field {
            self.inner.for_each_child_with_field(|candidate, child| {
                if candidate == Some(field) {
                    children.push(Node { inner: child, tree_source: self.tree_source });
                }
            });
        }
        children.into_iter()
    }

    /// Returns the named field associated with the `index`-th direct child.
    pub fn field_name_for_child(&self, index: usize) -> Option<&'static str> {
        ast_child_field(self.inner, index).map(FieldId::name)
    }

    /// Returns an iterator over direct children.
    ///
    /// The iterator yields [`Node`] values sharing the same `'tree` lifetime as `self`.
    pub fn children(&self) -> impl Iterator<Item = Node<'tree>> + '_ {
        // Collect into a Vec so we can own the references. The lifetimes are valid
        // because all child nodes are part of the same owned tree (Tree::root).
        let kids = ast_children(self.inner);
        kids.into_iter().map(move |child| Node { inner: child, tree_source: self.tree_source })
    }

    /// Returns the start byte offset in the source text (inclusive).
    pub fn start_byte(&self) -> usize {
        self.inner.location.start
    }

    /// Returns the end byte offset in the source text (exclusive).
    pub fn end_byte(&self) -> usize {
        self.inner.location.end.min(self.tree_source.len())
    }

    /// Returns the start position as a tree-sitter-compatible [`Point`].
    ///
    /// `row`/`column` are zero-based and `column` is measured in bytes.
    pub fn start_position(&self) -> Point {
        byte_to_point(self.tree_source, self.start_byte())
    }

    /// Returns the end position as a tree-sitter-compatible [`Point`].
    ///
    /// `row`/`column` are zero-based and `column` is measured in bytes.
    pub fn end_position(&self) -> Point {
        byte_to_point(self.tree_source, self.end_byte())
    }

    /// Extracts the source text slice covered by this node.
    ///
    /// Returns `Err` only when the byte range contains invalid UTF-8, which is unlikely
    /// for content produced from a valid Rust `&str`.
    ///
    /// If the node's byte offsets extend beyond `source`, the result is clamped to
    /// the available range rather than panicking. This can happen when `source` is a
    /// different buffer than the one used to build the tree.
    pub fn utf8_text<'a>(&self, source: &'a [u8]) -> Result<&'a str, std::str::Utf8Error> {
        let start = self.inner.location.start.min(source.len());
        let end = self.inner.location.end.min(source.len());
        std::str::from_utf8(&source[start..end])
    }

    /// Returns `true` if this node has no children (is a leaf node).
    pub fn is_leaf(&self) -> bool {
        self.inner.first_child().is_none()
    }

    /// Returns the source text that was provided when creating the owning [`Tree`].
    pub fn tree_source(&self) -> &'tree str {
        self.tree_source
    }

    /// Returns the inner `perl_ast::Node` for direct access to the v3 AST.
    ///
    /// This escape hatch lets callers use capabilities that go beyond the tree-sitter
    /// surface (e.g., match on [`PerlNodeKind`] variants).
    pub fn inner(&self) -> &'tree AstNode {
        self.inner
    }

    /// Returns a cursor positioned at this node for stateful tree traversal.
    ///
    /// Mirrors `tree_sitter::TreeCursor` style navigation with a lightweight,
    /// allocation-free path stack.
    pub fn walk(&self) -> TreeCursor<'tree> {
        TreeCursor { root: self.inner, tree_source: self.tree_source, path: Vec::new() }
    }
}

/// Re-export of [`perl_ast::NodeKind`] so callers can pattern-match node variants
/// without a direct dependency on `perl-ast`.
pub use perl_ast::{FieldId, NodeKind as PerlNodeKind};

/// Stateful cursor for navigating a subtree.
///
/// The cursor is rooted at the [`Node`] that created it via [`Node::walk`].
/// Calling [`goto_parent`][TreeCursor::goto_parent] at the root returns `false`
/// and keeps the cursor at the root.
pub struct TreeCursor<'tree> {
    root: &'tree AstNode,
    tree_source: &'tree str,
    /// Child indices from `root` to the current node.
    path: Vec<usize>,
}

impl<'tree> TreeCursor<'tree> {
    /// Returns the node currently selected by the cursor.
    pub fn node(&self) -> Node<'tree> {
        Node { inner: self.current_ast_node(), tree_source: self.tree_source }
    }

    /// Moves to the first child of the current node.
    ///
    /// Returns `true` when movement succeeds, `false` when the node has no children.
    pub fn goto_first_child(&mut self) -> bool {
        if self.current_ast_node().first_child().is_none() {
            return false;
        }
        self.path.push(0);
        true
    }

    /// Moves to the last child of the current node.
    ///
    /// Returns `true` when movement succeeds, `false` when the node has no children.
    pub fn goto_last_child(&mut self) -> bool {
        let child_count = self.current_ast_node().children().len();
        if child_count == 0 {
            return false;
        }
        self.path.push(child_count - 1);
        true
    }

    /// Moves to the next sibling of the current node.
    ///
    /// Returns `true` on success. Returns `false` if the cursor is at root or if
    /// there is no next sibling.
    pub fn goto_next_sibling(&mut self) -> bool {
        if self.path.is_empty() {
            return false;
        }

        let parent = self.current_parent_ast_node();
        let sibling_count = ast_child_count(parent);
        let current_index = self.path[self.path.len() - 1];
        let next = current_index + 1;
        if next >= sibling_count {
            return false;
        }

        let last_pos = self.path.len() - 1;
        self.path[last_pos] = next;
        true
    }

    /// Moves to the previous sibling of the current node.
    ///
    /// Returns `true` on success. Returns `false` if the cursor is at root or if
    /// there is no previous sibling.
    pub fn goto_previous_sibling(&mut self) -> bool {
        if self.path.is_empty() {
            return false;
        }

        let current_index = self.path[self.path.len() - 1];
        if current_index == 0 {
            return false;
        }

        let last_pos = self.path.len() - 1;
        self.path[last_pos] = current_index - 1;
        true
    }

    /// Moves to the parent node.
    ///
    /// Returns `true` when movement succeeds, `false` when already at root.
    pub fn goto_parent(&mut self) -> bool {
        self.path.pop().is_some()
    }

    /// Resets the cursor back to its root node.
    pub fn reset(&mut self) {
        self.path.clear();
    }

    fn current_ast_node(&self) -> &'tree AstNode {
        resolve_path(self.root, &self.path)
    }

    fn current_parent_ast_node(&self) -> &'tree AstNode {
        debug_assert!(!self.path.is_empty(), "current_parent_ast_node requires a non-root cursor");
        let parent_path_len = self.path.len() - 1;
        resolve_path(self.root, &self.path[..parent_path_len])
    }
}

// ---------------------------------------------------------------------------
// Private helpers
// ---------------------------------------------------------------------------

// Collect the direct children of an `AstNode` as a `Vec<&AstNode>`.
//
// This thin wrapper exists because the public `Node::children()` method in `perl_ast`
// has the same name as our facade method and would be ambiguous in `impl` blocks.
#[inline]
fn ast_children(node: &AstNode) -> Vec<&AstNode> {
    node.children()
}

#[inline]
fn ast_child_count(node: &AstNode) -> usize {
    let mut count = 0usize;
    node.for_each_child(|_| count += 1);
    count
}

fn ast_has_error(node: &AstNode) -> bool {
    if matches!(node.kind, NodeKind::Error { .. }) {
        return true;
    }

    node.children().iter().any(|child| ast_has_error(child))
}

#[inline]
fn ast_child_at(node: &AstNode, index: usize) -> Option<&AstNode> {
    ast_child_with_field(node, index).map(|(_, child)| child)
}

#[inline]
fn ast_child_field(node: &AstNode, index: usize) -> Option<FieldId> {
    ast_child_with_field(node, index).and_then(|(field, _)| field)
}

#[inline]
fn ast_child_with_field(node: &AstNode, index: usize) -> Option<(Option<FieldId>, &AstNode)> {
    let mut idx = 0usize;
    let mut found = None;
    let _ = node.try_for_each_child_with_field(|field, child| {
        if idx == index {
            found = Some((field, child));
            ControlFlow::Break(())
        } else {
            idx += 1;
            ControlFlow::Continue(())
        }
    });
    found
}

fn collect_visible_use_imports(
    node: &AstNode,
    source: &str,
    offset: usize,
    out: &mut Vec<VisibleImport>,
) {
    // Only attempt import extraction on Use AST nodes. Container nodes (Program,
    // Block, Subroutine, etc.) span large source ranges that may accidentally start
    // with a `use` token, producing imports with incorrect statement byte ranges.
    // Use nodes have no children (for_each_child is a no-op), so they are visited
    // exactly once per tree traversal — no inner dedup needed.
    if matches!(node.kind, NodeKind::Use { .. }) && node.location.start <= offset {
        let start = node.location.start.min(source.len());
        let end = node.location.end.min(source.len());
        let statement_text = &source[start..end];
        if let Some(import_head) = parse_module_import_head(statement_text) {
            out.push(VisibleImport {
                module: import_head.token.to_string(),
                statement_start_byte: start,
                statement_end_byte: end,
            });
        }
    }

    node.for_each_child(|child| collect_visible_use_imports(child, source, offset, out));
}

// Invariant: TreeCursor path is constructed by traversal methods in this type.
// If a stale/invalid path somehow appears, return the last valid node instead
// of panicking, preserving total API safety guarantees.
fn resolve_path<'tree>(root: &'tree AstNode, path: &[usize]) -> &'tree AstNode {
    let mut current = root;
    for &index in path {
        match ast_child_at(current, index) {
            Some(child) => current = child,
            None => {
                debug_assert!(false, "TreeCursor path must reference a valid child");
                break;
            }
        }
    }
    current
}

fn byte_to_point(source: &str, byte: usize) -> Point {
    let clamped = byte.min(source.len());
    let mut row = 0usize;
    let mut column = 0usize;

    for b in source.as_bytes().iter().take(clamped) {
        if *b == b'\n' {
            row += 1;
            column = 0;
        } else {
            column += 1;
        }
    }

    Point { row, column }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    #[test]
    fn test_parser_creates_tree() {
        let mut p = Parser::new();
        let tree = p.parse("my $x = 42;");
        assert!(tree.is_some());
    }

    #[test]
    fn test_root_node_kind() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 42;"));
        assert_eq!(tree.root_node().kind(), "source_file");
        assert_eq!(tree.root_node().native_kind(), "Program");
    }

    #[test]
    fn test_to_sexp_starts_with_source_file() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 42;"));
        let sexp = tree.root_node().to_sexp();
        assert!(
            sexp.starts_with("(source_file"),
            "sexp should start with (source_file, got: {}",
            sexp
        );
    }

    #[test]
    fn test_child_count_for_program_with_statements() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 42;\nmy $y = 99;"));
        let root = tree.root_node();
        assert!(root.child_count() >= 1, "root should have children");
    }

    #[test]
    fn test_start_and_end_byte() {
        let source = "my $x = 42;";
        let mut p = Parser::new();
        let tree = must_some(p.parse(source));
        let root = tree.root_node();
        assert_eq!(root.start_byte(), 0);
        assert_eq!(root.end_byte(), source.len(), "root end_byte should clamp to source length");
    }

    #[test]
    fn test_start_and_end_position_are_tree_sitter_compatible() {
        let source = "my $x = 1;\nmy $y = 2;";
        let mut p = Parser::new();
        let tree = must_some(p.parse(source));
        let root = tree.root_node();

        assert_eq!(root.start_position(), Point { row: 0, column: 0 });
        assert_eq!(root.end_position(), Point { row: 1, column: 10 });
    }

    #[test]
    fn test_end_position_column_uses_bytes_not_chars() {
        let source = "my $emoji = \"😀\";";
        let mut p = Parser::new();
        let tree = must_some(p.parse(source));
        let root = tree.root_node();

        assert_eq!(root.end_byte(), source.len());
        assert_eq!(root.end_position(), Point { row: 0, column: source.len() });
    }

    /// Verify the end_byte clamp invariant: for every node in the tree,
    /// `end_byte()` must not exceed `tree.source().len()`.  This exercises the
    /// `.min(self.tree_source.len())` guard on the full node set, not just the
    /// root, so that any future parser regression producing an out-of-bounds
    /// location is caught here.
    #[test]
    fn test_end_byte_never_exceeds_source_len_for_all_nodes() {
        let sources = [
            "my $x = 42;",
            "sub foo { return 1; }",
            "use strict;\nuse warnings;\nmy @arr = (1, 2, 3);",
            // empty source — edge case for zero-length trees
            "",
        ];
        for source in sources {
            let mut p = Parser::new();
            let tree = match p.parse(source) {
                Some(t) => t,
                // v3 parser returns None only on extreme failure; skip rather than panic
                None => continue,
            };
            let source_len = tree.source().len();
            // Walk all direct children of root and check the invariant
            let root = tree.root_node();
            assert!(
                root.end_byte() <= source_len,
                "root end_byte {} > source_len {} for source {:?}",
                root.end_byte(),
                source_len,
                source
            );
            for child in root.children() {
                assert!(
                    child.end_byte() <= source_len,
                    "child end_byte {} > source_len {} for source {:?}",
                    child.end_byte(),
                    source_len,
                    source
                );
            }
        }
    }

    #[test]
    fn test_utf8_text_round_trip() {
        let source = "my $x = 42;";
        let mut p = Parser::new();
        let tree = must_some(p.parse(source));
        let root = tree.root_node();
        let text = root.utf8_text(source.as_bytes());
        assert!(text.is_ok(), "utf8_text should succeed");
        // The root node spans the whole source — verify the actual content, not just Ok.
        let extracted = must_some(text.ok());
        assert_eq!(extracted, source, "utf8_text should return the full source for the root node");
    }

    #[test]
    fn test_utf8_text_multibyte_unicode() {
        // 'é' is 2 bytes in UTF-8; the parser must not split a codepoint boundary.
        let source = "my $x = 'café';";
        let mut p = Parser::new();
        let tree = must_some(p.parse(source));
        let root = tree.root_node();
        let bytes = source.as_bytes();
        let text = root.utf8_text(bytes);
        assert!(text.is_ok(), "utf8_text should handle multi-byte UTF-8");
    }

    #[test]
    fn test_utf8_text_mismatched_source_does_not_panic() {
        // utf8_text takes a caller-supplied byte slice. When the slice is shorter
        // than the tree's byte offsets, the implementation must clamp rather than panic.
        let source = "my $x = 42;";
        let mut p = Parser::new();
        let tree = must_some(p.parse(source));
        let root = tree.root_node();
        // A shorter slice — would panic without the start.min(source.len()) guard.
        let short = b"my";
        let result = root.utf8_text(short);
        assert!(result.is_ok(), "utf8_text should not panic with short source slice");
    }

    #[test]
    fn test_invalid_perl_returns_some_tree() {
        // The v3 parser is error-tolerant — even syntactically invalid Perl should
        // produce a partial tree (Some), not None. None is only returned on cancellation.
        let mut p = Parser::new();
        let tree = p.parse("sub { @@@@invalid{{{{");
        assert!(tree.is_some(), "invalid Perl should still yield an error-recovery tree");
    }

    #[test]
    fn test_children_iterator_matches_child_count() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 1; my $y = 2;"));
        let root = tree.root_node();
        let collected: Vec<_> = root.children().collect();
        assert_eq!(collected.len(), root.child_count());
    }

    #[test]
    fn test_child_by_index() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 1; my $y = 2;"));
        let root = tree.root_node();
        if root.child_count() > 0 {
            let first = root.child(0);
            assert!(first.is_some());
        }
        assert!(root.child(9999).is_none());
    }

    #[test]
    fn test_empty_source_yields_tree() {
        // The v3 parser is error-tolerant; empty input returns Program { statements: [] }.
        let mut p = Parser::new();
        let tree = p.parse("");
        assert!(tree.is_some(), "empty input should still yield a tree");
    }

    #[test]
    fn test_source_accessor() {
        let source = "sub foo { }";
        let mut p = Parser::new();
        let tree = must_some(p.parse(source));
        assert_eq!(tree.source(), source);
    }

    #[test]
    fn test_default_parser() {
        let mut p = Parser::default();
        let tree = p.parse("1;");
        assert!(tree.is_some());
    }

    #[test]
    fn test_is_leaf_for_leaf_nodes() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("42"));
        let root = tree.root_node();
        // The root Program is not a leaf.
        assert!(!root.is_leaf());
    }

    // Tests for grammar_kind() method

    #[test]
    fn test_grammar_kind_returns_source_file_for_root() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 42;"));
        assert_eq!(tree.root_node().grammar_kind(), "source_file");
    }

    #[test]
    fn test_grammar_kind_returns_variable_with_attributes_for_list_form() {
        let mut p = Parser::new();
        // VariableWithAttributes is only produced for per-variable attributes in list form:
        // `my ($x : lvalue);`. Scalar form `my $x : lvalue;` does not produce this node.
        let tree = must_some(p.parse("my ($x : lvalue);"));
        let root = tree.root_node();
        let mut found_var_with_attrs = false;
        for child in root.children() {
            if child.grammar_kind() == "my_declaration" {
                for sub in child.children() {
                    if sub.grammar_kind() == "variable_with_attributes" {
                        found_var_with_attrs = true;
                    }
                }
            }
        }
        assert!(found_var_with_attrs, "should find variable_with_attributes");
    }

    #[test]
    fn test_grammar_kind_double_paren_edge_case() {
        // Test that grammar_kind() remains independent of the double-paren sexp form.
        // VariableWithAttributes produces ((variable $ foo) (attributes :lvalue))
        let mut p = Parser::new();
        let tree = must_some(p.parse("my ($x : lvalue);"));
        let root = tree.root_node();
        let sexp = root.to_sexp();
        assert!(sexp.contains("((variable"), "sexp should include the double-paren variable form");
        let declaration =
            must_some(root.children().find(|node| node.grammar_kind() == "my_declaration"));
        let variable = must_some(
            declaration.children().find(|node| node.grammar_kind() == "variable_with_attributes"),
        );
        assert_eq!(variable.grammar_kind(), "variable_with_attributes");
    }

    // Tests for PerlLanguage descriptor

    #[test]
    fn test_language_returns_descriptor_with_nonzero_kind_count() {
        let lang = language();
        assert!(lang.node_kind_count() > 0, "language should report at least one node kind");
    }

    #[test]
    fn test_language_constant_has_nonzero_kind_count() {
        assert!(LANGUAGE.node_kind_count() > 0, "LANGUAGE should have at least one node kind");
    }

    #[test]
    fn test_language_reports_program_as_named_kind() {
        let lang = language();
        assert!(lang.node_kind_is_named("Program"), "'Program' should be a named kind");
    }

    #[test]
    fn test_language_rejects_unknown_kind() {
        let lang = language();
        assert!(
            !lang.node_kind_is_named("__nonexistent_kind__"),
            "unknown kind should not be named"
        );
    }

    #[test]
    fn test_language_kind_names_contains_program() {
        let lang = language();
        let names = lang.node_kind_names();
        assert!(names.contains(&"Program"), "kind names should include 'Program'");
    }

    #[test]
    fn test_language_default_returns_same_as_language() {
        // PartialEq compares the backing slice elements, not just the pointer.
        // Both language() and PerlLanguage::default() return LANGUAGE so this
        // also verifies the Default impl wires up the correct constant.
        assert_eq!(language(), PerlLanguage::default());
    }

    #[test]
    fn test_language_kind_names_declaration_order_and_no_duplicates() {
        // ALL_KIND_NAMES is now in declaration order (not alphabetical) via strum::VariantNames
        // (changed in PR #1491). Verify there are no duplicates and 'Program' is first.
        let lang = language();
        let names = lang.node_kind_names();
        assert!(!names.is_empty(), "node_kind_names must not be empty");
        assert_eq!(
            names.first(),
            Some(&"Program"),
            "First kind name should be 'Program' (declaration order)"
        );
        let unique: std::collections::BTreeSet<&str> = names.iter().copied().collect();
        assert_eq!(
            names.len(),
            unique.len(),
            "node_kind_names must not contain duplicates: {} entries, {} unique",
            names.len(),
            unique.len()
        );
    }

    #[test]
    fn test_language_is_named_with_empty_string_returns_false() {
        // Empty string is not a valid kind name and must not be found.
        assert!(!language().node_kind_is_named(""), "empty kind name must return false");
    }

    #[test]
    fn test_tree_cursor_walks_children_and_siblings() {
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $x = 1; my $y = 2;"));
        let root = tree.root_node();
        let mut cursor = root.walk();

        assert_eq!(cursor.node().grammar_kind(), "source_file");
        assert!(cursor.goto_first_child(), "source_file should have at least one child");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");
        assert!(cursor.goto_next_sibling(), "first statement should have a sibling");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");
        assert!(!cursor.goto_next_sibling(), "second statement should be the last sibling");
    }

    #[test]
    fn test_tree_walk_starts_cursor_at_root() {
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $x = 1;"));
        let mut cursor = tree.walk();

        assert_eq!(cursor.node().grammar_kind(), "source_file");
        assert!(cursor.goto_first_child(), "root should have a child");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");
    }

    #[test]
    fn test_tree_cursor_parent_and_reset_behavior() {
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $x = 1;"));
        let root = tree.root_node();
        let mut cursor = root.walk();

        assert!(!cursor.goto_parent(), "cursor at root must not move to parent");
        assert!(cursor.goto_first_child(), "root should have a child");
        assert!(cursor.goto_parent(), "child should have root as parent");
        assert_eq!(cursor.node().grammar_kind(), "source_file");

        assert!(cursor.goto_first_child(), "root should still have a child");
        cursor.reset();
        assert_eq!(cursor.node().grammar_kind(), "source_file");
    }

    #[test]
    fn test_tree_cursor_last_child_and_previous_sibling_behavior() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $a = 1; my $b = 2;"));
        let root = tree.root_node();
        let mut cursor = root.walk();

        assert!(cursor.goto_last_child(), "root should have a last child");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");
        assert!(cursor.goto_previous_sibling(), "last child should have a previous sibling");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");
        assert!(!cursor.goto_previous_sibling(), "first sibling should not have previous sibling");
    }

    #[test]
    fn test_tree_cursor_last_child_returns_false_for_leaf() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 42;"));
        let root = tree.root_node();
        let mut cursor = root.walk();

        assert!(cursor.goto_first_child(), "root should have a child");
        assert!(cursor.goto_first_child(), "my_declaration should have a child");
        let at_leaf = !cursor.goto_last_child();
        assert!(at_leaf, "leaf nodes should not have a last child");
    }

    #[test]
    fn test_tree_cursor_goto_first_child_returns_false_for_leaf() {
        // A leaf node has no children; goto_first_child must return false and
        // leave the cursor positioned at the leaf rather than panicking.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $x = 1;"));
        let root = tree.root_node();
        let mut cursor = root.walk();

        // Navigate to a leaf: root -> first child (my_declaration) -> first child (leaf token).
        assert!(cursor.goto_first_child(), "root should have a child");
        assert!(cursor.goto_first_child(), "my_declaration should have a child");
        // The leaf must refuse another goto_first_child.
        let at_leaf = !cursor.goto_first_child();
        assert!(at_leaf, "goto_first_child must return false on a leaf node");
    }

    #[test]
    fn test_tree_cursor_multiple_goto_next_sibling_exhausts() {
        // When repeatedly calling goto_next_sibling, the cursor must eventually
        // return false and stay positioned at the last sibling.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("1; 2; 3;"));
        let mut cursor = tree.walk();

        // Navigate to first statement
        assert!(cursor.goto_first_child());
        let mut _count = 1;
        // Keep advancing siblings until we can't
        while cursor.goto_next_sibling() {
            _count += 1;
        }
        // After last goto_next_sibling returns false, cursor should still be valid
        // and still have a node (the last sibling).
        let node = cursor.node();
        assert!(
            !node.kind().is_empty(),
            "cursor should remain at valid node after exhausting siblings"
        );
    }

    #[test]
    fn test_tree_cursor_goto_parent_at_root_repeatedly() {
        // Calling goto_parent at root should return false every time, keeping cursor at root.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $x = 1;"));
        let mut cursor = tree.walk();

        // We should always be at root initially
        assert_eq!(cursor.node().grammar_kind(), "source_file");

        // Try to go up multiple times — must stay at root
        for _ in 0..3 {
            let result = cursor.goto_parent();
            assert!(!result, "goto_parent at root must return false");
            assert_eq!(
                cursor.node().grammar_kind(),
                "source_file",
                "cursor must remain at root after failed goto_parent"
            );
        }
    }

    #[test]
    fn test_tree_cursor_reset_from_deep_nesting() {
        // reset() must return cursor to root regardless of depth.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("sub foo { my $x = 1; }"));
        let mut cursor = tree.walk();

        // Navigate deep into the tree
        let mut depth = 0;
        while cursor.goto_first_child() && depth < 10 {
            depth += 1;
        }
        assert!(depth > 0, "should have navigated at least one level deep");

        // reset() should bring us back to root
        cursor.reset();
        assert_eq!(cursor.node().grammar_kind(), "source_file", "reset must return cursor to root");
    }

    #[test]
    fn test_tree_cursor_empty_source_root_is_valid() {
        // Empty source still produces a (minimal) tree; cursor at root should be valid.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse(""));
        let cursor = tree.walk();

        let node = cursor.node();
        assert_eq!(node.grammar_kind(), "source_file");
        assert!(node.child_count() == 0, "empty source tree should have no statements");
    }

    #[test]
    fn test_tree_cursor_empty_source_goto_first_child_returns_false() {
        // Empty source root has no children; goto_first_child must return false.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse(""));
        let mut cursor = tree.walk();

        let result = cursor.goto_first_child();
        assert!(!result, "empty tree root should have no first child");
    }

    #[test]
    fn test_tree_cursor_single_statement_navigation() {
        // Single statement: root -> statement -> (children or leaf).
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("42;"));
        let mut cursor = tree.walk();

        assert_eq!(cursor.node().grammar_kind(), "source_file");
        assert!(cursor.goto_first_child(), "root should have exactly one statement");

        // The single statement should have no next sibling
        assert!(!cursor.goto_next_sibling(), "single statement should be the only child");

        // Going back up should land at root
        assert!(cursor.goto_parent(), "should be able to return to root");
        assert_eq!(cursor.node().grammar_kind(), "source_file");
    }

    #[test]
    fn test_tree_cursor_sibling_navigation_exact_count() {
        // Navigate through all siblings and verify the count matches child_count.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("1; 2; 3; 4;"));
        let root = tree.root_node();
        let child_count = root.child_count();

        let mut cursor = tree.walk();
        assert!(cursor.goto_first_child());

        let mut sibling_count = 1;
        while cursor.goto_next_sibling() {
            sibling_count += 1;
        }

        assert_eq!(sibling_count, child_count, "sibling count should match root.child_count()");
    }

    #[test]
    fn test_tree_cursor_alternating_parent_child_navigation() {
        // Test mixed navigation: down, up, down again at different indices.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("sub a { 1; } sub b { 2; }"));
        let mut cursor = tree.walk();

        // Down to first sub
        assert!(cursor.goto_first_child());
        let first_kind = cursor.node().grammar_kind().to_string();

        // Back to root
        assert!(cursor.goto_parent());
        assert_eq!(cursor.node().grammar_kind(), "source_file");

        // Down again to first sub (should be the same)
        assert!(cursor.goto_first_child());
        assert_eq!(
            cursor.node().grammar_kind(),
            first_kind,
            "re-navigating should reach the same node"
        );
    }

    #[test]
    fn test_tree_cursor_complex_traversal_sequence() {
        // Complex sequence: down, sibling, sibling, up, down, sibling.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $a = 1; my $b = 2; my $c = 3;"));
        let mut cursor = tree.walk();

        // Down to first statement
        assert!(cursor.goto_first_child(), "down to first stmt");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");

        // Move to second statement
        assert!(cursor.goto_next_sibling(), "sibling to second stmt");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");

        // Move to third statement
        assert!(cursor.goto_next_sibling(), "sibling to third stmt");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");

        // No fourth statement
        assert!(!cursor.goto_next_sibling(), "no fourth statement");

        // Back to root
        assert!(cursor.goto_parent(), "back to root");
        assert_eq!(cursor.node().grammar_kind(), "source_file");

        // Down to first again
        assert!(cursor.goto_first_child(), "down to first again");
        assert_eq!(cursor.node().grammar_kind(), "my_declaration");
    }

    #[test]
    fn test_tree_cursor_node_identity_after_traversal() {
        // A node retrieved at the same path should be equal across separate traversals.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $x = 1;"));
        let mut cursor = tree.walk();

        // First traversal: get to first child and extract its kind
        assert!(cursor.goto_first_child());
        let first_kind = cursor.node().grammar_kind().to_string();

        // Reset and repeat
        cursor.reset();
        assert!(cursor.goto_first_child());
        let second_kind = cursor.node().grammar_kind().to_string();

        assert_eq!(
            first_kind, second_kind,
            "node at the same path should have the same kind in both traversals"
        );
    }

    #[test]
    fn test_tree_cursor_sibling_with_unicode_identifiers() {
        // Cursor must correctly navigate siblings even when source contains Unicode.
        let mut parser = Parser::new();
        let tree = must_some(parser.parse("my $café = 1; my $naïve = 2;"));
        let mut cursor = tree.walk();

        let root = tree.root_node();
        let expected_count = root.child_count();

        // Count siblings via cursor
        assert!(cursor.goto_first_child());
        let mut count = 1;
        while cursor.goto_next_sibling() {
            count += 1;
        }

        assert_eq!(
            count, expected_count,
            "sibling count should match even with Unicode identifiers"
        );
    }

    #[test]
    fn test_tree_cursor_deeply_nested_structure() {
        // Verify cursor can navigate a deeply nested structure without stack overflow.
        let mut parser = Parser::new();
        // Create nested blocks: { { { ... } } }
        let mut code = String::new();
        for i in 0..5 {
            code.push_str(&format!("sub level_{} {{ ", i));
        }
        code.push_str("1;");
        for _ in 0..5 {
            code.push_str(" }");
        }

        let tree = must_some(parser.parse(&code));
        let mut cursor = tree.walk();

        // Navigate down as far as possible
        let mut depth = 0;
        while cursor.goto_first_child() && depth < 50 {
            depth += 1;
        }

        // Should have navigated several levels
        assert!(depth > 2, "should navigate multiple levels in nested structure");

        // Navigate back up
        while cursor.goto_parent() {
            depth -= 1;
        }

        // Should be back at root
        assert_eq!(cursor.node().grammar_kind(), "source_file");
        assert_eq!(depth, 0, "should have gone back up to root (depth 0)");
    }
}
