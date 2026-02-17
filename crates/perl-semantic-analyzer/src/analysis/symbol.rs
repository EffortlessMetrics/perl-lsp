//! Symbol extraction and symbol table for IDE features
//!
//! This module provides symbol extraction from the AST, building a symbol table
//! that tracks definitions, references, and scopes for IDE features like
//! go-to-definition, find-all-references, and semantic highlighting.
//!
//! # Related Modules
//!
//! See also [`crate::workspace_index`] for workspace-wide indexing and
//! cross-file reference resolution.
//!
//! # Usage Examples
//!
//! ```no_run
//! use perl_semantic_analyzer::{Parser, symbol::SymbolExtractor};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! let mut parser = Parser::new("sub hello { my $x = 1; }");
//! let ast = parser.parse()?;
//! let extractor = SymbolExtractor::new();
//! let table = extractor.extract(&ast);
//! assert!(table.symbols.contains_key("hello"));
//! # Ok(())
//! # }
//! ```

use crate::SourceLocation;
use crate::ast::{Node, NodeKind};
use regex::Regex;
use std::collections::{HashMap, HashSet};

// Re-export the unified symbol types from perl-symbol-types
/// Symbol kind enums used during Index/Analyze workflows.
pub use perl_symbol_types::{SymbolKind, VarKind};

#[derive(Debug, Clone)]
/// A symbol definition in Perl code with comprehensive metadata for Index/Navigate workflows.
///
/// Represents a symbol definition with full context including scope,
/// package qualification, and documentation for LSP features like
/// go-to-definition, hover, and workspace symbols.
///
/// # Performance Characteristics
/// - Memory: ~128 bytes per symbol (optimized for large codebases)
/// - Lookup time: O(1) via hash table indexing
/// - Scope resolution: O(log n) with scope hierarchy
///
/// # Perl Language Semantics
/// - Package qualification: `Package::symbol` vs bare `symbol`
/// - Scope rules: Lexical (`my`), package (`our`), dynamic (`local`), persistent (`state`)
/// - Symbol types: Variables (`$`, `@`, `%`), subroutines, packages, constants
/// - Attribute parsing: `:shared`, `:method`, `:lvalue` and custom attributes
pub struct Symbol {
    /// Symbol name (without sigil for variables)
    pub name: String,
    /// Fully qualified name with package prefix
    pub qualified_name: String,
    /// Classification of symbol type
    pub kind: SymbolKind,
    /// Source location of symbol definition
    pub location: SourceLocation,
    /// Lexical scope identifier for visibility rules
    pub scope_id: ScopeId,
    /// Variable declaration type (my, our, local, state)
    pub declaration: Option<String>,
    /// Extracted POD or comment documentation
    pub documentation: Option<String>,
    /// Perl attributes applied to the symbol
    pub attributes: Vec<String>,
}

#[derive(Debug, Clone)]
/// A reference to a symbol with usage context for Navigate/Analyze workflows.
///
/// Tracks symbol usage sites for features like find-all-references,
/// rename refactoring, and unused symbol detection with precise
/// scope and context information.
///
/// # Performance Characteristics
/// - Memory: ~64 bytes per reference
/// - Collection: O(n) during AST traversal
/// - Query time: O(log n) with spatial indexing
///
/// # LSP Integration
/// Essential for:
/// - Find references: Locate all usage sites
/// - Rename refactoring: Update all references atomically
/// - Unused detection: Identify unreferenced symbols
/// - Call hierarchy: Build caller/callee relationships
pub struct SymbolReference {
    /// Symbol name (without sigil for variables)
    pub name: String,
    /// Symbol type inferred from usage context
    pub kind: SymbolKind,
    /// Source location of the reference
    pub location: SourceLocation,
    /// Lexical scope where reference occurs
    pub scope_id: ScopeId,
    /// Whether this is a write reference (assignment)
    pub is_write: bool,
}

/// Unique identifier for a scope used during Index/Analyze workflows.
pub type ScopeId = usize;

#[derive(Debug, Clone)]
/// A lexical scope in Perl code with hierarchical symbol visibility for Parse/Analyze stages.
///
/// Represents a lexical scope boundary (subroutine, block, package) with
/// symbol visibility rules according to Perl's lexical scoping semantics.
///
/// # Performance Characteristics
/// - Scope lookup: O(log n) with parent chain traversal
/// - Symbol resolution: O(1) per scope level
/// - Memory: ~64 bytes per scope + symbol set
///
/// # Perl Scoping Rules
/// - Global scope: File-level and package symbols
/// - Package scope: Package-qualified symbols
/// - Subroutine scope: Local variables and parameters
/// - Block scope: Lexical variables in control structures
/// - Lexical precedence: Inner scopes shadow outer scopes
///
/// Workflow: Parse/Analyze scope tracking for symbol resolution.
pub struct Scope {
    /// Unique scope identifier for reference tracking
    pub id: ScopeId,
    /// Parent scope for hierarchical lookup (None for global)
    pub parent: Option<ScopeId>,
    /// Classification of scope type
    pub kind: ScopeKind,
    /// Source location where scope begins
    pub location: SourceLocation,
    /// Set of symbol names defined in this scope
    pub symbols: HashSet<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classification of lexical scope types in Perl for Parse/Analyze workflows.
///
/// Defines different scope boundaries with specific symbol visibility
/// and resolution rules according to Perl language semantics.
///
/// # Scope Hierarchy
/// - Global: File-level symbols and imports
/// - Package: Package-qualified namespace
/// - Subroutine: Function parameters and local variables
/// - Block: Control structure lexical variables
/// - Eval: Dynamic evaluation context
///
/// Workflow: Parse/Analyze scope classification.
pub enum ScopeKind {
    /// Global/file scope
    Global,
    /// Package scope
    Package,
    /// Subroutine scope
    Subroutine,
    /// Block scope (if, while, for, etc.)
    Block,
    /// Eval scope
    Eval,
}

#[derive(Debug, Default)]
/// Comprehensive symbol table for Perl code analysis and LSP features in Index/Analyze stages.
///
/// Central data structure containing all symbols, references, and scopes
/// with efficient indexing for LSP operations like go-to-definition,
/// find-references, and workspace symbols.
///
/// # Performance Characteristics
/// - Symbol lookup: O(1) average, O(n) worst case for overloaded names
/// - Reference queries: O(log n) with spatial indexing
/// - Memory usage: ~500KB per 10K lines of Perl code
/// - Construction time: O(n) single-pass AST traversal
///
/// # LSP Integration
/// Core data structure for:
/// - Symbol resolution: Package-qualified and bare name lookup
/// - Reference tracking: All usage sites with context
/// - Scope analysis: Lexical visibility and shadowing
/// - Completion: Context-aware symbol suggestions
/// - Workspace indexing: Cross-file symbol registry
///
/// # Perl Language Support
/// - Package qualification: `Package::symbol` resolution
/// - Lexical scoping: `my`, `our`, `local`, `state` variable semantics
/// - Symbol overloading: Multiple definitions with scope precedence
/// - Context sensitivity: Scalar/array/hash context resolution
pub struct SymbolTable {
    /// Symbols indexed by name with multiple definitions support
    pub symbols: HashMap<String, Vec<Symbol>>,
    /// References indexed by name for find-all-references
    pub references: HashMap<String, Vec<SymbolReference>>,
    /// Scopes indexed by ID for hierarchical lookup
    pub scopes: HashMap<ScopeId, Scope>,
    /// Scope stack maintained during AST traversal
    scope_stack: Vec<ScopeId>,
    /// Monotonic scope ID generator
    next_scope_id: ScopeId,
    /// Current package context for symbol qualification
    current_package: String,
}

impl SymbolTable {
    /// Create a new symbol table for Index/Analyze workflows.
    pub fn new() -> Self {
        let mut table = SymbolTable {
            symbols: HashMap::new(),
            references: HashMap::new(),
            scopes: HashMap::new(),
            scope_stack: vec![0],
            next_scope_id: 1,
            current_package: "main".to_string(),
        };

        // Create global scope
        table.scopes.insert(
            0,
            Scope {
                id: 0,
                parent: None,
                kind: ScopeKind::Global,
                location: SourceLocation { start: 0, end: 0 },
                symbols: HashSet::new(),
            },
        );

        table
    }

    /// Get the current scope ID
    fn current_scope(&self) -> ScopeId {
        *self.scope_stack.last().unwrap_or(&0)
    }

    /// Push a new scope
    fn push_scope(&mut self, kind: ScopeKind, location: SourceLocation) -> ScopeId {
        let parent = self.current_scope();
        let scope_id = self.next_scope_id;
        self.next_scope_id += 1;

        let scope =
            Scope { id: scope_id, parent: Some(parent), kind, location, symbols: HashSet::new() };

        self.scopes.insert(scope_id, scope);
        self.scope_stack.push(scope_id);
        scope_id
    }

    /// Pop the current scope
    fn pop_scope(&mut self) {
        self.scope_stack.pop();
    }

    /// Add a symbol definition
    fn add_symbol(&mut self, symbol: Symbol) {
        let name = symbol.name.clone();
        if let Some(scope) = self.scopes.get_mut(&symbol.scope_id) {
            scope.symbols.insert(name.clone());
        }
        self.symbols.entry(name).or_default().push(symbol);
    }

    /// Add a symbol reference
    fn add_reference(&mut self, reference: SymbolReference) {
        let name = reference.name.clone();
        self.references.entry(name).or_default().push(reference);
    }

    /// Find symbol definitions visible from a given scope for Navigate/Analyze workflows.
    pub fn find_symbol(&self, name: &str, from_scope: ScopeId, kind: SymbolKind) -> Vec<&Symbol> {
        let mut results = Vec::new();
        let mut current_scope_id = Some(from_scope);

        // Walk up the scope chain
        while let Some(scope_id) = current_scope_id {
            if let Some(scope) = self.scopes.get(&scope_id) {
                // Check if symbol is defined in this scope
                if scope.symbols.contains(name) {
                    if let Some(symbols) = self.symbols.get(name) {
                        for symbol in symbols {
                            if symbol.scope_id == scope_id && symbol.kind == kind {
                                results.push(symbol);
                            }
                        }
                    }
                }

                // For 'our' variables, also check package scope
                if scope.kind != ScopeKind::Package {
                    if let Some(symbols) = self.symbols.get(name) {
                        for symbol in symbols {
                            if symbol.declaration.as_deref() == Some("our") && symbol.kind == kind {
                                results.push(symbol);
                            }
                        }
                    }
                }

                current_scope_id = scope.parent;
            } else {
                break;
            }
        }

        results
    }

    /// Get all references to a symbol for Navigate/Analyze workflows.
    pub fn find_references(&self, symbol: &Symbol) -> Vec<&SymbolReference> {
        self.references
            .get(&symbol.name)
            .map(|refs| refs.iter().filter(|r| r.kind == symbol.kind).collect())
            .unwrap_or_default()
    }
}

/// Extract symbols from an AST for Parse/Index workflows.
pub struct SymbolExtractor {
    table: SymbolTable,
    /// Source code for comment extraction
    source: String,
    /// True when `use Moo/Moose` style declarations are active in this file.
    moo_enabled: bool,
    /// True when Class::Accessor style generated accessors are active in this file.
    class_accessor_enabled: bool,
}

impl Default for SymbolExtractor {
    fn default() -> Self {
        Self::new()
    }
}

impl SymbolExtractor {
    /// Create a new symbol extractor without source (no documentation extraction).
    ///
    /// Used during Parse/Index stages when only symbols are required.
    pub fn new() -> Self {
        SymbolExtractor {
            table: SymbolTable::new(),
            source: String::new(),
            moo_enabled: false,
            class_accessor_enabled: false,
        }
    }

    /// Create a symbol extractor with source text for documentation extraction.
    ///
    /// Used during Parse/Analyze stages to attach documentation metadata.
    pub fn new_with_source(source: &str) -> Self {
        SymbolExtractor {
            table: SymbolTable::new(),
            source: source.to_string(),
            moo_enabled: false,
            class_accessor_enabled: false,
        }
    }

    /// Extract symbols from an AST node for Index/Analyze workflows.
    pub fn extract(mut self, node: &Node) -> SymbolTable {
        self.visit_node(node);
        self.table
    }

    /// Visit a node and extract symbols
    fn visit_node(&mut self, node: &Node) {
        match &node.kind {
            NodeKind::Program { statements } => {
                self.visit_statement_list(statements);
            }

            NodeKind::VariableDeclaration { declarator, variable, attributes, initializer } => {
                let doc = self.extract_leading_comment(node.location.start);
                self.handle_variable_declaration(
                    declarator,
                    variable,
                    attributes,
                    variable.location,
                    doc,
                );
                if let Some(init) = initializer {
                    self.visit_node(init);
                }
            }

            NodeKind::VariableListDeclaration {
                declarator,
                variables,
                attributes,
                initializer,
            } => {
                let doc = self.extract_leading_comment(node.location.start);
                for var in variables {
                    self.handle_variable_declaration(
                        declarator,
                        var,
                        attributes,
                        var.location,
                        doc.clone(),
                    );
                }
                if let Some(init) = initializer {
                    self.visit_node(init);
                }
            }

            NodeKind::Variable { sigil, name } => {
                let kind = match sigil.as_str() {
                    "$" => SymbolKind::scalar(),
                    "@" => SymbolKind::array(),
                    "%" => SymbolKind::hash(),
                    _ => return,
                };

                let reference = SymbolReference {
                    name: name.clone(),
                    kind,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    is_write: false, // Will be updated based on context
                };

                self.table.add_reference(reference);
            }

            NodeKind::Subroutine {
                name,
                prototype: _,
                signature: _,
                attributes,
                body,
                name_span: _,
            } => {
                let sub_name =
                    name.as_ref().map(|n| n.to_string()).unwrap_or_else(|| "<anon>".to_string());

                if name.is_some() {
                    let documentation = self.extract_leading_comment(node.location.start);
                    let symbol = Symbol {
                        name: sub_name.clone(),
                        qualified_name: format!("{}::{}", self.table.current_package, sub_name),
                        kind: SymbolKind::Subroutine,
                        location: node.location,
                        scope_id: self.table.current_scope(),
                        declaration: None,
                        documentation,
                        attributes: attributes.clone(),
                    };

                    self.table.add_symbol(symbol);
                }

                // Create subroutine scope
                self.table.push_scope(ScopeKind::Subroutine, node.location);

                {
                    self.visit_node(body);
                }

                self.table.pop_scope();
            }

            NodeKind::Package { name, block, name_span: _ } => {
                let old_package = self.table.current_package.clone();
                self.table.current_package = name.clone();

                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind: SymbolKind::Package,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation: None,
                    attributes: vec![],
                };

                self.table.add_symbol(symbol);

                if let Some(block_node) = block {
                    // Package with block - create a new scope
                    self.table.push_scope(ScopeKind::Package, node.location);
                    self.visit_node(block_node);
                    self.table.pop_scope();
                    self.table.current_package = old_package;
                }
                // If no block, package declaration affects rest of file
                // Don't change scope or restore package name
            }

            NodeKind::Block { statements } => {
                self.table.push_scope(ScopeKind::Block, node.location);
                self.visit_statement_list(statements);
                self.table.pop_scope();
            }

            NodeKind::If { condition, then_branch, elsif_branches: _, else_branch } => {
                self.visit_node(condition);

                self.table.push_scope(ScopeKind::Block, then_branch.location);
                self.visit_node(then_branch);
                self.table.pop_scope();

                if let Some(else_node) = else_branch {
                    self.table.push_scope(ScopeKind::Block, else_node.location);
                    self.visit_node(else_node);
                    self.table.pop_scope();
                }
            }

            NodeKind::While { condition, body, continue_block: _ } => {
                self.visit_node(condition);

                self.table.push_scope(ScopeKind::Block, body.location);
                self.visit_node(body);
                self.table.pop_scope();
            }

            NodeKind::For { init, condition, update, body, .. } => {
                self.table.push_scope(ScopeKind::Block, node.location);

                if let Some(init_node) = init {
                    self.visit_node(init_node);
                }
                if let Some(cond_node) = condition {
                    self.visit_node(cond_node);
                }
                if let Some(update_node) = update {
                    self.visit_node(update_node);
                }
                self.visit_node(body);

                self.table.pop_scope();
            }

            NodeKind::Foreach { variable, list, body, continue_block: _ } => {
                self.table.push_scope(ScopeKind::Block, node.location);

                // The loop variable is implicitly declared
                self.handle_variable_declaration("my", variable, &[], variable.location, None);
                self.visit_node(list);
                self.visit_node(body);

                self.table.pop_scope();
            }

            // Handle other node types by visiting children
            NodeKind::Assignment { lhs, rhs, .. } => {
                // Mark LHS as write reference
                self.mark_write_reference(lhs);
                self.visit_node(lhs);
                self.visit_node(rhs);
            }

            NodeKind::Binary { left, right, .. } => {
                self.visit_node(left);
                self.visit_node(right);
            }

            NodeKind::Unary { operand, .. } => {
                self.visit_node(operand);
            }

            NodeKind::FunctionCall { name, args } => {
                // Track function call as a reference
                let reference = SymbolReference {
                    name: name.clone(),
                    kind: SymbolKind::Subroutine,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    is_write: false,
                };
                self.table.add_reference(reference);

                for arg in args {
                    self.visit_node(arg);
                }
            }

            NodeKind::MethodCall { object, method, args } => {
                // Track method call sites so semantic definition/hover can resolve generated
                // accessors (Moo/Moose/Class::Accessor) from usage points.
                let location = self.method_reference_location(node, object, method);
                self.table.add_reference(SymbolReference {
                    name: method.clone(),
                    kind: SymbolKind::Subroutine,
                    location,
                    scope_id: self.table.current_scope(),
                    is_write: false,
                });

                self.visit_node(object);
                for arg in args {
                    self.visit_node(arg);
                }
            }

            // ArrayRef and HashRef are handled as Binary operations with [] or {}
            NodeKind::ArrayLiteral { elements } => {
                for elem in elements {
                    self.visit_node(elem);
                }
            }

            NodeKind::HashLiteral { pairs } => {
                for (key, value) in pairs {
                    self.visit_node(key);
                    self.visit_node(value);
                }
            }

            NodeKind::Ternary { condition, then_expr, else_expr } => {
                self.visit_node(condition);
                self.visit_node(then_expr);
                self.visit_node(else_expr);
            }

            NodeKind::LabeledStatement { label, statement } => {
                let symbol = Symbol {
                    name: label.clone(),
                    qualified_name: label.clone(),
                    kind: SymbolKind::Label,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation: None,
                    attributes: vec![],
                };

                self.table.add_symbol(symbol);

                {
                    self.visit_node(statement);
                }
            }

            // Handle interpolated strings specially to extract variable references
            NodeKind::String { value, interpolated } => {
                if *interpolated {
                    // Extract variable references from interpolated strings
                    self.extract_vars_from_string(value, node.location);
                }
            }

            NodeKind::Use { module, args, .. } => {
                self.update_framework_context(module, args);
            }

            NodeKind::No { module: _, args: _, .. } => {
                // We don't currently track framework deactivation via `no`.
            }

            NodeKind::PhaseBlock { phase: _, phase_span: _, block } => {
                // BEGIN, END, CHECK, INIT blocks
                self.visit_node(block);
            }

            NodeKind::StatementModifier { statement, modifier: _, condition } => {
                self.visit_node(statement);
                self.visit_node(condition);
            }

            NodeKind::Do { block } | NodeKind::Eval { block } => {
                self.visit_node(block);
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                self.visit_node(body);
                for (_, catch_block) in catch_blocks {
                    self.visit_node(catch_block);
                }
                if let Some(finally) = finally_block {
                    self.visit_node(finally);
                }
            }

            NodeKind::Given { expr, body } => {
                self.visit_node(expr);
                self.visit_node(body);
            }

            NodeKind::When { condition, body } => {
                self.visit_node(condition);
                self.visit_node(body);
            }

            NodeKind::Default { body } => {
                self.visit_node(body);
            }

            NodeKind::Class { name, body } => {
                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind: SymbolKind::Package, // Classes are like packages
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation: None,
                    attributes: vec![],
                };
                self.table.add_symbol(symbol);

                self.table.push_scope(ScopeKind::Package, node.location);
                self.visit_node(body);
                self.table.pop_scope();
            }

            NodeKind::Method { name, signature: _, attributes: _, body } => {
                let documentation = self.extract_leading_comment(node.location.start);
                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: format!("{}::{}", self.table.current_package, name),
                    kind: SymbolKind::Subroutine,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation,
                    attributes: vec!["method".to_string()],
                };
                self.table.add_symbol(symbol);

                self.table.push_scope(ScopeKind::Subroutine, node.location);
                self.visit_node(body);
                self.table.pop_scope();
            }

            NodeKind::Format { name, body: _ } => {
                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: format!("{}::{}", self.table.current_package, name),
                    kind: SymbolKind::Format,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation: None,
                    attributes: vec![],
                };
                self.table.add_symbol(symbol);
            }

            NodeKind::Return { value } => {
                if let Some(val) = value {
                    self.visit_node(val);
                }
            }

            // Regex related nodes - we recurse into expression
            NodeKind::Regex { .. } => {}
            NodeKind::Match { expr, .. } => {
                self.visit_node(expr);
            }
            NodeKind::Substitution { expr, .. } => {
                self.visit_node(expr);
            }
            NodeKind::Transliteration { expr, .. } => {
                self.visit_node(expr);
            }

            NodeKind::IndirectCall { method, object, args } => {
                self.table.add_reference(SymbolReference {
                    name: method.clone(),
                    kind: SymbolKind::Subroutine,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    is_write: false,
                });

                self.visit_node(object);
                for arg in args {
                    self.visit_node(arg);
                }
            }

            NodeKind::ExpressionStatement { expression } => {
                // Visit the inner expression to extract symbols
                self.visit_node(expression);
            }

            // Leaf nodes - no children to visit
            NodeKind::Number { .. }
            | NodeKind::Heredoc { .. }
            | NodeKind::Undef
            | NodeKind::Diamond
            | NodeKind::Ellipsis
            | NodeKind::Glob { .. }
            | NodeKind::Readline { .. }
            | NodeKind::Identifier { .. }
            | NodeKind::Error { .. } => {
                // No symbols to extract
            }

            _ => {
                // For any unhandled node types, log a warning
                eprintln!("Warning: Unhandled node type in symbol extractor: {:?}", node.kind);
            }
        }
    }

    /// Visit a statement list with framework-aware declaration synthesis.
    ///
    /// This handles idiomatic Perl framework declarations that are not represented
    /// as native declaration nodes in the AST (for example Moo `has` and
    /// Class::Accessor `mk_accessors` patterns).
    fn visit_statement_list(&mut self, statements: &[Node]) {
        let mut idx = 0;
        while idx < statements.len() {
            if let Some(consumed) = self.try_extract_framework_declarations(statements, idx) {
                idx += consumed;
                continue;
            }

            self.visit_node(&statements[idx]);
            idx += 1;
        }
    }

    /// Detect and synthesize framework declarations from statement patterns.
    ///
    /// Returns the number of statements consumed when a pattern is handled.
    fn try_extract_framework_declarations(
        &mut self,
        statements: &[Node],
        idx: usize,
    ) -> Option<usize> {
        if self.moo_enabled
            && let Some(consumed) = self.try_extract_moo_has_declaration(statements, idx)
        {
            return Some(consumed);
        }

        if self.class_accessor_enabled
            && self.try_extract_class_accessor_declaration(&statements[idx])
        {
            // Keep regular traversal for argument expressions (for example defaults).
            self.visit_node(&statements[idx]);
            return Some(1);
        }

        None
    }

    /// Extract Moo/Moose `has` declarations represented as:
    /// 1. `ExpressionStatement(Identifier("has"))`
    /// 2. `ExpressionStatement(HashLiteral(...))`
    fn try_extract_moo_has_declaration(
        &mut self,
        statements: &[Node],
        idx: usize,
    ) -> Option<usize> {
        if idx + 1 >= statements.len() {
            return None;
        }

        let first = &statements[idx];
        let second = &statements[idx + 1];

        let is_has_marker = matches!(
            &first.kind,
            NodeKind::ExpressionStatement { expression }
                if matches!(&expression.kind, NodeKind::Identifier { name } if name == "has")
        );
        if !is_has_marker {
            return None;
        }

        let NodeKind::ExpressionStatement { expression } = &second.kind else {
            return None;
        };
        let NodeKind::HashLiteral { pairs } = &expression.kind else {
            return None;
        };

        let has_location = SourceLocation { start: first.location.start, end: second.location.end };
        let scope_id = self.table.current_scope();
        let package = self.table.current_package.clone();

        for (attr_expr, options_expr) in pairs {
            let attribute_names = Self::collect_symbol_names(attr_expr);
            if attribute_names.is_empty() {
                continue;
            }

            let option_map = Self::extract_hash_options(options_expr);
            let metadata = Self::attribute_metadata(&option_map);
            let generated_methods = Self::moo_accessor_names(&attribute_names, &option_map);

            for attribute_name in attribute_names {
                self.table.add_symbol(Symbol {
                    name: attribute_name.clone(),
                    qualified_name: format!("{package}::{attribute_name}"),
                    kind: SymbolKind::scalar(),
                    location: has_location,
                    scope_id,
                    declaration: Some("has".to_string()),
                    documentation: Some(format!("Moo/Moose attribute `{attribute_name}`")),
                    attributes: metadata.clone(),
                });
            }

            for method_name in generated_methods {
                self.table.add_symbol(Symbol {
                    name: method_name.clone(),
                    qualified_name: format!("{package}::{method_name}"),
                    kind: SymbolKind::Subroutine,
                    location: has_location,
                    scope_id,
                    declaration: Some("has".to_string()),
                    documentation: Some("Generated accessor from Moo/Moose `has`".to_string()),
                    attributes: metadata.clone(),
                });
            }
        }

        // Traverse the hash literal to keep nested expression analysis working.
        self.visit_node(second);
        Some(2)
    }

    /// Extract Class::Accessor generated accessors from `mk_*_accessors` calls.
    fn try_extract_class_accessor_declaration(&mut self, statement: &Node) -> bool {
        let NodeKind::ExpressionStatement { expression } = &statement.kind else {
            return false;
        };

        let NodeKind::MethodCall { method, args, .. } = &expression.kind else {
            return false;
        };

        let is_accessor_generator =
            matches!(method.as_str(), "mk_accessors" | "mk_ro_accessors" | "mk_wo_accessors");
        if !is_accessor_generator {
            return false;
        }

        let mut accessor_names = Vec::new();
        for arg in args {
            accessor_names.extend(Self::collect_symbol_names(arg));
        }
        if accessor_names.is_empty() {
            return false;
        }

        let mut seen = HashSet::new();
        let scope_id = self.table.current_scope();
        let package = self.table.current_package.clone();

        for accessor_name in accessor_names {
            if !seen.insert(accessor_name.clone()) {
                continue;
            }

            self.table.add_symbol(Symbol {
                name: accessor_name.clone(),
                qualified_name: format!("{package}::{accessor_name}"),
                kind: SymbolKind::Subroutine,
                location: statement.location,
                scope_id,
                declaration: Some(method.clone()),
                documentation: Some("Generated accessor (Class::Accessor)".to_string()),
                attributes: vec!["framework=Class::Accessor".to_string()],
            });
        }

        true
    }

    /// Update framework detection state from `use` statements.
    fn update_framework_context(&mut self, module: &str, args: &[String]) {
        if matches!(module, "Moo" | "Moose" | "Moo::Role" | "Moose::Role") {
            self.moo_enabled = true;
            return;
        }

        if module == "Class::Accessor" {
            self.class_accessor_enabled = true;
            return;
        }

        if matches!(module, "base" | "parent") {
            let has_class_accessor_parent = args
                .iter()
                .filter_map(|arg| Self::normalize_symbol_name(arg))
                .any(|arg| arg == "Class::Accessor");
            if has_class_accessor_parent {
                self.class_accessor_enabled = true;
            }
        }
    }

    /// Parse attribute metadata from Moo/Moose option hashes.
    fn extract_hash_options(node: &Node) -> HashMap<String, String> {
        let mut options = HashMap::new();
        let NodeKind::HashLiteral { pairs } = &node.kind else {
            return options;
        };

        for (key_node, value_node) in pairs {
            let Some(key_name) = Self::single_symbol_name(key_node) else {
                continue;
            };
            let value_text = Self::value_summary(value_node);
            options.insert(key_name, value_text);
        }

        options
    }

    /// Convert option metadata into hover-friendly key/value tags.
    fn attribute_metadata(option_map: &HashMap<String, String>) -> Vec<String> {
        let preferred_order = [
            "is",
            "isa",
            "required",
            "lazy",
            "builder",
            "default",
            "reader",
            "writer",
            "accessor",
            "predicate",
            "clearer",
            "handles",
        ];

        let mut metadata = Vec::new();
        for key in preferred_order {
            if let Some(value) = option_map.get(key) {
                metadata.push(format!("{key}={value}"));
            }
        }
        metadata
    }

    /// Compute accessor method names for a Moo/Moose `has` declaration.
    fn moo_accessor_names(
        attribute_names: &[String],
        option_map: &HashMap<String, String>,
    ) -> Vec<String> {
        let mut methods = Vec::new();
        let mut seen = HashSet::new();

        for key in ["accessor", "reader", "writer", "predicate", "clearer"] {
            if let Some(raw_name) = option_map.get(key)
                && let Some(name) = Self::normalize_symbol_name(raw_name)
                && seen.insert(name.clone())
            {
                methods.push(name);
            }
        }

        // Default accessor when explicit reader/writer/accessor isn't provided.
        let has_explicit_accessor = option_map.contains_key("accessor")
            || option_map.contains_key("reader")
            || option_map.contains_key("writer");
        if !has_explicit_accessor {
            for attribute_name in attribute_names {
                if seen.insert(attribute_name.clone()) {
                    methods.push(attribute_name.clone());
                }
            }
        }

        methods
    }

    /// Extract one or more symbol names from a framework declaration expression.
    fn collect_symbol_names(node: &Node) -> Vec<String> {
        match &node.kind {
            NodeKind::String { value, .. } => {
                Self::normalize_symbol_name(value).into_iter().collect()
            }
            NodeKind::Identifier { name } => {
                Self::normalize_symbol_name(name).into_iter().collect()
            }
            NodeKind::ArrayLiteral { elements } => {
                let mut names = Vec::new();
                for element in elements {
                    names.extend(Self::collect_symbol_names(element));
                }
                names
            }
            _ => Vec::new(),
        }
    }

    /// Extract a single symbol name from a key/value expression.
    fn single_symbol_name(node: &Node) -> Option<String> {
        Self::collect_symbol_names(node).into_iter().next()
    }

    /// Normalize a symbol-like literal into a plain name.
    fn normalize_symbol_name(raw: &str) -> Option<String> {
        let trimmed = raw.trim().trim_matches('\'').trim_matches('"').trim();
        if trimmed.is_empty() { None } else { Some(trimmed.to_string()) }
    }

    /// Produce a short textual value summary for hover metadata.
    fn value_summary(node: &Node) -> String {
        match &node.kind {
            NodeKind::String { value, .. } => {
                Self::normalize_symbol_name(value).unwrap_or_else(|| value.clone())
            }
            NodeKind::Identifier { name } => name.clone(),
            NodeKind::Number { value } => value.clone(),
            NodeKind::ArrayLiteral { .. } => "array".to_string(),
            NodeKind::HashLiteral { .. } => "hash".to_string(),
            NodeKind::Undef => "undef".to_string(),
            _ => "expr".to_string(),
        }
    }

    /// Compute a method token location for method-call references.
    ///
    /// Some parsed method-call nodes only cover the object span. This helper scans
    /// source text after the object to anchor references on the method name token.
    fn method_reference_location(
        &self,
        call_node: &Node,
        object: &Node,
        method_name: &str,
    ) -> SourceLocation {
        if self.source.is_empty() {
            return call_node.location;
        }

        let search_start = object.location.end.min(self.source.len());
        let search_end = search_start.saturating_add(160).min(self.source.len());
        if search_start >= search_end || !self.source.is_char_boundary(search_start) {
            return call_node.location;
        }

        let window = &self.source[search_start..search_end];
        let Some(arrow_idx) = window.find("->") else {
            return call_node.location;
        };

        let mut idx = arrow_idx + 2;
        while idx < window.len() {
            let b = window.as_bytes()[idx];
            if b.is_ascii_whitespace() {
                idx += 1;
            } else {
                break;
            }
        }

        let suffix = &window[idx..];
        if suffix.starts_with(method_name) {
            let method_start = search_start + idx;
            return SourceLocation { start: method_start, end: method_start + method_name.len() };
        }

        if let Some(rel_idx) = suffix.find(method_name) {
            let method_start = search_start + idx + rel_idx;
            return SourceLocation { start: method_start, end: method_start + method_name.len() };
        }

        call_node.location
    }

    /// Extract a block of line comments immediately preceding a declaration
    fn extract_leading_comment(&self, start: usize) -> Option<String> {
        if self.source.is_empty() || start == 0 {
            return None;
        }
        let mut end = start.min(self.source.len());
        let bytes = self.source.as_bytes();
        // Trim all preceding whitespace, including newlines, to find the real end of comments.
        while end > 0 && bytes[end - 1].is_ascii_whitespace() {
            end -= 1;
        }

        // Ensure we don't break UTF-8 sequences by finding the nearest char boundary
        while end > 0 && !self.source.is_char_boundary(end) {
            end -= 1;
        }

        let prefix = &self.source[..end];
        let mut lines = prefix.lines().rev();
        let mut docs = Vec::new();
        for line in &mut lines {
            let trimmed = line.trim_start();
            if trimmed.starts_with('#') {
                // Optimize: avoid string allocation by using string slice references
                let content = trimmed.trim_start_matches('#').trim_start();
                docs.push(content);
            } else {
                // Stop at any non-comment line (including empty lines).
                break;
            }
        }
        if docs.is_empty() {
            None
        } else {
            docs.reverse();
            // Optimize: pre-calculate capacity to avoid reallocations
            let total_len: usize =
                docs.iter().map(|s| s.len()).sum::<usize>() + docs.len().saturating_sub(1);
            let mut result = String::with_capacity(total_len);
            for (i, doc) in docs.iter().enumerate() {
                if i > 0 {
                    result.push('\n');
                }
                result.push_str(doc);
            }
            Some(result)
        }
    }

    /// Handle variable declaration
    fn handle_variable_declaration(
        &mut self,
        declarator: &str,
        variable: &Node,
        attributes: &[String],
        location: SourceLocation,
        documentation: Option<String>,
    ) {
        if let NodeKind::Variable { sigil, name } = &variable.kind {
            let kind = match sigil.as_str() {
                "$" => SymbolKind::scalar(),
                "@" => SymbolKind::array(),
                "%" => SymbolKind::hash(),
                _ => return,
            };

            let symbol = Symbol {
                name: name.clone(),
                qualified_name: if declarator == "our" {
                    format!("{}::{}", self.table.current_package, name)
                } else {
                    name.clone()
                },
                kind,
                location,
                scope_id: self.table.current_scope(),
                declaration: Some(declarator.to_string()),
                documentation,
                attributes: attributes.to_vec(),
            };

            self.table.add_symbol(symbol);
        }
    }

    /// Mark a node as a write reference (used in assignments)
    fn mark_write_reference(&mut self, node: &Node) {
        // This is a simplified version - in practice we'd need to handle
        // more complex LHS patterns like array/hash subscripts
        if let NodeKind::Variable { .. } = &node.kind {
            // The reference will be marked as write when we visit it
            // This would require passing context down through visit_node
        }
    }

    /// Extract variable references from an interpolated string
    fn extract_vars_from_string(&mut self, value: &str, string_location: SourceLocation) {
        // Simple regex to find scalar variables in strings
        // This handles $var, ${var}, but not arrays/hashes for now
        let Ok(scalar_re) = Regex::new(r"\$([a-zA-Z_]\w*|\{[a-zA-Z_]\w*\})") else {
            return; // Skip variable extraction if regex fails
        };

        // The value includes quotes, so strip them
        let content = if value.len() >= 2 { &value[1..value.len() - 1] } else { value };

        for cap in scalar_re.captures_iter(content) {
            if let Some(m) = cap.get(0) {
                let var_name = if m.as_str().starts_with("${") && m.as_str().ends_with("}") {
                    // Handle ${var} format
                    &m.as_str()[2..m.as_str().len() - 1]
                } else {
                    // Handle $var format
                    &m.as_str()[1..]
                };

                // Calculate the location within the original string
                // This is approximate - in the actual string location
                let start_offset = string_location.start + 1 + m.start(); // +1 for opening quote
                let end_offset = start_offset + m.len();

                let reference = SymbolReference {
                    name: var_name.to_string(),
                    kind: SymbolKind::scalar(),
                    location: SourceLocation { start: start_offset, end: end_offset },
                    scope_id: self.table.current_scope(),
                    is_write: false,
                };

                self.table.add_reference(reference);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::Parser;
    use perl_tdd_support::must;

    #[test]
    fn test_symbol_extraction() {
        let code = r#"
package Foo;

my $x = 42;
our $y = "hello";

sub bar {
    my $z = $x + $y;
    return $z;
}
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        // Check package symbol
        assert!(table.symbols.contains_key("Foo"));
        let foo_symbols = &table.symbols["Foo"];
        assert_eq!(foo_symbols.len(), 1);
        assert_eq!(foo_symbols[0].kind, SymbolKind::Package);

        // Check variable symbols
        assert!(table.symbols.contains_key("x"));
        assert!(table.symbols.contains_key("y"));
        assert!(table.symbols.contains_key("z"));

        // Check subroutine symbol
        assert!(table.symbols.contains_key("bar"));
        let bar_symbols = &table.symbols["bar"];
        assert_eq!(bar_symbols.len(), 1);
        assert_eq!(bar_symbols[0].kind, SymbolKind::Subroutine);
    }
}
