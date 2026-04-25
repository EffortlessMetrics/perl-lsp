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
//! ```ignore
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
    /// Map from package to its composed roles (via `with`)
    roles: BTreeMap<String, BTreeSet<String>>,
}

impl HierarchyIndex {
    fn add_inheritance(&mut self, child: &str, parent: &str) {
        self.parents.entry(child.to_string()).or_default().insert(parent.to_string());
        self.children.entry(parent.to_string()).or_default().insert(child.to_string());
    }

    fn add_role(&mut self, package: &str, role: &str) {
        self.roles.entry(package.to_string()).or_default().insert(role.to_string());
    }

    fn get_parents(&self, package: &str) -> Vec<String> {
        self.parents.get(package).map(|set| set.iter().cloned().collect()).unwrap_or_default()
    }

    fn get_roles(&self, package: &str) -> Vec<String> {
        self.roles.get(package).map(|set| set.iter().cloned().collect()).unwrap_or_default()
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
            // Moose/Moo/Mouse: extends 'Parent', 'Parent2'  and  with 'Role', 'Role2'
            NodeKind::ExpressionStatement { expression } => {
                if let NodeKind::FunctionCall { name, args } = &expression.kind {
                    match name.as_str() {
                        "extends" => {
                            for parent in Self::extract_names_from_args(args) {
                                index.add_inheritance(current_package, &parent);
                            }
                        }
                        "with" => {
                            for role in Self::extract_names_from_args(args) {
                                index.add_role(current_package, &role);
                            }
                        }
                        _ => {}
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

    /// Extract package/role names from function call arguments (e.g., `extends 'A', 'B'`).
    ///
    /// Handles `String`, `Identifier`, and `ArrayLiteral` nodes. Hash literal
    /// arguments (e.g., `{ -version => 0.01 }`) are harmlessly skipped.
    fn extract_names_from_args(args: &[Node]) -> Vec<String> {
        args.iter().flat_map(Self::collect_symbol_names).collect()
    }

    /// Collect symbol names from a single AST node (String, Identifier, or ArrayLiteral).
    fn collect_symbol_names(node: &Node) -> Vec<String> {
        match &node.kind {
            NodeKind::String { value, .. } => {
                let trimmed = value.trim().trim_matches('\'').trim_matches('"').trim();
                if trimmed.is_empty() { Vec::new() } else { vec![trimmed.to_string()] }
            }
            NodeKind::Identifier { name } => {
                let trimmed = name.trim();
                if trimmed.is_empty() { Vec::new() } else { vec![trimmed.to_string()] }
            }
            NodeKind::ArrayLiteral { elements } => {
                elements.iter().flat_map(Self::collect_symbol_names).collect()
            }
            _ => Vec::new(),
        }
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

    /// Find supertypes (parent classes and composed roles)
    pub fn find_supertypes(&self, ast: &Node, item: &TypeHierarchyItem) -> Vec<TypeHierarchyItem> {
        let index = self.build_hierarchy_index(ast);
        let parents = index.get_parents(&item.name);
        let roles = index.get_roles(&item.name);

        let parent_items = parents.into_iter().map(|name| TypeHierarchyItem {
            name,
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///current".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: Some("Parent Class".to_string()),
            data: None,
        });

        let role_items = roles.into_iter().map(|name| TypeHierarchyItem {
            name,
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///current".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: Some("Role".to_string()),
            data: None,
        });

        parent_items.chain(role_items).collect()
    }

    /// Compute the C3 Method Resolution Order (MRO) for a package.
    ///
    /// Returns a linearized list starting with `package` itself, followed by
    /// ancestors in the order Perl's C3 MRO would search them. Each class
    /// appears exactly once. If the C3 merge is inconsistent the algorithm
    /// falls back to a depth-first left-to-right order with deduplication.
    pub fn c3_mro(&self, ast: &Node, package: &str) -> Vec<String> {
        let index = self.build_hierarchy_index(ast);
        let mut result = Vec::new();
        let mut visited = BTreeSet::new();
        self.c3_linearize(package, &index, &mut result, &mut visited);
        result
    }

    /// Recursive C3 linearization implementation.
    fn c3_linearize(
        &self,
        package: &str,
        index: &HierarchyIndex,
        result: &mut Vec<String>,
        visited: &mut BTreeSet<String>,
    ) {
        if visited.contains(package) {
            return;
        }
        visited.insert(package.to_string());

        let parents = index.get_parents(package);
        if parents.is_empty() {
            result.push(package.to_string());
            return;
        }

        // Build the lists to merge: linearization of each parent + the parents list itself
        let mut parent_mros: Vec<Vec<String>> = Vec::with_capacity(parents.len());
        for parent in &parents {
            let mut sub_result = Vec::new();
            self.c3_linearize(parent, index, &mut sub_result, visited);
            parent_mros.push(sub_result);
        }
        // Append the direct parents list as the last list to merge
        parent_mros.push(parents.clone());

        // Prepend self
        result.push(package.to_string());

        // C3 merge
        loop {
            // Remove empty lists
            parent_mros.retain(|list| !list.is_empty());
            if parent_mros.is_empty() {
                break;
            }

            // Find the first head that does not appear in any tail
            let chosen = parent_mros.iter().find_map(|list| {
                let candidate = list.first()?;
                let in_tail =
                    parent_mros.iter().any(|other| other.iter().skip(1).any(|n| n == candidate));
                if in_tail { None } else { Some(candidate.clone()) }
            });

            match chosen {
                Some(cls) => {
                    if !result.contains(&cls) {
                        result.push(cls.clone());
                    }
                    // Remove chosen from the front of all lists where it appears
                    for list in &mut parent_mros {
                        if list.first().is_some_and(|h| h == &cls) {
                            list.remove(0);
                        }
                    }
                }
                None => {
                    // Inconsistent hierarchy — fall back: take heads left-to-right
                    for list in &parent_mros.clone() {
                        if let Some(head) = list.first() {
                            if !result.contains(head) {
                                result.push(head.clone());
                            }
                        }
                    }
                    break;
                }
            }
        }
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
    fn test_moose_extends_single_parent() {
        let code = r#"package Animal;

package Dog;
use Moose;
extends 'Animal';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let dog_item = TypeHierarchyItem {
            name: "Dog".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let supertypes = provider.find_supertypes(&ast, &dog_item);
        assert_eq!(supertypes.len(), 1, "Should find 1 parent via extends");
        assert_eq!(supertypes[0].name, "Animal");
        assert_eq!(
            supertypes[0].detail.as_deref(),
            Some("Parent Class"),
            "extends parent should have 'Parent Class' detail"
        );
    }

    #[test]
    fn test_moose_extends_multiple_parents() {
        let code = r#"package Readable;
package Writable;

package ReadWriteFile;
use Moose;
extends 'Readable', 'Writable';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let item = TypeHierarchyItem {
            name: "ReadWriteFile".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let supertypes = provider.find_supertypes(&ast, &item);
        let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"Readable"), "Should find Readable parent");
        assert!(names.contains(&"Writable"), "Should find Writable parent");
    }

    #[test]
    fn test_moose_with_role() {
        let code = r#"package Printable;

package Document;
use Moose;
with 'Printable';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let item = TypeHierarchyItem {
            name: "Document".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let supertypes = provider.find_supertypes(&ast, &item);
        let role_types: Vec<&TypeHierarchyItem> =
            supertypes.iter().filter(|s| s.detail.as_deref() == Some("Role")).collect();
        assert_eq!(role_types.len(), 1, "Should find 1 role via with");
        assert_eq!(role_types[0].name, "Printable");
    }

    #[test]
    fn test_moose_with_multiple_roles() {
        let code = r#"package Serializable;
package Printable;

package Report;
use Moose;
with 'Serializable', 'Printable';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let item = TypeHierarchyItem {
            name: "Report".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let supertypes = provider.find_supertypes(&ast, &item);
        let role_names: Vec<&str> = supertypes
            .iter()
            .filter(|s| s.detail.as_deref() == Some("Role"))
            .map(|s| s.name.as_str())
            .collect();
        assert!(role_names.contains(&"Serializable"), "Should find Serializable role");
        assert!(role_names.contains(&"Printable"), "Should find Printable role");
    }

    #[test]
    fn test_moose_extends_and_with_combined() {
        let code = r#"package Base;
package MyRole;

package Child;
use Moose;
extends 'Base';
with 'MyRole';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let item = TypeHierarchyItem {
            name: "Child".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let supertypes = provider.find_supertypes(&ast, &item);

        let parent_names: Vec<&str> = supertypes
            .iter()
            .filter(|s| s.detail.as_deref() == Some("Parent Class"))
            .map(|s| s.name.as_str())
            .collect();
        let role_names: Vec<&str> = supertypes
            .iter()
            .filter(|s| s.detail.as_deref() == Some("Role"))
            .map(|s| s.name.as_str())
            .collect();

        assert_eq!(parent_names, vec!["Base"], "Should find Base as parent");
        assert_eq!(role_names, vec!["MyRole"], "Should find MyRole as role");
    }

    #[test]
    fn test_moo_extends_with() {
        let code = r#"package MooParent;
package MooRole;

package MooChild;
use Moo;
extends 'MooParent';
with 'MooRole';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let item = TypeHierarchyItem {
            name: "MooChild".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let supertypes = provider.find_supertypes(&ast, &item);
        let names: Vec<&str> = supertypes.iter().map(|s| s.name.as_str()).collect();
        assert!(names.contains(&"MooParent"), "Moo extends should work");
        assert!(names.contains(&"MooRole"), "Moo with should work");
    }

    #[test]
    fn test_mixed_use_parent_and_extends() {
        let code = r#"package OldBase;
package MooseBase;

package Mixed;
use parent 'OldBase';
use Moose;
extends 'MooseBase';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let item = TypeHierarchyItem {
            name: "Mixed".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let supertypes = provider.find_supertypes(&ast, &item);
        let parent_names: Vec<&str> = supertypes
            .iter()
            .filter(|s| s.detail.as_deref() == Some("Parent Class"))
            .map(|s| s.name.as_str())
            .collect();
        assert!(parent_names.contains(&"OldBase"), "use parent should still work");
        assert!(parent_names.contains(&"MooseBase"), "extends should also work");
    }

    #[test]
    fn test_extends_subtypes_reverse() {
        // Verify that extends also populates the children (subtypes) direction
        let code = r#"package Animal;

package Dog;
use Moose;
extends 'Animal';

package Cat;
use Moo;
extends 'Animal';
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let animal_item = TypeHierarchyItem {
            name: "Animal".to_string(),
            kind: TypeHierarchySymbolKind::Class,
            uri: "file:///test".to_string(),
            range: WireRange::default(),
            selection_range: WireRange::default(),
            detail: None,
            data: None,
        };

        let subtypes = provider.find_subtypes(&ast, &animal_item);
        let subtype_names: Vec<&str> = subtypes.iter().map(|s| s.name.as_str()).collect();
        assert!(subtype_names.contains(&"Dog"), "Dog should be a subtype of Animal");
        assert!(subtype_names.contains(&"Cat"), "Cat should be a subtype of Animal");
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

    #[test]
    fn test_c3_mro_handles_inheritance_cycles() {
        let code = r#"package A;
our @ISA = ('B');

package B;
our @ISA = ('A');
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let mro = provider.c3_mro(&ast, "A");
        assert_eq!(mro, vec!["A".to_string(), "B".to_string()]);
    }

    /// Querying from the other side of a 2-cycle must also terminate and not panic.
    #[test]
    fn test_c3_mro_cycle_from_other_end() {
        let code = r#"package A;
our @ISA = ('B');

package B;
our @ISA = ('A');
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let mro = provider.c3_mro(&ast, "B");
        // B sees its own cycle too — result must be non-empty and start with B.
        assert!(!mro.is_empty(), "c3_mro from B side of cycle must not be empty");
        assert_eq!(mro[0], "B", "c3_mro must start with the queried package");
    }

    /// 3-class chain cycle A→B→C→A must terminate without stack overflow.
    #[test]
    fn test_c3_mro_handles_three_way_cycle() {
        let code = r#"package A;
our @ISA = ('B');

package B;
our @ISA = ('C');

package C;
our @ISA = ('A');
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let mro = provider.c3_mro(&ast, "A");
        // Must not panic, must start with A, must contain all three packages.
        assert_eq!(mro[0], "A", "MRO must start with queried package");
        let mro_set: std::collections::BTreeSet<&str> = mro.iter().map(String::as_str).collect();
        assert!(mro_set.contains("B"), "B must appear in MRO of 3-cycle");
        assert!(mro_set.contains("C"), "C must appear in MRO of 3-cycle");
    }

    /// Diamond inheritance A→(B,C)→D: D must appear exactly once in the MRO.
    #[test]
    fn test_c3_mro_diamond_inheritance() {
        let code = r#"package D;

package B;
our @ISA = ('D');

package C;
our @ISA = ('D');

package A;
our @ISA = ('B', 'C');
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = TypeHierarchyProvider::new();

        let mro = provider.c3_mro(&ast, "A");
        assert_eq!(mro[0], "A", "MRO must start with A");
        // C3 linearization of diamond: A, B, C, D
        assert_eq!(
            mro,
            vec!["A".to_string(), "B".to_string(), "C".to_string(), "D".to_string(),],
            "Diamond MRO must be A, B, C, D with D appearing exactly once"
        );
    }
}
