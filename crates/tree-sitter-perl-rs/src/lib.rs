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
use perl_parser_core::Parser as CoreParser;

/// Re-export of Edit type for tree-sitter-compatible incremental parsing.
///
/// Mirrors `tree_sitter::InputEdit` field layout for drop-in compatibility.
pub use perl_parser_core::edit::Edit as InputEdit;

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
    pub fn parse_with_old_tree(&mut self, source: &str, _old_tree: &Tree) -> Option<Tree> {
        self.parse(source)
    }
}

impl Default for Parser {
    fn default() -> Self {
        Self::new()
    }
}

/// The result of a successful parse: an owned syntax tree and the source text.
///
/// Use [`root_node`][Tree::root_node] to begin traversal.
pub struct Tree {
    root: AstNode,
    source: String,
    /// Pending edits recorded via [`Tree::edit`].
    pending_edits: Vec<InputEdit>,
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
    /// `"source_file"`. Use [`to_sexp`][Node::to_sexp] for tree-sitter-compatible
    /// S-expression output which uses the canonical grammar names.
    pub fn kind(&self) -> &'static str {
        self.inner.kind.kind_name()
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
        let mut count = 0usize;
        self.inner.for_each_child(|_| count += 1);
        count
    }

    /// Returns the `i`-th direct child, or `None` if out of range.
    pub fn child(&self, i: usize) -> Option<Node<'tree>> {
        let mut idx = 0usize;
        let mut found: Option<&'tree AstNode> = None;
        self.inner.for_each_child(|child| {
            if found.is_none() && idx == i {
                found = Some(child);
            }
            idx += 1;
        });
        found.map(|child| Node { inner: child, tree_source: self.tree_source })
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
        self.inner.location.end
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
}

/// Re-export of [`perl_ast::NodeKind`] so callers can pattern-match node variants
/// without a direct dependency on `perl-ast`.
pub use perl_ast::NodeKind as PerlNodeKind;

// ---------------------------------------------------------------------------
// Private helper — access AstNode's children without name collision
// ---------------------------------------------------------------------------

// Collect the direct children of an `AstNode` as a `Vec<&AstNode>`.
//
// This thin wrapper exists because the public `Node::children()` method in `perl_ast`
// has the same name as our facade method and would be ambiguous in `impl` blocks.
#[inline]
fn ast_children(node: &AstNode) -> Vec<&AstNode> {
    node.children()
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
        // End byte from the Program node spans to end of last statement.
        assert!(root.end_byte() <= source.len() + 1, "end_byte out of range");
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
        let extracted = text.unwrap();
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
}
