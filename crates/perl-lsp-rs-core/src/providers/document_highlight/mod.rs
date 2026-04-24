//! Document Highlight Provider for Perl LSP
//!
//! Highlights all occurrences of a symbol when cursor is positioned on it.
//! Distinguishes between read and write access.

use perl_ast::{Node, NodeKind, SourceLocation};

/// Types of symbol highlights
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DocumentHighlightKind {
    /// Regular text occurrence (read access)
    Text = 1,
    /// Read access to a symbol
    Read = 2,
    /// Write access to a symbol
    Write = 3,
}

/// A highlighted range in the document
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DocumentHighlight {
    /// Source location of the highlight
    pub location: SourceLocation,
    /// Type of highlight
    pub kind: DocumentHighlightKind,
}

/// Document Highlight Provider
pub struct DocumentHighlightProvider;

impl Default for DocumentHighlightProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl DocumentHighlightProvider {
    /// Create a new document highlight provider
    pub fn new() -> Self {
        Self
    }

    /// Find all highlights for the symbol at the given position in source code
    pub fn find_highlights(
        &self,
        ast: &Node,
        source: &str,
        byte_offset: usize,
    ) -> Vec<DocumentHighlight> {
        // Find the node at the cursor position
        let target_node = self.find_node_at_offset(ast, byte_offset);

        // Get the symbol name and kind
        let symbol_info = if let Some(ref node) = target_node {
            // Check if this variable is inside a subscript operation and normalize
            // the sigil accordingly (e.g., $array[0] -> @array, $hash{k} -> %hash)
            self.extract_symbol_info_with_context(node, source, ast, byte_offset)
        } else {
            // Fallback: check for synthetic positions (e.g., catch parameters)
            self.extract_symbol_at_offset(ast, source, byte_offset)
        };

        let symbol_info = match symbol_info {
            Some(info) => info,
            None => return Vec::new(),
        };

        // Find all occurrences of this symbol
        let mut highlights = Vec::new();
        self.collect_highlights(ast, source, &symbol_info, &mut highlights);

        // Deduplicate highlights by location, preferring Write over Read
        self.deduplicate_highlights(highlights)
    }

    /// Deduplicate highlights by location, preferring Write kind over Read
    fn deduplicate_highlights(&self, highlights: Vec<DocumentHighlight>) -> Vec<DocumentHighlight> {
        use std::collections::HashMap;

        // Group by location (start, end)
        let mut by_location: HashMap<(usize, usize), DocumentHighlight> = HashMap::new();

        for h in highlights {
            let key = (h.location.start, h.location.end);
            by_location
                .entry(key)
                .and_modify(|existing| {
                    // Prefer Write (3) over Read (2) over Text (1)
                    if (h.kind as u8) > (existing.kind as u8) {
                        *existing = h.clone();
                    }
                })
                .or_insert(h);
        }

        // Return sorted by position
        let mut result: Vec<_> = by_location.into_values().collect();
        result.sort_by_key(|h| h.location.start);
        result
    }

    /// Find the node at the given byte offset
    fn find_node_at_offset(&self, node: &Node, offset: usize) -> Option<Node> {
        // Check if offset is within this node
        if offset < node.location.start || offset >= node.location.end {
            return None;
        }

        // Check children first for more specific matches
        for child in self.get_children(node) {
            if let Some(found) = self.find_node_at_offset(child, offset) {
                return Some(found);
            }
        }

        // Check if this node is a relevant symbol
        if self.is_symbol_node(node) {
            return Some(node.clone());
        }

        None
    }

    /// Extract symbol info at an offset not covered by normal AST nodes
    /// (e.g., catch parameter variables stored as strings in Try nodes)
    fn extract_symbol_at_offset(
        &self,
        node: &Node,
        source: &str,
        offset: usize,
    ) -> Option<SymbolInfo> {
        if offset < node.location.start || offset >= node.location.end {
            return None;
        }

        // Check for Try catch parameters
        if let NodeKind::Try { catch_blocks, .. } = &node.kind {
            for (param, _) in catch_blocks {
                if let Some(var_str) = param {
                    // Find the catch parameter location in the source within this node
                    let node_source = source.get(node.location.start..node.location.end)?;
                    let relative_offset = offset - node.location.start;
                    // Search for the variable string near the offset
                    for (pos, _) in node_source.match_indices(var_str.as_str()) {
                        if pos <= relative_offset && relative_offset < pos + var_str.len() {
                            let first_char = var_str.chars().next()?;
                            if matches!(first_char, '$' | '@' | '%') {
                                return Some(SymbolInfo {
                                    name: var_str.get(1..)?.to_string(),
                                    sigil: Some(first_char.to_string()),
                                    is_method: false,
                                    is_function: false,
                                });
                            }
                        }
                    }
                }
            }
        }

        // Check for subroutine/method name at cursor position
        if let NodeKind::Subroutine { name: Some(sub_name), name_span: Some(span), .. } = &node.kind
        {
            if offset >= span.start && offset <= span.end {
                return Some(SymbolInfo {
                    name: sub_name.clone(),
                    sigil: None,
                    is_method: false,
                    is_function: true,
                });
            }
        }

        // Recurse into children
        for child in self.get_children(node) {
            if let Some(info) = self.extract_symbol_at_offset(child, source, offset) {
                return Some(info);
            }
        }

        None
    }

    /// Get children of a node
    fn get_children<'a>(&self, node: &'a Node) -> Vec<&'a Node> {
        let mut children = Vec::new();
        node.for_each_child(|child| children.push(child));
        children
    }

    /// Check if a node represents a symbol we can highlight
    fn is_symbol_node(&self, node: &Node) -> bool {
        matches!(
            node.kind,
            NodeKind::Variable { .. }
                | NodeKind::FunctionCall { .. }
                | NodeKind::MethodCall { .. }
                | NodeKind::Identifier { .. }
        )
    }

    /// Extract symbol information from a node
    fn extract_symbol_info(&self, node: &Node, source: &str) -> Option<SymbolInfo> {
        match &node.kind {
            NodeKind::Variable { sigil, name } => Some(SymbolInfo {
                name: name.clone(),
                sigil: Some(sigil.clone()),
                is_method: false,
                is_function: false,
            }),
            NodeKind::Identifier { name } => Some(SymbolInfo {
                name: name.clone(),
                sigil: None,
                is_method: false,
                is_function: false,
            }),
            NodeKind::FunctionCall { name, .. } => Some(SymbolInfo {
                name: name.clone(),
                sigil: None,
                is_method: false,
                is_function: true,
            }),
            NodeKind::MethodCall { method, .. } => Some(SymbolInfo {
                name: method.clone(),
                sigil: None,
                is_method: true,
                is_function: false,
            }),
            _ => {
                // Try to extract from source text
                let text = source.get(node.location.start..node.location.end)?;
                // Check for sigil prefix and extract safely
                let first = text.chars().next();
                match first {
                    Some(sigil @ ('$' | '@' | '%')) => Some(SymbolInfo {
                        name: text.get(1..).unwrap_or("").to_string(),
                        sigil: Some(sigil.to_string()),
                        is_method: false,
                        is_function: false,
                    }),
                    _ => None,
                }
            }
        }
    }

    /// Extract symbol info with AST context awareness.
    ///
    /// When the cursor is on a variable inside a subscript operation, this
    /// normalizes the sigil to the canonical container type:
    /// - `$array[0]` -> canonical sigil `@` (array access)
    /// - `$hash{key}` -> canonical sigil `%` (hash access)
    /// - `$#array` -> canonical sigil `@` (array last index)
    fn extract_symbol_info_with_context(
        &self,
        node: &Node,
        source: &str,
        ast: &Node,
        byte_offset: usize,
    ) -> Option<SymbolInfo> {
        let base_info = self.extract_symbol_info(node, source)?;

        // Only normalize when we have a $ sigil variable
        if base_info.sigil.as_deref() != Some("$") {
            return Some(base_info);
        }

        // Handle $#array -> normalize to @array
        if let Some(bare_name) = base_info.name.strip_prefix('#') {
            if !bare_name.is_empty() {
                return Some(SymbolInfo {
                    name: bare_name.to_string(),
                    sigil: Some("@".to_string()),
                    is_method: false,
                    is_function: false,
                });
            }
        }

        // Check if this $var is the left child of a Binary { op: "[]" | "{}" }
        if let Some(parent_op) = self.find_subscript_parent(ast, byte_offset) {
            match parent_op.as_str() {
                "[]" => {
                    return Some(SymbolInfo {
                        name: base_info.name,
                        sigil: Some("@".to_string()),
                        is_method: false,
                        is_function: false,
                    });
                }
                "{}" => {
                    return Some(SymbolInfo {
                        name: base_info.name,
                        sigil: Some("%".to_string()),
                        is_method: false,
                        is_function: false,
                    });
                }
                _ => {}
            }
        }

        Some(base_info)
    }

    /// Find the subscript operator of a Binary node that is the parent of the
    /// variable at the given offset, but only if the variable is the `left` child
    /// (the container being subscripted, not the index/key).
    fn find_subscript_parent(&self, node: &Node, offset: usize) -> Option<String> {
        if offset < node.location.start || offset >= node.location.end {
            return None;
        }

        // If this is a Binary subscript and the offset falls inside the left child
        if let NodeKind::Binary { op, left, .. } = &node.kind {
            if (op == "[]" || op == "{}")
                && offset >= left.location.start
                && offset < left.location.end
            {
                // Verify the left child is a Variable with $ sigil
                if let NodeKind::Variable { sigil, .. } = &left.kind {
                    if sigil == "$" {
                        return Some(op.clone());
                    }
                }
            }
        }

        // Recurse into children
        for child in self.get_children(node) {
            if let Some(op) = self.find_subscript_parent(child, offset) {
                return Some(op);
            }
        }

        None
    }

    /// Collect all highlights for a symbol
    fn collect_highlights(
        &self,
        node: &Node,
        source: &str,
        target: &SymbolInfo,
        highlights: &mut Vec<DocumentHighlight>,
    ) {
        self.collect_highlights_with_parent(node, source, target, highlights, None);
    }

    /// Collect all highlights for a symbol with parent context
    fn collect_highlights_with_parent(
        &self,
        node: &Node,
        source: &str,
        target: &SymbolInfo,
        highlights: &mut Vec<DocumentHighlight>,
        parent: Option<&Node>,
    ) {
        // Check if this node matches our symbol
        if self.node_matches_symbol(node, source, target) {
            let kind = self.determine_highlight_kind_with_parent(node, parent);
            // Use the full location including the sigil
            highlights.push(DocumentHighlight { location: node.location, kind });
        }

        // Cross-sigil matching for variables that refer to the same underlying
        // container but use a different sigil due to Perl's context rules:
        //   %hash  <-> $hash{key}   (hash element access)
        //   %hash  <-> @hash{@keys} (hash slice)
        //   @array <-> $array[idx]  (array element access)
        //   @array <-> $#array      (array last index)
        if let NodeKind::Variable { sigil, name } = &node.kind {
            if !self.node_matches_symbol(node, source, target) {
                if let Some(target_sigil) = &target.sigil {
                    let cross_match =
                        self.is_cross_sigil_match(sigil, name, target_sigil, &target.name, parent);
                    if cross_match {
                        let kind = self.determine_highlight_kind_with_parent(node, parent);
                        highlights.push(DocumentHighlight { location: node.location, kind });
                    }
                }
            }
        }

        // Emit highlight for subroutine definition name_span
        if let NodeKind::Subroutine { name: Some(sub_name), name_span: Some(span), .. } = &node.kind
        {
            if target.is_function && sub_name == &target.name {
                highlights.push(DocumentHighlight {
                    location: *span,
                    kind: DocumentHighlightKind::Write,
                });
            }
        }

        // Recursively check children with this node as parent
        for child in self.get_children(node) {
            self.collect_highlights_with_parent(child, source, target, highlights, Some(node));
        }

        // Emit synthetic highlights for Try catch parameter variables
        if let NodeKind::Try { catch_blocks, body, .. } = &node.kind {
            if let Some(target_sigil) = &target.sigil {
                let expected = format!("{}{}", target_sigil, target.name);
                let mut search_from = body.location.end;
                for (param, catch_body) in catch_blocks {
                    if let Some(var_str) = param {
                        if var_str == &expected {
                            // Search between previous body/catch end and catch body start
                            let search_end = catch_body.location.start;
                            if search_from < search_end && search_end <= source.len() {
                                if let Some(search_area) = source.get(search_from..search_end) {
                                    if let Some(pos) = search_area.find(var_str.as_str()) {
                                        let var_start = search_from + pos;
                                        highlights.push(DocumentHighlight {
                                            location: SourceLocation {
                                                start: var_start,
                                                end: var_start + var_str.len(),
                                            },
                                            kind: DocumentHighlightKind::Write,
                                        });
                                    }
                                }
                            }
                        }
                    }
                    search_from = catch_body.location.end;
                }
            }
        }

        // Scan interpolated strings for variable references
        if let NodeKind::String { interpolated: true, .. } = &node.kind {
            if let Some(target_sigil) = &target.sigil {
                let expected = format!("{}{}", target_sigil, target.name);
                if let Some(node_text) = source.get(node.location.start..node.location.end) {
                    for (pos, _) in node_text.match_indices(expected.as_str()) {
                        // Avoid matching prefixes of longer variable names
                        let end_pos = pos + expected.len();
                        if end_pos < node_text.len() {
                            let next = node_text.as_bytes()[end_pos];
                            if next.is_ascii_alphanumeric() || next == b'_' {
                                continue;
                            }
                        }
                        let abs_start = node.location.start + pos;
                        // Skip if this is the whole node (already matched by normal traversal)
                        if abs_start == node.location.start
                            && node.location.end == abs_start + expected.len()
                        {
                            continue;
                        }
                        highlights.push(DocumentHighlight {
                            location: SourceLocation {
                                start: abs_start,
                                end: abs_start + expected.len(),
                            },
                            kind: DocumentHighlightKind::Read,
                        });
                    }
                }
            }
        }
    }

    /// Check whether a variable occurrence with `(sigil, name)` is a cross-sigil
    /// match for the target `(target_sigil, target_name)`.
    ///
    /// Cross-sigil relationships in Perl:
    /// - `$hash{key}` accesses `%hash` -> `$` + `{}` parent = `%`
    /// - `@hash{qw(a b)}` slices `%hash` -> `@` + `{}` parent = `%`
    /// - `$array[idx]` accesses `@array` -> `$` + `[]` parent = `@`
    /// - `$#array` is the last index of `@array` -> name `#foo` maps to `@foo`
    fn is_cross_sigil_match(
        &self,
        sigil: &str,
        name: &str,
        target_sigil: &str,
        target_name: &str,
        parent: Option<&Node>,
    ) -> bool {
        // Handle $#array <-> @array
        // $#array is Variable { sigil: "$", name: "#array" }
        if target_sigil == "@" && sigil == "$" {
            if let Some(bare) = name.strip_prefix('#') {
                if bare == target_name {
                    return true;
                }
            }
        }
        // Reverse: target is $#array (normalized to @array), node is @array
        // This case is handled by the normal sigil matching since we normalized
        // the target sigil in extract_symbol_info_with_context.

        // Same-name checks with subscript context
        if name != target_name {
            return false;
        }

        if let Some(parent_node) = parent {
            if let NodeKind::Binary { op, .. } = &parent_node.kind {
                // $hash{key} when target is %hash
                if target_sigil == "%" && sigil == "$" && op == "{}" {
                    return true;
                }
                // @hash{@keys} (hash slice) when target is %hash
                if target_sigil == "%" && sigil == "@" && op == "{}" {
                    return true;
                }
                // $array[idx] when target is @array
                if target_sigil == "@" && sigil == "$" && op == "[]" {
                    return true;
                }
                // @array[0,1] (array slice) when target is @array
                // This is already matched by normal sigil matching since both are @.
            }
        }

        false
    }

    /// Check if a node matches the target symbol
    fn node_matches_symbol(&self, node: &Node, source: &str, target: &SymbolInfo) -> bool {
        match &node.kind {
            NodeKind::Variable { sigil, name } => {
                if let Some(target_sigil) = &target.sigil {
                    sigil == target_sigil && name == &target.name
                } else {
                    false
                }
            }
            NodeKind::Identifier { name } => {
                !target.is_method && target.sigil.is_none() && name == &target.name
            }
            NodeKind::FunctionCall { name, .. } => target.is_function && name == &target.name,
            NodeKind::MethodCall { method, .. } => target.is_method && method == &target.name,
            _ => {
                // Check source text as fallback
                if let Some(target_sigil) = &target.sigil {
                    let expected = format!("{}{}", target_sigil, target.name);
                    source
                        .get(node.location.start..node.location.end)
                        .is_some_and(|text| text == expected)
                } else {
                    false
                }
            }
        }
    }

    /// Determine the kind of highlight based on context with parent information
    fn determine_highlight_kind_with_parent(
        &self,
        node: &Node,
        parent: Option<&Node>,
    ) -> DocumentHighlightKind {
        // Check if this variable is being written to (declaration or assignment)
        // Look for parent nodes that indicate write access
        match &node.kind {
            NodeKind::Variable { .. } => {
                // Check parent context to determine if this is a write or read
                if let Some(parent_node) = parent {
                    match &parent_node.kind {
                        // Variable declarations are writes
                        NodeKind::VariableDeclaration { variable, .. } => {
                            if std::ptr::eq(variable.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Variables in list declarations are writes
                        NodeKind::VariableListDeclaration { variables, .. } => {
                            if variables.iter().any(|v| std::ptr::eq(v, node)) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Left side of assignment is write (includes compound assignments)
                        NodeKind::Assignment { lhs, .. } => {
                            if std::ptr::eq(lhs.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Increment/decrement operations are writes
                        NodeKind::Unary { op, operand, .. } => {
                            if (op == "++" || op == "--") && std::ptr::eq(operand.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Foreach loop variable is a write (iterator binding)
                        NodeKind::Foreach { variable, .. } => {
                            if std::ptr::eq(variable.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Signature parameters are writes (value binding on call)
                        NodeKind::MandatoryParameter { variable }
                        | NodeKind::SlurpyParameter { variable }
                        | NodeKind::NamedParameter { variable } => {
                            if std::ptr::eq(variable.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        NodeKind::OptionalParameter { variable, .. } => {
                            if std::ptr::eq(variable.as_ref(), node) {
                                DocumentHighlightKind::Write
                            } else {
                                DocumentHighlightKind::Read
                            }
                        }
                        // Default to read for other contexts
                        _ => DocumentHighlightKind::Read,
                    }
                } else {
                    // If we don't have parent context, default to read
                    DocumentHighlightKind::Read
                }
            }
            _ => DocumentHighlightKind::Read,
        }
    }
}

// Internal SymbolInfo structure
struct SymbolInfo {
    name: String,
    sigil: Option<String>,
    is_method: bool,
    is_function: bool,
}
