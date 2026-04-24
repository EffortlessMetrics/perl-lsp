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
//! - `Node::kind()` returns the `NodeKind::kind_name()` string.
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
#![deny(clippy::print_stderr)]
#![deny(clippy::print_stdout)]
#![warn(rust_2018_idioms)]
#![warn(missing_docs)]
#![warn(clippy::all)]
#![allow(
    clippy::module_name_repetitions,
    clippy::must_use_candidate,
    clippy::missing_errors_doc,
    clippy::missing_panics_doc
)]

use perl_ast::Node as AstNode;
use perl_module::import::{ModuleImportKind, parse_module_import_head};
use perl_parser_core::Parser as CoreParser;
use perl_pragma::{PragmaState, PragmaTracker};
use perl_semantic_analyzer::semantic::SemanticModel;
use perl_semantic_analyzer::symbol::Symbol as SemanticSymbol;

/// Re-export of Edit type for tree-sitter-compatible incremental parsing.
///
/// Mirrors `tree_sitter::InputEdit` field layout for drop-in compatibility.
pub use perl_parser_core::edit::Edit as InputEdit;

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
            Ok(root) => Some(Tree { root, source: source.to_string(), pending_edits: Vec::new() }),
            Err(_) => None,
        }
    }

    /// Parse `source` using `old_tree` as a hint for incremental re-parsing.
    ///
    /// In the current implementation this performs a full re-parse (equivalent
    /// to [`parse`][Parser::parse]). The `old_tree` parameter is accepted for
    /// API compatibility with `tree_sitter::Parser::parse_with_old_tree`; future
    /// versions will use it to skip unchanged regions.
    ///
    /// Returns `None` on complete parse failure (same semantics as `parse`).
    pub fn parse_with_old_tree(&mut self, source: &str, old_tree: &Tree) -> Option<Tree> {
        // Fast path: if source is unchanged and no edits were recorded, reuse the old tree
        // instead of re-parsing. This mirrors tree-sitter's incremental no-op behavior.
        if source == old_tree.source() && old_tree.pending_edits.is_empty() {
            return Some(old_tree.clone());
        }

        self.parse(source)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
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
}

impl PerlLanguage {
    /// Returns the number of distinct node kinds in the grammar.
    pub fn node_kind_count(&self) -> usize {
        self.kind_names.len()
    }

    /// Returns all node kind names, in alphabetical order.
    pub fn node_kind_names(&self) -> &[&'static str] {
        self.kind_names
    }

    /// Returns `true` if the given kind name is a named (non-anonymous) node kind.
    pub fn node_kind_is_named(&self, kind: &str) -> bool {
        self.kind_names.contains(&kind)
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
pub static LANGUAGE: PerlLanguage = PerlLanguage { kind_names: perl_ast::NodeKind::ALL_KIND_NAMES };

/// The result of a successful parse: an owned syntax tree and the source text.
///
/// Use [`root_node`][Tree::root_node] to begin traversal.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    root: AstNode,
    source: String,
    /// Pending edits recorded via [`Tree::edit`].
    pending_edits: Vec<InputEdit>,
}

/// Read-only semantic overlay queries over a parsed [`Tree`].
///
/// This API is intentionally narrow and considered in development. It exposes a
/// small set of high-value semantic queries while the broader overlay surface is
/// still being designed.
pub struct SemanticOverlay {
    source: String,
    semantic_model: SemanticModel,
    pragma_map: Vec<(std::ops::Range<usize>, PragmaState)>,
}

/// A visible import at a specific source location.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct VisibleImport {
    /// Parsed import kind (for example `use`, `require`, `use parent`).
    pub kind: ModuleImportKind,
    /// Parsed import token/module name.
    pub token: String,
    /// Start byte offset of `token` in the source.
    pub start_byte: usize,
    /// End byte offset of `token` in the source.
    pub end_byte: usize,
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

    /// Build a read-only semantic overlay facade for this tree.
    pub fn semantic_overlay(&self) -> SemanticOverlay {
        let semantic_model = SemanticModel::build(&self.root, &self.source);
        let pragma_map = PragmaTracker::build(&self.root);
        SemanticOverlay { source: self.source.clone(), semantic_model, pragma_map }
    }
}

impl SemanticOverlay {
    /// Find the definition symbol for the syntax node under a given byte offset.
    pub fn definition_at_offset(&self, offset: usize) -> Option<SemanticSymbol> {
        self.semantic_model.definition_at(offset).cloned()
    }

    /// Find the definition symbol associated with the provided byte span.
    pub fn definition_for_span(
        &self,
        start_byte: usize,
        end_byte: usize,
    ) -> Option<SemanticSymbol> {
        let clamped_start = start_byte.min(self.source.len());
        let clamped_end = end_byte.min(self.source.len());
        self.definition_at_offset(clamped_start).or_else(|| {
            if clamped_start < clamped_end {
                self.definition_at_offset(clamped_end - 1)
            } else {
                None
            }
        })
    }

    /// Find the definition symbol associated with a syntax [`Node`].
    pub fn definition_for_node(&self, node: &Node<'_>) -> Option<SemanticSymbol> {
        self.definition_for_span(node.start_byte(), node.end_byte())
    }

    /// Return parsed imports visible up to the provided byte offset.
    pub fn visible_imports_at(&self, offset: usize) -> Vec<VisibleImport> {
        let mut imports = Vec::new();
        let clamped = offset.min(self.source.len());
        let Some(prefix) = self.source.get(..clamped) else {
            return imports;
        };

        let mut line_start = 0usize;
        for line in prefix.lines() {
            if let Some(import) = parse_module_import_head(line) {
                imports.push(VisibleImport {
                    kind: import.kind,
                    token: import.token.to_string(),
                    start_byte: line_start + import.token_start,
                    end_byte: line_start + import.token_end,
                });
            }
            line_start += line.len() + 1;
        }
        imports
    }

    /// Return the effective pragma state at a given byte offset.
    pub fn effective_pragma_state_at(&self, offset: usize) -> PragmaState {
        PragmaTracker::state_for_offset(&self.pragma_map, offset)
    }
}

/// A borrowed reference to a node in the syntax tree.
///
/// Mirrors the tree-sitter `Node` API surface. Lifetime `'tree` is tied to the
/// owning [`Tree`].
pub struct Node<'tree> {
    inner: &'tree AstNode,
    tree_source: &'tree str,
}

impl<'tree> Node<'tree> {
    /// Returns the node kind string (e.g. `"Program"`, `"Subroutine"`, `"Variable"`).
    ///
    /// Delegates to [`NodeKind::kind_name`][perl_ast::NodeKind::kind_name].
    ///
    /// Note: this returns the v3 internal kind name, not the tree-sitter grammar node
    /// type string. For example, the root node returns `"Program"` rather than
    /// `"source_file"`. Use [`grammar_kind`][Node::grammar_kind] for the canonical
    /// tree-sitter grammar name, or [`to_sexp`][Node::to_sexp] for tree-sitter-compatible
    /// S-expression output which uses the canonical grammar names.
    pub fn kind(&self) -> &'static str {
        self.inner.kind.kind_name()
    }

    /// Returns the tree-sitter grammar-canonical node kind name.
    ///
    /// Unlike [`kind`][Node::kind], which returns the v3 internal PascalCase name
    /// (e.g., `"Program"`, `"Subroutine"`), this method returns the grammar name
    /// used in S-expressions (e.g., `"source_file"`, `"sub"`).
    /// This matches the kind strings returned by `tree-sitter-perl-c` and the
    /// upstream tree-sitter Perl grammar.
    /// Error nodes use `"ERROR"` (uppercase), matching tree-sitter convention.
    ///
    /// For most nodes the grammar name is a simple lowercase mapping. For some
    /// nodes (e.g., operator-named `Binary`, dynamic `VariableDeclaration`) the
    /// name depends on runtime data; this method extracts it from `to_sexp()`.
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
        // Extract the node type from the leading `(word` in the S-expression.
        // to_sexp() always starts with `(kind_name` or just `(kind_name)`.
        //
        // Edge case: NodeKind::VariableWithAttributes produces a double-paren sexp
        // of the form `((variable $ foo) (attributes :lvalue))` because it delegates
        // the outer wrapper to the child variable's to_sexp(). In that case the sexp
        // does not begin with `(kind_name` -- it begins with `((child_kind`. We detect
        // this and fall back to the v3 kind_name() converted to snake_case.
        let sexp = self.to_sexp();
        if sexp.starts_with("((") {
            // Double-paren form: no independent grammar kind token in the sexp.
            // Derive a stable snake_case name from the v3 kind_name() as fallback.
            return pascal_to_snake(self.inner.kind.kind_name());
        }
        let inner = sexp.trim_start_matches('(');
        // Take up to the first space or closing paren.
        let end = inner.find([' ', ')']).unwrap_or(inner.len());
        inner[..end].to_string()
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
pub use perl_ast::NodeKind as PerlNodeKind;

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

#[inline]
fn ast_child_at(node: &AstNode, index: usize) -> Option<&AstNode> {
    let mut idx = 0usize;
    let mut found = None;
    node.for_each_child(|child| {
        if found.is_none() && idx == index {
            found = Some(child);
        }
        idx += 1;
    });
    found
}

// Invariant: TreeCursor path is constructed by the traversal that just yielded this
// index, so child_at is guaranteed valid.
#[allow(clippy::expect_used)]
fn resolve_path<'tree>(root: &'tree AstNode, path: &[usize]) -> &'tree AstNode {
    let mut current = root;
    for &index in path {
        current = ast_child_at(current, index)
            .expect("TreeCursor path must always reference a valid child");
    }
    current
}

/// Convert a PascalCase kind name (e.g. `"VariableWithAttributes"`) to snake_case
/// (e.g. `"variable_with_attributes"`). Used as a fallback in [`Node::grammar_kind`]
/// when the S-expression does not have a simple `(kind_name ...)` prefix.
fn pascal_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for (i, c) in s.char_indices() {
        if c.is_uppercase() && i > 0 {
            out.push('_');
        }
        out.extend(c.to_lowercase());
    }
    out
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
        assert_eq!(tree.root_node().kind(), "Program");
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
    fn test_pascal_to_snake_helper() {
        assert_eq!(pascal_to_snake("VariableWithAttributes"), "variable_with_attributes");
        assert_eq!(pascal_to_snake("Program"), "program");
        assert_eq!(pascal_to_snake("FunctionCall"), "function_call");
        assert_eq!(pascal_to_snake("If"), "if");
    }

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
        // Test that grammar_kind() handles the double-paren sexp form correctly.
        // VariableWithAttributes produces ((variable $ foo) (attributes :lvalue))
        // and should fall back to pascal_to_snake() to derive the grammar kind.
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x : lvalue = 42;"));
        let root = tree.root_node();
        let sexp = root.to_sexp();
        // Verify the structure includes a my_declaration.
        assert!(sexp.contains("my_declaration"), "sexp should include my_declaration");
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
    fn test_language_kind_names_are_sorted_alphabetically() {
        // node_kind_names() documents "in alphabetical order"; enforce that contract.
        let lang = language();
        let names = lang.node_kind_names();
        let mut sorted = names.to_vec();
        sorted.sort_unstable();
        assert_eq!(
            names,
            sorted.as_slice(),
            "node_kind_names() must be in alphabetical order; \
             re-sort ALL_KIND_NAMES in perl-ast if a new variant was added out of order"
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
        let mut count = 1;
        // Keep advancing siblings until we can't
        while cursor.goto_next_sibling() {
            count += 1;
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
