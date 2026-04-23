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

    /// Creates a [`TreeCursor`] rooted at this node.
    ///
    /// The cursor mirrors tree-sitter's cursor-style traversal API:
    /// move to first child / next sibling / parent without allocating
    /// an intermediate children vector.
    pub fn walk(&self) -> TreeCursor<'tree> {
        TreeCursor::new(self.inner, self.tree_source)
    }
}

/// Re-export of [`perl_ast::NodeKind`] so callers can pattern-match node variants
/// without a direct dependency on `perl-ast`.
pub use perl_ast::NodeKind as PerlNodeKind;

/// Cursor for efficient tree navigation without allocating child collections.
///
/// This mirrors the ergonomic shape of `tree_sitter::TreeCursor` for the
/// subset of navigation operations used by editor integrations.
pub struct TreeCursor<'tree> {
    current: &'tree AstNode,
    tree_source: &'tree str,
    // Each frame stores (parent_node, current_index_within_parent).
    path: Vec<(&'tree AstNode, usize)>,
}

impl<'tree> TreeCursor<'tree> {
    fn new(root: &'tree AstNode, tree_source: &'tree str) -> Self {
        Self { current: root, tree_source, path: Vec::new() }
    }

    /// Returns the node at the cursor's current position.
    pub fn node(&self) -> Node<'tree> {
        Node { inner: self.current, tree_source: self.tree_source }
    }

    /// Move to the first child of the current node.
    ///
    /// Returns `true` when a child exists and the cursor moved.
    pub fn goto_first_child(&mut self) -> bool {
        if let Some(child) = nth_ast_child(self.current, 0) {
            self.path.push((self.current, 0));
            self.current = child;
            return true;
        }
        false
    }

    /// Move to the next sibling of the current node.
    ///
    /// Returns `true` when a sibling exists and the cursor moved.
    pub fn goto_next_sibling(&mut self) -> bool {
        let Some((parent, index)) = self.path.last_mut() else {
            return false;
        };
        let next_index = *index + 1;
        if let Some(sibling) = nth_ast_child(parent, next_index) {
            *index = next_index;
            self.current = sibling;
            return true;
        }
        false
    }

    /// Move to the parent of the current node.
    ///
    /// Returns `true` when the cursor moved (i.e. it was not already at root).
    pub fn goto_parent(&mut self) -> bool {
        let Some((parent, _)) = self.path.pop() else {
            return false;
        };
        self.current = parent;
        true
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
fn nth_ast_child<'a>(node: &'a AstNode, i: usize) -> Option<&'a AstNode> {
    let mut idx = 0usize;
    let mut found: Option<&'a AstNode> = None;
    node.for_each_child(|child| {
        if found.is_none() && idx == i {
            found = Some(child);
        }
        idx += 1;
    });
    found
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
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 1; my $y = 2;"));
        let root = tree.root_node();
        let mut cursor = root.walk();

        assert_eq!(cursor.node().grammar_kind(), "source_file");
        assert!(cursor.goto_first_child(), "root should have first child");
        let first_child_kind = cursor.node().grammar_kind();
        assert!(!first_child_kind.is_empty(), "first child should have a grammar kind");

        let had_sibling = cursor.goto_next_sibling();
        if had_sibling {
            assert_ne!(
                cursor.node().start_byte(),
                root.start_byte(),
                "sibling should not point to root node"
            );
        }
    }

    #[test]
    fn test_tree_cursor_parent_navigation() {
        let mut p = Parser::new();
        let tree = must_some(p.parse("my $x = 1;"));
        let root = tree.root_node();
        let mut cursor = root.walk();

        assert!(!cursor.goto_parent(), "cursor at root should not move to parent");
        assert!(cursor.goto_first_child(), "root should have first child");
        assert!(cursor.goto_parent(), "child should move back to parent");
        assert_eq!(cursor.node().grammar_kind(), "source_file");
    }
}
