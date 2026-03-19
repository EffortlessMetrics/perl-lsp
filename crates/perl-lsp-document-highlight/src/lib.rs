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
        if let Some(children) = self.get_children(node) {
            for child in children {
                if let Some(found) = self.find_node_at_offset(child, offset) {
                    return Some(found);
                }
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
        if let Some(children) = self.get_children(node) {
            for child in children {
                if let Some(info) = self.extract_symbol_at_offset(child, source, offset) {
                    return Some(info);
                }
            }
        }

        None
    }

    /// Get children of a node
    fn get_children<'a>(&self, node: &'a Node) -> Option<Vec<&'a Node>> {
        match &node.kind {
            NodeKind::Program { statements } => Some(statements.iter().collect()),
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                let mut children = vec![variable.as_ref()];
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                Some(children)
            }
            NodeKind::VariableListDeclaration { variables, initializer, .. } => {
                let mut children: Vec<&Node> = variables.iter().collect();
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                Some(children)
            }
            NodeKind::Assignment { lhs, rhs, .. } => Some(vec![lhs.as_ref(), rhs.as_ref()]),
            NodeKind::Binary { left, right, .. } => Some(vec![left.as_ref(), right.as_ref()]),
            NodeKind::Unary { operand, .. } => Some(vec![operand.as_ref()]),
            NodeKind::MethodCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter().map(|a| a as &Node));
                Some(children)
            }
            NodeKind::FunctionCall { args, .. } => Some(args.iter().collect()),
            NodeKind::Block { statements } => Some(statements.iter().collect()),
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let mut children = vec![condition.as_ref(), then_branch.as_ref()];
                for (cond, branch) in elsif_branches {
                    children.push(cond.as_ref());
                    children.push(branch.as_ref());
                }
                if let Some(else_b) = else_branch {
                    children.push(else_b.as_ref());
                }
                Some(children)
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
                Some(children)
            }
            NodeKind::Foreach { variable, list, body, continue_block } => {
                if let Some(cb) = continue_block {
                    Some(vec![variable.as_ref(), list.as_ref(), body.as_ref(), cb.as_ref()])
                } else {
                    Some(vec![variable.as_ref(), list.as_ref(), body.as_ref()])
                }
            }
            NodeKind::While { condition, body, .. } => {
                Some(vec![condition.as_ref(), body.as_ref()])
            }
            NodeKind::Subroutine { body, signature, .. } => {
                let mut children = Vec::new();
                if let Some(sig) = signature {
                    // Signature node may have zero-width span; expose parameters directly
                    if let NodeKind::Signature { parameters } = &sig.kind {
                        children.extend(parameters.iter());
                    } else {
                        children.push(sig.as_ref());
                    }
                }
                children.push(body.as_ref());
                Some(children)
            }
            NodeKind::Return { value } => value.as_ref().map(|v| vec![v.as_ref()]),
            NodeKind::ArrayLiteral { elements } => Some(elements.iter().collect()),
            NodeKind::HashLiteral { pairs } => {
                let mut children = Vec::new();
                for (k, v) in pairs {
                    children.push(k);
                    children.push(v);
                }
                Some(children)
            }
            NodeKind::Ternary { condition, then_expr, else_expr } => {
                Some(vec![condition.as_ref(), then_expr.as_ref(), else_expr.as_ref()])
            }
            NodeKind::VariableWithAttributes { variable, .. } => Some(vec![variable.as_ref()]),
            NodeKind::ExpressionStatement { expression } => Some(vec![expression.as_ref()]),
            // Statement modifiers (Issue #191)
            NodeKind::StatementModifier { statement, condition, .. } => {
                Some(vec![statement.as_ref(), condition.as_ref()])
            }
            // Regex operations - only expr is a child node, patterns are strings (Issue #191)
            NodeKind::Match { expr, .. }
            | NodeKind::Substitution { expr, .. }
            | NodeKind::Transliteration { expr, .. } => Some(vec![expr.as_ref()]),
            // Control flow (Issue #191)
            NodeKind::Given { expr, body } => Some(vec![expr.as_ref(), body.as_ref()]),
            NodeKind::When { condition, body } => Some(vec![condition.as_ref(), body.as_ref()]),
            NodeKind::Default { body } => Some(vec![body.as_ref()]),
            NodeKind::LabeledStatement { statement, .. } => Some(vec![statement.as_ref()]),
            // Code evaluation (Issue #191)
            NodeKind::Eval { block } | NodeKind::Do { block } => Some(vec![block.as_ref()]),
            // Error handling (Issue #191)
            NodeKind::Try { body, catch_blocks, finally_block } => {
                let mut children = vec![body.as_ref()];
                for (_, catch_body) in catch_blocks {
                    children.push(catch_body.as_ref());
                }
                if let Some(finally) = finally_block {
                    children.push(finally.as_ref());
                }
                Some(children)
            }
            // Method declarations (Issue #191)
            NodeKind::Method { body, signature, .. } => {
                let mut children = Vec::new();
                if let Some(sig) = signature {
                    // Signature node may have zero-width span; expose parameters directly
                    if let NodeKind::Signature { parameters } = &sig.kind {
                        children.extend(parameters.iter());
                    } else {
                        children.push(sig.as_ref());
                    }
                }
                children.push(body.as_ref());
                Some(children)
            }
            // Indirect calls (Issue #191)
            NodeKind::IndirectCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter());
                Some(children)
            }
            // Class declarations (Issue #191)
            NodeKind::Class { body, .. } => Some(vec![body.as_ref()]),
            // Signature and parameter types (Issue #191)
            NodeKind::Signature { parameters } => Some(parameters.iter().collect()),
            NodeKind::MandatoryParameter { variable } => Some(vec![variable.as_ref()]),
            NodeKind::OptionalParameter { variable, default_value } => {
                Some(vec![variable.as_ref(), default_value.as_ref()])
            }
            NodeKind::SlurpyParameter { variable } => Some(vec![variable.as_ref()]),
            NodeKind::NamedParameter { variable } => Some(vec![variable.as_ref()]),
            _ => None,
        }
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
        if let Some(children) = self.get_children(node) {
            for child in children {
                if let Some(op) = self.find_subscript_parent(child, offset) {
                    return Some(op);
                }
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
        if let Some(children) = self.get_children(node) {
            for child in children {
                self.collect_highlights_with_parent(child, source, target, highlights, Some(node));
            }
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

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser::Parser;

    /// Helper to parse code and find highlights at a given byte offset.
    fn highlights_at(
        code: &str,
        byte_offset: usize,
    ) -> Result<Vec<DocumentHighlight>, Box<dyn std::error::Error>> {
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;
        let provider = DocumentHighlightProvider::new();
        Ok(provider.find_highlights(&ast, code, byte_offset))
    }

    // ---------------------------------------------------------------
    // Scalar variable highlighting
    // ---------------------------------------------------------------

    #[test]
    fn test_highlight_scalar_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $foo = 42;\nprint $foo;\n$foo = 100;";
        let highlights = highlights_at(code, 3)?; // on $foo

        assert!(!highlights.is_empty());
        Ok(())
    }

    #[test]
    fn scalar_all_occurrences() -> Result<(), Box<dyn std::error::Error>> {
        //              0         1         2         3
        //              0123456789012345678901234567890123456
        let code = "my $foo = 1;\nprint $foo;\n$foo = $foo + 1;";
        let highlights = highlights_at(code, 3)?; // first $foo

        // 4 occurrences: declaration, print arg, assignment lhs, addition operand
        assert_eq!(
            highlights.len(),
            4,
            "Expected 4 highlights for $foo, found {}: {:?}",
            highlights.len(),
            highlights
        );
        Ok(())
    }

    // ---------------------------------------------------------------
    // Array variable cross-sigil highlighting
    // ---------------------------------------------------------------

    #[test]
    fn array_cross_sigil_from_at() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor on @array should highlight @array, $array[0], $#array
        let code = "my @array = (1,2,3);\nmy $x = $array[0];\nmy $len = $#array;";
        // @array starts at offset 3
        let highlights = highlights_at(code, 3)?;

        // Should find: @array (decl), $array (in $array[0]), $#array
        assert!(
            highlights.len() >= 3,
            "Expected at least 3 highlights for @array (got {}): {:?}",
            highlights.len(),
            highlights
        );
        Ok(())
    }

    #[test]
    fn array_cross_sigil_from_dollar_subscript() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor on $array in $array[0] should highlight @array too
        let code = "my @array = (1,2,3);\nmy $x = $array[0];\nmy $len = $#array;";
        // $array in "$array[0]" starts after "my @array = (1,2,3);\nmy $x = "
        let offset = code.find("$array[0]").ok_or("test setup")?;
        let highlights = highlights_at(code, offset)?;

        // Should find: @array (decl), $array (in $array[0]), $#array
        assert!(
            highlights.len() >= 3,
            "Expected at least 3 highlights from $array[0] cursor (got {}): {:?}",
            highlights.len(),
            highlights
        );
        Ok(())
    }

    #[test]
    fn array_cross_sigil_dollar_hash() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor on $#array should highlight @array too
        let code = "my @array = (1,2,3);\nmy $len = $#array;";
        let offset = code.find("$#array").ok_or("test setup")?;
        let highlights = highlights_at(code, offset)?;

        // Should find: @array (decl), $#array
        assert!(
            highlights.len() >= 2,
            "Expected at least 2 highlights from $#array cursor (got {}): {:?}",
            highlights.len(),
            highlights
        );
        Ok(())
    }

    // ---------------------------------------------------------------
    // Hash variable cross-sigil highlighting
    // ---------------------------------------------------------------

    #[test]
    fn hash_cross_sigil_from_percent() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor on %hash should highlight %hash, $hash{key}
        let code = "my %hash = (a => 1);\n$hash{b} = 2;\nmy $v = $hash{a};";
        let highlights = highlights_at(code, 3)?; // on %hash

        // Should find: %hash (decl), $hash (in $hash{b}), $hash (in $hash{a})
        assert!(
            highlights.len() >= 3,
            "Expected at least 3 highlights for %%hash (got {}): {:?}",
            highlights.len(),
            highlights
        );
        Ok(())
    }

    #[test]
    fn hash_cross_sigil_from_dollar_brace() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor on $hash in $hash{key} should highlight %hash too
        let code = "my %hash = (a => 1);\nmy $v = $hash{a};";
        let offset = code.find("$hash{a}").ok_or("test setup")?;
        let highlights = highlights_at(code, offset)?;

        // Should find: %hash (decl), $hash (in $hash{a})
        assert!(
            highlights.len() >= 2,
            "Expected at least 2 highlights from $hash{{a}} cursor (got {}): {:?}",
            highlights.len(),
            highlights
        );
        Ok(())
    }

    #[test]
    fn hash_slice_cross_sigil() -> Result<(), Box<dyn std::error::Error>> {
        // @hash{@keys} should match %hash
        let code = "my %hash = (a => 1, b => 2);\nmy @vals = @hash{qw(a b)};";
        let highlights = highlights_at(code, 3)?; // on %hash

        // Should find: %hash (decl), @hash (in @hash{qw(a b)})
        assert!(
            highlights.len() >= 2,
            "Expected at least 2 highlights for %%hash with slice (got {}): {:?}",
            highlights.len(),
            highlights
        );
        Ok(())
    }

    // ---------------------------------------------------------------
    // Write vs Read highlighting
    // ---------------------------------------------------------------

    #[test]
    fn write_vs_read_declaration() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $foo = 42;\nprint $foo;";
        let highlights = highlights_at(code, 3)?;

        assert!(highlights.len() >= 2, "Expected at least 2 highlights");

        // First highlight (declaration) should be Write
        let decl_highlight = highlights.iter().find(|h| h.location.start == 3);
        assert!(
            decl_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
            "Declaration should be Write"
        );

        // Second highlight (print usage) should be Read
        let print_offset = code.find("print $foo").ok_or("test setup")? + 6;
        let read_highlight = highlights.iter().find(|h| h.location.start == print_offset);
        assert!(
            read_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Read),
            "Usage in print should be Read"
        );

        Ok(())
    }

    #[test]
    fn write_vs_read_assignment() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 1;\n$x = 2;\nprint $x;";
        let highlights = highlights_at(code, 3)?;

        assert!(highlights.len() >= 3, "Expected at least 3 highlights");

        // Declaration is Write
        assert_eq!(highlights[0].kind, DocumentHighlightKind::Write);

        // Assignment is Write
        let assign_offset = code.find("\n$x = 2").ok_or("test setup")? + 1;
        let assign_highlight = highlights.iter().find(|h| h.location.start == assign_offset);
        assert!(
            assign_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
            "Assignment LHS should be Write"
        );

        Ok(())
    }

    #[test]
    fn write_vs_read_increment() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 0;\n$x++;\nprint $x;";
        let highlights = highlights_at(code, 3)?;

        assert!(highlights.len() >= 3, "Expected at least 3 highlights");

        // Increment should be Write
        let incr_offset = code.find("\n$x++").ok_or("test setup")? + 1;
        let incr_highlight = highlights.iter().find(|h| h.location.start == incr_offset);
        assert!(
            incr_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
            "Increment should be Write"
        );

        Ok(())
    }

    #[test]
    fn write_vs_read_foreach_variable() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my @items = (1,2,3);\nfor my $item (@items) {\n    print $item;\n}";
        // Find the offset of "$item" in "for my $item"
        let item_offset = code.find("$item").ok_or("test setup")?;
        let highlights = highlights_at(code, item_offset)?;

        assert!(
            highlights.len() >= 2,
            "Expected at least 2 highlights for $item (got {}): {:?}",
            highlights.len(),
            highlights
        );

        // The loop variable declaration should be Write
        let decl_highlight = highlights.iter().find(|h| h.location.start == item_offset);
        assert!(
            decl_highlight.is_some_and(|h| h.kind == DocumentHighlightKind::Write),
            "Foreach loop variable should be Write"
        );

        Ok(())
    }

    // ---------------------------------------------------------------
    // Existing tests (preserved)
    // ---------------------------------------------------------------

    #[test]
    fn test_highlight_function_call() -> Result<(), Box<dyn std::error::Error>> {
        let code = "sub hello { print \"Hello\" }\nhello();\nhello();";
        let highlights = highlights_at(code, 29)?; // first hello() call

        assert!(
            highlights.len() >= 2,
            "Expected at least 2 highlights for function calls, found {}",
            highlights.len()
        );
        Ok(())
    }

    #[test]
    fn test_no_highlights_for_non_symbol() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = \"Hello World\";";
        let highlights = highlights_at(code, 12)?; // inside string "Hello"

        assert_eq!(highlights.len(), 0);
        Ok(())
    }

    #[test]
    fn test_highlight_statement_modifier() -> Result<(), Box<dyn std::error::Error>> {
        let code = "my $x = 5;\nprint $x if $x > 0;";
        let highlights = highlights_at(code, 3)?; // first $x

        assert!(
            highlights.len() >= 3,
            "Expected at least 3 highlights for $x, found {}",
            highlights.len()
        );
        Ok(())
    }
}
