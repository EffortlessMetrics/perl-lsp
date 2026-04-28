//! Call hierarchy provider types and traversal helpers for LSP requests.

use std::collections::HashMap;

use perl_parser::PositionMapper;
use perl_parser::ast::{Node, NodeKind};
use perl_position_tracking::{WirePosition, WireRange};
use serde_json::{Value, json};

/// LSP wire type alias for position (0-based line/character with UTF-16 counting)
pub type Position = WirePosition;

/// LSP wire type alias for range (start/end positions)
pub type Range = WireRange;

/// Call hierarchy item representing a function or method in Perl code
///
/// This structure represents a single item in a call hierarchy, containing all the
/// information needed to navigate to and display the function or method in LSP clients.
#[derive(Debug, Clone)]
pub struct CallHierarchyItem {
    /// Name of the function or method
    pub name: String,
    /// Symbol kind (e.g., "Function", "Method")
    pub kind: String,
    /// URI of the file containing this symbol
    pub uri: String,
    /// Full range of the symbol definition
    pub range: Range,
    /// Range for the symbol name (for selection highlighting)
    pub selection_range: Range,
    /// Optional additional detail about the symbol
    pub detail: Option<String>,
    /// Optional package/class name containing this callable
    pub package_name: Option<String>,
    /// Optional fully-qualified callable name
    pub qualified_name: Option<String>,
}

/// Call Hierarchy Provider
pub struct CallHierarchyProvider {
    source: String,
    uri: String,
    position_mapper: PositionMapper,
}

impl CallHierarchyProvider {
    fn extract_qualified_call_name(&self, node: &Node) -> Option<String> {
        let snippet = self.source.get(node.location.start..node.location.end)?.trim();
        let callee = snippet.split('(').next()?.trim();
        callee.contains("::").then(|| callee.to_string())
    }

    fn looks_like_package_name(name: &str) -> bool {
        name.contains("::") || name.chars().next().is_some_and(|c| c.is_ascii_uppercase())
    }

    fn node_variable_name(node: &Node) -> Option<&str> {
        if let NodeKind::Variable { name, .. } = &node.kind { Some(name.as_str()) } else { None }
    }

    fn current_package_for_function(
        &self,
        func_node: &Node,
        item: &CallHierarchyItem,
    ) -> Option<String> {
        item.package_name
            .clone()
            .or_else(|| {
                item.qualified_name.as_deref().and_then(|qualified| {
                    qualified.rsplit_once("::").map(|(package_name, _)| package_name.to_string())
                })
            })
            .or_else(|| {
                self.source.get(..func_node.location.start).and_then(|prefix| {
                    prefix.lines().rev().find_map(|line| {
                        let line = line.trim();
                        line.strip_prefix("package ")
                            .map(|rest| rest.trim_end_matches(';').trim())
                            .filter(|package_name| !package_name.is_empty())
                            .map(|package_name| package_name.to_string())
                    })
                })
            })
    }

    fn infer_receiver_package(
        &self,
        object: &Node,
        current_package: Option<&str>,
        receiver_packages: &HashMap<String, String>,
    ) -> Option<String> {
        if let Some(name) = Self::node_variable_name(object) {
            if let Some(package_name) = receiver_packages.get(name) {
                return Some(package_name.clone());
            }

            if matches!(name, "self" | "class") {
                return current_package.map(|package_name| package_name.to_string());
            }

            if Self::looks_like_package_name(name) {
                return Some(name.to_string());
            }
        }

        None
    }

    fn infer_constructor_package(
        &self,
        rhs: &Node,
        current_package: Option<&str>,
        receiver_packages: &HashMap<String, String>,
    ) -> Option<String> {
        match &rhs.kind {
            NodeKind::MethodCall { method, object, .. } if method == "new" => {
                self.infer_receiver_package(object, current_package, receiver_packages)
            }
            NodeKind::FunctionCall { name, .. } => {
                name.rsplit_once("::").map(|(package_name, _)| package_name.to_string())
            }
            _ => None,
        }
    }

    fn record_receiver_assignment(
        &self,
        lhs: &Node,
        rhs: &Node,
        current_package: Option<&str>,
        receiver_packages: &mut HashMap<String, String>,
    ) {
        if let Some(variable_name) = Self::node_variable_name(lhs) {
            if let Some(package_name) =
                self.infer_constructor_package(rhs, current_package, receiver_packages)
            {
                receiver_packages.insert(variable_name.to_string(), package_name);
            }
        }
    }

    fn outgoing_call_key(item: &CallHierarchyItem) -> &str {
        item.qualified_name.as_deref().unwrap_or(item.name.as_str())
    }

    /// Create a new call hierarchy provider for a source file
    ///
    /// # Arguments
    ///
    /// * `source` - The source code content
    /// * `uri` - The URI of the source file
    ///
    /// # Returns
    ///
    /// A new [`CallHierarchyProvider`] configured for the given source file
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_lsp::call_hierarchy_provider::CallHierarchyProvider;
    ///
    /// let source = "sub hello { print 'world'; }";
    /// let uri = "file:///path/to/file.pl";
    /// let provider = CallHierarchyProvider::new(source.to_string(), uri.to_string());
    /// ```
    pub fn new(source: String, uri: String) -> Self {
        // Validate that URI is well-formed (basic security check)
        let uri = if uri.is_empty() { "file:///unknown".to_string() } else { uri };
        let position_mapper = PositionMapper::new(&source);
        Self { source, uri, position_mapper }
    }

    /// Prepare call hierarchy - find items at a given position
    pub fn prepare(&self, ast: &Node, line: u32, character: u32) -> Option<Vec<CallHierarchyItem>> {
        let byte_offset = self.position_to_offset(line, character);
        let item = self.find_callable_at_position(ast, byte_offset)?;
        Some(vec![item])
    }

    /// Get incoming calls (callers of a function)
    pub fn incoming_calls(
        &self,
        ast: &Node,
        item: &CallHierarchyItem,
    ) -> Vec<CallHierarchyIncomingCall> {
        let mut calls = Vec::new();
        self.find_incoming_calls(ast, &item.name, &mut calls, None);
        calls
    }

    /// Get outgoing calls (functions called by this function)
    pub fn outgoing_calls(
        &self,
        ast: &Node,
        item: &CallHierarchyItem,
    ) -> Vec<CallHierarchyOutgoingCall> {
        // Find the function node
        if let Some(func_node) = self.find_function_by_name(ast, &item.name) {
            let mut calls = Vec::new();
            let current_package = self.current_package_for_function(func_node, item);
            if let NodeKind::Subroutine { body, .. } = &func_node.kind {
                let mut receiver_packages = HashMap::new();
                self.find_outgoing_calls(
                    body,
                    &mut calls,
                    current_package.as_deref(),
                    &mut receiver_packages,
                );
            }
            calls
        } else {
            Vec::new()
        }
    }

    /// Find a callable item at the given position
    fn find_callable_at_position(&self, node: &Node, offset: usize) -> Option<CallHierarchyItem> {
        if offset >= node.location.start && offset <= node.location.end {
            let uri = &self.uri;
            match &node.kind {
                NodeKind::Subroutine { name, prototype: _, signature, name_span, .. } => {
                    if let Some(name_str) = name {
                        if let Some(span) = name_span {
                            if offset >= span.start && offset <= span.end {
                                let range = self.node_to_range(node);
                                let selection_range =
                                    self.selection_range_from_name_span(name_span, &range);

                                let detail = if signature.is_some() {
                                    Some("(signature)".to_string())
                                } else {
                                    None
                                };

                                return Some(CallHierarchyItem {
                                    name: name_str.clone(),
                                    kind: "function".to_string(),
                                    uri: uri.clone(),
                                    range,
                                    selection_range,
                                    detail,
                                    package_name: None,
                                    qualified_name: None,
                                });
                            }
                        } else {
                            let range = self.node_to_range(node);
                            let selection_range =
                                self.selection_range_from_name_span(name_span, &range);

                            let detail = if signature.is_some() {
                                Some("(signature)".to_string())
                            } else {
                                None
                            };

                            return Some(CallHierarchyItem {
                                name: name_str.clone(),
                                kind: "function".to_string(),
                                uri: uri.clone(),
                                range,
                                selection_range,
                                detail,
                                package_name: None,
                                qualified_name: None,
                            });
                        }
                    }
                }
                NodeKind::MethodCall { method, .. } => {
                    let range = self.node_to_range(node);
                    return Some(CallHierarchyItem {
                        name: method.clone(),
                        kind: "method".to_string(),
                        uri: uri.clone(),
                        range,
                        selection_range: range,
                        detail: None,
                        package_name: None,
                        qualified_name: None,
                    });
                }
                NodeKind::FunctionCall { name, .. } => {
                    let range = self.node_to_range(node);
                    return Some(CallHierarchyItem {
                        name: name.clone(),
                        kind: "function".to_string(),
                        uri: uri.clone(),
                        range,
                        selection_range: range,
                        detail: None,
                        package_name: None,
                        qualified_name: None,
                    });
                }
                _ => {}
            }

            // Check children
            self.visit_children(node, |child| self.find_callable_at_position(child, offset))
        } else {
            None
        }
    }

    /// Find all calls to a function
    fn find_incoming_calls(
        &self,
        node: &Node,
        target_name: &str,
        calls: &mut Vec<CallHierarchyIncomingCall>,
        current_function: Option<&CallHierarchyItem>,
    ) {
        let uri = &self.uri;
        match &node.kind {
            NodeKind::Subroutine { name, name_span, .. } => {
                if let Some(name_str) = name {
                    let range = self.node_to_range(node);
                    let selection_range = self.selection_range_from_name_span(name_span, &range);
                    let item = CallHierarchyItem {
                        name: name_str.clone(),
                        kind: "function".to_string(),
                        uri: uri.clone(),
                        range,
                        selection_range,
                        detail: None,
                        package_name: None,
                        qualified_name: None,
                    };

                    // Search within this function
                    self.visit_children(node, |child| {
                        self.find_incoming_calls(child, target_name, calls, Some(&item));
                        None::<()>
                    });
                }
            }
            NodeKind::FunctionCall { name, .. } => {
                // Match exact name or package-qualified name (e.g. "Utils::format_string")
                let matches = name == target_name || name.ends_with(&format!("::{}", target_name));
                if matches {
                    if let Some(from) = current_function {
                        let ranges = vec![self.node_to_range(node)];

                        // Check if we already have a call from this function
                        if let Some(existing) = calls.iter_mut().find(|c| c.from.name == from.name)
                        {
                            existing.from_ranges.extend(ranges);
                        } else {
                            calls.push(CallHierarchyIncomingCall {
                                from: from.clone(),
                                from_ranges: ranges,
                            });
                        }
                    }
                }
            }
            NodeKind::MethodCall { method, .. } => {
                if method == target_name {
                    if let Some(from) = current_function {
                        let ranges = vec![self.node_to_range(node)];

                        if let Some(existing) = calls.iter_mut().find(|c| c.from.name == from.name)
                        {
                            existing.from_ranges.extend(ranges);
                        } else {
                            calls.push(CallHierarchyIncomingCall {
                                from: from.clone(),
                                from_ranges: ranges,
                            });
                        }
                    }
                }
            }
            _ => {}
        }

        // Visit children
        self.visit_children(node, |child| {
            self.find_incoming_calls(child, target_name, calls, current_function);
            None::<()>
        });
    }

    /// Find all function calls within a node
    fn find_outgoing_calls(
        &self,
        node: &Node,
        calls: &mut Vec<CallHierarchyOutgoingCall>,
        current_package: Option<&str>,
        receiver_packages: &mut HashMap<String, String>,
    ) {
        let uri = &self.uri;
        match &node.kind {
            NodeKind::FunctionCall { name, .. } => {
                let qualified_name = self.extract_qualified_call_name(node);
                let item = CallHierarchyItem {
                    name: name.clone(),
                    kind: "function".to_string(),
                    uri: uri.clone(),
                    range: self.node_to_range(node),
                    selection_range: self.node_to_range(node),
                    detail: None,
                    package_name: qualified_name.as_deref().and_then(|qualified| {
                        qualified.rsplit_once("::").map(|(pkg, _)| pkg.to_string())
                    }),
                    qualified_name,
                };

                let ranges = vec![self.node_to_range(node)];

                // Check if we already have a call to this function
                let item_key = Self::outgoing_call_key(&item);
                if let Some(existing) =
                    calls.iter_mut().find(|c| Self::outgoing_call_key(&c.to) == item_key)
                {
                    existing.from_ranges.extend(ranges);
                } else {
                    calls.push(CallHierarchyOutgoingCall { to: item, from_ranges: ranges });
                }
            }
            NodeKind::MethodCall { method, object, .. } => {
                let detail = if let NodeKind::Variable { name, .. } = &object.kind {
                    Some(format!("on ${}", name))
                } else {
                    None
                };

                let package_name =
                    self.infer_receiver_package(object, current_package, receiver_packages);
                let qualified_name =
                    package_name.as_ref().map(|package_name| format!("{package_name}::{method}"));

                let item = CallHierarchyItem {
                    name: method.clone(),
                    kind: "method".to_string(),
                    uri: uri.clone(),
                    range: self.node_to_range(node),
                    selection_range: self.node_to_range(node),
                    detail,
                    package_name,
                    qualified_name,
                };

                let ranges = vec![self.node_to_range(node)];

                let item_key = Self::outgoing_call_key(&item);
                if let Some(existing) =
                    calls.iter_mut().find(|c| Self::outgoing_call_key(&c.to) == item_key)
                {
                    existing.from_ranges.extend(ranges);
                } else {
                    calls.push(CallHierarchyOutgoingCall { to: item, from_ranges: ranges });
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                if let Some(initializer) = initializer {
                    self.record_receiver_assignment(
                        variable,
                        initializer,
                        current_package,
                        receiver_packages,
                    );
                }
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                self.record_receiver_assignment(lhs, rhs, current_package, receiver_packages);
            }
            _ => {}
        }

        // Visit children
        self.visit_children(node, |child| {
            self.find_outgoing_calls(child, calls, current_package, receiver_packages);
            None::<()>
        });
    }

    /// Find the definition of a named subroutine in this document and return a
    /// `CallHierarchyItem` pointing at it.  Returns `None` if not found.
    pub fn find_definition(&self, name: &str, ast: &Node) -> Option<CallHierarchyItem> {
        let func_node = self.find_function_by_name(ast, name)?;
        if let NodeKind::Subroutine { name: func_name, name_span, signature, .. } = &func_node.kind
        {
            let range = self.node_to_range(func_node);
            let selection_range = self.selection_range_from_name_span(name_span, &range);
            let detail = signature.is_some().then(|| "(signature)".to_string());
            Some(CallHierarchyItem {
                name: func_name.clone().unwrap_or_else(|| name.to_string()),
                kind: "function".to_string(),
                uri: self.uri.clone(),
                range,
                selection_range,
                detail,
                package_name: None,
                qualified_name: None,
            })
        } else {
            None
        }
    }

    /// Find a function by name
    fn find_function_by_name<'a>(&self, node: &'a Node, target_name: &str) -> Option<&'a Node> {
        if let NodeKind::Subroutine { name, .. } = &node.kind {
            if name.as_ref() == Some(&target_name.to_string()) {
                return Some(node);
            }
        }

        self.visit_children(node, |child| self.find_function_by_name(child, target_name))
    }

    /// Visit children of a node
    fn visit_children<'a, T, F>(&self, node: &'a Node, mut f: F) -> Option<T>
    where
        F: FnMut(&'a Node) -> Option<T>,
    {
        match &node.kind {
            NodeKind::Program { statements } => {
                for stmt in statements {
                    if let Some(result) = f(stmt) {
                        return Some(result);
                    }
                }
            }
            NodeKind::Block { statements } => {
                for stmt in statements {
                    if let Some(result) = f(stmt) {
                        return Some(result);
                    }
                }
            }
            NodeKind::ExpressionStatement { expression } => {
                if let Some(result) = f(expression) {
                    return Some(result);
                }
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                if let Some(result) = f(condition) {
                    return Some(result);
                }
                if let Some(result) = f(then_branch) {
                    return Some(result);
                }
                for (elsif_cond, elsif_body) in elsif_branches {
                    if let Some(result) = f(elsif_cond) {
                        return Some(result);
                    }
                    if let Some(result) = f(elsif_body) {
                        return Some(result);
                    }
                }
                if let Some(else_b) = else_branch {
                    if let Some(result) = f(else_b) {
                        return Some(result);
                    }
                }
            }
            NodeKind::While { condition, body, .. } => {
                if let Some(result) = f(condition) {
                    return Some(result);
                }
                if let Some(result) = f(body) {
                    return Some(result);
                }
            }
            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(init_node) = init {
                    if let Some(result) = f(init_node) {
                        return Some(result);
                    }
                }
                if let Some(cond) = condition {
                    if let Some(result) = f(cond) {
                        return Some(result);
                    }
                }
                if let Some(upd) = update {
                    if let Some(result) = f(upd) {
                        return Some(result);
                    }
                }
                if let Some(result) = f(body) {
                    return Some(result);
                }
            }
            NodeKind::Foreach { variable, list, body, continue_block: _ } => {
                if let Some(result) = f(variable) {
                    return Some(result);
                }
                if let Some(result) = f(list) {
                    return Some(result);
                }
                if let Some(result) = f(body) {
                    return Some(result);
                }
            }
            NodeKind::Subroutine { signature, body, .. } => {
                if let Some(sig) = signature {
                    if let NodeKind::Signature { parameters } = &sig.kind {
                        for param in parameters {
                            if let Some(result) = f(param) {
                                return Some(result);
                            }
                        }
                    }
                }
                if let Some(result) = f(body) {
                    return Some(result);
                }
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    if let Some(result) = f(arg) {
                        return Some(result);
                    }
                }
            }
            NodeKind::MethodCall { object, args, .. } => {
                if let Some(result) = f(object) {
                    return Some(result);
                }
                for arg in args {
                    if let Some(result) = f(arg) {
                        return Some(result);
                    }
                }
            }
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    if let Some(result) = f(elem) {
                        return Some(result);
                    }
                }
            }
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    if let Some(result) = f(key) {
                        return Some(result);
                    }
                    if let Some(result) = f(value) {
                        return Some(result);
                    }
                }
            }
            NodeKind::Binary { left, right, .. } => {
                if let Some(result) = f(left) {
                    return Some(result);
                }
                if let Some(result) = f(right) {
                    return Some(result);
                }
            }
            NodeKind::Unary { operand, .. } => {
                if let Some(result) = f(operand) {
                    return Some(result);
                }
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                if let Some(result) = f(lhs) {
                    return Some(result);
                }
                if let Some(result) = f(rhs) {
                    return Some(result);
                }
            }
            NodeKind::Return { value } => {
                if let Some(val) = value {
                    if let Some(result) = f(val) {
                        return Some(result);
                    }
                }
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                if let Some(result) = f(variable) {
                    return Some(result);
                }
                if let Some(val) = initializer {
                    if let Some(result) = f(val) {
                        return Some(result);
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Convert node to LSP range
    fn node_to_range(&self, node: &Node) -> Range {
        let start = self.offset_to_position(node.location.start);
        let end = self.offset_to_position(node.location.end);
        Range { start, end }
    }

    /// Convert byte offset to line/character position using PositionMapper for UTF-16 compliance
    fn offset_to_position(&self, offset: usize) -> Position {
        let pos = self.position_mapper.byte_to_lsp_pos(offset);
        Position { line: pos.line, character: pos.character }
    }

    /// Convert line/character position to byte offset using PositionMapper for UTF-16 compliance
    fn position_to_offset(&self, line: u32, character: u32) -> usize {
        let pos = WirePosition { line, character };
        self.position_mapper.lsp_pos_to_byte(pos).unwrap_or(self.source.len())
    }

    /// Compute selection range from an optional name_span, falling back to full range
    ///
    /// If `name_span` is `Some`, returns a precise range covering just the symbol name.
    /// Otherwise, returns the full range as a fallback for backward compatibility.
    fn selection_range_from_name_span(
        &self,
        name_span: &Option<crate::SourceLocation>,
        full_range: &Range,
    ) -> Range {
        match name_span {
            Some(span) => Range {
                start: self.offset_to_position(span.start),
                end: self.offset_to_position(span.end),
            },
            None => *full_range,
        }
    }
}

/// Incoming call information representing a caller of a function
///
/// This structure represents a function that calls the target function,
/// including the location of the caller and all the ranges where it calls the target.
#[derive(Debug, Clone)]
pub struct CallHierarchyIncomingCall {
    /// The function or method that is calling the target
    pub from: CallHierarchyItem,
    /// All the ranges in the caller where it invokes the target function
    pub from_ranges: Vec<Range>,
}

/// Outgoing call information representing a function being called
///
/// This structure represents a function that is called by the current function,
/// including the location of the callee and all the ranges where it is called.
#[derive(Debug, Clone)]
pub struct CallHierarchyOutgoingCall {
    /// The function or method being called
    pub to: CallHierarchyItem,
    /// All the ranges in the current function where the target is called
    pub from_ranges: Vec<Range>,
}

/// Convert to JSON for LSP
impl CallHierarchyItem {
    /// Convert the call hierarchy item to JSON format for LSP protocol.
    ///
    /// # Returns
    /// A JSON value containing the item name, symbol kind, URI, and range information
    /// compatible with LSP CallHierarchyItem specification.
    pub fn to_json(&self) -> Value {
        let mut item = json!({
            "name": self.name,
            "kind": match self.kind.as_str() {
                "function" => 12, // SymbolKind.Function
                "method" => 6,    // SymbolKind.Method
                _ => 12,
            },
            "uri": self.uri,
            "range": {
                "start": {
                    "line": self.range.start.line,
                    "character": self.range.start.character
                },
                "end": {
                    "line": self.range.end.line,
                    "character": self.range.end.character
                }
            },
            "selectionRange": {
                "start": {
                    "line": self.selection_range.start.line,
                    "character": self.selection_range.start.character
                },
                "end": {
                    "line": self.selection_range.end.line,
                    "character": self.selection_range.end.character
                }
            }
        });

        if let Some(detail) = &self.detail {
            item["detail"] = json!(detail);
        }

        if self.package_name.is_some() || self.qualified_name.is_some() {
            item["data"] = json!({
                "packageName": self.package_name,
                "qualifiedName": self.qualified_name,
            });
        }

        item
    }
}

impl CallHierarchyIncomingCall {
    /// Convert the incoming call to JSON format for LSP protocol.
    ///
    /// # Returns
    /// A JSON value containing the source item and ranges where the call originates.
    pub fn to_json(&self) -> Value {
        json!({
            "from": self.from.to_json(),
            "fromRanges": self.from_ranges.iter().map(|r| json!({
                "start": {
                    "line": r.start.line,
                    "character": r.start.character
                },
                "end": {
                    "line": r.end.line,
                    "character": r.end.character
                }
            })).collect::<Vec<_>>()
        })
    }
}

impl CallHierarchyOutgoingCall {
    /// Convert the outgoing call to JSON format for LSP protocol.
    ///
    /// # Returns
    /// A JSON value containing the target item and ranges where the call is made.
    pub fn to_json(&self) -> Value {
        json!({
            "to": self.to.to_json(),
            "fromRanges": self.from_ranges.iter().map(|r| json!({
                "start": {
                    "line": r.start.line,
                    "character": r.start.character
                },
                "end": {
                    "line": r.end.line,
                    "character": r.end.character
                }
            })).collect::<Vec<_>>()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;

    #[test]
    fn test_call_hierarchy_prepare() {
        let code = r#"
sub main {
    helper();
    process_data();
}

sub helper {
    print "Helper\n";
}

sub process_data {
    helper();
}
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let provider =
                CallHierarchyProvider::new(code.to_string(), "file:///test.pl".to_string());

            // Find function at position (line 1, char 5) - "main"
            let items = provider.prepare(&ast, 1, 5);
            assert!(items.is_some());
            let items = items.ok_or("expected items").map_err(|e| e.to_string());
            if let Ok(items) = items {
                assert_eq!(items.len(), 1);
                assert_eq!(items[0].name, "main");
            }
        }
    }

    #[test]
    fn test_incoming_calls() {
        let code = r#"
sub caller1 {
    target_func();
}

sub caller2 {
    target_func();
    target_func(); # called twice
}

sub target_func {
    print "Target\n";
}
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let provider =
                CallHierarchyProvider::new(code.to_string(), "file:///test.pl".to_string());

            let target_item = CallHierarchyItem {
                name: "target_func".to_string(),
                kind: "function".to_string(),
                uri: "file:///test.pl".to_string(),
                range: Range {
                    start: Position { line: 10, character: 0 },
                    end: Position { line: 12, character: 1 },
                },
                selection_range: Range {
                    start: Position { line: 10, character: 4 },
                    end: Position { line: 10, character: 15 },
                },
                detail: None,
                package_name: None,
                qualified_name: None,
            };

            let incoming = provider.incoming_calls(&ast, &target_item);
            assert_eq!(incoming.len(), 2);

            // Check callers
            let caller_names: Vec<_> = incoming.iter().map(|c| &c.from.name).collect();
            assert!(caller_names.contains(&&"caller1".to_string()));
            assert!(caller_names.contains(&&"caller2".to_string()));

            // caller2 should have 2 ranges (called twice)
            let caller2_opt = incoming.iter().find(|c| c.from.name == "caller2");
            assert!(caller2_opt.is_some(), "caller2 not found in incoming calls");
            if let Some(caller2) = caller2_opt {
                assert_eq!(caller2.from_ranges.len(), 2);
            }
        }
    }

    #[test]
    fn test_outgoing_calls() {
        let code = r#"
sub main {
    helper();
    process_data();
    $obj->method_call();
}

sub helper {
    print "Helper\n";
}
"#;

        let mut parser = Parser::new(code);
        if let Ok(ast) = parser.parse() {
            let provider =
                CallHierarchyProvider::new(code.to_string(), "file:///test.pl".to_string());

            let main_item = CallHierarchyItem {
                name: "main".to_string(),
                kind: "function".to_string(),
                uri: "file:///test.pl".to_string(),
                range: Range {
                    start: Position { line: 1, character: 0 },
                    end: Position { line: 5, character: 1 },
                },
                selection_range: Range {
                    start: Position { line: 1, character: 4 },
                    end: Position { line: 1, character: 8 },
                },
                detail: None,
                package_name: None,
                qualified_name: None,
            };

            let outgoing = provider.outgoing_calls(&ast, &main_item);
            assert_eq!(outgoing.len(), 3);

            // Check called functions
            let called_names: Vec<_> = outgoing.iter().map(|c| &c.to.name).collect();
            assert!(called_names.contains(&&"helper".to_string()));
            assert!(called_names.contains(&&"process_data".to_string()));
            assert!(called_names.contains(&&"method_call".to_string()));
        }
    }
}
