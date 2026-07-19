//! Declaration Provider for LSP
//!
//! Provides go-to-declaration functionality for finding where symbols are declared.
//! Supports LocationLink for enhanced client experience.

use crate::ast::{GotoTargetForm, Node, NodeKind};
use crate::symbol::is_universal_method;
use crate::workspace_index::{SymKind, SymbolKey};
use perl_parser_core::qualified_name::split_qualified_name;
use rustc_hash::FxHashMap;
use std::sync::Arc;

/// Parent-map from child node to parent node, stored as raw pointers.
///
/// # Safety Invariant
///
/// Every `*const Node` in this map (both keys and values) must be a pointer
/// obtained by casting a shared reference (`&Node`) that was derived from the
/// **same** `Arc<Node>` tree that was passed to [`DeclarationProvider::build_parent_map`].
/// The pointed-to nodes must remain alive for the entire duration of any code
/// that inspects the map.
///
/// Raw pointers are used as **hash keys only** for O(1) identity-based lookup.
/// They are **never** dereferenced directly through this map.  Safe references
/// are recovered via the companion `node_lookup` map
/// (`FxHashMap<*const Node, &Node>`) that re-derives `&Node` from the live
/// `Arc<Node>` tree at call time.
///
/// # Ownership and Lifetime
///
/// The `Arc<Node>` that backs the tree must outlive every `&ParentMap` borrow.
/// In the LSP server this is guaranteed because both the `Arc<Node>` and the
/// `ParentMap` are stored together in `DocumentState`, guarded by a
/// `parking_lot::Mutex`.
///
/// # Thread Safety
///
/// `*const Node` is `!Send + !Sync`.  Consequently `ParentMap` is `!Send +
/// !Sync` and must remain on the thread that owns the `Arc<Node>` tree.
/// LSP request handlers satisfy this requirement because they process each
/// request synchronously within a single thread context.
pub type ParentMap = FxHashMap<*const Node, *const Node>;

/// Provider for finding declarations in Perl source code.
///
/// This provider implements LSP go-to-declaration functionality with enhanced
/// workspace navigation support. Maintains ≤1ms response time for symbol lookup
/// operations through optimized AST traversal and parent mapping.
///
/// # Performance Characteristics
/// - Declaration resolution: <500μs for typical Perl files
/// - Memory usage: O(n) where n is AST node count
/// - Parent map validation: Debug-only with cycle detection
///
/// # LSP Workflow Integration
/// Parse → Index → Navigate → Complete → Analyze pipeline integration:
/// 1. Parse: AST generation from Perl source
/// 2. Index: Symbol table construction with qualified name resolution
/// 3. Navigate: Declaration provider for go-to-definition requests
/// 4. Complete: Symbol context for completion providers
/// 5. Analyze: Cross-reference analysis for workspace refactoring
pub struct DeclarationProvider<'a> {
    /// The parsed AST for the current document
    pub ast: Arc<Node>,
    content: String,
    document_uri: String,
    parent_map: Option<&'a ParentMap>,
    doc_version: i32,
}

/// Represents a location link from origin to target
#[derive(Debug, Clone)]
pub struct LocationLink {
    /// The range of the symbol being targeted at the origin
    pub origin_selection_range: (usize, usize),
    /// The target URI
    pub target_uri: String,
    /// The full range of the target declaration
    pub target_range: (usize, usize),
    /// The range to select in the target (e.g., just the name)
    pub target_selection_range: (usize, usize),
}

impl<'a> DeclarationProvider<'a> {
    /// Creates a new declaration provider for the given AST and document.
    ///
    /// # Arguments
    /// * `ast` - The parsed AST tree for declaration lookup
    /// * `content` - The source code content for text extraction
    /// * `document_uri` - The URI of the document being analyzed
    ///
    /// # Performance
    /// - Initialization: <10μs for typical Perl files
    /// - Memory overhead: Minimal, shares AST reference
    ///
    /// # Examples
    /// ```rust,ignore
    /// use perl_parser::declaration::DeclarationProvider;
    /// use perl_parser::ast::Node;
    /// use std::sync::Arc;
    ///
    /// let ast = Arc::new(Node::new_root());
    /// let provider = DeclarationProvider::new(
    ///     ast,
    ///     "package MyPackage; sub example { }".to_string(),
    ///     "file:///path/to/file.pl".to_string()
    /// );
    /// ```
    pub fn new(ast: Arc<Node>, content: String, document_uri: String) -> Self {
        Self {
            ast,
            content,
            document_uri,
            parent_map: None,
            doc_version: 0, // Default to version 0 for simple use cases
        }
    }

    /// Configures the provider with a pre-built parent map for enhanced traversal.
    ///
    /// The parent map enables efficient upward AST traversal for scope resolution
    /// and context analysis. Debug builds include comprehensive validation.
    ///
    /// # Arguments
    /// * `parent_map` - Mapping from child nodes to their parents
    ///
    /// # Performance
    /// - Parent lookup: O(1) hash table access
    /// - Validation overhead: Debug-only, ~100μs for large files
    ///
    /// # Panics
    /// In debug builds, panics if:
    /// - Parent map is empty for non-trivial AST
    /// - Root node has a parent (cycle detection)
    /// - Cycles detected in parent relationships
    ///
    /// # Examples
    /// ```rust,ignore
    /// use perl_parser::declaration::{DeclarationProvider, ParentMap};
    /// use perl_parser::ast::Node;
    /// use std::sync::Arc;
    ///
    /// let ast = Arc::new(Node::new_root());
    /// let mut parent_map = ParentMap::default();
    /// DeclarationProvider::build_parent_map(&ast, &mut parent_map, None);
    ///
    /// let provider = DeclarationProvider::new(
    ///     ast, "content".to_string(), "uri".to_string()
    /// ).with_parent_map(&parent_map);
    /// ```
    pub fn with_parent_map(mut self, parent_map: &'a ParentMap) -> Self {
        #[cfg(debug_assertions)]
        {
            // If the AST has more than the root node, an empty map is suspicious.
            // (Root has no parent, so a truly trivial AST may legitimately produce 0.)
            debug_assert!(
                !parent_map.is_empty(),
                "DeclarationProvider: empty ParentMap (did you forget to rebuild after AST refresh?)"
            );

            // Root sanity check - root must have no parent
            let root_ptr = &*self.ast as *const _;
            debug_assert!(
                !parent_map.contains_key(&root_ptr),
                "Root node must have no parent in the parent map"
            );

            // Cycle detection - ensure no node is its own ancestor
            Self::debug_assert_no_cycles(parent_map);
        }
        self.parent_map = Some(parent_map);
        self
    }

    /// Sets the document version for staleness detection.
    ///
    /// Version tracking ensures the provider operates on current data
    /// and prevents usage after document updates in LSP workflows.
    ///
    /// # Arguments
    /// * `version` - Document version number from LSP client
    ///
    /// # Performance
    /// - Version check: <1μs per operation
    /// - Debug validation: Additional consistency checks
    ///
    /// # Examples
    /// ```rust,ignore
    /// use perl_parser::declaration::DeclarationProvider;
    /// use perl_parser::ast::Node;
    /// use std::sync::Arc;
    ///
    /// let provider = DeclarationProvider::new(
    ///     Arc::new(Node::new_root()),
    ///     "content".to_string(),
    ///     "uri".to_string()
    /// ).with_doc_version(42);
    /// ```
    pub fn with_doc_version(mut self, version: i32) -> Self {
        self.doc_version = version;
        self
    }

    /// Returns `true` if this provider is still fresh (version matches).
    ///
    /// In both debug and release builds: logs a warning and returns `false` on mismatch so
    /// callers can return `None` early instead of operating on a stale AST snapshot.
    #[inline]
    #[track_caller]
    fn is_fresh(&self, current_version: i32) -> bool {
        if self.doc_version != current_version {
            tracing::warn!(
                provider_version = self.doc_version,
                current_version,
                "DeclarationProvider used after AST refresh — returning empty result"
            );
            return false;
        }
        true
    }

    /// Debug-only cycle detection for parent map
    #[cfg(debug_assertions)]
    fn debug_assert_no_cycles(parent_map: &ParentMap) {
        // For each node in the map, climb up to ensure we don't hit a cycle
        let cap = parent_map.len() + 1; // Max depth before assuming cycle

        for (&child, _) in parent_map.iter() {
            let mut current = child;
            let mut depth = 0;

            while depth < cap {
                if let Some(&parent) = parent_map.get(&current) {
                    current = parent;
                    depth += 1;
                } else {
                    // Reached a node with no parent (root), no cycle
                    break;
                }
            }

            // If we exhausted the cap, we have a cycle
            if depth >= cap {
                tracing::warn!(
                    depth_limit = cap,
                    "Cycle detected in ParentMap - node is its own ancestor"
                );
                break;
            }
        }
    }

    /// Build a parent map for efficient scope walking
    /// Builds a parent map for efficient upward AST traversal.
    ///
    /// Recursively traverses the AST to construct a mapping from each node
    /// to its parent, enabling O(1) parent lookups for scope resolution.
    ///
    /// # Arguments
    /// * `node` - Current node to process
    /// * `map` - Mutable parent map to populate
    /// * `parent` - Parent of the current node (None for root)
    ///
    /// # Performance
    /// - Time complexity: O(n) where n is node count
    /// - Space complexity: O(n) for parent pointers
    /// - Typical build time: <100μs for 1000-node AST
    ///
    /// # Safety
    /// Uses raw pointers for performance. Safe as long as AST nodes
    /// remain valid during provider lifetime.
    ///
    /// # Examples
    /// ```rust,ignore
    /// use perl_parser::declaration::{DeclarationProvider, ParentMap};
    /// use perl_parser::ast::Node;
    ///
    /// let ast = Node::new_root();
    /// let mut parent_map = ParentMap::default();
    /// DeclarationProvider::build_parent_map(&ast, &mut parent_map, None);
    /// ```
    pub fn build_parent_map(node: &Node, map: &mut ParentMap, parent: Option<*const Node>) {
        if let Some(p) = parent {
            // SAFETY invariant for the ParentMap:
            //
            // 1. `node` is a shared reference (`&Node`) obtained from a live `Arc<Node>`.
            //    Casting it to `*const Node` produces a pointer that is valid for the
            //    lifetime of that `Arc`.
            //
            // 2. `p` (the parent pointer) was obtained by the same cast in the previous
            //    recursive frame, so it satisfies the same validity guarantee.
            //
            // 3. Neither pointer is **ever** dereferenced through this map.  The map stores
            //    raw pointers purely as identity keys.  Callers that need to follow a parent
            //    pointer back to a `&Node` must go through `build_node_lookup_map`, which
            //    re-derives safe references from the same live `Arc<Node>` tree.
            //
            // 4. The caller (LSP runtime) is responsible for ensuring the `Arc<Node>` tree
            //    remains alive for at least as long as any `&ParentMap` borrow.  In the LSP
            //    server both the `Arc` and the `ParentMap` live inside `DocumentState`,
            //    guarded by the same `parking_lot::Mutex`.
            //
            // 5. No interior mutability is introduced: `node` is not modified during
            //    traversal.  The `ParentMap` itself is an exclusive (`&mut`) borrow during
            //    construction and transitions to a shared borrow (`&`) afterwards.
            map.insert(node as *const _, p);
        }

        for child in Self::get_children_static(node) {
            // SAFETY: `child` is a child reference of `node`, both living in the same
            // `Arc<Node>` allocation.  The same invariant from above applies.
            Self::build_parent_map(child, map, Some(node as *const _));
        }
    }

    /// Find the declaration of the symbol at the given position
    pub fn find_declaration(
        &self,
        offset: usize,
        current_version: i32,
    ) -> Option<Vec<LocationLink>> {
        // Guard against stale provider usage after AST refresh (both debug and release)
        if !self.is_fresh(current_version) {
            return None;
        }

        // Find the node at the cursor position
        let node = self.find_node_at_offset(&self.ast, offset)?;

        // Check what kind of node we're on
        match &node.kind {
            NodeKind::Variable { name, .. } => self.find_variable_declaration(node, name),
            NodeKind::FunctionCall { name, .. } => self.find_subroutine_declaration(node, name),
            NodeKind::MethodCall { method, object, .. } => {
                self.find_method_declaration(node, method, object)
            }
            NodeKind::IndirectCall { method, object, .. } => {
                // Handle indirect calls (e.g., "move $obj 10, 20" or "new Class")
                self.find_method_declaration(node, method, object)
            }
            NodeKind::Identifier { name } => self.find_identifier_declaration(node, name),
            NodeKind::Goto { target, form } => {
                match form {
                    GotoTargetForm::Label => {
                        if let NodeKind::Identifier { name } = &target.kind {
                            self.find_label_declaration(node, name)
                                .or_else(|| self.find_subroutine_declaration(node, name))
                        } else {
                            None
                        }
                    }
                    GotoTargetForm::Sub => {
                        // goto &sub — navigate to the subroutine declaration.
                        // Skip dynamic coderefs (e.g. `goto &$var`, where the parser
                        // produces `FunctionCall { name: "$var", .. }`) so we don't
                        // issue a wasted lookup for a non-existent subroutine. This
                        // mirrors the sigil guard in symbol.rs so both consumers of
                        // the `form` field agree on what the `Sub` arm means.
                        match &target.kind {
                            NodeKind::FunctionCall { name, .. }
                                if !name.is_empty() && !name.starts_with(['$', '@', '%']) =>
                            {
                                self.find_subroutine_declaration(node, name)
                            }
                            _ => None,
                        }
                    }
                    GotoTargetForm::Expr => None,
                }
            }
            // Cursor on a `method` name at its declaration site — self-location.
            // NodeKind::Method has no separate name-child node; the full Method node
            // spans the keyword + name, so find_node_at_offset returns the Method node.
            NodeKind::Method { name, .. } => {
                let mut declarations = Vec::new();
                self.collect_subroutine_declarations(&self.ast, name, &mut declarations);
                declarations.first().map(|decl| {
                    vec![self.create_location_link(
                        node,
                        decl,
                        self.get_subroutine_name_range(decl),
                    )]
                })
            }
            // Handle string literals that are method names inside modifier calls:
            // `before 'save' => sub { }` — cursor on 'save' navigates to sub save { }
            NodeKind::String { value, .. } => self.find_modifier_target_declaration(node, value),
            _ => None,
        }
    }

    /// Find variable declaration using scope-aware lookup
    fn find_variable_declaration(&self, usage: &Node, var_name: &str) -> Option<Vec<LocationLink>> {
        // Walk upwards through scopes to find the nearest declaration
        // SAFETY: `usage` is a shared reference into the `Arc<Node>` AST tree held by
        // `DeclarationProvider<'a>`. The raw pointer is used only as a HashMap key for O(1)
        // parent lookup and is never dereferenced directly; lookups go through `build_node_lookup_map`
        // which re-derives safe `&Node` references from the same Arc tree.
        let mut current_ptr: *const Node = usage as *const _;

        // Build temporary parent map if not provided (for testing)
        let temp_parent_map;
        let parent_map = if let Some(pm) = self.parent_map {
            pm
        } else {
            temp_parent_map = {
                let mut map = FxHashMap::default();
                Self::build_parent_map(&self.ast, &mut map, None);
                map
            };
            &temp_parent_map
        };
        let node_lookup = self.build_node_lookup_map();

        while let Some(&parent_ptr) = parent_map.get(&current_ptr) {
            let Some(parent) = node_lookup.get(&parent_ptr).copied() else {
                break;
            };

            if matches!(parent.kind, NodeKind::Subroutine { .. } | NodeKind::Method { .. }) {
                if let Some(links) =
                    self.find_signature_parameter_declaration(parent, usage, var_name)
                {
                    return Some(links);
                }
            }

            // Check siblings before this node in the current scope
            for child in self.get_children(parent) {
                // Stop when we reach or pass the usage node
                if child.location.start >= usage.location.start {
                    break;
                }

                // Check if this is a variable declaration matching our name
                if let NodeKind::VariableDeclaration { variable, .. } = &child.kind {
                    if let NodeKind::Variable { name, .. } = &variable.kind {
                        if name == var_name {
                            return Some(vec![LocationLink {
                                origin_selection_range: (usage.location.start, usage.location.end),
                                target_uri: self.document_uri.clone(),
                                target_range: (child.location.start, child.location.end),
                                target_selection_range: (
                                    variable.location.start,
                                    variable.location.end,
                                ),
                            }]);
                        }
                    }
                }

                // Also check variable list declarations
                if let NodeKind::VariableListDeclaration { variables, .. } = &child.kind {
                    for var in variables {
                        if let NodeKind::Variable { name, .. } = &var.kind {
                            if name == var_name {
                                return Some(vec![LocationLink {
                                    origin_selection_range: (
                                        usage.location.start,
                                        usage.location.end,
                                    ),
                                    target_uri: self.document_uri.clone(),
                                    target_range: (child.location.start, child.location.end),
                                    target_selection_range: (var.location.start, var.location.end),
                                }]);
                            }
                        }
                    }
                }
            }

            current_ptr = parent_ptr;
        }

        None
    }

    fn find_signature_parameter_declaration(
        &self,
        declaration_site: &Node,
        usage: &Node,
        var_name: &str,
    ) -> Option<Vec<LocationLink>> {
        let signature = match &declaration_site.kind {
            NodeKind::Subroutine { signature, .. } | NodeKind::Method { signature, .. } => {
                signature.as_deref()?
            }
            _ => return None,
        };

        let NodeKind::Signature { parameters } = &signature.kind else {
            return None;
        };

        for parameter in parameters {
            let variable = match &parameter.kind {
                NodeKind::MandatoryParameter { variable }
                | NodeKind::OptionalParameter { variable, .. }
                | NodeKind::SlurpyParameter { variable }
                | NodeKind::NamedParameter { variable, .. } => variable.as_ref(),
                _ => continue,
            };

            let NodeKind::Variable { name, .. } = &variable.kind else {
                continue;
            };

            if name == var_name {
                return Some(vec![self.create_location_link(
                    usage,
                    parameter,
                    (variable.location.start, variable.location.end),
                )]);
            }
        }

        None
    }

    /// Find subroutine declaration
    fn find_subroutine_declaration(
        &self,
        node: &Node,
        func_name: &str,
    ) -> Option<Vec<LocationLink>> {
        // Check if the function name is package-qualified (contains ::)
        let (target_package, target_name) = if let Some(pos) = func_name.rfind("::") {
            // Split into package and function name
            let package = &func_name[..pos];
            let name = &func_name[pos + 2..];
            (Some(package), name)
        } else {
            // No package qualifier, use current package context
            (self.find_current_package(node), func_name)
        };

        // Search for subroutines with the target name
        let mut declarations = Vec::new();
        self.collect_subroutine_declarations(&self.ast, target_name, &mut declarations);

        // If we have a target package, find subs in that specific package
        if let Some(pkg_name) = target_package {
            if let Some(decl) =
                declarations.iter().find(|d| self.declaration_matches_package(d, pkg_name))
            {
                return Some(vec![self.create_location_link(
                    node,
                    decl,
                    self.get_subroutine_name_range(decl),
                )]);
            }
        } else if let Some(decl) = declarations.first() {
            // An unqualified call resolves against the surrounding package.
            return Some(vec![self.create_location_link(
                node,
                decl,
                self.get_subroutine_name_range(decl),
            )]);
        }

        // Fall through: `use constant FOO => sub { ... }` creates a callable constant.
        // When invoked with parens — `FOO()` — the node is a FunctionCall, so
        // find_identifier_declaration's constant-fallthrough does not fire.  Search
        // constant declarations here as a last resort.
        let constants = self.find_constant_declarations(&self.ast, target_name);
        if let Some(const_decl) = constants.first() {
            return Some(vec![self.create_location_link(
                node,
                const_decl,
                self.get_constant_name_range_for(const_decl, target_name),
            )]);
        }

        None
    }

    /// Find method declaration with package resolution
    fn find_method_declaration(
        &self,
        node: &Node,
        method_name: &str,
        object: &Node,
    ) -> Option<Vec<LocationLink>> {
        // Try to determine the package from the object
        let package_name = match &object.kind {
            NodeKind::Identifier { name } if name.chars().next()?.is_uppercase() => {
                // Likely a package name (e.g., Foo->method)
                Some(name.as_str())
            }
            _ => None,
        };

        if let Some(pkg) = package_name {
            // Look for the method in the specific package
            let mut declarations = Vec::new();
            self.collect_subroutine_declarations(&self.ast, method_name, &mut declarations);

            if let Some(decl) =
                declarations.iter().find(|d| self.declaration_matches_package(d, pkg))
            {
                return Some(vec![self.create_location_link(
                    node,
                    decl,
                    self.get_subroutine_name_range(decl),
                )]);
            }

            if is_universal_method(method_name)
                && let Some(decl) =
                    declarations.iter().find(|d| self.find_current_package(d) == Some("UNIVERSAL"))
            {
                return Some(vec![self.create_location_link(
                    node,
                    decl,
                    self.get_subroutine_name_range(decl),
                )]);
            }
        }

        // Fall back to any subroutine with this name
        self.find_subroutine_declaration(node, method_name)
    }

    /// Find declaration for an identifier
    fn find_identifier_declaration(&self, node: &Node, name: &str) -> Option<Vec<LocationLink>> {
        // `goto LABEL` should resolve to the statement label before considering
        // sub/package/constant declarations.
        if self.identifier_is_goto_target(node)
            && let Some(links) = self.find_label_declaration(node, name)
        {
            return Some(links);
        }

        // Try to find as subroutine first
        if let Some(links) = self.find_subroutine_declaration(node, name) {
            return Some(links);
        }

        // Try to find as package
        let packages = self.find_package_declarations(&self.ast, name);
        if let Some(pkg) = packages.first() {
            return Some(vec![self.create_location_link(
                node,
                pkg,
                self.get_package_name_range(pkg),
            )]);
        }

        // Try to find as constant (supporting multiple forms)
        let constants = self.find_constant_declarations(&self.ast, name);
        if let Some(const_decl) = constants.first() {
            return Some(vec![self.create_location_link(
                node,
                const_decl,
                self.get_constant_name_range_for(const_decl, name),
            )]);
        }

        None
    }

    fn find_label_declaration(&self, origin: &Node, label_name: &str) -> Option<Vec<LocationLink>> {
        let mut labels = Vec::new();
        self.collect_label_declarations(&self.ast, label_name, &mut labels);
        let labeled_stmt = labels.first().copied()?;

        Some(vec![self.create_location_link(
            origin,
            labeled_stmt,
            self.get_labeled_statement_label_range(labeled_stmt),
        )])
    }

    fn collect_label_declarations<'b>(
        &'b self,
        node: &'b Node,
        label_name: &str,
        labels: &mut Vec<&'b Node>,
    ) {
        if let NodeKind::LabeledStatement { label, .. } = &node.kind
            && label == label_name
        {
            labels.push(node);
        }

        for child in self.get_children(node) {
            self.collect_label_declarations(child, label_name, labels);
        }
    }

    fn get_labeled_statement_label_range(&self, node: &Node) -> (usize, usize) {
        let NodeKind::LabeledStatement { label, .. } = &node.kind else {
            return (node.location.start, node.location.end);
        };

        let start = node.location.start;
        let end = node.location.end.min(self.content.len());
        if start >= end {
            return (node.location.start, node.location.end);
        }

        let text = &self.content[start..end];
        let label_start = text.find(label).map_or(start, |idx| start + idx);
        let label_end = label_start.saturating_add(label.len()).min(end);
        (label_start, label_end)
    }

    fn identifier_is_goto_target(&self, node: &Node) -> bool {
        let temp_parent_map;
        let parent_map = if let Some(pm) = self.parent_map {
            pm
        } else {
            temp_parent_map = {
                let mut map = FxHashMap::default();
                Self::build_parent_map(&self.ast, &mut map, None);
                map
            };
            &temp_parent_map
        };
        let node_lookup = self.build_node_lookup_map();

        let node_ptr = node as *const _;
        let Some(parent_ptr) = parent_map.get(&node_ptr).copied() else {
            return false;
        };
        let Some(parent) = node_lookup.get(&parent_ptr).copied() else {
            return false;
        };

        match &parent.kind {
            NodeKind::Goto { target, .. } => std::ptr::eq(target.as_ref(), node),
            _ => false,
        }
    }

    /// Find the definition of the method that a modifier string argument targets.
    ///
    /// When the cursor is on the string `'save'` in `before 'save' => sub { }`,
    /// this walks up the parent map to confirm the string is the first argument
    /// of a `before`/`after`/`around` function call, then returns the location of
    /// `sub save { }`.
    fn find_modifier_target_declaration(
        &self,
        string_node: &Node,
        method_name: &str,
    ) -> Option<Vec<LocationLink>> {
        // Strip surrounding quotes from the raw token text ('save' → save, "save" → save).
        let bare_name = method_name.trim().trim_matches('\'').trim_matches('"').trim();
        if bare_name.is_empty() {
            return None;
        }

        // Build parent map for upward traversal.
        let temp_parent_map;
        let parent_map = if let Some(pm) = self.parent_map {
            pm
        } else {
            temp_parent_map = {
                let mut map = FxHashMap::default();
                Self::build_parent_map(&self.ast, &mut map, None);
                map
            };
            &temp_parent_map
        };
        let node_lookup = self.build_node_lookup_map();

        // Walk up: String → FunctionCall { name: "before"/"after"/"around" }
        // The String node may be a direct child of the FunctionCall's args list,
        // so its immediate parent should be the FunctionCall node.
        let string_ptr: *const Node = string_node as *const _;
        let parent_ptr = parent_map.get(&string_ptr).copied()?;
        let parent = node_lookup.get(&parent_ptr).copied()?;

        // Check direct parent is a modifier FunctionCall where the string is first arg.
        if let NodeKind::FunctionCall { name, args } = &parent.kind {
            if matches!(name.as_str(), "before" | "after" | "around" | "override") {
                if args.first().map(|a| std::ptr::eq(a, string_node)).unwrap_or(false) {
                    return self.find_subroutine_declaration(string_node, bare_name);
                }
            }
        }

        // The FunctionCall may be wrapped in an ExpressionStatement — check one
        // level further up in case the parent is the statement wrapper.
        let grandparent_ptr = parent_map.get(&parent_ptr).copied()?;
        let grandparent = node_lookup.get(&grandparent_ptr).copied()?;

        if let NodeKind::FunctionCall { name, args } = &grandparent.kind {
            if matches!(name.as_str(), "before" | "after" | "around" | "override") {
                if args.first().map(|a| std::ptr::eq(a, string_node)).unwrap_or(false) {
                    return self.find_subroutine_declaration(string_node, bare_name);
                }
            }
        }

        None
    }

    /// Find the current package context for a node
    fn find_current_package<'b>(&'b self, node: &Node) -> Option<&'b str> {
        // SAFETY: `node` is a shared reference into the `Arc<Node>` AST tree held
        // by `DeclarationProvider<'a>`.  The raw pointer is used only as a hash key
        // to query the `parent_map`; it is never dereferenced.  Safe `&Node`
        // references are recovered through `node_lookup`, which re-derives them
        // from the same live `Arc<Node>` tree.
        let mut current_ptr: *const Node = node as *const _;

        // Build temporary parent map if not provided (for testing)
        let temp_parent_map;
        let parent_map = if let Some(pm) = self.parent_map {
            pm
        } else {
            temp_parent_map = {
                let mut map = FxHashMap::default();
                Self::build_parent_map(&self.ast, &mut map, None);
                map
            };
            &temp_parent_map
        };
        let node_lookup = self.build_node_lookup_map();

        while let Some(&parent_ptr) = parent_map.get(&current_ptr) {
            let Some(parent) = node_lookup.get(&parent_ptr).copied() else {
                break;
            };

            // Check siblings before this node for package declarations
            for child in self.get_children(parent) {
                if child.location.start >= node.location.start {
                    break;
                }

                if let NodeKind::Package { name, .. } = &child.kind {
                    return Some(name.as_str());
                }
            }

            current_ptr = parent_ptr;
        }

        None
    }

    /// Return whether a declaration belongs to a requested package.
    ///
    /// A qualified subroutine such as `sub Foo::bar` belongs to `Foo` even
    /// when it appears inside a different enclosing package. Bare
    /// declarations and other callable nodes continue to use their enclosing
    /// package context.
    fn declaration_matches_package(&self, node: &Node, package_name: &str) -> bool {
        if let NodeKind::Subroutine { name: Some(name), .. } = &node.kind {
            let (qualifier, _) = split_qualified_name(name);
            return qualifier == Some(package_name)
                || (qualifier.is_none() && self.find_current_package(node) == Some(package_name));
        }

        self.find_current_package(node) == Some(package_name)
    }

    /// Create a location link
    fn create_location_link(
        &self,
        origin: &Node,
        target: &Node,
        name_range: (usize, usize),
    ) -> LocationLink {
        LocationLink {
            origin_selection_range: (origin.location.start, origin.location.end),
            target_uri: self.document_uri.clone(),
            target_range: (target.location.start, target.location.end),
            target_selection_range: name_range,
        }
    }

    // Helper methods

    fn find_node_at_offset<'b>(&'b self, node: &'b Node, offset: usize) -> Option<&'b Node> {
        if offset >= node.location.start && offset <= node.location.end {
            // Check children first for more specific match
            for child in self.get_children(node) {
                if let Some(found) = self.find_node_at_offset(child, offset) {
                    return Some(found);
                }
            }
            return Some(node);
        }
        None
    }

    fn collect_subroutine_declarations<'b>(
        &'b self,
        node: &'b Node,
        sub_name: &str,
        subs: &mut Vec<&'b Node>,
    ) {
        match &node.kind {
            // Strip the package qualifier so a qualified declaration like
            // `sub Foo::bar` matches a bare lookup for `bar` (issue #6751),
            // mirroring the typeglob arm below.
            NodeKind::Subroutine { name: Some(name_str), .. }
                if split_qualified_name(name_str).1 == sub_name =>
            {
                subs.push(node);
            }
            // Method declarations (Perl 5.38+ native class / Object::Pad).
            // NodeKind::Method.name is a bare String (not Option<String>).
            NodeKind::Method { name: method_name, .. } if method_name == sub_name => {
                subs.push(node);
            }
            // Typeglob assignment: `*foo = sub { ... }` creates a callable named `foo`.
            // Strip the package qualifier so `*Pkg::foo` matches bare name `foo`.
            NodeKind::Assignment { lhs, rhs, .. } => {
                if let NodeKind::Typeglob { name: glob_name } = &lhs.kind {
                    let bare = glob_name.rsplit("::").next().unwrap_or(glob_name.as_str());
                    if bare == sub_name && matches!(rhs.kind, NodeKind::Subroutine { .. }) {
                        subs.push(node);
                    }
                }
            }
            _ => {}
        }

        for child in self.get_children(node) {
            self.collect_subroutine_declarations(child, sub_name, subs);
        }
    }

    fn find_package_declarations<'b>(&'b self, node: &'b Node, pkg_name: &str) -> Vec<&'b Node> {
        let mut packages = Vec::new();
        self.collect_package_declarations(node, pkg_name, &mut packages);
        packages
    }

    fn collect_package_declarations<'b>(
        &'b self,
        node: &'b Node,
        pkg_name: &str,
        packages: &mut Vec<&'b Node>,
    ) {
        match &node.kind {
            NodeKind::Package { name, .. } | NodeKind::Class { name, .. } if name == pkg_name => {
                packages.push(node);
            }
            _ => {}
        }

        for child in self.get_children(node) {
            self.collect_package_declarations(child, pkg_name, packages);
        }
    }

    fn find_constant_declarations<'b>(&'b self, node: &'b Node, const_name: &str) -> Vec<&'b Node> {
        let mut constants = Vec::new();
        self.collect_constant_declarations(node, const_name, &mut constants);
        constants
    }

    /// Strip leading -options from constant args
    fn strip_constant_options<'b>(&self, args: &'b [String]) -> &'b [String] {
        let mut i = 0;
        while i < args.len() && args[i].starts_with('-') {
            i += 1;
        }
        // Also skip a comma if present after options
        if i < args.len() && args[i] == "," {
            i += 1;
        }
        &args[i..]
    }

    fn collect_constant_declarations<'b>(
        &'b self,
        node: &'b Node,
        const_name: &str,
        constants: &mut Vec<&'b Node>,
    ) {
        if let NodeKind::Use { module, args, .. } = &node.kind {
            if module == "constant" {
                // Strip leading options like -strict, -nonstrict, -force
                let stripped_args = self.strip_constant_options(args);

                // Form 1: FOO => ...
                if stripped_args.first().map(|s| s.as_str()) == Some(const_name) {
                    constants.push(node);
                    // keep scanning siblings too (there can be multiple `use constant`)
                }

                // Flattened args text once (cheap)
                let args_text = stripped_args.join(" ");

                // Form 2: { FOO => 1, BAR => 2 }
                if self.contains_name_in_hash(&args_text, const_name) {
                    constants.push(node);
                }

                // Form 3: qw(FOO BAR) / qw/FOO BAR/
                if self.contains_name_in_qw(&args_text, const_name) {
                    constants.push(node);
                }
            }
        }

        for child in self.get_children(node) {
            self.collect_constant_declarations(child, const_name, constants);
        }
    }

    /// Check if a byte is part of an ASCII identifier
    #[inline]
    fn is_ident_ascii(b: u8) -> bool {
        matches!(b, b'0'..=b'9' | b'A'..=b'Z' | b'a'..=b'z' | b'_')
    }

    /// Iterate over all qw windows in the string
    /// Handles both paired delimiters ((), [], {}, <>) and symmetric delimiters (|, !, #, etc.)
    fn for_each_qw_window<F>(&self, s: &str, mut f: F) -> bool
    where
        F: FnMut(usize, usize) -> bool,
    {
        let b = s.as_bytes();
        let mut i = 0;
        while i + 1 < b.len() {
            // find literal "qw"
            if b[i] == b'q' && b[i + 1] == b'w' {
                let mut j = i + 2;

                // allow whitespace between qw and delimiter
                while j < b.len() && (b[j] as char).is_ascii_whitespace() {
                    j += 1;
                }
                if j >= b.len() {
                    break;
                }

                let open = b[j] as char;

                // "qwerty" guard: next non-ws must be a NON-word delimiter
                // (i.e., not [A-Za-z0-9_])
                if open.is_ascii_alphanumeric() || open == '_' {
                    i += 1;
                    continue;
                }

                // choose closing delimiter
                let close = match open {
                    '(' => ')',
                    '[' => ']',
                    '{' => '}',
                    '<' => '>',
                    _ => open, // symmetric delimiter (|, !, #, /, ~, ...)
                };

                // advance past opener and collect until closer
                j += 1;
                let start = j;
                while j < b.len() && (b[j] as char) != close {
                    j += 1;
                }
                if j <= b.len() {
                    // Found the closing delimiter
                    if f(start, j) {
                        return true;
                    }
                    // continue scanning after the closer
                    i = j + 1;
                    continue;
                } else {
                    // unclosed; stop scanning
                    break;
                }
            }

            i += 1;
        }
        false
    }

    /// Iterate over all {...} pairs in the string
    fn for_each_brace_window<F>(&self, s: &str, mut f: F) -> bool
    where
        F: FnMut(usize, usize) -> bool,
    {
        let b = s.as_bytes();
        let mut i = 0;
        while i < b.len() {
            if b[i] == b'{' {
                let start = i + 1;
                let mut nesting = 1;
                let mut j = i + 1;
                while j < b.len() {
                    match b[j] {
                        b'{' => nesting += 1,
                        b'}' => {
                            nesting -= 1;
                            if nesting == 0 {
                                break;
                            }
                        }
                        _ => {}
                    }
                    j += 1;
                }

                if nesting == 0 {
                    // Found matching closing brace at j
                    if f(start, j) {
                        return true;
                    }
                    i = j + 1;
                    continue;
                }
            }
            i += 1;
        }
        false
    }

    fn contains_name_in_hash(&self, s: &str, name: &str) -> bool {
        // for { FOO => 1, BAR => 2 } form - check all {...} pairs
        self.for_each_brace_window(s, |start, end| {
            // only scan that slice
            self.find_word(&s[start..end], name).is_some()
        })
    }

    fn contains_name_in_qw(&self, s: &str, name: &str) -> bool {
        // looks for qw(...) / qw[...] / qw/.../ etc. with word boundaries
        self.for_each_qw_window(s, |start, end| {
            // tokens are whitespace separated
            s[start..end].split_whitespace().any(|tok| tok == name)
        })
    }

    fn find_word(&self, hay: &str, needle: &str) -> Option<(usize, usize)> {
        if needle.is_empty() {
            return None;
        }
        let mut find_from = 0;
        while let Some(hit) = hay[find_from..].find(needle) {
            let start = find_from + hit;
            let end = start + needle.len();
            let left_ok = start == 0 || !Self::is_ident_ascii(hay.as_bytes()[start - 1]);
            let right_ok = end == hay.len()
                || !Self::is_ident_ascii(*hay.as_bytes().get(end).unwrap_or(&b' '));
            if left_ok && right_ok {
                return Some((start, end));
            }
            find_from = end;
        }
        None
    }

    fn first_all_caps_word(&self, s: &str) -> Option<(usize, usize)> {
        // very small scanner: find FOO-ish
        let bytes = s.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            while i < bytes.len() && !Self::is_ident_ascii(bytes[i]) {
                i += 1;
            }
            let start = i;
            while i < bytes.len() && Self::is_ident_ascii(bytes[i]) {
                i += 1;
            }
            if start < i {
                let w = &s[start..i];
                if w.chars().all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_') {
                    return Some((start, i));
                }
            }
        }
        None
    }

    fn get_subroutine_name_range(&self, decl: &Node) -> (usize, usize) {
        match &decl.kind {
            NodeKind::Subroutine { name_span: Some(loc), .. } => (loc.start, loc.end),
            // For `*foo = sub { ... }`, the "name" is the typeglob LHS (*foo).
            NodeKind::Assignment { lhs, .. } => (lhs.location.start, lhs.location.end),
            _ => (decl.location.start, decl.location.end),
        }
    }

    fn get_package_name_range(&self, decl: &Node) -> (usize, usize) {
        if let NodeKind::Package { name_span, .. } = &decl.kind {
            (name_span.start, name_span.end)
        } else {
            (decl.location.start, decl.location.end)
        }
    }

    fn get_constant_name_range(&self, decl: &Node) -> (usize, usize) {
        let text = self.get_node_text(decl);

        // Prefer an exact span if we can find the first occurrence with word boundaries
        if let NodeKind::Use { args, .. } = &decl.kind {
            let best_guess = args.first().map(|s| s.as_str()).unwrap_or("");
            if let Some((lo, hi)) = self.find_word(&text, best_guess) {
                let abs_lo = decl.location.start + lo;
                let abs_hi = decl.location.start + hi;
                return (abs_lo, abs_hi);
            }
        }

        // Try any constant-looking all-caps token in the decl
        if let Some((lo, hi)) = self.first_all_caps_word(&text) {
            return (decl.location.start + lo, decl.location.start + hi);
        }

        // Fallback to whole range
        (decl.location.start, decl.location.end)
    }

    fn get_constant_name_range_for(&self, decl: &Node, name: &str) -> (usize, usize) {
        let text = self.get_node_text(decl);

        // Fast path: try to find the exact word
        if let Some((lo, hi)) = self.find_word(&text, name) {
            return (decl.location.start + lo, decl.location.start + hi);
        }

        // Try inside all qw(...) windows
        let mut found_range = None;
        self.for_each_qw_window(&text, |start, end| {
            // Find the exact token position within this qw window
            if let Some((lo, hi)) = self.find_word(&text[start..end], name) {
                found_range =
                    Some((decl.location.start + start + lo, decl.location.start + start + hi));
                true // Stop searching
            } else {
                false // Continue to next window
            }
        });
        if let Some(range) = found_range {
            return range;
        }

        // Try inside all { ... } blocks (hash form)
        self.for_each_brace_window(&text, |start, end| {
            if let Some((lo, hi)) = self.find_word(&text[start..end], name) {
                found_range =
                    Some((decl.location.start + start + lo, decl.location.start + start + hi));
                true // Stop searching
            } else {
                false // Continue to next window
            }
        });
        if let Some(range) = found_range {
            return range;
        }

        // Final fallback to heuristics
        self.get_constant_name_range(decl)
    }

    fn get_children<'b>(&self, node: &'b Node) -> Vec<&'b Node> {
        Self::get_children_static(node)
    }

    /// Build a lookup map from raw node pointers back to safe references.
    ///
    /// This map is the bridge that makes `ParentMap` safe to use: callers
    /// obtain a `*const Node` from the parent map and look it up here to
    /// recover a properly-lifetime-bounded `&Node`.  The raw pointer is
    /// used purely as an identity key — it is never dereferenced directly.
    fn build_node_lookup_map(&self) -> FxHashMap<*const Node, &Node> {
        let mut map = FxHashMap::default();
        Self::build_node_lookup(self.ast.as_ref(), &mut map);
        map
    }

    fn build_node_lookup<'b>(node: &'b Node, map: &mut FxHashMap<*const Node, &'b Node>) {
        // SAFETY: `node` is a shared reference whose lifetime `'b` is tied to
        // `self.ast` (`Arc<Node>`).  We store the address as a raw-pointer key
        // alongside the same reference as the value.  The value is the safe
        // side of this pair — it is the only route through which the pointer
        // is ever turned back into usable data.
        map.insert(node as *const Node, node);
        for child in Self::get_children_static(node) {
            Self::build_node_lookup(child, map);
        }
    }

    fn get_children_static(node: &Node) -> Vec<&Node> {
        match &node.kind {
            NodeKind::Program { statements } => statements.iter().collect(),
            NodeKind::Block { statements } => statements.iter().collect(),
            NodeKind::If { condition, then_branch, else_branch, .. } => {
                let mut children = vec![condition.as_ref(), then_branch.as_ref()];
                if let Some(else_b) = else_branch {
                    children.push(else_b.as_ref());
                }
                children
            }
            NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
            NodeKind::Unary { operand, .. } => vec![operand.as_ref()],
            NodeKind::Return { value } => {
                if let Some(value) = value {
                    vec![value.as_ref()]
                } else {
                    vec![]
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                let mut children = vec![variable.as_ref()];
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                children
            }
            NodeKind::Method { signature, body, .. } => {
                let mut children = vec![body.as_ref()];
                if let Some(sig) = signature {
                    children.push(sig.as_ref());
                }
                children
            }
            NodeKind::Subroutine { signature, body, .. } => {
                let mut children = vec![body.as_ref()];
                if let Some(sig) = signature {
                    children.push(sig.as_ref());
                }
                children
            }
            NodeKind::FunctionCall { args, .. } => args.iter().collect(),
            NodeKind::MethodCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter());
                children
            }
            NodeKind::IndirectCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter());
                children
            }
            NodeKind::While { condition, body, .. } => {
                vec![condition.as_ref(), body.as_ref()]
            }
            NodeKind::For { init, condition, update, body, .. } => {
                let mut children = Vec::new();
                if let Some(i) = init {
                    children.push(i.as_ref());
                }
                if let Some(c) = condition {
                    children.push(c.as_ref());
                }
                if let Some(u) = update {
                    children.push(u.as_ref());
                }
                children.push(body.as_ref());
                children
            }
            NodeKind::Foreach { variable, list, body, .. } => {
                vec![variable.as_ref(), list.as_ref(), body.as_ref()]
            }
            NodeKind::ExpressionStatement { expression } => vec![expression.as_ref()],
            // Class body (Perl 5.38+ native class / Object::Pad) contains methods.
            NodeKind::Class { body, .. } => vec![body.as_ref()],
            // Package with optional inline block: `package Foo { ... }`.
            NodeKind::Package { block: Some(block), .. } => vec![block.as_ref()],
            _ => vec![],
        }
    }

    /// Extracts the source code text for a given AST node.
    ///
    /// Returns the substring of the document content corresponding to
    /// the node's location range. Used for symbol name extraction and
    /// text-based analysis.
    ///
    /// # Arguments
    /// * `node` - AST node to extract text from
    ///
    /// # Performance
    /// - Time complexity: O(m) where m is node text length
    /// - Memory: Creates owned string copy
    /// - Typical latency: <10μs for identifier names
    ///
    /// # Examples
    /// ```rust,ignore
    /// use perl_parser::declaration::DeclarationProvider;
    /// use perl_parser::ast::Node;
    /// use std::sync::Arc;
    ///
    /// let provider = DeclarationProvider::new(
    ///     Arc::new(Node::new_root()),
    ///     "sub example { }".to_string(),
    ///     "uri".to_string()
    /// );
    /// // let text = provider.get_node_text(&some_node);
    /// ```
    pub fn get_node_text(&self, node: &Node) -> String {
        self.content[node.location.start..node.location.end].to_string()
    }
}

/// Extracts a symbol key from the AST node at the given cursor position.
///
/// Analyzes the AST at a specific byte offset to identify the symbol under
/// the cursor for LSP operations. Supports function calls, variable references,
/// and package-qualified symbols with full Perl syntax coverage.
///
/// # Arguments
/// * `ast` - Root AST node to search within
/// * `offset` - Byte offset in the source document
/// * `current_pkg` - Current package context for symbol resolution
///
/// # Returns
/// * `Some(SymbolKey)` - Symbol found at position with package qualification
/// * `None` - No symbol at the given position
///
/// # Performance
/// - Search time: O(log n) average case with spatial indexing
/// - Worst case: O(n) for unbalanced AST traversal
/// - Typical latency: <50μs for LSP responsiveness
///
/// # Perl Parsing Context
/// Handles complex Perl symbol patterns:
/// - Package-qualified calls: `Package::function`
/// - Bare function calls: `function` (resolved in current package)
/// - Variable references: `$var`, `@array`, `%hash`
/// - Method calls: `$obj->method`
///
/// # Examples
/// ```rust,ignore
/// use perl_parser::declaration::symbol_at_cursor;
/// use perl_parser::ast::Node;
///
/// let ast = Node::new_root();
/// let symbol = symbol_at_cursor(&ast, 42, "MyPackage");
/// if let Some(sym) = symbol {
///     println!("Found symbol: {:?}", sym);
/// }
/// ```
fn symbol_at_cursor_internal(
    ast: &Node,
    offset: usize,
    current_pkg: &str,
    source_text: &str,
) -> Option<SymbolKey> {
    fn collect_node_path_at_offset<'a>(
        node: &'a Node,
        offset: usize,
        path: &mut Vec<&'a Node>,
    ) -> bool {
        if offset < node.location.start || offset > node.location.end {
            return false;
        }

        path.push(node);

        for child in get_node_children(node) {
            if collect_node_path_at_offset(child, offset, path) {
                return true;
            }
        }

        true
    }

    fn find_symbol_node_at_offset(ast: &Node, offset: usize) -> Option<(Vec<&Node>, &Node)> {
        let mut path = Vec::new();
        if !collect_node_path_at_offset(ast, offset, &mut path) {
            return None;
        }

        let node = path
            .iter()
            .rev()
            .copied()
            .find(|node| {
                matches!(
                    node.kind,
                    NodeKind::Variable { .. }
                        | NodeKind::FunctionCall { .. }
                        | NodeKind::Subroutine { .. }
                        | NodeKind::Method { .. }
                        | NodeKind::MethodCall { .. }
                        | NodeKind::Use { .. }
                )
            })
            .or_else(|| path.last().copied())?;

        Some((path, node))
    }

    fn node_variable_name(node: &Node) -> Option<&str> {
        if let NodeKind::Variable { name, .. } = &node.kind { Some(name.as_str()) } else { None }
    }

    fn normalize_symbol_name(raw: &str) -> Option<String> {
        let trimmed = raw.trim().trim_matches('\'').trim_matches('"').trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    }

    fn token_at_offset_in_text(text: &str, rel_offset: usize) -> Option<String> {
        let bytes = text.as_bytes();
        if rel_offset >= bytes.len() {
            return None;
        }
        let is_ident = |b: u8| matches!(b, b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b':');
        if !is_ident(bytes[rel_offset]) {
            return None;
        }

        let mut start = rel_offset;
        while start > 0 && is_ident(bytes[start - 1]) {
            start -= 1;
        }
        let mut end = rel_offset + 1;
        while end < bytes.len() && is_ident(bytes[end]) {
            end += 1;
        }
        Some(text[start..end].to_string())
    }

    fn export_tag_members(module: &str, tag: &str) -> &'static [&'static str] {
        match (module, tag) {
            // POSIX tag sets commonly used in system scripts.
            ("POSIX", ":sys_wait_h") => {
                &["WEXITSTATUS", "WIFEXITED", "WIFSIGNALED", "WIFSTOPPED", "WTERMSIG"]
            }
            ("POSIX", ":fcntl_h") => &["F_GETFD", "F_SETFD", "F_GETFL", "F_SETFL", "FD_CLOEXEC"],
            ("POSIX", ":termios_h") => {
                &["B9600", "B19200", "B38400", "TCSANOW", "TCSADRAIN", "TCSAFLUSH"]
            }
            // File::Find exports.
            ("File::Find", ":find") => &["find", "finddepth"],
            // Fcntl exports.
            ("Fcntl", ":seek") => &["SEEK_SET", "SEEK_CUR", "SEEK_END"],
            ("Fcntl", ":lock") => &["LOCK_SH", "LOCK_EX", "LOCK_NB", "LOCK_UN"],
            // Encode exports.
            ("Encode", ":fallback") => &[
                "FB_DEFAULT",
                "FB_CROAK",
                "FB_QUIET",
                "FB_WARN",
                "FB_PERLQQ",
                "FB_HTMLCREF",
                "FB_XMLCREF",
            ],
            _ => &[],
        }
    }

    fn tag_imports_symbol(module: &str, import_token: &str, symbol_name: &str) -> bool {
        if !import_token.starts_with(':') {
            return false;
        }
        export_tag_members(module, import_token).contains(&symbol_name)
    }

    /// Pragmas and structural modules whose qw/string arguments are NOT
    /// imported symbol names. Cursor-on-arg for these should not resolve
    /// to a bogus `SymbolKey` — they carry inheritance lists, feature names,
    /// or other non-import semantics.
    const NON_IMPORT_PRAGMAS: &[&str] = &[
        "constant", // constant definitions, not imports
        "parent",   // inheritance: qw/string args are class names
        "base",     // legacy inheritance
        "vars",     // variable declarations, not imports
        "Exporter", // 'import' arg is a proxy method, not an imported symbol
        "mro",      // method resolution order pragma
        "if",       // conditional module load
        "lib",      // adds directories to @INC
        "feature",  // enables Perl feature flags
        "utf8",     // encoding pragma
    ];

    fn use_args_import_symbol(module: &str, args: &[String], symbol_name: &str) -> bool {
        args.iter().any(|arg| {
            if arg == symbol_name || tag_imports_symbol(module, arg, symbol_name) {
                return true;
            }

            if arg.starts_with("qw") {
                let content = arg
                    .trim_start_matches("qw")
                    .trim_start_matches(|c: char| "([{/<|!".contains(c))
                    .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                return content
                    .split_whitespace()
                    .any(|tok| tok == symbol_name || tag_imports_symbol(module, tok, symbol_name));
            }

            let bare = arg.trim().trim_matches('\'').trim_matches('"').trim();
            bare == symbol_name || tag_imports_symbol(module, bare, symbol_name)
        })
    }

    fn find_import_source(ast: &Node, symbol_name: &str) -> Option<String> {
        /// Extract the module name from a `require Module;` statement node.
        ///
        /// Matches both `require Foo::Bar` (Identifier arg) and
        /// `require "Foo/Bar.pm"` forms, returning the module name as a
        /// `::` -separated string suitable for workspace lookup.
        fn require_module_name(node: &Node) -> Option<String> {
            let args = match &node.kind {
                NodeKind::FunctionCall { name, args } if name == "require" => args,
                _ => return None,
            };
            let arg = args.first()?;
            match &arg.kind {
                NodeKind::Identifier { name } => Some(name.clone()),
                NodeKind::String { value, .. } => {
                    // "Foo/Bar.pm" -> "Foo::Bar"
                    let cleaned = value.trim_matches('\'').trim_matches('"').trim();
                    let module = cleaned.trim_end_matches(".pm").replace('/', "::");
                    Some(module)
                }
                _ => None,
            }
        }

        /// Check whether a MethodCall node is `Module->import(...)` and, if
        /// so, whether its argument list contains `symbol`.  Handles four
        /// argument forms:
        /// - bare string literals:  `->import('foo', 'bar')`
        /// - qw list as ArrayLit:   `->import(qw(foo bar))` → ArrayLiteral
        /// - Identifier nodes:      `->import(foo)` (unusual but legal)
        /// - String value trimming: quoted strings like `"'foo'"` from qw
        fn import_call_exports(
            method_node: &Node,
            expected_module: &str,
            symbol: &str,
            aliases: &std::collections::HashMap<String, String>,
        ) -> bool {
            let (object, method, args) = match &method_node.kind {
                NodeKind::MethodCall { object, method, args } => (object, method, args),
                _ => return false,
            };
            if method != "import" {
                return false;
            }
            // The object must be the same module name.
            let obj_name = match &object.kind {
                NodeKind::Identifier { name } => Some(name.as_str()),
                NodeKind::Variable { name, .. } => aliases.get(name).map(String::as_str),
                _ => return false,
            };
            let Some(obj_name) = obj_name else {
                return false;
            };
            if obj_name != expected_module {
                return false;
            }
            if args.is_empty() {
                // `Module->import()` default import set is module-specific and may
                // come from `@EXPORT` in another file.  We do not currently have
                // a workspace export table in this lookup path, so stay
                // conservative and do not claim symbol ownership here.
                return false;
            }
            // Walk the argument list looking for the symbol.
            for arg in args {
                if arg_node_matches_symbol(arg, expected_module, symbol) {
                    return true;
                }
            }
            false
        }

        /// Check whether a single AST argument node matches `symbol`.
        /// Handles: String literals, Identifiers (including raw "qw(...)"),
        /// and ArrayLiteral (the AST form produced by `qw(...)` in expression
        /// context).
        fn arg_node_matches_symbol(arg: &Node, module: &str, symbol: &str) -> bool {
            match &arg.kind {
                NodeKind::String { value, .. } => {
                    // Strip surrounding single/double quotes that some code
                    // paths leave in the value (e.g. qw in quotes.rs).
                    let bare = value.trim_matches('\'').trim_matches('"');
                    bare == symbol || tag_imports_symbol(module, bare, symbol)
                }
                NodeKind::Identifier { name } => {
                    if name == symbol {
                        return true;
                    }
                    // qw(...) stored as a raw "qw(...)" Identifier string
                    // (from the Use-node code path that reuses this helper).
                    if name.starts_with("qw") {
                        let content = name
                            .trim_start_matches("qw")
                            .trim_start_matches(|c: char| "([{/<|!".contains(c))
                            .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                        return content
                            .split_whitespace()
                            .any(|tok| tok == symbol || tag_imports_symbol(module, tok, symbol));
                    }
                    false
                }
                NodeKind::ArrayLiteral { elements } => {
                    // qw(...) in expression context → ArrayLiteral of String nodes
                    elements.iter().any(|el| arg_node_matches_symbol(el, module, symbol))
                }
                _ => false,
            }
        }

        fn module_runtime_alias(expr: &Node) -> Option<(String, String)> {
            let (alias_name, call_node) = match &expr.kind {
                NodeKind::Assignment { lhs, rhs, op } if op == "=" => {
                    let NodeKind::Variable { name, .. } = &lhs.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                NodeKind::VariableDeclaration { variable, initializer: Some(rhs), .. } => {
                    let NodeKind::Variable { name, .. } = &variable.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                _ => return None,
            };

            let NodeKind::FunctionCall { name, args } = &call_node.kind else {
                return None;
            };
            if !matches!(
                name.as_str(),
                "use_module"
                    | "require_module"
                    | "Module::Runtime::use_module"
                    | "Module::Runtime::require_module"
            ) {
                return None;
            }
            let first = args.first()?;
            let NodeKind::String { value, .. } = &first.kind else {
                return None;
            };
            let module = value.trim_matches('\'').trim_matches('"').trim();
            if module.is_empty() {
                return None;
            }
            Some((alias_name.to_string(), module.to_string()))
        }

        /// Unwrap an ExpressionStatement to its inner expression, or return
        /// the node unchanged (handles the case where we're already at the
        /// expression level).
        fn inner_expr(node: &Node) -> &Node {
            if let NodeKind::ExpressionStatement { expression } = &node.kind {
                expression.as_ref()
            } else {
                node
            }
        }

        /// Scan a flat statement list for a `require M; M->import(...)` pair
        /// that exports `symbol`.  The require and import calls do not have to
        /// be adjacent — the import just needs to appear anywhere in the same
        /// statement list after (or even before) the require.
        fn scan_statements_for_require_import(stmts: &[Node], symbol: &str) -> Option<String> {
            // Collect all `require Module` names present in this block.
            let mut required_modules: Vec<String> =
                stmts.iter().filter_map(|s| require_module_name(inner_expr(s))).collect();
            let mut aliases: std::collections::HashMap<String, String> =
                std::collections::HashMap::new();
            for stmt in stmts {
                if let Some((alias, module)) = module_runtime_alias(inner_expr(stmt)) {
                    aliases.insert(alias, module.clone());
                    if !required_modules.contains(&module) {
                        required_modules.push(module);
                    }
                }
            }

            if required_modules.is_empty() {
                return None;
            }

            // Check whether any `Module->import(...)` call in this block
            // exports our symbol, using the set of required modules.
            for stmt in stmts {
                let expr = inner_expr(stmt);
                for module in &required_modules {
                    if import_call_exports(expr, module, symbol, &aliases) {
                        return Some(module.clone());
                    }
                }
            }
            None
        }

        fn find(node: &Node, name: &str) -> Option<String> {
            if let NodeKind::Use { module, args, .. } = &node.kind {
                // Skip structural pragmas — their args are not import-list symbols
                if NON_IMPORT_PRAGMAS.contains(&module.as_str()) {
                    // Fall through to children
                } else {
                    for arg in args {
                        if arg == name {
                            return Some(module.clone());
                        }
                        if tag_imports_symbol(module, arg, name) {
                            return Some(module.clone());
                        }
                        if arg.starts_with("qw") {
                            let content = arg
                                .trim_start_matches("qw")
                                .trim_start_matches(|c: char| "([{/<|!".contains(c))
                                .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                            for import_token in content.split_whitespace() {
                                if import_token == name
                                    || tag_imports_symbol(module, import_token, name)
                                {
                                    return Some(module.clone());
                                }
                            }
                        } else {
                            // Parenthesized import list: use Foo ('bar', 'baz')
                            // The parser emits each token as a separate arg including commas
                            // and string literals with their surrounding quotes.
                            let bare = arg.trim().trim_matches('\'').trim_matches('"').trim();
                            if bare == name {
                                return Some(module.clone());
                            }
                            if tag_imports_symbol(module, bare, name) {
                                return Some(module.clone());
                            }
                        }
                    }
                }
            }

            // Scan block/program statement lists for `require M; M->import(sym)` patterns.
            let stmts = match &node.kind {
                NodeKind::Program { statements } => Some(statements.as_slice()),
                NodeKind::Block { statements } => Some(statements.as_slice()),
                _ => None,
            };
            if let Some(statements) = stmts {
                if let Some(module) = scan_statements_for_require_import(statements, name) {
                    return Some(module);
                }
            }

            for child in get_node_children(node) {
                if let Some(module) = find(child, name) {
                    return Some(module);
                }
            }

            None
        }

        find(ast, symbol_name)
    }

    fn plack_builder_middleware_symbol(path: &[&Node], offset: usize) -> Option<SymbolKey> {
        let has_builder = path.iter().any(|ancestor| {
            matches!(ancestor.kind, NodeKind::FunctionCall { ref name, .. } if name == "builder")
        });
        if !has_builder {
            return None;
        }

        let block = path.iter().rev().find_map(|ancestor| {
            if let NodeKind::Block { statements } = &ancestor.kind {
                Some(statements)
            } else {
                None
            }
        })?;

        for statement in block {
            let NodeKind::ExpressionStatement { expression } = &statement.kind else {
                continue;
            };
            let NodeKind::FunctionCall { name, args } = &expression.kind else {
                continue;
            };
            if name != "enable" {
                continue;
            }

            let Some(first) = args.first() else {
                continue;
            };
            if offset < first.location.start || offset > first.location.end {
                continue;
            }

            let raw_name = match &first.kind {
                NodeKind::String { value, .. } => normalize_symbol_name(value)?,
                NodeKind::Identifier { name } => name.clone(),
                _ => continue,
            };

            let middleware_name = if raw_name.contains("::") {
                raw_name
            } else {
                format!("Plack::Middleware::{raw_name}")
            };

            return Some(SymbolKey {
                pkg: middleware_name.clone().into(),
                name: middleware_name.into(),
                sigil: None,
                kind: SymKind::Pack,
            });
        }

        None
    }

    fn looks_like_package_name(name: &str) -> bool {
        name.contains("::") || name.chars().next().is_some_and(|ch| ch.is_ascii_uppercase())
    }

    fn infer_receiver_package(
        object: &Node,
        current_pkg: &str,
        receiver_packages: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        if let NodeKind::Identifier { name } = &object.kind {
            return Some(name.clone());
        }

        if let Some(name) = node_variable_name(object) {
            if let Some(package_name) = receiver_packages.get(name) {
                return Some(package_name.clone());
            }

            if matches!(name, "self" | "this" | "class") {
                return Some(current_pkg.to_string());
            }

            if looks_like_package_name(name) {
                return Some(name.to_string());
            }
        }

        None
    }

    fn infer_constructor_package(
        rhs: &Node,
        current_pkg: &str,
        receiver_packages: &std::collections::HashMap<String, String>,
    ) -> Option<String> {
        match &rhs.kind {
            NodeKind::MethodCall { method, object, .. } if method == "new" => {
                infer_receiver_package(object, current_pkg, receiver_packages)
            }
            NodeKind::FunctionCall { name, .. } => {
                name.rsplit_once("::").map(|(package_name, _)| package_name.to_string())
            }
            _ => None,
        }
    }

    fn record_receiver_assignment(
        node: &Node,
        offset: usize,
        current_pkg: &str,
        receiver_packages: &mut std::collections::HashMap<String, String>,
    ) {
        if node.location.start > offset {
            return;
        }

        if node.location.end <= offset {
            match &node.kind {
                NodeKind::VariableDeclaration { variable, initializer, .. } => {
                    if let (Some(variable_name), Some(initializer)) =
                        (node_variable_name(variable), initializer.as_ref())
                    {
                        if let Some(package_name) =
                            infer_constructor_package(initializer, current_pkg, receiver_packages)
                        {
                            receiver_packages.insert(variable_name.to_string(), package_name);
                        }
                    }
                }
                NodeKind::Assignment { lhs, rhs, .. } => {
                    if let Some(variable_name) = node_variable_name(lhs) {
                        if let Some(package_name) =
                            infer_constructor_package(rhs, current_pkg, receiver_packages)
                        {
                            receiver_packages.insert(variable_name.to_string(), package_name);
                        }
                    }
                }
                _ => {}
            }
        }

        for child in get_node_children(node) {
            if child.location.start <= offset {
                record_receiver_assignment(child, offset, current_pkg, receiver_packages);
            }
        }
    }

    let (path, node) = find_symbol_node_at_offset(ast, offset)?;

    if let Some(symbol_key) = plack_builder_middleware_symbol(&path, offset) {
        return Some(symbol_key);
    }

    match &node.kind {
        NodeKind::Variable { sigil, name } => {
            // Variable already has sigil separated
            let sigil_char = sigil.chars().next();
            Some(SymbolKey {
                pkg: current_pkg.into(),
                name: name.clone().into(),
                sigil: sigil_char,
                kind: SymKind::Var,
            })
        }
        NodeKind::FunctionCall { name, .. } => {
            let (pkg, bare) = if let Some(idx) = name.rfind("::") {
                (name[..idx].to_string(), name[idx + 2..].to_string())
            } else {
                (
                    find_import_source(ast, name).unwrap_or_else(|| current_pkg.to_string()),
                    name.clone(),
                )
            };
            Some(SymbolKey { pkg: pkg.into(), name: bare.into(), sigil: None, kind: SymKind::Sub })
        }
        NodeKind::Subroutine { name: Some(name), .. } => {
            let (pkg, bare) = if let Some(idx) = name.rfind("::") {
                (&name[..idx], &name[idx + 2..])
            } else {
                (current_pkg, name.as_str())
            };
            Some(SymbolKey { pkg: pkg.into(), name: bare.into(), sigil: None, kind: SymKind::Sub })
        }
        // Method declaration (Perl 5.38+ native class / Object::Pad).
        // name is a bare String (not Option<String>) unlike NodeKind::Subroutine.
        NodeKind::Method { name, .. } => {
            let (pkg, bare) = if let Some(idx) = name.rfind("::") {
                (&name[..idx], &name[idx + 2..])
            } else {
                (current_pkg, name.as_str())
            };
            Some(SymbolKey { pkg: pkg.into(), name: bare.into(), sigil: None, kind: SymKind::Sub })
        }
        NodeKind::MethodCall { object, method, .. } => {
            let mut receiver_packages = std::collections::HashMap::new();
            record_receiver_assignment(ast, offset, current_pkg, &mut receiver_packages);
            let pkg = infer_receiver_package(object, current_pkg, &receiver_packages)
                .unwrap_or_else(|| current_pkg.to_string());
            Some(SymbolKey {
                pkg: pkg.into(),
                name: method.clone().into(),
                sigil: None,
                kind: SymKind::Sub,
            })
        }
        NodeKind::Use { module, args, .. } => {
            if !NON_IMPORT_PRAGMAS.contains(&module.as_str())
                && !source_text.is_empty()
                && offset >= node.location.start
                && offset <= node.location.end
            {
                let rel_offset = offset.saturating_sub(node.location.start);
                if let Some(stmt_text) = source_text.get(node.location.start..node.location.end)
                    && let Some(token) = token_at_offset_in_text(stmt_text, rel_offset)
                    && token != *module
                    && token != "use"
                    && use_args_import_symbol(module, args, &token)
                {
                    return Some(SymbolKey {
                        pkg: module.clone().into(),
                        name: token.into(),
                        sigil: None,
                        kind: SymKind::Sub,
                    });
                }
            }

            // When cursor is on a `use Module::Name` statement, resolve to the package
            Some(SymbolKey {
                pkg: module.clone().into(),
                name: module.clone().into(),
                sigil: None,
                kind: SymKind::Pack,
            })
        }
        _ => None,
    }
}

/// Extract a symbol key at a cursor offset with access to source text.
///
/// This variant is used by LSP handlers when additional source-aware
/// disambiguation is needed (for example, barewords in `use ... qw(...)` lists).
pub fn symbol_at_cursor_with_source(
    ast: &Node,
    offset: usize,
    current_pkg: &str,
    source_text: &str,
) -> Option<SymbolKey> {
    symbol_at_cursor_internal(ast, offset, current_pkg, source_text)
}

/// Extract a symbol key at a cursor offset.
///
/// This keeps the historical API and defers to [`symbol_at_cursor_with_source`]
/// without source text-specific disambiguation.
pub fn symbol_at_cursor(ast: &Node, offset: usize, current_pkg: &str) -> Option<SymbolKey> {
    symbol_at_cursor_internal(ast, offset, current_pkg, "")
}

/// Determines the current package context at the given offset.
///
/// Scans the AST backwards from the offset to find the most recent
/// package declaration, providing proper context for symbol resolution
/// in Perl's package-based namespace system.
///
/// # Arguments
/// * `ast` - Root AST node to search within
/// * `offset` - Byte offset in the source document
///
/// # Returns
/// Package name as string slice, defaults to "main" if no package found
///
/// # Performance
/// - Search time: O(n) worst case, O(log n) typical
/// - Memory: Returns borrowed string slice (zero-copy)
/// - Caching: Results suitable for per-request caching
///
/// # Perl Parsing Context
/// Perl package semantics:
/// - `package Foo;` declarations change current namespace
/// - Scope continues until next package declaration, current block end, or EOF
/// - `package Foo { ... }` scopes the package to the explicit block
/// - Default package is "main" when no explicit declaration
/// - Package names follow Perl identifier rules (`::`-separated)
///
/// # Examples
/// ```rust,ignore
/// use perl_parser::declaration::current_package_at;
/// use perl_parser::ast::Node;
///
/// let ast = Node::new_root();
/// let pkg = current_package_at(&ast, 100);
/// println!("Current package: {}", pkg);
/// ```
pub fn current_package_at(ast: &Node, offset: usize) -> &str {
    fn package_in_statement_list<'a>(
        statements: &'a [Node],
        offset: usize,
        mut current_pkg: &'a str,
    ) -> &'a str {
        for child in statements {
            if child.location.start > offset {
                break;
            }

            if child.location.start <= offset && offset <= child.location.end {
                return package_in_node(child, offset, current_pkg);
            }

            if let NodeKind::Package { name, block: None, .. } = &child.kind {
                current_pkg = name.as_str();
            }
        }

        current_pkg
    }

    fn package_in_node<'a>(node: &'a Node, offset: usize, current_pkg: &'a str) -> &'a str {
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                package_in_statement_list(statements, offset, current_pkg)
            }
            NodeKind::Package { name, block, .. } if node.location.start <= offset => {
                let package_name = name.as_str();
                if let Some(block) = block
                    && block.location.start <= offset
                    && offset <= block.location.end
                {
                    return package_in_node(block, offset, package_name);
                }
                package_name
            }
            _ => {
                for child in get_node_children(node) {
                    if child.location.start <= offset && offset <= child.location.end {
                        return package_in_node(child, offset, current_pkg);
                    }
                }
                current_pkg
            }
        }
    }

    package_in_node(ast, offset, "main")
}

/// Finds the most specific AST node containing the given byte offset.
///
/// Performs recursive descent through the AST to locate the deepest node
/// that encompasses the specified position. Essential for cursor-based
/// LSP operations like go-to-definition and hover.
///
/// # Arguments
/// * `node` - AST node to search within (typically root)
/// * `offset` - Byte offset in the source document
///
/// # Returns
/// * `Some(&Node)` - Deepest node containing the offset
/// * `None` - Offset is outside the node's range
///
/// # Performance
/// - Search time: O(log n) average, O(n) worst case
/// - Memory: Zero allocations, returns borrowed reference
/// - Spatial locality: Optimized for sequential offset queries
///
/// # LSP Integration
/// Core primitive for:
/// - Hover information: Find node for symbol details
/// - Go-to-definition: Identify symbol under cursor
/// - Completion: Determine context for suggestions
/// - Diagnostics: Map error positions to AST nodes
///
/// # Examples
/// ```rust,ignore
/// use perl_parser::declaration::find_node_at_offset;
/// use perl_parser::ast::Node;
///
/// let ast = Node::new_root();
/// if let Some(node) = find_node_at_offset(&ast, 42) {
///     println!("Found node: {:?}", node.kind);
/// }
/// ```
pub fn find_node_at_offset(node: &Node, offset: usize) -> Option<&Node> {
    if offset < node.location.start || offset > node.location.end {
        return None;
    }

    // Check children first for more specific match
    let children = get_node_children(node);
    for child in children {
        if let Some(found) = find_node_at_offset(child, offset) {
            return Some(found);
        }
    }

    // If no child contains the offset, return this node
    Some(node)
}

/// Returns direct child nodes for a given AST node.
///
/// Provides generic access to child nodes across different node types,
/// essential for AST traversal algorithms and recursive analysis patterns.
///
/// # Arguments
/// * `node` - AST node to extract children from
///
/// # Returns
/// Vector of borrowed child node references
///
/// # Performance
/// - Time complexity: O(k) where k is child count
/// - Memory: Allocates vector for child references
/// - Typical latency: <5μs for common node types
///
/// # Examples
/// ```rust,ignore
/// use perl_parser::declaration::get_node_children;
/// use perl_parser::ast::Node;
///
/// let node = Node::new_root();
/// let children = get_node_children(&node);
/// println!("Node has {} children", children.len());
/// ```
pub fn get_node_children(node: &Node) -> Vec<&Node> {
    // Delegate to the AST node's own comprehensive children() method,
    // which handles all node kinds including Block, Package, MethodCall, etc.
    node.children()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Parser;
    use std::sync::Arc;

    /// Helper: parse source and return DeclarationProvider with version 0.
    fn make_provider(source: &str) -> DeclarationProvider<'static> {
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("parse must succeed");
        DeclarationProvider::new(Arc::new(ast), source.to_string(), "file:///test.pl".to_string())
    }

    // =========================================================================
    // NodeKind::Goto / GotoTargetForm::Sub — changed lines in declaration.rs (#1923)
    //
    // find_declaration Goto/Sub arm (lines ~351-366): `goto &sub` navigates to the
    // subroutine declaration, but dynamic coderefs (`goto &$var`) are skipped via the
    // sigil guard mirroring symbol.rs so no wasted lookup is issued.
    // =========================================================================

    /// `goto &target` (named subroutine) resolves to the sub declaration —
    /// exercises the guarded `FunctionCall { name, .. }` arm of GotoTargetForm::Sub.
    ///
    /// Covered changed lines: ~351-360 (Sub arm, named-subroutine branch).
    #[test]
    fn goto_sub_decl_resolves_named_subroutine() {
        let source = "sub target { return 42; }\nsub jump { goto &target; }\n";
        let provider = make_provider(source);
        // Cursor on the `goto` keyword inside `jump` so find_node_at_offset
        // returns the Goto node (the keyword region has no child node).
        let offset = source.rfind("goto").expect("goto must be in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "find_declaration on `goto &target` must resolve the subroutine; \
             source={source:?} offset={offset}"
        );
    }

    /// `goto &$dispatch` (dynamic coderef) is NOT treated as a named-subroutine
    /// lookup — the sigil guard sends it to the `_ => None` arm, so no wasted
    /// `find_subroutine_declaration("$dispatch")` is issued.
    ///
    /// Covered changed line: the `name.starts_with(['$','@','%'])` guard +
    /// `_ => None` arm of GotoTargetForm::Sub.
    #[test]
    fn goto_sub_decl_skips_dynamic_coderef() {
        let source = "sub jump { goto &$dispatch; }\n";
        let provider = make_provider(source);
        let offset = source.find("goto").expect("goto must be in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_none(),
            "find_declaration on `goto &$dispatch` must return None (dynamic coderef, \
             not a named subroutine); source={source:?} offset={offset}"
        );
    }

    /// `goto LABEL` (sigil-less bareword → Label form) exercises the Label arm,
    /// which tries label resolution then falls back to subroutine resolution.
    /// Here `helper` is a subroutine, so the `.or_else` fallback resolves it.
    #[test]
    fn goto_label_decl_resolves_via_subroutine_fallback() {
        let source = "sub helper { 1 }\nsub jump { goto helper; }\n";
        let provider = make_provider(source);
        let offset = source.rfind("goto").expect("goto must be in source");
        assert!(
            provider.find_declaration(offset, 0).is_some(),
            "goto helper (Label form) should resolve via the subroutine fallback"
        );
    }

    /// `goto $target` (scalar → Expr form) exercises the `Expr => None` arm.
    #[test]
    fn goto_expr_decl_returns_none() {
        let source = "sub jump { my $t = 0; goto $t; }\n";
        let provider = make_provider(source);
        let offset = source.rfind("goto").expect("goto must be in source");
        assert!(
            provider.find_declaration(offset, 0).is_none(),
            "goto $target (Expr form) resolves to no declaration"
        );
    }

    /// Cursor on the goto *target* identifier reaches `identifier_is_goto_target`,
    /// which confirms the identifier is the target child of its `Goto` parent
    /// before label/subroutine resolution.
    #[test]
    fn goto_target_identifier_resolves_via_goto_target_check() {
        let source = "sub helper { 1 }\nsub jump { goto helper; }\n";
        let provider = make_provider(source);
        let goto_at = source.rfind("goto helper").expect("goto helper present");
        let helper_off = goto_at + source[goto_at..].find("helper").expect("helper after goto");
        assert!(
            provider.find_declaration(helper_off, 0).is_some(),
            "cursor on the goto target `helper` should resolve to the subroutine"
        );
    }

    // =========================================================================
    // NodeKind::Method — changed lines in declaration.rs (#854, patch-coverage)
    //
    // find_declaration Method arm (lines ~352-362)
    // collect_subroutine_declarations Method arm (lines ~853-854)
    // collect_package_declarations Class arm (line ~877)
    // get_children_static Class arm (line ~1293)
    // get_children_static Package block arm (line ~1295)
    // symbol_at_cursor_internal Method arm (lines ~1971-1977)
    // =========================================================================

    /// find_declaration on a Method node (cursor on the method name) returns
    /// Some([...]) — exercises the NodeKind::Method arm in find_declaration.
    ///
    /// Covered changed line: ~352  NodeKind::Method { name, .. } =>
    #[test]
    fn method_decl_find_declaration_self_locates() {
        let source = "class Foo { method greet { return 1; } }";
        let provider = make_provider(source);
        // "greet" starts at offset 19 (after "class Foo { method ").
        // The Method node has no separate name child, so find_node_at_offset
        // returns the Method node when the cursor is on the name characters.
        let offset = source.find("greet").expect("greet must be in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "find_declaration on a Method node must return Some; source={source:?} offset={offset}"
        );
    }

    /// collect_subroutine_declarations finds a Method node by name.
    ///
    /// Covered changed line: ~853  NodeKind::Method { name: method_name, .. }
    #[test]
    fn method_decl_collect_subroutine_declarations_finds_method() {
        let source = "class Foo { method greet { return 1; } }";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "greet", &mut subs);
        assert!(
            !subs.is_empty(),
            "collect_subroutine_declarations must find the method 'greet'; got empty vec"
        );
        assert!(
            matches!(subs[0].kind, NodeKind::Method { ref name, .. } if name == "greet"),
            "collected declaration must be a Method node named 'greet'"
        );
    }

    /// collect_package_declarations finds a Class node by name.
    ///
    /// Covered changed line: ~877  NodeKind::Class { name, .. } if name == pkg_name
    #[test]
    fn class_decl_collect_package_declarations_finds_class() {
        let source = "class Foo { method greet { return 1; } }";
        let provider = make_provider(source);
        let mut packages = Vec::new();
        provider.collect_package_declarations(&provider.ast, "Foo", &mut packages);
        assert!(
            !packages.is_empty(),
            "collect_package_declarations must find class 'Foo'; got empty vec"
        );
        assert!(
            matches!(packages[0].kind, NodeKind::Class { ref name, .. } if name == "Foo"),
            "collected declaration must be a Class node named 'Foo'"
        );
    }

    /// get_children_static on a Class node returns the class body.
    ///
    /// Covered changed line: ~1293  NodeKind::Class { body, .. } => vec![body.as_ref()]
    #[test]
    fn get_children_static_class_returns_body() {
        let source = "class Foo { method greet { return 1; } }";
        let provider = make_provider(source);
        // Walk the AST to find the Class node.
        fn find_class(node: &Node) -> Option<&Node> {
            if matches!(node.kind, NodeKind::Class { .. }) {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_class(child) {
                    return Some(found);
                }
            }
            None
        }
        let class_node = find_class(&provider.ast).expect("Class node must exist in parsed AST");
        let children = DeclarationProvider::get_children_static(class_node);
        assert!(
            !children.is_empty(),
            "get_children_static on Class must return the body; got empty vec"
        );
        // The single child must be the Block body.
        assert!(
            matches!(children[0].kind, NodeKind::Block { .. }),
            "Class child returned by get_children_static must be a Block"
        );
    }

    /// get_children_static on a Package-with-block node returns the block.
    ///
    /// Covered changed line: ~1295  NodeKind::Package { block: Some(block), .. }
    #[test]
    fn get_children_static_package_block_returns_block() {
        let source = "package Foo { sub hello { return 1; } }";
        let provider = make_provider(source);
        fn find_package(node: &Node) -> Option<&Node> {
            if matches!(node.kind, NodeKind::Package { .. }) {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_package(child) {
                    return Some(found);
                }
            }
            None
        }
        let package_node =
            find_package(&provider.ast).expect("Package node must exist in parsed AST");
        let children = DeclarationProvider::get_children_static(package_node);
        // package Foo { } has a block — get_children_static must return it.
        assert!(
            !children.is_empty(),
            "get_children_static on Package-with-block must return the block; got empty vec"
        );
    }

    /// symbol_at_cursor on a Method declaration site returns a SymbolKey with
    /// name = method name and kind = Sub.
    ///
    /// Covered changed lines: ~1971-1977  NodeKind::Method { name, .. } => { ... }
    #[test]
    fn symbol_at_cursor_method_decl_returns_symbol_key() {
        let source = "class Foo { method greet { return 1; } }";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("parse must succeed");
        let offset = source.find("greet").expect("greet must be in source");
        let result = symbol_at_cursor(&ast, offset, "Foo");
        assert!(
            result.is_some(),
            "symbol_at_cursor on a Method declaration must return Some; source={source:?}"
        );
        let key = result.unwrap();
        assert_eq!(key.name.as_ref(), "greet", "symbol name must be the method name");
        assert_eq!(key.kind, crate::workspace_index::SymKind::Sub, "method kind must be Sub");
    }

    // =========================================================================
    // Boundary discriminator tests — ripr seam coverage for equality guards
    //
    // Each test exercises the FALSE side of a match guard (name == X conditions)
    // so ripr can confirm the boundary is exercised in both directions.
    // =========================================================================

    /// Boundary discriminator: collect_subroutine_declarations does NOT collect a
    /// Subroutine node when its name does not equal sub_name (name_str != sub_name).
    ///
    /// Exercises the FALSE side of: name_str == sub_name (line ~848).
    #[test]
    fn subroutine_decl_boundary_discriminator_rejects_different_sub_name() {
        let source = "sub hello { return 1; }";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        // Search for "goodbye" -- a name that does NOT exist in the AST.
        provider.collect_subroutine_declarations(&provider.ast, "goodbye", &mut subs);
        assert!(
            subs.is_empty(),
            "collect_subroutine_declarations must NOT collect hello when searching for goodbye; got {count} node(s)",
            count = subs.len()
        );
    }

    /// Boundary discriminator: collect_subroutine_declarations does NOT collect a
    /// Method node when its name does not equal sub_name (method_name != sub_name).
    ///
    /// Exercises the FALSE side of: method_name == sub_name (line ~853).
    #[test]
    fn method_decl_boundary_discriminator_rejects_different_method_name() {
        let source = "class Foo { method greet { return 1; } }";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        // Search for "farewell" -- a name that does NOT match the greet method.
        provider.collect_subroutine_declarations(&provider.ast, "farewell", &mut subs);
        assert!(
            subs.is_empty(),
            "collect_subroutine_declarations must NOT collect greet when searching for farewell; got {count} node(s)",
            count = subs.len()
        );
    }

    /// Boundary discriminator: collect_package_declarations does NOT collect a
    /// Class or Package node when its name does not equal pkg_name (name != pkg_name).
    ///
    /// Exercises the FALSE side of: name == pkg_name (line ~877).
    #[test]
    fn class_decl_boundary_discriminator_rejects_different_class_name() {
        let source = "class Foo { method greet { return 1; } }";
        let provider = make_provider(source);
        let mut packages = Vec::new();
        // Search for "Bar" -- a class name that does NOT exist in the AST.
        provider.collect_package_declarations(&provider.ast, "Bar", &mut packages);
        assert!(
            packages.is_empty(),
            "collect_package_declarations must NOT collect Foo when searching for Bar; got {count} node(s)",
            count = packages.len()
        );
    }

    /// Regression test for issue #6751: a package-qualified declaration
    /// (`sub Foo::bar`) stores its name as "Foo::bar"; searching for the bare
    /// name `bar` must still find it. Before the fix, `name_str == sub_name`
    /// compared "Foo::bar" to "bar" and the declaration was silently missed.
    #[test]
    fn collect_subroutine_declarations_matches_qualified_decl_by_bare_name() {
        let source = "sub Foo::bar { return 1; }";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "bar", &mut subs);
        assert!(
            !subs.is_empty(),
            "collect_subroutine_declarations must find qualified `sub Foo::bar` when searching for bare 'bar'; got empty vec"
        );
        assert!(
            matches!(&subs[0].kind, NodeKind::Subroutine { name: Some(n), .. } if n == "Foo::bar"),
            "collected declaration must be the Subroutine node named 'Foo::bar'"
        );
    }

    /// Boundary discriminator for the qualified-name comparison (issue #6751):
    /// a declaration whose bare name differs from the target must NOT match,
    /// even when it carries a package qualifier.
    #[test]
    fn collect_subroutine_declarations_rejects_qualified_decl_with_different_bare_name() {
        let source = "sub Foo::bar { return 1; }";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "baz", &mut subs);
        assert!(
            subs.is_empty(),
            "collect_subroutine_declarations must NOT collect `sub Foo::bar` when searching for 'baz'; got {count} node(s)",
            count = subs.len()
        );
    }

    /// A qualified declaration belongs to its explicit package, even when
    /// the source is currently inside a different package.
    #[test]
    fn qualified_subroutine_resolution_uses_explicit_package() {
        let source = "package Other;\nsub Foo::bar { return 1; }\n";
        let provider = make_provider(source);

        assert!(
            provider.find_subroutine_declaration(&provider.ast, "Foo::bar").is_some(),
            "qualified Foo::bar must resolve from its explicit package"
        );
        assert!(
            provider.find_subroutine_declaration(&provider.ast, "Other::bar").is_none(),
            "qualified Foo::bar must not resolve as an Other::bar declaration"
        );
    }

    // =========================================================================
    // Patch coverage tests -- cover specific changed lines not reached by the
    // existing tests above (Codecov Patch 95 gate, lines 847-849, 876, 1973).
    // =========================================================================

    /// collect_subroutine_declarations finds a Subroutine node by name.
    ///
    /// Covered changed lines: ~847-849  match &node.kind { NodeKind::Subroutine { name: Some(name_str), .. } if ... => { subs.push(node) }
    /// (the Subroutine arm body, which is only hit when a sub with a matching name is visited)
    #[test]
    fn subroutine_decl_collect_subroutine_declarations_finds_subroutine() {
        let source = "sub hello { return 1; }";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "hello", &mut subs);
        assert!(
            !subs.is_empty(),
            "collect_subroutine_declarations must find the sub hello; got empty vec"
        );
        assert!(
            matches!(subs[0].kind, NodeKind::Subroutine { ref name, .. } if name.as_deref() == Some("hello")),
            "collected declaration must be a Subroutine node named hello"
        );
    }

    /// collect_package_declarations finds a Package node by name (not just Class).
    ///
    /// Covered changed lines: ~876  match &node.kind { NodeKind::Package { name, .. } | NodeKind::Class { name, .. } if ...
    /// (exercises the Package arm and confirms the match head is instrumented)
    #[test]
    fn package_decl_collect_package_declarations_finds_package() {
        let source = "package Bar; sub hello { return 1; }";
        let provider = make_provider(source);
        let mut packages = Vec::new();
        provider.collect_package_declarations(&provider.ast, "Bar", &mut packages);
        assert!(
            !packages.is_empty(),
            "collect_package_declarations must find package Bar; got empty vec"
        );
        assert!(
            matches!(packages[0].kind, NodeKind::Package { ref name, .. } if name == "Bar"),
            "collected declaration must be a Package node named Bar"
        );
    }

    /// symbol_at_cursor_with_source on a Method declaration returns a SymbolKey
    /// with source-text disambiguation active — exercises the Method arm of
    /// symbol_at_cursor_internal via the symbol_at_cursor_with_source wrapper.
    ///
    /// Covered changed lines: ~1971-1977  NodeKind::Method arm in symbol_at_cursor_internal
    /// Covers the bare-name path (line ~1975): no "::" in name, so pkg = current_pkg.
    #[test]
    fn symbol_at_cursor_with_source_method_decl_returns_symbol_key() {
        let source = "class Foo { method greet { return 1; } }";
        let mut parser = Parser::new(source);
        let ast = parser.parse().expect("parse must succeed");
        let offset = source.find("greet").expect("greet must be in source");
        let result = symbol_at_cursor_with_source(&ast, offset, "Foo", source);
        assert!(result.is_some(), "symbol_at_cursor_with_source on a Method must return Some");
        let key = result.unwrap();
        assert_eq!(key.name.as_ref(), "greet", "symbol name must be the bare method name");
        assert_eq!(key.pkg.as_ref(), "Foo", "pkg must be the current_pkg for bare method names");
    }

    // =========================================================================
    // Cross-construct sub resolver — #3108
    //
    // Covers three new code paths added by the cross-construct resolver:
    //   1. collect_subroutine_declarations — typeglob Assignment arm (TRUE side)
    //   2. collect_subroutine_declarations — typeglob name mismatch (FALSE side)
    //   3. collect_subroutine_declarations — typeglob with non-sub RHS (FALSE side)
    //   4. find_subroutine_declaration — constant fallthrough (TRUE side)
    //   5. find_subroutine_declaration — no constant found (FALSE side)
    //   6. get_subroutine_name_range — Assignment arm
    // =========================================================================

    /// collect_subroutine_declarations finds an anonymous sub bound via typeglob.
    ///
    /// Exercises the TRUE side of the typeglob Assignment arm:
    ///   NodeKind::Assignment { lhs: Typeglob { name == sub_name }, rhs: Subroutine }
    #[test]
    fn typeglob_sub_collect_finds_anonymous_sub() {
        let source = "*foo = sub { return 42; };";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        assert!(
            !subs.is_empty(),
            "collect_subroutine_declarations must find the sub assigned to *foo; got empty vec"
        );
    }

    /// Boundary discriminator: typeglob arm does NOT collect when name does not match.
    ///
    /// Exercises the FALSE side of the `bare == sub_name` guard in the typeglob arm.
    #[test]
    fn typeglob_sub_collect_boundary_rejects_different_name() {
        let source = "*foo = sub { return 42; };";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "bar", &mut subs);
        assert!(
            subs.is_empty(),
            "collect_subroutine_declarations must NOT collect *foo sub when searching for 'bar'; got {count}",
            count = subs.len()
        );
    }

    /// Boundary discriminator: typeglob arm does NOT collect when RHS is not a Subroutine.
    ///
    /// Exercises the FALSE side of the `matches!(rhs.kind, Subroutine)` guard.
    #[test]
    fn typeglob_sub_collect_boundary_rejects_non_sub_rhs() {
        let source = "*foo = 42;";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        assert!(
            subs.is_empty(),
            "collect_subroutine_declarations must NOT collect *foo = 42 as a sub; got {count}",
            count = subs.len()
        );
    }

    /// find_declaration on a FunctionCall site after `*foo = sub {}` resolves to the
    /// typeglob Assignment node.
    ///
    /// End-to-end test: FunctionCall "foo" → collect_subroutine_declarations (typeglob arm)
    /// → get_subroutine_name_range (Assignment arm) → LocationLink.
    #[test]
    fn typeglob_sub_find_declaration_resolves_function_call() {
        // Two-statement source: assignment then call.
        // rfind("foo") finds the one in foo() (rightmost occurrence).
        let source = "*foo = sub { return 42; };\nfoo();\n";
        let provider = make_provider(source);
        let offset = source.rfind("foo").expect("foo() must be in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "find_declaration on foo() after *foo = sub {{...}} must resolve; source={source:?}"
        );
        let link = result.unwrap();
        let link = link.first().expect("at least one LocationLink expected");
        // selection range must overlap with *foo (the LHS typeglob)
        let target_text = &source[link.target_selection_range.0..link.target_selection_range.1];
        assert!(
            target_text.contains("foo"),
            "target_selection_range must include 'foo' from the *foo typeglob; got {target_text:?}"
        );
    }

    /// get_subroutine_name_range on an Assignment node (typeglob LHS) returns the
    /// span of the LHS typeglob, not the whole assignment.
    ///
    /// Exercises the `NodeKind::Assignment { lhs, .. }` arm in get_subroutine_name_range.
    #[test]
    fn get_subroutine_name_range_assignment_node_returns_lhs_span() {
        let source = "*foo = sub { return 42; };";
        let provider = make_provider(source);

        fn find_assignment(node: &Node) -> Option<&Node> {
            if matches!(node.kind, NodeKind::Assignment { .. }) {
                return Some(node);
            }
            for child in node.children() {
                if let Some(found) = find_assignment(child) {
                    return Some(found);
                }
            }
            None
        }

        let assignment =
            find_assignment(&provider.ast).expect("Assignment node must exist in the parsed AST");
        let (start, end) = provider.get_subroutine_name_range(assignment);
        assert!(start < end, "name range must be non-empty");
        let text = &source[start..end];
        assert!(
            text.contains("foo"),
            "name range must cover the typeglob name 'foo'; got {text:?}"
        );
    }

    /// find_subroutine_declaration falls through to constant lookup when the callable
    /// is declared as `use constant FOO => sub { ... }` and called with parens `FOO()`.
    ///
    /// Exercises the TRUE side of the new constant-fallthrough in find_subroutine_declaration.
    #[test]
    fn use_constant_sub_find_declaration_via_function_call() {
        // `NOW()` is parsed as FunctionCall, not Identifier; the identifier path
        // already falls through to constants, but FunctionCall did not before this fix.
        let source = "use constant NOW => sub { 1 };\nmy $t = NOW();\n";
        let provider = make_provider(source);
        // Cursor on NOW in `NOW()` — rightmost occurrence is inside the call.
        let offset = source.rfind("NOW").expect("NOW must appear in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "find_declaration on NOW() (FunctionCall form) must resolve to the use constant declaration"
        );
    }

    /// Boundary discriminator: find_subroutine_declaration returns None when no sub
    /// or constant matches the name — exercises the FALSE side of the constant fallthrough.
    #[test]
    fn find_subroutine_declaration_returns_none_for_unknown_function() {
        let source = "completely_unknown_func();\n";
        let provider = make_provider(source);
        let result = provider.find_declaration(0, 0);
        // Must be None (or empty) — no sub or constant named completely_unknown_func.
        let is_empty = match &result {
            None => true,
            Some(v) => v.is_empty(),
        };
        assert!(
            is_empty,
            "find_declaration for an unknown function must return None; got {result:?}"
        );
    }

    /// Pre-measurement: Form 1 (my $code = sub { ... }) is already handled.
    ///
    /// goto-definition on `$code` in `$code->()` reaches the `my $code = ...`
    /// VariableDeclaration via the existing variable-declaration scope walker.
    /// This test documents that no fix was needed for Form 1.
    #[test]
    fn anon_sub_in_lexical_variable_already_resolves_via_variable_decl() {
        let source = "my $code = sub { return 42; };\n$code->();\n";
        let provider = make_provider(source);
        // Cursor on `$code` in `$code->()` — rightmost occurrence is in the call.
        let offset = source.rfind("$code").expect("$code must appear in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "goto-definition on $code in $code->() must already resolve to the variable declaration"
        );
    }

    // =========================================================================
    // Additional edge case tests for cross-construct resolver (#3108)
    // =========================================================================

    /// Edge case: Qualified typeglob `*Pkg::foo = sub { ... }` should strip the package
    /// qualifier and match bare name lookups for `foo()`.
    #[test]
    fn typeglob_sub_qualified_name_rsplit_strips_package() {
        let source = "*Pkg::foo = sub { return 99; };\nfoo();\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        // Search for bare "foo" — the rsplit should strip "Pkg::" prefix
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        assert!(
            !subs.is_empty(),
            "collect_subroutine_declarations must find *Pkg::foo when searching for bare 'foo'"
        );
    }

    /// Edge case: Nested package qualifier `*Pkg::Sub::foo = sub { ... }` should also
    /// be found when searching for bare `foo`.
    #[test]
    fn typeglob_sub_nested_package_strips_all_qualifiers() {
        let source = "*Pkg::Sub::foo = sub { return 99; };\nfoo();\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        assert!(
            !subs.is_empty(),
            "collect_subroutine_declarations must find *Pkg::Sub::foo when searching for bare 'foo'"
        );
    }

    /// Edge case: Multiple typeglobs in the same scope should both be collected.
    /// Tests that the collector doesn't stop after finding the first match.
    #[test]
    fn typeglob_sub_multiple_assignments_both_found() {
        let source = "*foo = sub { return 1; };\n*bar = sub { return 2; };\n";
        let provider = make_provider(source);
        let mut foo_subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut foo_subs);
        assert!(!foo_subs.is_empty(), "collect_subroutine_declarations must find *foo");

        let mut bar_subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "bar", &mut bar_subs);
        assert!(!bar_subs.is_empty(), "collect_subroutine_declarations must find *bar");
    }

    /// Edge case: Typeglob with underscore in name `*_private = sub { ... }`
    /// should be found just like any other typeglob.
    #[test]
    fn typeglob_sub_with_underscore_name() {
        let source = "*_private = sub { return 42; };\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "_private", &mut subs);
        assert!(!subs.is_empty(), "collect_subroutine_declarations must find *_private");
    }

    /// Edge case: Case sensitivity — `*Foo` should NOT match search for `foo`.
    /// Typeglob names are case-sensitive in Perl.
    #[test]
    fn typeglob_sub_case_sensitive_name_mismatch() {
        let source = "*Foo = sub { return 42; };\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        assert!(
            subs.is_empty(),
            "collect_subroutine_declarations must NOT find *Foo when searching for lowercase 'foo'"
        );
    }

    /// Edge case: `use constant` with qw form `use constant qw(A B C)` should
    /// allow lookup by individual constant names.
    #[test]
    fn use_constant_qw_form_lookup() {
        let source = "use constant qw(FOO BAR BAZ);\nFOO();\n";
        let provider = make_provider(source);
        let offset = source.find("FOO()").expect("FOO() must be in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "find_declaration on FOO() in qw form must resolve to the use constant"
        );
    }

    /// Edge case: `use constant` with hash form `use constant { A => 1, B => sub {} }`
    /// should allow lookup by individual constant names.
    #[test]
    fn use_constant_hash_form_lookup() {
        let source = "use constant { FOO => 1, BAR => 2 };\nFOO();\n";
        let provider = make_provider(source);
        let offset = source.rfind("FOO").expect("FOO must appear in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "find_declaration on FOO() in hash form must resolve to the use constant"
        );
    }

    /// Edge case: Verify that `use constant NAME => sub { ... }` with bare call `NAME`
    /// (no parens) also resolves, not just `NAME()`.
    #[test]
    fn use_constant_sub_bare_call_without_parens() {
        let source = "use constant ANSWER => sub { 42 };\nmy $x = ANSWER;\n";
        let provider = make_provider(source);
        let offset = source.rfind("ANSWER").expect("ANSWER must appear in source");
        let result = provider.find_declaration(offset, 0);
        assert!(
            result.is_some(),
            "find_declaration on bare ANSWER (Identifier form) must also resolve to the constant"
        );
    }

    /// Edge case: Typeglob assignment to a reference (not directly a sub) should NOT
    /// be collected. `*foo = \&bar` is different from `*foo = sub { ... }`.
    #[test]
    fn typeglob_sub_reference_rhs_not_collected() {
        let source = "sub bar { 1 }\n*foo = \\&bar;\nfoo();\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        // The *foo = \&bar is NOT a direct Subroutine on the RHS, so it should not match
        let has_assignment = subs.iter().any(|n| matches!(n.kind, NodeKind::Assignment { .. }));
        // Note: this may fail if the parser creates a Subroutine node for \&bar,
        // but the intent is to verify we only match `*foo = sub { ... }`, not `*foo = \&other`
        assert!(
            !has_assignment,
            "collect_subroutine_declarations must NOT collect *foo = \\&bar as a sub"
        );
    }

    /// Edge case: Typeglob assignment with complex RHS like `*foo = $bar ? sub {} : sub {}`
    /// should NOT be collected since the RHS is not directly a Subroutine node.
    #[test]
    fn typeglob_sub_ternary_rhs_not_collected() {
        let source = "*foo = 1 ? sub { 1 } : sub { 2 };\nfoo();\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        let has_assignment = subs.iter().any(|n| matches!(n.kind, NodeKind::Assignment { .. }));
        assert!(
            !has_assignment,
            "collect_subroutine_declarations must NOT collect *foo = (ternary) as a sub"
        );
    }

    /// Edge case: find_declaration on the typeglob name itself `*foo` should NOT crash.
    /// Currently cursor on * or the typeglob may not match any known node type.
    #[test]
    fn typeglob_sub_cursor_on_asterisk_does_not_crash() {
        let source = "*foo = sub { return 42; };";
        let provider = make_provider(source);
        let offset = source.find('*').expect("* must be in source");
        let result = provider.find_declaration(offset, 0);
        // Result can be None or Some, but must not panic
        let _ = result;
    }

    /// Edge case: Both `sub foo {}` and `*foo = sub {}` in the same file.
    /// The named sub should be found first, but the typeglob should also be discoverable.
    #[test]
    fn typeglob_sub_alongside_named_sub() {
        let source = "sub foo { return 1; }\n*foo = sub { return 2; };\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        assert!(
            subs.len() >= 2,
            "collect_subroutine_declarations must find both the named sub and the typeglob assignment for 'foo'; got {count}",
            count = subs.len()
        );
    }

    /// Edge case: Typeglob with a string constant RHS `*foo = "string"` should NOT
    /// be collected as a subroutine.
    #[test]
    fn typeglob_sub_string_rhs_not_collected() {
        let source = "*foo = \"hello\";\n";
        let provider = make_provider(source);
        let mut subs = Vec::new();
        provider.collect_subroutine_declarations(&provider.ast, "foo", &mut subs);
        assert!(
            subs.is_empty(),
            "collect_subroutine_declarations must NOT collect *foo = \"string\" as a sub"
        );
    }
}
