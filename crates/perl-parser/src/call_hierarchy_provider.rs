use crate::ast::{Node, NodeKind};
use serde_json::{Value, json};

/// Call Hierarchy Item
#[derive(Debug, Clone)]
pub struct CallHierarchyItem {
    pub name: String,
    pub kind: String,
    pub uri: String,
    pub range: Range,
    pub selection_range: Range,
    pub detail: Option<String>,
}

#[derive(Debug, Clone)]
pub struct Range {
    pub start: Position,
    pub end: Position,
}

#[derive(Debug, Clone)]
pub struct Position {
    pub line: u32,
    pub character: u32,
}

/// Call Hierarchy Provider
pub struct CallHierarchyProvider {
    source: String,
    uri: String,
}

impl CallHierarchyProvider {
    pub fn new(source: String, uri: String) -> Self {
        Self { source, uri }
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
            if let NodeKind::Subroutine { body, .. } = &func_node.kind {
                self.find_outgoing_calls(body, &mut calls);
            }
            calls
        } else {
            Vec::new()
        }
    }

    /// Find a callable item at the given position
    fn find_callable_at_position(&self, node: &Node, offset: usize) -> Option<CallHierarchyItem> {
        if offset >= node.location.start && offset <= node.location.end {
            match &node.kind {
                NodeKind::Subroutine { name, params, .. } => {
                    if let Some(name_str) = name {
                        let range = self.node_to_range(node);
                        let selection_range = range.clone(); // TODO: Calculate name range

                        let detail = if params.is_empty() {
                            None
                        } else {
                            Some(format!("({} params)", params.len()))
                        };

                        return Some(CallHierarchyItem {
                            name: name_str.clone(),
                            kind: "function".to_string(),
                            uri: self.uri.clone(),
                            range,
                            selection_range,
                            detail,
                        });
                    }
                }
                NodeKind::MethodCall { method, .. } => {
                    let range = self.node_to_range(node);
                    return Some(CallHierarchyItem {
                        name: method.clone(),
                        kind: "method".to_string(),
                        uri: self.uri.clone(),
                        range: range.clone(),
                        selection_range: range,
                        detail: None,
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
        match &node.kind {
            NodeKind::Subroutine { name, .. } => {
                if let Some(name_str) = name {
                    let item = CallHierarchyItem {
                        name: name_str.clone(),
                        kind: "function".to_string(),
                        uri: self.uri.clone(),
                        range: self.node_to_range(node),
                        selection_range: self.node_to_range(node),
                        detail: None,
                    };

                    // Search within this function
                    self.visit_children(node, |child| {
                        self.find_incoming_calls(child, target_name, calls, Some(&item));
                        None::<()>
                    });
                }
            }
            NodeKind::FunctionCall { name, .. } => {
                if name == target_name {
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
    fn find_outgoing_calls(&self, node: &Node, calls: &mut Vec<CallHierarchyOutgoingCall>) {
        match &node.kind {
            NodeKind::FunctionCall { name, .. } => {
                let item = CallHierarchyItem {
                    name: name.clone(),
                    kind: "function".to_string(),
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    selection_range: self.node_to_range(node),
                    detail: None,
                };

                let ranges = vec![self.node_to_range(node)];

                // Check if we already have a call to this function
                if let Some(existing) = calls.iter_mut().find(|c| &c.to.name == name) {
                    existing.from_ranges.extend(ranges);
                } else {
                    calls.push(CallHierarchyOutgoingCall {
                        to: item,
                        from_ranges: ranges,
                    });
                }
            }
            NodeKind::MethodCall { method, object, .. } => {
                let detail = if let NodeKind::Variable { name, .. } = &object.kind {
                    Some(format!("on ${}", name))
                } else {
                    None
                };

                let item = CallHierarchyItem {
                    name: method.clone(),
                    kind: "method".to_string(),
                    uri: self.uri.clone(),
                    range: self.node_to_range(node),
                    selection_range: self.node_to_range(node),
                    detail,
                };

                let ranges = vec![self.node_to_range(node)];

                if let Some(existing) = calls.iter_mut().find(|c| &c.to.name == method) {
                    existing.from_ranges.extend(ranges);
                } else {
                    calls.push(CallHierarchyOutgoingCall {
                        to: item,
                        from_ranges: ranges,
                    });
                }
            }
            _ => {}
        }

        // Visit children
        self.visit_children(node, |child| {
            self.find_outgoing_calls(child, calls);
            None::<()>
        });
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
            NodeKind::If {
                condition,
                then_branch,
                elsif_branches,
                else_branch,
            } => {
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
            NodeKind::While {
                condition, body, ..
            } => {
                if let Some(result) = f(condition) {
                    return Some(result);
                }
                if let Some(result) = f(body) {
                    return Some(result);
                }
            }
            NodeKind::For {
                init,
                condition,
                update,
                body,
                ..
            } => {
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
            NodeKind::Foreach {
                variable,
                list,
                body,
            } => {
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
            NodeKind::Subroutine { params, body, .. } => {
                for param in params {
                    if let Some(result) = f(param) {
                        return Some(result);
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
            NodeKind::VariableDeclaration {
                variable,
                initializer,
                ..
            } => {
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

    /// Convert byte offset to line/character position
    fn offset_to_position(&self, offset: usize) -> Position {
        let mut line = 0;
        let mut col = 0;

        for (i, ch) in self.source.chars().enumerate() {
            if i >= offset {
                break;
            }
            if ch == '\n' {
                line += 1;
                col = 0;
            } else {
                col += 1;
            }
        }

        Position {
            line,
            character: col,
        }
    }

    /// Convert line/character position to byte offset
    fn position_to_offset(&self, line: u32, character: u32) -> usize {
        let mut current_line = 0;
        let mut current_col = 0;
        let mut offset = 0;

        for (i, ch) in self.source.chars().enumerate() {
            if current_line == line && current_col == character {
                return i;
            }
            if ch == '\n' {
                current_line += 1;
                current_col = 0;
            } else {
                current_col += 1;
            }
            offset = i;
        }

        offset + 1
    }
}

/// Incoming call information
#[derive(Debug, Clone)]
pub struct CallHierarchyIncomingCall {
    pub from: CallHierarchyItem,
    pub from_ranges: Vec<Range>,
}

/// Outgoing call information
#[derive(Debug, Clone)]
pub struct CallHierarchyOutgoingCall {
    pub to: CallHierarchyItem,
    pub from_ranges: Vec<Range>,
}

/// Convert to JSON for LSP
impl CallHierarchyItem {
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

        item
    }
}

impl CallHierarchyIncomingCall {
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
            let items = items.unwrap();
            assert_eq!(items.len(), 1);
            assert_eq!(items[0].name, "main");
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
                    start: Position {
                        line: 10,
                        character: 0,
                    },
                    end: Position {
                        line: 12,
                        character: 1,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: 10,
                        character: 4,
                    },
                    end: Position {
                        line: 10,
                        character: 15,
                    },
                },
                detail: None,
            };

            let incoming = provider.incoming_calls(&ast, &target_item);
            assert_eq!(incoming.len(), 2);

            // Check callers
            let caller_names: Vec<_> = incoming.iter().map(|c| &c.from.name).collect();
            assert!(caller_names.contains(&&"caller1".to_string()));
            assert!(caller_names.contains(&&"caller2".to_string()));

            // caller2 should have 2 ranges (called twice)
            let caller2 = incoming.iter().find(|c| c.from.name == "caller2").unwrap();
            assert_eq!(caller2.from_ranges.len(), 2);
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
                    start: Position {
                        line: 1,
                        character: 0,
                    },
                    end: Position {
                        line: 5,
                        character: 1,
                    },
                },
                selection_range: Range {
                    start: Position {
                        line: 1,
                        character: 4,
                    },
                    end: Position {
                        line: 1,
                        character: 8,
                    },
                },
                detail: None,
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
