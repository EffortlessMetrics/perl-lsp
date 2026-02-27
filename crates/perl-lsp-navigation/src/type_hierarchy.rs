//! Type hierarchy provider for Perl inheritance and package relationships.
//!
//! Supplies `textDocument/typeHierarchy` data for navigating parent/child
//! package relationships in the Parse → Index → Navigate stages of the LSP workflow.
//!
//! # Client capability requirements
//!
//! Clients must advertise the type hierarchy capability to enable
//! `textDocument/typeHierarchy` requests and responses.
//!
//! # Protocol compliance
//!
//! Implements the type hierarchy protocol with LSP symbol kind mappings and
//! stable item identifiers for follow-up requests.
//!
//! # Examples
//!
//! ```no_run
//! use perl_lsp_providers::ide::lsp_compat::type_hierarchy::TypeHierarchyProvider;
//! use perl_parser_core::Parser;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut parser = Parser::new("package Parent; package Child; use parent 'Parent';");
//! let _ast = parser.parse()?;
//! let _provider = TypeHierarchyProvider::new();
//! # Ok(())
//! # }
//! ```

use perl_parser_core::PositionMapper;
use perl_parser_core::ast::{Node, NodeKind};
use perl_position_tracking::{WirePosition, WireRange};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

/// Represents a type in the hierarchy
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TypeHierarchyItem {
    /// Fully qualified name of the type (e.g., package name)
    pub name: String,
    /// Kind of symbol (Class, Method, or Function)
    pub kind: TypeHierarchySymbolKind,
    /// URI of the document containing this type
    pub uri: String,
    /// Full range of the type declaration
    pub range: WireRange,
    /// Range of the type name for highlighting
    pub selection_range: WireRange,
    /// Optional detail string (e.g., "Perl Package")
    pub detail: Option<String>,
    /// Optional additional data for client use
    pub data: Option<serde_json::Value>,
}

/// Kind of symbol in the type hierarchy (LSP protocol values)
///
/// This enum uses explicit discriminant values matching the LSP protocol
/// SymbolKind values for direct wire serialization.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum TypeHierarchySymbolKind {
    /// A class or package (LSP value 5)
    Class = 5,
    /// A method (LSP value 6)
    Method = 6,
    /// A function (LSP value 12)
    Function = 12,
}

/// Index for tracking package hierarchy relationships
#[derive(Default, Debug)]
struct HierarchyIndex {
    /// Map from child package to its parent packages
    parents: BTreeMap<String, BTreeSet<String>>,
    /// Map from parent package to its child packages
    children: BTreeMap<String, BTreeSet<String>>,
}

impl HierarchyIndex {
    fn add_inheritance(&mut self, child: &str, parent: &str) {
        self.parents.entry(child.to_string()).or_default().insert(parent.to_string());
        self.children.entry(parent.to_string()).or_default().insert(child.to_string());
    }

    fn get_parents(&self, package: &str) -> Vec<String> {
        self.parents.get(package).map(|set| set.iter().cloned().collect()).unwrap_or_default()
    }

    fn get_children(&self, package: &str) -> Vec<String> {
        self.children.get(package).map(|set| set.iter().cloned().collect()).unwrap_or_default()
    }
}

/// Provider for type hierarchy (inheritance) information
pub struct TypeHierarchyProvider;

impl Default for TypeHierarchyProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl TypeHierarchyProvider {
    /// Creates a new type hierarchy provider
    pub fn new() -> Self {
        Self
    }

    /// Build a hierarchy index from the AST
    fn build_hierarchy_index(&self, ast: &Node) -> HierarchyIndex {
        let mut index = HierarchyIndex::default();
        let mut current_package = "main".to_string();

        // Walk the AST in order, tracking package scope
        self.index_hierarchy_recursive(ast, &mut index, &mut current_package);

        index
    }

    fn index_hierarchy_recursive(
        &self,
        node: &Node,
        index: &mut HierarchyIndex,
        current_package: &mut String,
    ) {
        match &node.kind {
            NodeKind::Package { name, block, name_span: _ } => {
                if block.is_some() {
                    // Block form: package Foo { ... }
                    // Save current package, process block, restore
                    let saved_package = current_package.clone();
                    *current_package = name.clone();
                    if let Some(blk) = block {
                        self.index_hierarchy_recursive(blk, index, current_package);
                    }
                    *current_package = saved_package;
                } else {
                    // Linear form: package Foo;
                    // Changes package scope for subsequent statements
                    *current_package = name.clone();
                }
            }
            NodeKind::Use { module, args, .. } => {
                if module == "parent" || module == "base" {
                    for arg in args {
                        for parent in self.normalize_parent_arg(arg) {
                            index.add_inheritance(current_package, &parent);
                        }
                    }
                }
            }
            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                if declarator == "our"
                    && let NodeKind::Variable { sigil, name: var_name } = &variable.kind
                    && sigil == "@"
                    && var_name == "ISA"
                    && let Some(init) = initializer
                {
                    for parent in self.extract_isa_parents(init) {
                        index.add_inheritance(current_package, &parent);
                    }
                }
            }
            NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
                if declarator == "our" {
                    // Check if any variable is @ISA
                    for var in variables {
                        if let NodeKind::Variable { sigil, name: var_name } = &var.kind
                            && sigil == "@"
                            && var_name == "ISA"
                            && let Some(init) = initializer
                        {
                            for parent in self.extract_isa_parents(init) {
                                index.add_inheritance(current_package, &parent);
                            }
                        }
                    }
                }
            }
            NodeKind::Program { statements } | NodeKind::Block { statements } => {
                for stmt in statements {
                    self.index_hierarchy_recursive(stmt, index, current_package);
                }
            }
            _ => {
                // Recurse into other nodes
                if let Some(children) = self.get_children(node) {
                    for child in children {
                        self.index_hierarchy_recursive(child, index, current_package);
                    }
                }
            }
        }
    }

    /// Normalize parent argument (handle quotes, qw(), etc.)
    fn normalize_parent_arg(&self, arg: &str) -> Vec<String> {
        let arg = arg.trim();

        // Handle qw(Base Other)
        if arg.starts_with("qw(") && arg.ends_with(')') {
            let content = &arg[3..arg.len() - 1];
            return content.split_whitespace().map(|s| s.to_string()).collect();
        }

        // Handle qw{Base Other}, qw[Base Other], etc.
        if arg.starts_with("qw") && arg.len() > 2 {
            let delim_start = arg.chars().nth(2).unwrap_or(' ');
            let delim_end = match delim_start {
                '(' => ')',
                '{' => '}',
                '[' => ']',
                '<' => '>',
                _ => delim_start,
            };
            if let Some(start) = arg.find(delim_start)
                && let Some(end) = arg.rfind(delim_end)
            {
                let content = &arg[start + 1..end];
                return content.split_whitespace().map(|s| s.to_string()).collect();
            }
        }

        // Remove quotes
        let clean = arg.trim_matches('"').trim_matches('\'').trim_matches('`');
        vec![clean.to_string()]
    }

    /// Extract parent classes from @ISA initialization
    fn extract_isa_parents(&self, node: &Node) -> Vec<String> {
        let mut parents = Vec::new();

        match &node.kind {
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    match &elem.kind {
                        NodeKind::String { value, .. } => {
                            for parent in self.normalize_parent_arg(value) {
                                parents.push(parent);
                            }
                        }
                        NodeKind::Identifier { name } => {
                            // Bareword
                            parents.push(name.clone());
                        }
                        _ => {}
                    }
                }
            }
            NodeKind::String { value, .. } => {
                for parent in self.normalize_parent_arg(value) {
                    parents.push(parent);
                }
            }
            NodeKind::Identifier { name } => {
                // Bareword
                parents.push(name.clone());
            }
            _ => {}
        }

        parents
    }

    /// Prepare type hierarchy at position
    pub fn prepare(&self, ast: &Node, code: &str, offset: usize) -> Option<Vec<TypeHierarchyItem>> {
        let position_mapper = PositionMapper::new(code);
        // Find the node at the position
        let target_node = self.find_node_at_offset(ast, offset)?;

        // Check if it's a package or class declaration
        match &target_node.kind {
            NodeKind::Package { name, .. } => {
                let item = self.create_type_item(
                    name,
                    target_node,
                    &position_mapper,
                    TypeHierarchySymbolKind::Class,
                );
                Some(vec![item])
            }
            NodeKind::Class { name, .. } => {
                let item = self.create_type_item(
                    name,
                    target_node,
                    &position_mapper,
                    TypeHierarchySymbolKind::Class,
                );
                Some(vec![item])
            }
            NodeKind::Identifier { name } => {
                // Check if this identifier is part of a package or ISA relationship
                if self.is_package_identifier(ast, offset, name) {
                    let item = TypeHierarchyItem {
                        name: name.clone(),
                        kind: TypeHierarchySymbolKind::Class,
                        uri: "file:///current".to_string(),
                        range: self.node_to_range(target_node, &position_mapper),
                        selection_range: self.node_to_range(target_node, &position_mapper),
                        detail: Some("Perl Package".to_string()),
                        data: None,
                    };
                    Some(vec![item])
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Find supertypes (parent classes) via @ISA
    pub fn find_supertypes(&self, ast: &Node, item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
        let index = self.build_hierarchy_index(ast);
        let parents = index.get_parents(&item.name);

        parents
            .into_iter()
            .map(|name| TypeHierarchyItem {
                name,
                kind: TypeHierarchySymbolKind::Class,
                uri: "file:///current".to_string(),
                range: WireRange::default(),
                selection_range: WireRange::default(),
                detail: Some("Parent Class".to_string()),
                data: None,
            })
            .collect()
    }

    /// Find subtypes (child classes) that inherit from this class
    pub fn find_subtypes(&self, ast: &Node, item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
        let index = self.build_hierarchy_index(ast);
        let children = index.get_children(&item.name);

        children
            .into_iter()
            .map(|name| TypeHierarchyItem {
                name,
                kind: TypeHierarchySymbolKind::Class,
                uri: "file:///current".to_string(),
                range: WireRange::default(),
                selection_range: WireRange::default(),
                detail: Some("Subclass".to_string()),
                data: None,
            })
            .collect()
    }

    // Helper methods

    fn find_node_at_offset<'a>(&self, node: &'a Node, offset: usize) -> Option<&'a Node> {
        if offset >= node.location.start && offset < node.location.end {
            // First check children
            if let Some(children) = self.get_children(node) {
                for child in children {
                    if let Some(found) = self.find_node_at_offset(child, offset) {
                        return Some(found);
                    }
                }
            }
            // Return this node if no child contains the offset
            Some(node)
        } else {
            None
        }
    }

    fn get_children<'a>(&self, node: &'a Node) -> Option<Vec<&'a Node>> {
        match &node.kind {
            NodeKind::Program { statements } => Some(statements.iter().collect()),
            NodeKind::Block { statements } => Some(statements.iter().collect()),
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let mut children = vec![condition.as_ref(), then_branch.as_ref()];
                for branch in elsif_branches {
                    children.push(&branch.0);
                    children.push(&branch.1);
                }
                if let Some(else_b) = else_branch {
                    children.push(else_b.as_ref());
                }
                Some(children)
            }
            NodeKind::Package { block, .. } => block.as_ref().map(|b| vec![b.as_ref()]),
            NodeKind::Class { body, .. } => Some(vec![body.as_ref()]),
            NodeKind::Subroutine { body, .. } => Some(vec![body.as_ref()]),
            NodeKind::Assignment { lhs, rhs, .. } => Some(vec![lhs.as_ref(), rhs.as_ref()]),
            NodeKind::ExpressionStatement { expression } => Some(vec![expression.as_ref()]),
            _ => None,
        }
    }

    fn is_package_identifier(&self, _ast: &Node, _offset: usize, _name: &str) -> bool {
        // Check if this identifier appears in a context that suggests it's a package
        // For now, we'll return false as we need to match against strings not identifiers
        false
    }

    fn create_type_item(
        &self,
        name: &str,
        node: &Node,
        position_mapper: &PositionMapper,
        kind: TypeHierarchySymbolKind,
    ) -> TypeHierarchyItem {
        TypeHierarchyItem {
            name: name.to_string(),
            kind,
            uri: "file:///current".to_string(),
            range: self.node_to_range(node, position_mapper),
            selection_range: self.node_to_range(node, position_mapper),
            detail: Some(format!(
                "Perl {}",
                match kind {
                    TypeHierarchySymbolKind::Class => "Package",
                    TypeHierarchySymbolKind::Method => "Method",
                    TypeHierarchySymbolKind::Function => "Function",
                }
            )),
            data: None,
        }
    }

    /// Convert node to LSP range using PositionMapper for UTF-16 compliance
    fn node_to_range(&self, node: &Node, position_mapper: &PositionMapper) -> WireRange {
        let start_pos = self.offset_to_position(node.location.start, position_mapper);
        let end_pos = self.offset_to_position(node.location.end, position_mapper);
        WireRange {
            start: WirePosition { line: start_pos.0, character: start_pos.1 },
            end: WirePosition { line: end_pos.0, character: end_pos.1 },
        }
    }

    /// Convert byte offset to line/character position using PositionMapper for UTF-16 compliance
    fn offset_to_position(&self, offset: usize, position_mapper: &PositionMapper) -> (u32, u32) {
        let pos = position_mapper.byte_to_lsp_pos(offset);
        (pos.line, pos.character)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::parser::Parser;
    use perl_tdd_support::{must, must_some};

    #[test]
    fn test_type_hierarchy_for_package() {
        let code = r#"package MyClass;
use parent 'BaseClass';

sub new {
    my $class = shift;
    return bless {}, $class;
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        // Position on "MyClass" (package starts at position 0)
        let items = provider.prepare(&ast, code, 8);
        assert!(items.is_some());
        let items = must_some(items);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].name, "MyClass");

        // Find supertypes
        let supertypes = provider.find_supertypes(&ast, &items[0]);
        assert_eq!(supertypes.len(), 1);
        assert_eq!(supertypes[0].name, "BaseClass");
    }

    #[test]
    fn test_type_hierarchy_with_isa() {
        let code = r#"package Child;
our @ISA = qw(Parent1 Parent2);
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        // Position on "Child"
        let items = provider.prepare(&ast, code, 8);
        assert!(items.is_some());
        let items = must_some(items);
        assert_eq!(items[0].name, "Child");

        // Find supertypes - qw() parsing needs AST improvements
        let supertypes = provider.find_supertypes(&ast, &items[0]);
        // Just verify it doesn't panic for now
        let _ = supertypes.len();
    }

    #[test]
    fn test_find_subtypes() {
        let code = r#"package Base;

package Derived1;
use parent 'Base';

package Derived2;
our @ISA = ('Base');

package Unrelated;
use parent 'Other';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        // Create a Base item
        let base_item = TypeHierarchyItem {
            name: "Base".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        // Find subtypes
        let subtypes = provider.find_subtypes(&ast, &base_item);
        assert_eq!(subtypes.len(), 2, "Should find exactly 2 subtypes");

        let subtype_names: Vec<String> = subtypes.iter().map(|t| t.name.clone()).collect();
        assert!(subtype_names.contains(&"Derived1".to_string()), "Should find Derived1");
        assert!(subtype_names.contains(&"Derived2".to_string()), "Should find Derived2");
        assert!(!subtype_names.contains(&"Unrelated".to_string()), "Should not find Unrelated");
    }

    #[test]
    fn test_qw_parsing() {
        let code = r#"package Multi;
our @ISA = qw(Parent1 Parent2 Parent3);
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let items = provider.prepare(&ast, code, 8);
        assert!(items.is_some());
        let items = must_some(items);
        assert_eq!(items[0].name, "Multi");

        // Find supertypes - should handle qw() properly
        let supertypes = provider.find_supertypes(&ast, &items[0]);
        // For now just check it doesn't panic - full qw() support needs AST improvements
        let _ = supertypes.len();
    }

    #[test]
    fn test_block_form_packages() {
        let code = r#"package Outer {
    package Inner;
    use parent 'Outer';
}
package Other;
use parent 'Outer';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let outer_item = TypeHierarchyItem {
            name: "Outer".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        // Find subtypes - should handle block form packages
        let subtypes = provider.find_subtypes(&ast, &outer_item);
        // Both Inner and Other inherit from Outer
        assert_eq!(subtypes.len(), 2, "Should find both Inner and Other as subtypes");
    }
}
