//! Type definition support for Perl LSP
//!
//! This module provides go-to-type-definition functionality,
//! finding the type/class definition for variables and references.

#[cfg(feature = "lsp-compat")]
use perl_parser_core::ast::{Node, NodeKind};

#[cfg(feature = "lsp-compat")]
use lsp_types::LocationLink;
#[cfg(feature = "lsp-compat")]
use std::collections::HashMap;
#[cfg(feature = "lsp-compat")]
use std::str::FromStr;

/// Provides go-to-type-definition functionality for Perl code.
///
/// Finds and locates type/class definitions for variables and references,
/// enabling LSP clients to navigate to the source of type definitions.
pub struct TypeDefinitionProvider;

impl TypeDefinitionProvider {
    /// Creates a new type definition provider instance.
    pub fn new() -> Self {
        Self
    }

    /// Find type definition for a position in the AST
    #[cfg(feature = "lsp-compat")]
    pub fn find_type_definition(
        &self,
        ast: &Node,
        line: u32,
        character: u32,
        uri: &str,
        documents: &HashMap<String, String>,
    ) -> Option<Vec<LocationLink>> {
        // Get source text for position conversion
        let source_text = documents.get(uri)?;

        // Find the node at the given position
        let target_node = self.find_node_at_position(ast, line, character, source_text)?;

        // First try Moose/Moo `has(... isa => Type)` attribute forms.
        let type_name = self
            .extract_has_type_constraint_name(
                ast,
                target_node.location.start,
                target_node.location.end,
            )
            // Fall back to generic class / object / `isa` expression handling.
            .or_else(|| self.extract_type_name(&target_node))?;

        // Try to resolve custom type declarations before package/class names.
        if let Some(locations) =
            self.find_custom_type_definition_in_docs(&type_name, uri, documents)
        {
            return Some(locations);
        }

        // Find the package/class definition — search all open documents
        self.find_package_definition_in_docs(&type_name, uri, documents)
    }

    /// Find a package definition across all open documents.
    ///
    /// Re-parses every document in `documents` (including the current file) and
    /// collects all locations where `package <package_name>` is declared. This
    /// enables cross-file go-to-type-definition (Fix A).
    #[cfg(feature = "lsp-compat")]
    fn find_package_definition_in_docs(
        &self,
        package_name: &str,
        _origin_uri: &str,
        documents: &HashMap<String, String>,
    ) -> Option<Vec<LocationLink>> {
        let mut locations = Vec::new();

        for (doc_uri, source_text) in documents {
            if let Ok(ast) = perl_parser_core::Parser::new(source_text).parse() {
                self.find_package_in_node(&ast, package_name, doc_uri, source_text, &mut locations);
            }
        }

        if !locations.is_empty() { Some(locations) } else { None }
    }

    /// Find a custom Moose/Type::Tiny type declaration across all open documents.
    ///
    /// Supports bounded type-declaration forms such as `type UserID, ...` and
    /// `subtype PositiveInt, ...`, which are sufficient for MooseX::Types and
    /// Type::Tiny libraries used by the editor-facing provider path.
    #[cfg(feature = "lsp-compat")]
    fn find_custom_type_definition_in_docs(
        &self,
        type_name: &str,
        _origin_uri: &str,
        documents: &HashMap<String, String>,
    ) -> Option<Vec<LocationLink>> {
        let mut locations = Vec::new();

        for (doc_uri, source_text) in documents {
            if let Ok(ast) = perl_parser_core::Parser::new(source_text).parse() {
                self.find_custom_type_in_node(
                    &ast,
                    type_name,
                    doc_uri,
                    source_text,
                    &mut locations,
                );
            }

            if locations.is_empty() {
                self.find_custom_type_in_source(type_name, doc_uri, source_text, &mut locations);
            }
        }

        if !locations.is_empty() { Some(locations) } else { None }
    }

    /// Extract type name from a node
    #[cfg(feature = "lsp-compat")]
    fn extract_type_name(&self, node: &Node) -> Option<String> {
        match &node.kind {
            // Variable declaration with type: my ClassName $var
            NodeKind::VariableDeclaration { variable, attributes, .. } => {
                // Check if there's a type attribute (Perl 5.20+ style)
                // Attributes are Vec<String>
                for attr in attributes {
                    // Check if the attribute looks like a package name
                    if attr.contains("::") || attr.chars().next().is_some_and(|c| c.is_uppercase())
                    {
                        // Type is specified as an attribute
                        return Some(attr.clone());
                    }
                }
                // For typed variables, the type might be in the variable node itself
                if let NodeKind::Variable { name, .. } = &variable.kind {
                    // Check if name contains a type prefix pattern
                    if name.contains("::") {
                        // Extract package name from qualified variable
                        let parts: Vec<&str> = name.split("::").collect();
                        if parts.len() >= 2 {
                            return Some(parts[..parts.len() - 1].join("::"));
                        }
                    }
                }
                None
            }
            // Method call: $obj->method
            NodeKind::MethodCall { object, .. } => {
                // Try to infer the type of the object
                self.infer_object_type(object)
            }
            // Variable reference - look for its type
            NodeKind::Variable { .. } => {
                // Would need to track variable types through semantic analysis
                // For now, return None and rely on context
                None
            }
            // Package identifier or Package::method
            NodeKind::Identifier { name } => {
                if name.contains("::") {
                    // Qualified name like Package::method
                    let parts: Vec<&str> = name.split("::").collect();
                    if parts.len() >= 2 {
                        // Get the package name (everything except the last part)
                        return Some(parts[..parts.len() - 1].join("::"));
                    }
                }
                // Check if this identifier looks like a package name (starts with uppercase)
                if name.chars().next().is_some_and(|c| c.is_uppercase()) {
                    // Likely a package/class name
                    return Some(name.clone());
                }
                None
            }
            // Constructor: Package->new or new Package
            NodeKind::Binary { op, left, right } if op == "->" => {
                // Handle Package->new pattern
                if let NodeKind::Identifier { name: pkg } = &left.kind {
                    if let NodeKind::Identifier { name: method } = &right.kind
                        && method == "new"
                    {
                        return Some(pkg.clone());
                    }
                    // Also handle Package->method where we want Package
                    return Some(pkg.clone());
                }
                None
            }
            // Blessed reference: bless $ref, 'Package'
            NodeKind::FunctionCall { name, args } if name == "bless" => {
                if args.len() >= 2 {
                    // Second argument is the package name
                    match &args[1].kind {
                        NodeKind::String { value, .. } => Some(value.clone()),
                        NodeKind::Identifier { name } => Some(name.clone()),
                        NodeKind::Variable { name, .. } => {
                            // Handle bless {}, $class where $class holds the package name
                            Some(name.clone())
                        }
                        _ => None,
                    }
                } else if args.len() == 1 {
                    // bless $ref (uses caller's package)
                    None
                } else {
                    None
                }
            }
            // ISA check: $obj isa Package
            NodeKind::Binary { op, right, .. } if op == "isa" => match &right.kind {
                NodeKind::String { value, .. } => Some(value.clone()),
                NodeKind::Identifier { name } => Some(name.clone()),
                _ => None,
            },
            // Expression statement - unwrap to inner expression
            NodeKind::ExpressionStatement { expression } => self.extract_type_name(expression),
            _ => None,
        }
    }

    /// Extract the type name from a Moose/Moo `has(... isa => Type)` attribute.
    #[cfg(feature = "lsp-compat")]
    fn extract_has_type_constraint_name(
        &self,
        node: &Node,
        target_start: usize,
        target_end: usize,
    ) -> Option<String> {
        match &node.kind {
            NodeKind::FunctionCall { name, args } if name == "has" => {
                for arg in args {
                    if let Some(type_name) = self.extract_has_type_constraint_name_from_node(
                        arg,
                        target_start,
                        target_end,
                    ) {
                        return Some(type_name);
                    }
                }
            }
            _ => {}
        }

        let mut found = None;
        self.visit_children(node, |child| {
            if found.is_none() {
                found = self.extract_has_type_constraint_name(child, target_start, target_end);
            }
        });
        found
    }

    /// Search a node for the `isa => Type` value that encloses the cursor.
    #[cfg(feature = "lsp-compat")]
    fn extract_has_type_constraint_name_from_node(
        &self,
        node: &Node,
        target_start: usize,
        target_end: usize,
    ) -> Option<String> {
        match &node.kind {
            NodeKind::HashLiteral { pairs } => {
                for (key, pair_value) in pairs {
                    if matches!(&key.kind, NodeKind::String { value: key_name, .. } if key_name == "isa")
                        && target_start >= pair_value.location.start
                        && target_end <= pair_value.location.end
                    {
                        return match &pair_value.kind {
                            NodeKind::Identifier { name } => Some(name.clone()),
                            NodeKind::String { value, .. } => Some(value.clone()),
                            NodeKind::Variable { name, .. } => Some(name.clone()),
                            _ => None,
                        };
                    }
                }
            }
            NodeKind::Binary { op, left, right } if op == "=>" => {
                if matches!(&left.kind, NodeKind::Identifier { name } if name == "isa")
                    && target_start >= right.location.start
                    && target_end <= right.location.end
                {
                    return match &right.kind {
                        NodeKind::Identifier { name } => Some(name.clone()),
                        NodeKind::String { value, .. } => Some(value.clone()),
                        NodeKind::Variable { name, .. } => Some(name.clone()),
                        _ => None,
                    };
                }
            }
            _ => {}
        }

        let mut found = None;
        self.visit_children(node, |child| {
            if found.is_none() {
                found = self.extract_has_type_constraint_name_from_node(
                    child,
                    target_start,
                    target_end,
                );
            }
        });
        found
    }

    /// Return `true` when a function call looks like a type declaration.
    #[cfg(feature = "lsp-compat")]
    fn is_type_declaration_call(name: &str) -> bool {
        matches!(name, "type" | "subtype" | "class_type" | "role_type" | "enum" | "declare")
    }

    /// Extract a declared type name from the arguments of a type declaration call.
    #[cfg(feature = "lsp-compat")]
    fn declared_type_name(args: &[Node]) -> Option<String> {
        args.first().and_then(|arg| match &arg.kind {
            NodeKind::Identifier { name } => Some(name.clone()),
            NodeKind::String { value, .. } => Some(value.clone()),
            NodeKind::Variable { name, .. } => Some(name.clone()),
            _ => None,
        })
    }

    /// Fallback textual scan for custom type declarations when the AST does not
    /// expose the MooseX::Types DSL shape directly.
    #[cfg(feature = "lsp-compat")]
    fn find_custom_type_in_source(
        &self,
        type_name: &str,
        uri: &str,
        source_text: &str,
        locations: &mut Vec<LocationLink>,
    ) {
        let mut offset = 0usize;

        for line in source_text.split_inclusive('\n') {
            let line_end = offset + line.len();
            let body = line.strip_suffix('\n').unwrap_or(line);
            let body = body.strip_suffix('\r').unwrap_or(body);
            let trimmed = body.trim_start();
            let leading_ws = body.len().saturating_sub(trimmed.len());
            let start_offset = offset + leading_ws;

            if Self::line_declares_custom_type(trimmed, type_name) {
                self.push_location_link_from_offsets(
                    start_offset,
                    line_end,
                    uri,
                    source_text,
                    locations,
                );
                return;
            }

            offset = line_end;
        }
    }

    /// Return `true` when a source line looks like a supported custom type declaration.
    #[cfg(feature = "lsp-compat")]
    fn line_declares_custom_type(line: &str, type_name: &str) -> bool {
        let keywords = ["type", "subtype", "class_type", "role_type", "enum", "declare"];

        keywords.iter().any(|keyword| {
            let Some(rest) = line.strip_prefix(keyword) else {
                return false;
            };

            Self::first_declared_name(rest).as_deref().is_some_and(|declared| declared == type_name)
        })
    }

    /// Extract the first declared identifier or string from a type declaration tail.
    #[cfg(feature = "lsp-compat")]
    fn first_declared_name(text: &str) -> Option<String> {
        let mut rest = text.trim_start();
        while rest.starts_with([',', '(', ')']) {
            rest = rest[1..].trim_start();
        }

        if let Some(quoted) = rest.strip_prefix('"').or_else(|| rest.strip_prefix('\'')) {
            let quote = rest.chars().next()?;
            let value_end = quoted.find(quote)?;
            return Some(quoted[..value_end].to_string());
        }

        let name: String = rest
            .chars()
            .take_while(|ch| ch.is_alphanumeric() || matches!(ch, '_' | ':' | '-'))
            .collect();

        if name.is_empty() { None } else { Some(name) }
    }

    /// Try to infer the type of an object from its declaration or assignment
    #[cfg(feature = "lsp-compat")]
    fn infer_object_type(&self, object: &Node) -> Option<String> {
        match &object.kind {
            NodeKind::Variable { name, .. } => {
                // Would need to track variable types through analysis
                // For now, try common patterns like $self
                if name == "$self" || name == "$this" {
                    // Would need to find the enclosing package
                    None
                } else {
                    None
                }
            }
            // Direct constructor call
            NodeKind::FunctionCall { name, .. } if name == "new" => {
                // The package should be in the parent context
                None
            }
            _ => None,
        }
    }

    /// Find package definition in the AST (used in unit tests).
    #[cfg(feature = "lsp-compat")]
    #[cfg_attr(not(test), allow(dead_code))]
    fn find_package_definition(
        &self,
        ast: &Node,
        package_name: &str,
        uri: &str,
        source_text: &str,
    ) -> Option<Vec<LocationLink>> {
        let mut locations = Vec::new();
        self.find_package_in_node(ast, package_name, uri, source_text, &mut locations);

        if !locations.is_empty() { Some(locations) } else { None }
    }

    /// Recursively find package definitions
    #[cfg(feature = "lsp-compat")]
    fn find_package_in_node(
        &self,
        node: &Node,
        package_name: &str,
        uri: &str,
        source_text: &str,
        locations: &mut Vec<LocationLink>,
    ) {
        match &node.kind {
            NodeKind::Package { name, .. } if name == package_name => {
                self.push_location_link(node, uri, source_text, locations);
            }
            _ => {}
        }

        // Recurse into children based on node type
        self.visit_children(node, |child| {
            self.find_package_in_node(child, package_name, uri, source_text, locations);
        });
    }

    /// Recursively find type declarations by matching the declared name.
    #[cfg(feature = "lsp-compat")]
    fn find_custom_type_in_node(
        &self,
        node: &Node,
        type_name: &str,
        uri: &str,
        source_text: &str,
        locations: &mut Vec<LocationLink>,
    ) {
        match &node.kind {
            NodeKind::FunctionCall { name, args } if Self::is_type_declaration_call(name) => {
                if Self::declared_type_name(args).as_deref() == Some(type_name) {
                    self.push_location_link(node, uri, source_text, locations);
                }
            }
            _ => {}
        }

        self.visit_children(node, |child| {
            self.find_custom_type_in_node(child, type_name, uri, source_text, locations);
        });
    }

    /// Convert the current node range into an LSP `LocationLink` and push it.
    #[cfg(feature = "lsp-compat")]
    fn push_location_link(
        &self,
        node: &Node,
        uri: &str,
        source_text: &str,
        locations: &mut Vec<LocationLink>,
    ) {
        let (target_start_line, target_start_char) =
            perl_parser_core::engine::position::offset_to_utf16_line_col(
                source_text,
                node.location.start,
            );
        let (target_end_line, target_end_char) =
            perl_parser_core::engine::position::offset_to_utf16_line_col(
                source_text,
                node.location.end,
            );

        let target_range = lsp_types::Range {
            start: lsp_types::Position { line: target_start_line, character: target_start_char },
            end: lsp_types::Position { line: target_end_line, character: target_end_char },
        };

        if let Ok(target_uri) = lsp_types::Uri::from_str(uri) {
            locations.push(LocationLink {
                origin_selection_range: None,
                target_uri,
                target_range,
                target_selection_range: target_range,
            });
        }
    }

    /// Convert raw byte offsets into an LSP `LocationLink` and push it.
    #[cfg(feature = "lsp-compat")]
    fn push_location_link_from_offsets(
        &self,
        start_offset: usize,
        end_offset: usize,
        uri: &str,
        source_text: &str,
        locations: &mut Vec<LocationLink>,
    ) {
        let (target_start_line, target_start_char) =
            perl_parser_core::engine::position::offset_to_utf16_line_col(source_text, start_offset);
        let (target_end_line, target_end_char) =
            perl_parser_core::engine::position::offset_to_utf16_line_col(source_text, end_offset);

        let target_range = lsp_types::Range {
            start: lsp_types::Position { line: target_start_line, character: target_start_char },
            end: lsp_types::Position { line: target_end_line, character: target_end_char },
        };

        if let Ok(target_uri) = lsp_types::Uri::from_str(uri) {
            locations.push(LocationLink {
                origin_selection_range: None,
                target_uri,
                target_range,
                target_selection_range: target_range,
            });
        }
    }

    /// Helper to visit children of a node
    #[cfg(feature = "lsp-compat")]
    fn visit_children<F>(&self, node: &Node, mut f: F)
    where
        F: FnMut(&Node),
    {
        match &node.kind {
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    f(stmt);
                }
            }
            NodeKind::Package { block: Some(b), .. } => {
                f(b);
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                f(variable);
                if let Some(init) = initializer {
                    f(init);
                }
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                f(lhs);
                f(rhs);
            }
            NodeKind::Binary { left, right, .. } => {
                f(left);
                f(right);
            }
            NodeKind::MethodCall { object, args, .. } => {
                f(object);
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::FunctionCall { args, .. } => {
                for arg in args {
                    f(arg);
                }
            }
            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    f(key);
                    f(value);
                }
            }
            NodeKind::Subroutine { body, .. } => {
                f(body);
            }
            NodeKind::ExpressionStatement { expression } => {
                f(expression);
            }
            NodeKind::If { condition, then_branch, else_branch, .. } => {
                f(condition);
                f(then_branch);
                if let Some(else_b) = else_branch {
                    f(else_b);
                }
            }
            NodeKind::While { condition, body, .. } => {
                f(condition);
                f(body);
            }
            NodeKind::For { init, condition, update, body, .. } => {
                if let Some(i) = init {
                    f(i);
                }
                if let Some(c) = condition {
                    f(c);
                }
                if let Some(upd) = update {
                    f(upd);
                }
                f(body);
            }
            NodeKind::Foreach { variable, list, body, continue_block } => {
                f(variable);
                if let Some(cb) = continue_block {
                    f(cb);
                }
                f(list);
                f(body);
                if let Some(cb) = continue_block {
                    f(cb);
                }
            }
            _ => {
                // Other node types don't have children we need to traverse
            }
        }
    }

    /// Find node at the given position
    #[cfg(feature = "lsp-compat")]
    fn find_node_at_position(
        &self,
        node: &Node,
        line: u32,
        character: u32,
        source_text: &str,
    ) -> Option<Node> {
        // Convert UTF-16 line/char to byte offset using perl-parser-core
        let offset = perl_parser_core::engine::position::utf16_line_col_to_offset(
            source_text,
            line,
            character,
        );

        // Find the most specific node at this offset
        self.find_node_at_offset(node, offset)
    }

    /// Find the most specific node containing the given offset
    #[cfg(feature = "lsp-compat")]
    fn find_node_at_offset(&self, node: &Node, offset: usize) -> Option<Node> {
        // Check if offset is within this node's range
        if offset < node.location.start || offset > node.location.end {
            return None;
        }

        // Check children first for more specific match
        let mut best_match = None;
        self.visit_children(node, |child| {
            if let Some(found) = self.find_node_at_offset(child, offset) {
                // Prefer the smallest (most specific) node
                if best_match.is_none()
                    || found.location.end - found.location.start
                        < best_match
                            .as_ref()
                            .map_or(usize::MAX, |n: &Node| n.location.end - n.location.start)
                {
                    best_match = Some(found);
                }
            }
        });

        // If we found a child, return it; otherwise return this node
        best_match.or_else(|| Some(node.clone()))
    }
}

impl Default for TypeDefinitionProvider {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(all(test, feature = "lsp-compat"))]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_find_package_definition() {
        let code = r#"
package MyClass;

sub new {
    my $class = shift;
    bless {}, $class;
}

package main;

my $obj = MyClass->new();
$obj->method();
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = TypeDefinitionProvider::new();
        let uri = "file:///test.pl";

        // Test finding MyClass definition
        let locations = provider.find_package_definition(&ast, "MyClass", uri, code);
        assert!(locations.is_some());
        let locs = must_some(locations);
        assert_eq!(locs.len(), 1);
    }

    #[test]
    fn test_extract_type_from_constructor() {
        let code = "my $obj = Package::Name->new();";
        let mut parser = Parser::new(code);
        let _ast = must(parser.parse());

        let _provider = TypeDefinitionProvider::new();

        // Would need to traverse to find the right node
        // This is a simplified test
    }

    #[test]
    fn test_full_type_definition_flow() {
        let code = r#"
package MyClass;

sub new {
    my $class = shift;
    bless {}, $class;
}

package main;

my $obj = MyClass->new();
$obj->method();
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = TypeDefinitionProvider::new();
        let uri = "file:///test.pl";

        let mut documents = std::collections::HashMap::new();
        documents.insert(uri.to_string(), code.to_string());

        // Line 10 (0-indexed: 10) is "my $obj = MyClass->new();"
        // Character position 10 should be around "MyClass"
        let line = 10;
        let character = 10;

        let locations = provider.find_type_definition(&ast, line, character, uri, &documents);

        // Debug: print what we found
        if let Some(ref locs) = locations {
            eprintln!("Found {} locations", locs.len());
            for loc in locs {
                eprintln!("Location: {:?}", loc);
            }
        } else {
            eprintln!("No locations found");

            // Debug: try to find what node we're getting
            // Use perl-parser-core for offset calculation
            let offset =
                perl_parser_core::engine::position::utf16_line_col_to_offset(code, line, character);
            eprintln!("Offset: {}", offset);
            if let Some(node) = provider.find_node_at_offset(&ast, offset) {
                eprintln!("Node kind: {:?}", node.kind);
                if let Some(type_name) = provider.extract_type_name(&node) {
                    eprintln!("Extracted type name: {}", type_name);
                } else {
                    eprintln!("Could not extract type name from node");
                }
            } else {
                eprintln!("Could not find node at offset");
            }
        }

        assert!(locations.is_some(), "Should find type definition for MyClass->new()");
        let locs = must_some(locations);
        assert_eq!(locs.len(), 1, "Should find exactly one definition");
    }
}
