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
use std::sync::OnceLock;

const UNIVERSAL_METHODS: [&str; 4] = ["can", "isa", "DOES", "VERSION"];

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

/// Return `true` if the method is one of Perl's always-available `UNIVERSAL` methods.
///
/// Used in analyze/index workflow stages to keep method lookup behavior
/// consistent across parser and LSP navigation flows.
pub fn is_universal_method(method_name: &str) -> bool {
    UNIVERSAL_METHODS.contains(&method_name)
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Classification of Moo/Moose framework variant detected via `use` statements during Parse/Analyze workflows.
pub enum FrameworkKind {
    /// `use Moo;`
    Moo,
    /// `use Moo::Role;`
    MooRole,
    /// `use Moose;`
    Moose,
    /// `use Moose::Role;`
    MooseRole,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Web framework variant detected via `use` statements during Parse/Analyze workflows.
pub enum WebFrameworkKind {
    /// `use Dancer;`
    Dancer,
    /// `use Dancer2;` or `use Dancer2::Core;`
    Dancer2,
    /// `use Mojolicious::Lite;`
    MojoliciousLite,
    /// `use Plack::Builder;`
    PlackBuilder,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
/// Async framework variant detected via `use` statements during Parse/Analyze workflows.
pub enum AsyncFrameworkKind {
    /// `use AnyEvent;`
    AnyEvent,
    /// `use EV;`
    EV,
    /// `use Future;`
    Future,
    /// `use Future::XS;`
    FutureXS,
    /// `use Promise;`
    Promise,
    /// `use Promise::XS;`
    PromiseXS,
    /// `use POE;`
    POE,
    /// `use IO::Async;`
    IOAsync,
    /// `use Mojo::Redis;`
    MojoRedis,
    /// `use Mojo::Pg;`
    MojoPg,
}

#[derive(Debug, Clone, Default)]
/// Per-package framework detection flags used in Parse/Analyze workflows.
pub struct FrameworkFlags {
    /// Moo/Moose framework variant, if any.
    pub moo: bool,
    /// Class::Accessor style generated accessors.
    pub class_accessor: bool,
    /// Which specific Moo/Moose variant was detected.
    pub kind: Option<FrameworkKind>,
    /// Web framework variant, if any (Dancer, Dancer2, Mojolicious::Lite).
    pub web_framework: Option<WebFrameworkKind>,
    /// Async framework variant, if any (IO::Async).
    pub async_framework: Option<AsyncFrameworkKind>,
    /// Catalyst controller/package marker used for action synthesis.
    pub catalyst_controller: bool,
}

/// Extract symbols from an AST for Parse/Index workflows.
pub struct SymbolExtractor {
    table: SymbolTable,
    /// Source code for comment extraction
    source: String,
    /// Per-package framework detection flags, keyed by package name.
    framework_flags: HashMap<String, FrameworkFlags>,
    /// Packages that imported `Const::Fast`.
    const_fast_packages: HashSet<String>,
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
            framework_flags: HashMap::new(),
            const_fast_packages: HashSet::new(),
        }
    }

    /// Create a symbol extractor with source text for documentation extraction.
    ///
    /// Used during Parse/Analyze stages to attach documentation metadata.
    pub fn new_with_source(source: &str) -> Self {
        SymbolExtractor {
            table: SymbolTable::new(),
            source: source.to_string(),
            framework_flags: HashMap::new(),
            const_fast_packages: HashSet::new(),
        }
    }

    /// Extract symbols from an AST node for Index/Analyze workflows.
    pub fn extract(mut self, node: &Node) -> SymbolTable {
        self.visit_node(node);
        self.upgrade_package_symbols_from_framework_flags();
        self.table
    }

    /// Post-processing: upgrade `SymbolKind::Package` to `Class` or `Role`
    /// based on the framework flags discovered during traversal.
    fn upgrade_package_symbols_from_framework_flags(&mut self) {
        for (pkg_name, flags) in &self.framework_flags {
            let Some(kind) = flags.kind else {
                continue;
            };
            let new_kind = match kind {
                FrameworkKind::Moo | FrameworkKind::Moose => SymbolKind::Class,
                FrameworkKind::MooRole | FrameworkKind::MooseRole => SymbolKind::Role,
            };
            if let Some(symbols) = self.table.symbols.get_mut(pkg_name) {
                for symbol in symbols.iter_mut() {
                    if symbol.kind == SymbolKind::Package {
                        symbol.kind = new_kind;
                    }
                }
            }
        }
    }

    fn const_fast_enabled(&self) -> bool {
        self.const_fast_packages.contains(&self.table.current_package)
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
                signature,
                attributes,
                body,
                name_span: _,
            } => {
                let sub_name =
                    name.as_ref().map(|n| n.to_string()).unwrap_or_else(|| "<anon>".to_string());

                if name.is_some() {
                    let documentation = self.extract_leading_comment(node.location.start);
                    let mut symbol_attributes = attributes.clone();
                    let documentation = if self.current_package_is_catalyst_controller()
                        && let Some((action_kind, action_details)) =
                            Self::catalyst_action_metadata(attributes)
                    {
                        symbol_attributes.push("framework=Catalyst".to_string());
                        symbol_attributes.push("catalyst_controller=true".to_string());
                        symbol_attributes.push("catalyst_action=true".to_string());
                        symbol_attributes.push(format!("catalyst_action_kind={action_kind}"));
                        if !action_details.is_empty() {
                            symbol_attributes.push(format!(
                                "catalyst_action_attributes={}",
                                action_details.join(", ")
                            ));
                        }

                        let action_doc = if action_details.is_empty() {
                            format!("Catalyst action ({action_kind})")
                        } else {
                            format!(
                                "Catalyst action ({action_kind}; {})",
                                action_details.join(", ")
                            )
                        };
                        match documentation {
                            Some(doc) => Some(format!("{doc}\n{action_doc}")),
                            None => Some(action_doc),
                        }
                    } else {
                        documentation
                    };
                    let symbol = Symbol {
                        name: sub_name.clone(),
                        qualified_name: format!("{}::{}", self.table.current_package, sub_name),
                        kind: SymbolKind::Subroutine,
                        location: node.location,
                        scope_id: self.table.current_scope(),
                        declaration: None,
                        documentation,
                        attributes: symbol_attributes,
                    };

                    self.table.add_symbol(symbol);
                }

                // Create subroutine scope
                self.table.push_scope(ScopeKind::Subroutine, node.location);

                // Register signature parameters as implicit `my` declarations
                if let Some(sig) = signature {
                    self.register_signature_params(sig);
                }

                self.visit_node(body);

                self.table.pop_scope();
            }

            NodeKind::Package { name, block, name_span: _ } => {
                let old_package = self.table.current_package.clone();
                self.table.current_package = name.clone();
                if Self::is_catalyst_controller_package_name(name) {
                    self.mark_catalyst_controller_package(name);
                }

                let documentation = self.extract_package_documentation(name, node.location);
                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind: SymbolKind::Package,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation,
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
                if self.const_fast_enabled()
                    && name == "const"
                    && self.try_extract_const_fast_declaration(args)
                {
                    return;
                }

                // Track function call as a reference
                let reference = SymbolReference {
                    name: name.clone(),
                    kind: SymbolKind::Subroutine,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    is_write: false,
                };
                self.table.add_reference(reference);

                self.synthesize_plack_builder_symbols(name, args);
                self.synthesize_ev_symbols(name, node.location);

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

                self.synthesize_async_framework_class_symbol(object);
                self.synthesize_future_api_symbols(object, method, node.location);
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
                if module == "Const::Fast" {
                    self.const_fast_packages.insert(self.table.current_package.clone());
                }
                if module == "EV" {
                    self.synthesize_ev_framework_symbol(node.location);
                }
            }

            NodeKind::No { module: _, args: _, .. } => {
                // We don't currently track framework deactivation via `no`.
            }

            NodeKind::PhaseBlock { phase, phase_span: _, block } => {
                // BEGIN, END, CHECK, INIT, UNITCHECK blocks — expose as named symbols
                // so they appear in document outline / Outline View (#3464).
                let symbol = Symbol {
                    name: phase.clone(),
                    qualified_name: format!("{}::{}", self.table.current_package, phase),
                    kind: SymbolKind::Subroutine,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation: None,
                    attributes: vec![],
                };
                self.table.add_symbol(symbol);

                self.table.push_scope(ScopeKind::Block, node.location);
                self.visit_node(block);
                self.table.pop_scope();
            }

            NodeKind::StatementModifier { statement, modifier: _, condition } => {
                self.visit_node(statement);
                self.visit_node(condition);
            }

            NodeKind::Do { block } | NodeKind::Eval { block } | NodeKind::Defer { block } => {
                self.visit_node(block);
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                self.visit_node(body);
                for (catch_var, catch_block) in catch_blocks {
                    self.table.push_scope(ScopeKind::Block, catch_block.location);
                    if let Some(full_name) = catch_var.as_deref() {
                        self.register_catch_variable(full_name, catch_block.location);
                    }
                    self.visit_node(catch_block);
                    self.table.pop_scope();
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

            NodeKind::Class { name, parents, body } => {
                let documentation = self.extract_leading_comment(node.location.start);
                if Self::is_catalyst_controller_package_name(name)
                    || parents.iter().any(|parent| parent == "Catalyst::Controller")
                {
                    self.mark_catalyst_controller_package(name);
                }
                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind: SymbolKind::Package, // Classes are like packages
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation,
                    attributes: vec![],
                };
                self.table.add_symbol(symbol);

                self.table.push_scope(ScopeKind::Package, node.location);
                self.visit_node(body);
                self.table.pop_scope();
            }

            NodeKind::Method { name, signature, attributes, body } => {
                let documentation = self.extract_leading_comment(node.location.start);
                let mut symbol_attributes = Vec::with_capacity(attributes.len() + 1);
                symbol_attributes.push("method".to_string());
                symbol_attributes.extend(attributes.iter().cloned());
                let symbol = Symbol {
                    name: name.clone(),
                    qualified_name: format!("{}::{}", self.table.current_package, name),
                    kind: SymbolKind::Method,
                    location: node.location,
                    scope_id: self.table.current_scope(),
                    declaration: None,
                    documentation,
                    attributes: symbol_attributes,
                };
                self.table.add_symbol(symbol);

                self.table.push_scope(ScopeKind::Subroutine, node.location);

                // Register signature parameters as implicit `my` declarations
                if let Some(sig) = signature {
                    self.register_signature_params(sig);
                }

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

            NodeKind::Tie { variable, package, args } => {
                self.visit_node(variable);
                self.visit_node(package);
                for arg in args {
                    self.visit_node(arg);
                }
            }

            NodeKind::Untie { variable } | NodeKind::Goto { target: variable } => {
                self.visit_node(variable);
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
            | NodeKind::Typeglob { .. }
            | NodeKind::DataSection { .. }
            | NodeKind::LoopControl { .. }
            | NodeKind::MissingExpression
            | NodeKind::MissingStatement
            | NodeKind::MissingIdentifier
            | NodeKind::MissingBlock
            | NodeKind::UnknownRest
            | NodeKind::Error { .. } => {
                // No symbols to extract
            }

            _ => {
                // For any unhandled node types, log a warning
                tracing::warn!(kind = ?node.kind, "Unhandled node type in symbol extractor");
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
        let flags = self.framework_flags.get(&self.table.current_package).cloned();
        let flags = flags.as_ref();

        let is_moo = flags.is_some_and(|f| f.moo);

        if is_moo {
            if let Some(consumed) = self.try_extract_moo_has_declaration(statements, idx) {
                return Some(consumed);
            }
            if let Some(consumed) = self.try_extract_method_modifier(statements, idx) {
                return Some(consumed);
            }
            if let Some(consumed) = self.try_extract_extends_with(statements, idx) {
                return Some(consumed);
            }
            if let Some(consumed) = self.try_extract_role_requires(statements, idx) {
                return Some(consumed);
            }
        }

        if flags.is_some_and(|f| f.class_accessor)
            && self.try_extract_class_accessor_declaration(&statements[idx])
        {
            // Keep regular traversal for argument expressions (for example defaults).
            self.visit_node(&statements[idx]);
            return Some(1);
        }

        if flags.is_some_and(|f| f.web_framework.is_some()) {
            if let Some(consumed) = self.try_extract_web_route_declaration(statements, idx) {
                return Some(consumed);
            }
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
        let first = &statements[idx];

        // Form A:
        // 1) ExpressionStatement(Identifier("has"))
        // 2) ExpressionStatement(HashLiteral(...))
        // OR
        // 1) ExpressionStatement(Identifier("has"))
        // 2) ExpressionStatement(ArrayLiteral([..., HashLiteral]))
        if idx + 1 < statements.len() {
            let second = &statements[idx + 1];
            let is_has_marker = matches!(
                &first.kind,
                NodeKind::ExpressionStatement { expression }
                    if matches!(&expression.kind, NodeKind::Identifier { name } if name == "has")
            );

            if is_has_marker {
                if let NodeKind::ExpressionStatement { expression } = &second.kind {
                    let has_location =
                        SourceLocation { start: first.location.start, end: second.location.end };

                    match &expression.kind {
                        NodeKind::HashLiteral { pairs } => {
                            self.synthesize_moo_has_pairs(pairs, has_location, false);
                            self.visit_node(second);
                            return Some(2);
                        }
                        NodeKind::ArrayLiteral { elements } => {
                            if let Some(Node { kind: NodeKind::HashLiteral { pairs }, .. }) =
                                elements.last()
                            {
                                // Extract the names from the preceding elements
                                let mut names = Vec::new();
                                for el in elements.iter().take(elements.len() - 1) {
                                    names.extend(Self::collect_symbol_names(el));
                                }
                                if !names.is_empty() {
                                    self.synthesize_moo_has_attrs_with_options(
                                        &names,
                                        pairs,
                                        has_location,
                                    );
                                    self.visit_node(second);
                                    return Some(2);
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
        }

        // Form B:
        // ExpressionStatement(HashLiteral((Binary("[]", Identifier("has"), attr_expr), options)))
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::HashLiteral { pairs } = &expression.kind
        {
            let has_embedded_marker = pairs.iter().any(|(key_node, _)| {
                matches!(
                    &key_node.kind,
                    NodeKind::Binary { op, left, .. }
                        if op == "[]" && matches!(&left.kind, NodeKind::Identifier { name } if name == "has")
                )
            });

            if has_embedded_marker {
                self.synthesize_moo_has_pairs(pairs, first.location, true);
                self.visit_node(first);
                return Some(1);
            }
        }

        // Form C: FunctionCall { name: "has", args: [name_expr, HashLiteral { ... }] }
        // Produced when the parser recognises `has 'name' => (is => 'ro', ...)` as a bare call.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
            && name == "has"
            && !args.is_empty()
        {
            let options_hash_idx =
                args.iter().rposition(|a| matches!(a.kind, NodeKind::HashLiteral { .. }));
            if let Some(opts_idx) = options_hash_idx {
                if let NodeKind::HashLiteral { pairs } = &args[opts_idx].kind {
                    let names: Vec<String> =
                        args[..opts_idx].iter().flat_map(Self::collect_symbol_names).collect();
                    if !names.is_empty() {
                        self.synthesize_moo_has_attrs_with_options(&names, pairs, first.location);
                        self.visit_node(first);
                        return Some(1);
                    }
                }
            }
        }

        None
    }

    /// Detect Moo/Moose method modifiers (`before`, `after`, `around`, `override`, `augment`).
    ///
    /// Pattern (two statements):
    /// 1. `ExpressionStatement(Identifier("around"))` (or `before`/`after`/`override`/`augment`)
    /// 2. `ExpressionStatement(HashLiteral([ (method_name, Subroutine{...}) ]))`
    ///
    /// Also handles FunctionCall form: `around 'name' => sub { }` (post parser fix).
    fn try_extract_method_modifier(&mut self, statements: &[Node], idx: usize) -> Option<usize> {
        let first = &statements[idx];

        // FunctionCall form: `around 'name' => sub { }` parsed as a bare call.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
            && Self::is_moose_method_modifier(name)
        {
            let modifier_name = name.as_str();
            let method_names: Vec<String> =
                args.first().map(Self::collect_symbol_names).unwrap_or_default();
            if !method_names.is_empty() {
                let scope_id = self.table.current_scope();
                let package = self.table.current_package.clone();
                for method_name in method_names {
                    self.table.add_symbol(Symbol {
                        name: method_name.clone(),
                        qualified_name: format!("{package}::{method_name}"),
                        kind: SymbolKind::Subroutine,
                        location: first.location,
                        scope_id,
                        declaration: Some(modifier_name.to_string()),
                        documentation: Some(format!(
                            "Method modifier `{modifier_name}` for `{method_name}`"
                        )),
                        attributes: vec![format!("modifier={modifier_name}")],
                    });
                }
                return Some(1);
            }
        }

        if idx + 1 >= statements.len() {
            return None;
        }

        let second = &statements[idx + 1];

        // Check: first is ExpressionStatement(Identifier("before"|"after"|"around"|"override"|"augment"))
        let modifier_name = match &first.kind {
            NodeKind::ExpressionStatement { expression } => match &expression.kind {
                NodeKind::Identifier { name } if Self::is_moose_method_modifier(name) => {
                    name.as_str()
                }
                _ => return None,
            },
            _ => return None,
        };

        // Check: second is ExpressionStatement(HashLiteral(...)) with method names
        let NodeKind::ExpressionStatement { expression } = &second.kind else {
            return None;
        };
        let NodeKind::HashLiteral { pairs } = &expression.kind else {
            return None;
        };

        let modifier_location =
            SourceLocation { start: first.location.start, end: second.location.end };
        let scope_id = self.table.current_scope();
        let package = self.table.current_package.clone();

        for (key_node, _value_node) in pairs {
            let method_names = Self::collect_symbol_names(key_node);
            for method_name in method_names {
                self.table.add_symbol(Symbol {
                    name: method_name.clone(),
                    qualified_name: format!("{package}::{method_name}"),
                    kind: SymbolKind::Subroutine,
                    location: modifier_location,
                    scope_id,
                    declaration: Some(modifier_name.to_string()),
                    documentation: Some(format!(
                        "Method modifier `{modifier_name}` for `{method_name}`"
                    )),
                    attributes: vec![format!("modifier={modifier_name}")],
                });
            }
        }

        // Visit the body of the modifier subroutines
        self.visit_node(second);

        Some(2)
    }

    fn is_moose_method_modifier(name: &str) -> bool {
        matches!(name, "before" | "after" | "around" | "override" | "augment")
    }

    /// Detect Moo/Moose `extends 'Parent'` and `with 'Role'` declarations.
    ///
    /// Pattern (two statements):
    /// 1. `ExpressionStatement(Identifier("extends"))` or `ExpressionStatement(Identifier("with"))`
    /// 2. `ExpressionStatement(String(...))` or `ExpressionStatement(ArrayLiteral(...))`
    ///
    /// Also handles FunctionCall form: `extends 'Parent'` (post parser fix).
    fn try_extract_extends_with(&mut self, statements: &[Node], idx: usize) -> Option<usize> {
        let first = &statements[idx];

        // FunctionCall form: `extends 'Parent'` / `with 'Role'` parsed as bare calls.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
            && matches!(name.as_str(), "extends" | "with")
        {
            let keyword = name.as_str();
            let names: Vec<String> = args.iter().flat_map(Self::collect_symbol_names).collect();
            if !names.is_empty() {
                if names.iter().any(|name| name == "Catalyst::Controller") {
                    let package = self.table.current_package.clone();
                    self.mark_catalyst_controller_package(&package);
                }
                let ref_kind =
                    if keyword == "extends" { SymbolKind::Class } else { SymbolKind::Role };
                for ref_name in names {
                    self.table.add_reference(SymbolReference {
                        name: ref_name,
                        kind: ref_kind,
                        location: first.location,
                        scope_id: self.table.current_scope(),
                        is_write: false,
                    });
                }
                return Some(1);
            }
        }

        if idx + 1 >= statements.len() {
            return None;
        }

        let second = &statements[idx + 1];

        // Check: first is ExpressionStatement(Identifier("extends"|"with"))
        let keyword = match &first.kind {
            NodeKind::ExpressionStatement { expression } => match &expression.kind {
                NodeKind::Identifier { name } if matches!(name.as_str(), "extends" | "with") => {
                    name.as_str()
                }
                _ => return None,
            },
            _ => return None,
        };

        // Check: second is ExpressionStatement with name(s)
        let NodeKind::ExpressionStatement { expression } = &second.kind else {
            return None;
        };

        let names = Self::collect_symbol_names(expression);
        if names.is_empty() {
            return None;
        }

        if names.iter().any(|name| name == "Catalyst::Controller") {
            let package = self.table.current_package.clone();
            self.mark_catalyst_controller_package(&package);
        }

        let ref_location = SourceLocation { start: first.location.start, end: second.location.end };

        let ref_kind = if keyword == "extends" { SymbolKind::Class } else { SymbolKind::Role };

        for name in names {
            self.table.add_reference(SymbolReference {
                name,
                kind: ref_kind,
                location: ref_location,
                scope_id: self.table.current_scope(),
                is_write: false,
            });
        }

        Some(2)
    }

    /// Detect Moo/Moose `requires 'method'` declarations.
    ///
    /// Pattern:
    /// `ExpressionStatement(Identifier("requires"))` followed by `ExpressionStatement(String(...))` or similar
    ///
    /// Also handles FunctionCall form: `requires 'method'` (post parser fix).
    fn try_extract_role_requires(&mut self, statements: &[Node], idx: usize) -> Option<usize> {
        let first = &statements[idx];

        // FunctionCall form: `requires 'method'` parsed as a bare call.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
            && name == "requires"
        {
            let names: Vec<String> = args.iter().flat_map(Self::collect_symbol_names).collect();
            if !names.is_empty() {
                let scope_id = self.table.current_scope();
                let package = self.table.current_package.clone();
                for method_name in names {
                    self.table.add_symbol(Symbol {
                        name: method_name.clone(),
                        qualified_name: format!("{package}::{method_name}"),
                        kind: SymbolKind::Subroutine,
                        location: first.location,
                        scope_id,
                        declaration: Some("requires".to_string()),
                        documentation: Some(format!("Required method `{method_name}` from role")),
                        attributes: vec!["requires=true".to_string()],
                    });
                }
                return Some(1);
            }
        }

        if idx + 1 >= statements.len() {
            return None;
        }

        let second = &statements[idx + 1];

        // Check: first is ExpressionStatement(Identifier("requires"))
        let is_requires = match &first.kind {
            NodeKind::ExpressionStatement { expression } => {
                matches!(&expression.kind, NodeKind::Identifier { name } if name == "requires")
            }
            _ => false,
        };

        if !is_requires {
            return None;
        }

        let NodeKind::ExpressionStatement { expression } = &second.kind else {
            return None;
        };

        let names = Self::collect_symbol_names(expression);
        if names.is_empty() {
            return None;
        }

        let location = SourceLocation { start: first.location.start, end: second.location.end };
        let scope_id = self.table.current_scope();
        let package = self.table.current_package.clone();

        for name in names {
            self.table.add_symbol(Symbol {
                name: name.clone(),
                qualified_name: format!("{package}::{name}"),
                kind: SymbolKind::Subroutine,
                location,
                scope_id,
                declaration: Some("requires".to_string()),
                documentation: Some(format!("Required method `{name}` from role")),
                attributes: vec!["requires=true".to_string()],
            });
        }

        Some(2)
    }

    /// Synthesize symbols from parsed `has` key/value pairs.
    fn synthesize_moo_has_pairs(
        &mut self,
        pairs: &[(Node, Node)],
        has_location: SourceLocation,
        require_embedded_marker: bool,
    ) {
        for (attr_expr, options_expr) in pairs {
            let Some(attr_expr) = Self::moo_attribute_expr(attr_expr, require_embedded_marker)
            else {
                continue;
            };

            let attribute_names = Self::collect_symbol_names(attr_expr);
            if attribute_names.is_empty() {
                continue;
            }

            if let NodeKind::HashLiteral { pairs: option_pairs } = &options_expr.kind {
                self.synthesize_moo_has_attrs_with_options(
                    &attribute_names,
                    option_pairs,
                    has_location,
                );
            }
        }
    }

    /// Synthesize Moo symbols for a known list of attributes and options.
    fn synthesize_moo_has_attrs_with_options(
        &mut self,
        attribute_names: &[String],
        option_pairs: &[(Node, Node)],
        has_location: SourceLocation,
    ) {
        let scope_id = self.table.current_scope();
        let package = self.table.current_package.clone();

        // Create a dummy options_expr Node to pass to existing helpers
        // (a bit hacky, but avoids rewriting the helpers that take Node)
        let options_expr = Node {
            kind: NodeKind::HashLiteral { pairs: option_pairs.to_vec() },
            location: has_location,
        };

        let option_map = Self::extract_hash_options(&options_expr);
        let metadata = Self::attribute_metadata(&option_map);
        let generated_methods =
            Self::moo_accessor_names(attribute_names, &option_map, &options_expr);

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

        // Build accessor documentation that includes the isa type when available.
        let accessor_doc = Self::moo_accessor_doc(&option_map);

        for method_name in generated_methods {
            self.table.add_symbol(Symbol {
                name: method_name.clone(),
                qualified_name: format!("{package}::{method_name}"),
                kind: SymbolKind::Subroutine,
                location: has_location,
                scope_id,
                declaration: Some("has".to_string()),
                documentation: Some(accessor_doc.clone()),
                attributes: metadata.clone(),
            });
        }
    }

    /// Resolve the attribute-expression node used in a parsed `has` declaration pair.
    fn moo_attribute_expr(attr_expr: &Node, require_embedded_marker: bool) -> Option<&Node> {
        if let NodeKind::Binary { op, left, right } = &attr_expr.kind
            && op == "[]"
            && matches!(&left.kind, NodeKind::Identifier { name } if name == "has")
        {
            return Some(right.as_ref());
        }

        if require_embedded_marker { None } else { Some(attr_expr) }
    }

    /// Detect Dancer/Dancer2/Mojolicious::Lite route declarations and synthesize route symbols.
    ///
    /// Pattern (two statements):
    /// 1. `ExpressionStatement(Identifier("get"|"post"|"put"|"del"|"patch"|"any"))`
    /// 2. `ExpressionStatement(HashLiteral([ (String("/path"), Subroutine{...}) ]))`
    ///
    /// Synthesizes a `Subroutine` symbol named by the route path with
    /// `http_method=<METHOD>` in attributes and a human-readable documentation string.
    fn try_extract_web_route_declaration(
        &mut self,
        statements: &[Node],
        idx: usize,
    ) -> Option<usize> {
        let web_framework = self
            .framework_flags
            .get(&self.table.current_package)
            .and_then(|flags| flags.web_framework);
        let first = &statements[idx];

        // FunctionCall form: `get '/path' => sub { }` parsed as a bare call.
        if let NodeKind::ExpressionStatement { expression } = &first.kind
            && let NodeKind::FunctionCall { name, args } = &expression.kind
            && matches!(name.as_str(), "get" | "post" | "put" | "del" | "delete" | "patch" | "any")
        {
            let method_name = name.as_str();
            // args[0] is the route path (String), rest is the handler
            if let Some(path_node) = args.first() {
                if let NodeKind::String { value, .. } = &path_node.kind {
                    if let Some(path) = Self::normalize_symbol_name(value) {
                        let http_method = match method_name {
                            "get" => "GET",
                            "post" => "POST",
                            "put" => "PUT",
                            "del" | "delete" => "DELETE",
                            "patch" => "PATCH",
                            "any" => "ANY",
                            _ => method_name,
                        };
                        let scope_id = self.table.current_scope();
                        self.table.add_symbol(Symbol {
                            name: path.clone(),
                            qualified_name: path.clone(),
                            kind: SymbolKind::Subroutine,
                            location: first.location,
                            scope_id,
                            declaration: Some(method_name.to_string()),
                            documentation: Some(format!("{http_method} {path}")),
                            attributes: vec![format!("http_method={http_method}")],
                        });

                        if matches!(
                            web_framework,
                            Some(WebFrameworkKind::Dancer | WebFrameworkKind::Dancer2)
                        ) && let Some(target_node) = args.get(1)
                        {
                            if let Some(target_name) =
                                Self::collect_symbol_names(target_node).first().cloned()
                            {
                                self.table.add_reference(SymbolReference {
                                    name: target_name,
                                    kind: SymbolKind::Subroutine,
                                    location: target_node.location,
                                    scope_id: self.table.current_scope(),
                                    is_write: false,
                                });
                            }
                        }

                        self.visit_node(first);
                        return Some(1);
                    }
                }
            }
        }

        if idx + 1 >= statements.len() {
            return None;
        }

        let second = &statements[idx + 1];

        // First statement must be ExpressionStatement(Identifier(<route_method>))
        let method_name = match &first.kind {
            NodeKind::ExpressionStatement { expression } => match &expression.kind {
                NodeKind::Identifier { name }
                    if matches!(
                        name.as_str(),
                        "get" | "post" | "put" | "del" | "delete" | "patch" | "any"
                    ) =>
                {
                    name.as_str()
                }
                _ => return None,
            },
            _ => return None,
        };

        // Second statement must be ExpressionStatement(HashLiteral([ (path, handler) ]))
        let NodeKind::ExpressionStatement { expression } = &second.kind else {
            return None;
        };
        let NodeKind::HashLiteral { pairs } = &expression.kind else {
            return None;
        };

        // Extract route path from the first key in the hash literal (strip surrounding quotes)
        let (path_node, _handler_node) = pairs.first()?;
        let path = match &path_node.kind {
            NodeKind::String { value, .. } => Self::normalize_symbol_name(value)?,
            _ => return None,
        };

        let http_method = match method_name {
            "get" => "GET",
            "post" => "POST",
            "put" => "PUT",
            "del" | "delete" => "DELETE",
            "patch" => "PATCH",
            "any" => "ANY",
            _ => method_name,
        };

        let route_location =
            SourceLocation { start: first.location.start, end: second.location.end };
        let scope_id = self.table.current_scope();

        self.table.add_symbol(Symbol {
            name: path.clone(),
            qualified_name: path.clone(),
            kind: SymbolKind::Subroutine,
            location: route_location,
            scope_id,
            declaration: Some(method_name.to_string()),
            documentation: Some(format!("{http_method} {path}")),
            attributes: vec![format!("http_method={http_method}")],
        });

        // Visit the handler body so variables inside the sub are still indexed
        self.visit_node(second);

        Some(2)
    }

    /// Synthesize Plack::Builder middleware and mount symbols from a builder block.
    fn synthesize_plack_builder_symbols(&mut self, name: &str, args: &[Node]) {
        let Some(flags) = self.framework_flags.get(&self.table.current_package) else {
            return;
        };
        if flags.web_framework != Some(WebFrameworkKind::PlackBuilder) || name != "builder" {
            return;
        }

        let Some(block) = args.first() else {
            return;
        };
        let NodeKind::Block { statements } = &block.kind else {
            return;
        };

        let scope_id = self.table.current_scope();
        let package = self.table.current_package.clone();

        for statement in statements {
            let NodeKind::ExpressionStatement { expression } = &statement.kind else {
                continue;
            };
            let NodeKind::FunctionCall { name: stmt_name, args: stmt_args } = &expression.kind
            else {
                continue;
            };

            match stmt_name.as_str() {
                "enable" => {
                    self.synthesize_plack_enable_symbol(statement, stmt_args, scope_id, &package);
                }
                "mount" => {
                    self.synthesize_plack_mount_symbol(statement, stmt_args, scope_id, &package);
                }
                _ => {}
            }
        }
    }

    fn synthesize_plack_enable_symbol(
        &mut self,
        statement: &Node,
        args: &[Node],
        scope_id: ScopeId,
        _package: &str,
    ) {
        let Some(first) = args.first() else {
            return;
        };
        let Some(raw_name) = Self::single_symbol_name(first) else {
            return;
        };
        let middleware_name = if raw_name.contains("::") {
            raw_name
        } else {
            format!("Plack::Middleware::{raw_name}")
        };
        if middleware_name.is_empty() {
            return;
        }

        if self.table.symbols.get(&middleware_name).is_some_and(|symbols| {
            symbols.iter().any(|symbol| {
                symbol.kind == SymbolKind::Package
                    && symbol.declaration.as_deref() == Some("enable")
                    && symbol
                        .attributes
                        .iter()
                        .any(|attr| attr == &format!("middleware={middleware_name}"))
            })
        }) {
            return;
        }

        self.table.add_symbol(Symbol {
            name: middleware_name.clone(),
            qualified_name: middleware_name.clone(),
            kind: SymbolKind::Package,
            location: statement.location,
            scope_id,
            declaration: Some("enable".to_string()),
            documentation: Some(format!("PSGI middleware {middleware_name}")),
            attributes: vec![
                "framework=Plack::Builder".to_string(),
                format!("middleware={middleware_name}"),
            ],
        });
    }

    fn synthesize_plack_mount_symbol(
        &mut self,
        statement: &Node,
        args: &[Node],
        scope_id: ScopeId,
        _package: &str,
    ) {
        let Some(path_node) = args.first() else {
            return;
        };
        let Some(path) = Self::single_symbol_name(path_node) else {
            return;
        };
        if path.is_empty() {
            return;
        }

        let target = args
            .get(1)
            .map(Self::value_summary)
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "$app".to_string());

        if self.table.symbols.get(&path).is_some_and(|symbols| {
            symbols.iter().any(|symbol| {
                symbol.kind == SymbolKind::Subroutine
                    && symbol.declaration.as_deref() == Some("mount")
                    && symbol.attributes.iter().any(|attr| attr == &format!("mount_path={path}"))
            })
        }) {
            return;
        }

        self.table.add_symbol(Symbol {
            name: path.clone(),
            qualified_name: path.clone(),
            kind: SymbolKind::Subroutine,
            location: statement.location,
            scope_id,
            declaration: Some("mount".to_string()),
            documentation: Some(format!("PSGI mount {path} -> {target}")),
            attributes: vec![
                "framework=Plack::Builder".to_string(),
                format!("mount_path={path}"),
                format!("mount_target={target}"),
            ],
        });
    }

    /// Extract Class::Accessor generated accessors from `mk_*_accessors` calls.
    fn try_extract_class_accessor_declaration(&mut self, statement: &Node) -> bool {
        let NodeKind::ExpressionStatement { expression } = &statement.kind else {
            return false;
        };

        let NodeKind::MethodCall { method, args, .. } = &expression.kind else {
            return false;
        };

        let is_accessor_generator = matches!(
            method.as_str(),
            "mk_accessors" | "mk_ro_accessors" | "mk_rw_accessors" | "mk_wo_accessors"
        );
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

    /// Synthesize class symbols for async framework namespaces used in method-call form.
    fn synthesize_async_framework_class_symbol(&mut self, object: &Node) -> bool {
        let Some(flags) = self.framework_flags.get(&self.table.current_package) else {
            return false;
        };

        let (module_name, framework_name, exact_match) = match flags.async_framework {
            Some(AsyncFrameworkKind::AnyEvent) => ("AnyEvent", "AnyEvent", false),
            Some(AsyncFrameworkKind::EV) => ("EV", "EV", true),
            Some(AsyncFrameworkKind::Future) => ("Future", "Future", true),
            Some(AsyncFrameworkKind::FutureXS) => ("Future::XS", "Future::XS", true),
            Some(AsyncFrameworkKind::Promise) => ("Promise", "Promise", true),
            Some(AsyncFrameworkKind::PromiseXS) => ("Promise::XS", "Promise::XS", true),
            Some(AsyncFrameworkKind::POE) => ("POE", "POE", false),
            Some(AsyncFrameworkKind::IOAsync) => ("IO::Async", "IO::Async", false),
            Some(AsyncFrameworkKind::MojoRedis) => ("Mojo::Redis", "Mojo::Redis", true),
            Some(AsyncFrameworkKind::MojoPg) => ("Mojo::Pg", "Mojo::Pg", true),
            None => return false,
        };

        let Some(name) = Self::single_symbol_name(object) else {
            return false;
        };
        if flags.async_framework == Some(AsyncFrameworkKind::AnyEvent) {
            if !matches!(
                name.as_str(),
                "AnyEvent" | "AnyEvent::CondVar" | "AnyEvent::Timer" | "AnyEvent::IO"
            ) {
                return false;
            }
        } else if exact_match {
            if name != module_name {
                return false;
            }
        } else if !name.starts_with(&format!("{module_name}::")) {
            return false;
        }

        let already_synthesized = self.table.symbols.get(&name).is_some_and(|symbols| {
            symbols.iter().any(|symbol| {
                symbol.kind == SymbolKind::Class
                    && symbol.declaration.as_deref() == Some(&format!("framework={framework_name}"))
            })
        });
        if already_synthesized {
            return true;
        }

        let framework_attr = format!("framework={framework_name}");

        self.table.add_symbol(Symbol {
            name: name.clone(),
            qualified_name: name.clone(),
            kind: SymbolKind::Class,
            location: object.location,
            scope_id: self.table.current_scope(),
            declaration: Some(framework_attr.clone()),
            documentation: Some(format!("Synthetic {framework_name} class")),
            attributes: vec![framework_attr],
        });

        true
    }

    /// Synthesize the `EV` namespace symbol when the framework is imported.
    fn synthesize_ev_framework_symbol(&mut self, location: SourceLocation) {
        let Some(flags) = self.framework_flags.get(&self.table.current_package) else {
            return;
        };
        if flags.async_framework != Some(AsyncFrameworkKind::EV) {
            return;
        }

        let name = "EV";
        if self.table.symbols.get(name).is_some_and(|symbols| {
            symbols.iter().any(|symbol| {
                symbol.kind == SymbolKind::Class
                    && symbol.declaration.as_deref() == Some("framework=EV")
            })
        }) {
            return;
        }

        self.table.add_symbol(Symbol {
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind: SymbolKind::Class,
            location,
            scope_id: self.table.current_scope(),
            declaration: Some("framework=EV".to_string()),
            documentation: Some("Synthetic EV namespace".to_string()),
            attributes: vec!["framework=EV".to_string()],
        });
    }

    /// Synthesize narrow EV watcher / loop API symbols used in function-call form.
    fn synthesize_ev_symbols(&mut self, name: &str, location: SourceLocation) -> bool {
        let Some(flags) = self.framework_flags.get(&self.table.current_package) else {
            return false;
        };
        if flags.async_framework != Some(AsyncFrameworkKind::EV) {
            return false;
        }

        let Some(ev_suffix) = name.strip_prefix("EV::") else {
            return false;
        };
        if !matches!(ev_suffix, "timer" | "io" | "signal" | "idle") {
            return false;
        }

        let already_synthesized = self.table.symbols.get(name).is_some_and(|symbols| {
            symbols.iter().any(|symbol| {
                symbol.kind == SymbolKind::Subroutine
                    && symbol.declaration.as_deref() == Some("framework=EV")
            })
        });
        if already_synthesized {
            return true;
        }

        self.table.add_symbol(Symbol {
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind: SymbolKind::Subroutine,
            location,
            scope_id: self.table.current_scope(),
            declaration: Some("framework=EV".to_string()),
            documentation: Some(format!("Synthetic EV API `{ev_suffix}`")),
            attributes: vec!["framework=EV".to_string(), format!("ev_api={ev_suffix}")],
        });

        true
    }

    /// Synthesize a narrow async framework API surface for common entrypoints.
    ///
    /// This intentionally avoids type inference. It only exposes the canonical
    /// constructor / class methods and the common chain methods that are most
    /// useful for navigation and references when a file opts into an async
    /// framework such as Future or Promise.
    fn synthesize_future_api_symbols(
        &mut self,
        object: &Node,
        method: &str,
        location: SourceLocation,
    ) -> bool {
        let Some(flags) = self.framework_flags.get(&self.table.current_package) else {
            return false;
        };

        let (framework_name, root_name, chain_methods, class_entrypoints) =
            match flags.async_framework {
                Some(AsyncFrameworkKind::Future) => (
                    "Future",
                    "Future",
                    vec!["then", "catch", "finally", "get", "is_done", "is_ready"],
                    vec!["new", "done", "fail", "wait_all", "needs_all", "needs_any"],
                ),
                Some(AsyncFrameworkKind::FutureXS) => (
                    "Future::XS",
                    "Future::XS",
                    vec!["then", "catch", "finally", "get", "is_done", "is_ready"],
                    vec!["new", "done", "fail", "wait_all", "needs_all", "needs_any"],
                ),
                Some(AsyncFrameworkKind::Promise) => (
                    "Promise",
                    "Promise",
                    vec!["then", "catch", "finally", "resolve", "reject"],
                    vec!["new", "all", "race", "any"],
                ),
                Some(AsyncFrameworkKind::PromiseXS) => (
                    "Promise::XS",
                    "Promise::XS",
                    vec!["then", "catch", "finally", "resolve", "reject"],
                    vec!["new", "all", "race", "any"],
                ),
                _ => return false,
            };

        let object_name = Self::single_symbol_name(object);

        let should_synthesize = if chain_methods.contains(&method) {
            true
        } else if class_entrypoints.contains(&method) {
            object_name.is_some_and(|name| name == root_name)
        } else {
            false
        };
        if !should_synthesize {
            return false;
        }

        let already_synthesized = self.table.symbols.get(method).is_some_and(|symbols| {
            symbols.iter().any(|symbol| {
                symbol.kind == SymbolKind::Subroutine
                    && symbol.declaration.as_deref() == Some(&format!("framework={framework_name}"))
                    && symbol.attributes.iter().any(|attr| attr == &format!("future_api={method}"))
            })
        });
        if already_synthesized {
            return true;
        }

        self.table.add_symbol(Symbol {
            name: method.to_string(),
            qualified_name: format!("{framework_name}::{method}"),
            kind: SymbolKind::Subroutine,
            location,
            scope_id: self.table.current_scope(),
            declaration: Some(format!("framework={framework_name}")),
            documentation: Some(format!("Synthetic {framework_name} API `{method}`")),
            attributes: vec![format!("framework={framework_name}"), format!("future_api={method}")],
        });

        true
    }

    /// Update framework detection state from `use` statements.
    fn update_framework_context(&mut self, module: &str, args: &[String]) {
        let pkg = self.table.current_package.clone();

        let framework_kind = match module {
            "Moo" | "Mouse" => Some(FrameworkKind::Moo),
            "Moo::Role" | "Mouse::Role" => Some(FrameworkKind::MooRole),
            "Moose" => Some(FrameworkKind::Moose),
            "Moose::Role" => Some(FrameworkKind::MooseRole),
            _ => None,
        };

        if let Some(kind) = framework_kind {
            let flags = self.framework_flags.entry(pkg.clone()).or_default();
            flags.moo = true;
            flags.kind = Some(kind);
            return;
        }

        if module == "Class::Accessor" {
            self.framework_flags.entry(pkg.clone()).or_default().class_accessor = true;
            return;
        }

        let web_kind = match module {
            "Dancer" => Some(WebFrameworkKind::Dancer),
            "Dancer2" | "Dancer2::Core" => Some(WebFrameworkKind::Dancer2),
            "Mojolicious::Lite" => Some(WebFrameworkKind::MojoliciousLite),
            "Plack::Builder" => Some(WebFrameworkKind::PlackBuilder),
            _ => None,
        };
        if let Some(kind) = web_kind {
            self.framework_flags.entry(pkg.clone()).or_default().web_framework = Some(kind);
            return;
        }

        if module == "IO::Async" || module.starts_with("IO::Async::") {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::IOAsync);
            return;
        }

        if module == "AnyEvent" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::AnyEvent);
            return;
        }

        if module == "EV" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::EV);
            return;
        }

        if module == "Future" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::Future);
            return;
        }

        if module == "Future::XS" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::FutureXS);
            return;
        }

        if module == "Promise" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::Promise);
            return;
        }

        if module == "Promise::XS" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::PromiseXS);
            return;
        }

        if module == "POE" || module.starts_with("POE::") {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::POE);
            return;
        }

        if module == "Mojo::Redis" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::MojoRedis);
            return;
        }

        if module == "Mojo::Pg" {
            self.framework_flags.entry(pkg.clone()).or_default().async_framework =
                Some(AsyncFrameworkKind::MojoPg);
            return;
        }

        if matches!(module, "base" | "parent") {
            let has_class_accessor_parent = args
                .iter()
                .filter_map(|arg| Self::normalize_symbol_name(arg))
                .any(|arg| arg == "Class::Accessor");
            if has_class_accessor_parent {
                self.framework_flags.entry(pkg.clone()).or_default().class_accessor = true;
            }
            let has_catalyst_controller_parent = args
                .iter()
                .filter_map(|arg| Self::normalize_symbol_name(arg))
                .any(|arg| arg == "Catalyst::Controller");
            if has_catalyst_controller_parent {
                self.mark_catalyst_controller_package(&pkg);
            }
        }
    }

    fn mark_catalyst_controller_package(&mut self, package: &str) {
        self.framework_flags.entry(package.to_string()).or_default().catalyst_controller = true;
    }

    fn current_package_is_catalyst_controller(&self) -> bool {
        self.framework_flags
            .get(&self.table.current_package)
            .is_some_and(|flags| flags.catalyst_controller)
            || Self::is_catalyst_controller_package_name(&self.table.current_package)
    }

    fn is_catalyst_controller_package_name(package: &str) -> bool {
        package.contains("::Controller::") || package.ends_with("::Controller")
    }

    fn catalyst_action_metadata(attributes: &[String]) -> Option<(String, Vec<String>)> {
        let mut kind = None;
        let mut details = Vec::new();
        let mut seen = HashSet::new();

        for attr in attributes {
            let attr_name = Self::attribute_base_name(attr);
            if !Self::is_catalyst_action_attribute(&attr_name) {
                continue;
            }

            if kind.is_none()
                || matches!(kind.as_deref(), Some("Args" | "CaptureArgs" | "PathPart"))
            {
                if matches!(attr_name.as_str(), "Path" | "Local" | "Global" | "Regex" | "Chained") {
                    kind = Some(attr_name.clone());
                } else if kind.is_none() {
                    kind = Some(attr_name.clone());
                }
            }

            if seen.insert(attr.clone()) {
                details.push(attr.clone());
            }
        }

        if let Some(action_kind) = kind.as_deref()
            && matches!(action_kind, "Path" | "Local" | "Global" | "Regex" | "Chained")
        {
            details.retain(|attr| Self::attribute_base_name(attr) != action_kind);
        }

        kind.map(|kind| (kind, details))
    }

    fn is_catalyst_action_attribute(attr_name: &str) -> bool {
        matches!(
            attr_name,
            "Path" | "Local" | "Global" | "Regex" | "Chained" | "PathPart" | "Args" | "CaptureArgs"
        )
    }

    fn attribute_base_name(attr: &str) -> String {
        attr.trim_start_matches(':')
            .split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == ':'))
            .next()
            .unwrap_or("")
            .to_string()
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

    /// Build a documentation string for a generated Moo/Moose accessor method.
    ///
    /// Includes the `isa` type constraint and access mode when present in the
    /// option map, producing hover-friendly documentation such as:
    ///
    /// ```text
    /// Moo/Moose accessor (isa: Str, ro)
    /// ```
    fn moo_accessor_doc(option_map: &HashMap<String, String>) -> String {
        let mut parts = Vec::new();

        if let Some(isa) = option_map.get("isa") {
            parts.push(format!("isa: {isa}"));
        }
        if let Some(is) = option_map.get("is") {
            parts.push(is.clone());
        }

        if parts.is_empty() {
            "Generated accessor from Moo/Moose `has`".to_string()
        } else {
            format!("Moo/Moose accessor ({})", parts.join(", "))
        }
    }

    /// Compute accessor method names for a Moo/Moose `has` declaration.
    fn moo_accessor_names(
        attribute_names: &[String],
        option_map: &HashMap<String, String>,
        options_expr: &Node,
    ) -> Vec<String> {
        let mut methods = Vec::new();
        let mut seen = HashSet::new();

        for key in ["accessor", "reader", "writer", "predicate", "clearer", "builder"] {
            for name in Self::option_method_names(options_expr, key, attribute_names) {
                if seen.insert(name.clone()) {
                    methods.push(name);
                }
            }
        }

        for name in Self::handles_method_names(options_expr) {
            if seen.insert(name.clone()) {
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

    /// Find an option value node inside a hash-literal options list.
    fn find_hash_option_value<'a>(options_expr: &'a Node, key: &str) -> Option<&'a Node> {
        let NodeKind::HashLiteral { pairs } = &options_expr.kind else {
            return None;
        };

        for (key_node, value_node) in pairs {
            if Self::single_symbol_name(key_node).as_deref() == Some(key) {
                return Some(value_node);
            }
        }

        None
    }

    /// Compute method names from a single Moo/Moose option key.
    fn option_method_names(
        options_expr: &Node,
        key: &str,
        attribute_names: &[String],
    ) -> Vec<String> {
        let Some(value_node) = Self::find_hash_option_value(options_expr, key) else {
            return Vec::new();
        };

        let mut names = Self::collect_symbol_names(value_node);
        if !names.is_empty() {
            names.sort();
            names.dedup();
            return names;
        }

        // Moo/Moose shorthand: `predicate => 1`, `clearer => 1`, `builder => 1`.
        if !Self::is_truthy_shorthand(value_node) {
            return Vec::new();
        }

        match key {
            "predicate" => attribute_names.iter().map(|name| format!("has_{name}")).collect(),
            "clearer" => attribute_names.iter().map(|name| format!("clear_{name}")).collect(),
            "builder" => attribute_names.iter().map(|name| format!("_build_{name}")).collect(),
            _ => Vec::new(),
        }
    }

    /// Determine if an option node is a static truthy shorthand literal (`1`, `true`, `'1'`).
    fn is_truthy_shorthand(node: &Node) -> bool {
        match &node.kind {
            NodeKind::Number { value } => value.trim() == "1",
            NodeKind::Identifier { name } => {
                let lower = name.trim().to_ascii_lowercase();
                lower == "1" || lower == "true"
            }
            NodeKind::String { value, .. } => {
                Self::normalize_symbol_name(value).is_some_and(|value| {
                    let lower = value.to_ascii_lowercase();
                    value == "1" || lower == "true"
                })
            }
            _ => false,
        }
    }

    /// Extract delegated method names from a Moo/Moose `handles` option.
    fn handles_method_names(options_expr: &Node) -> Vec<String> {
        let Some(handles_node) = Self::find_hash_option_value(options_expr, "handles") else {
            return Vec::new();
        };

        let mut names = Vec::new();
        match &handles_node.kind {
            NodeKind::HashLiteral { pairs } => {
                for (key_node, _) in pairs {
                    names.extend(Self::collect_symbol_names(key_node));
                }
            }
            _ => {
                names.extend(Self::collect_symbol_names(handles_node));
            }
        }

        names.sort();
        names.dedup();
        names
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
            NodeKind::Variable { sigil, name } => format!("{sigil}{name}"),
            NodeKind::Number { value } => value.clone(),
            NodeKind::ArrayLiteral { elements } => {
                let mut entries = Vec::new();
                for element in elements {
                    entries.extend(Self::collect_symbol_names(element));
                }
                entries.sort();
                entries.dedup();
                if entries.is_empty() {
                    "array".to_string()
                } else {
                    format!("[{}]", entries.join(","))
                }
            }
            NodeKind::HashLiteral { pairs } => {
                let mut entries = Vec::new();
                for (key_node, value_node) in pairs {
                    let Some(key_name) = Self::single_symbol_name(key_node) else {
                        continue;
                    };
                    if let Some(value_name) = Self::single_symbol_name(value_node) {
                        entries.push(format!("{key_name}->{value_name}"));
                    } else {
                        entries.push(key_name);
                    }
                }
                entries.sort();
                entries.dedup();
                if entries.is_empty() {
                    "hash".to_string()
                } else {
                    format!("{{{}}}", entries.join(","))
                }
            }
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

    /// Extract documentation for a package declaration.
    ///
    /// Looks for:
    /// 1. A POD `=head1 NAME` section that mentions the package name
    /// 2. Leading comments immediately before the `package` statement
    /// 3. An `=head1 DESCRIPTION` section as fallback
    fn extract_package_documentation(
        &self,
        package_name: &str,
        location: SourceLocation,
    ) -> Option<String> {
        // First try leading comments (cheapest check)
        let leading = self.extract_leading_comment(location.start);
        if leading.is_some() {
            return leading;
        }

        // Then search for POD NAME section in the source text
        if self.source.is_empty() {
            return None;
        }

        // Look for =head1 NAME section anywhere in the file
        let mut in_name_section = false;
        let mut name_lines: Vec<&str> = Vec::new();

        for line in self.source.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("=head1") {
                if in_name_section {
                    // We hit the next =head1, stop collecting
                    break;
                }
                let heading = trimmed.strip_prefix("=head1").map(|s| s.trim());
                if heading == Some("NAME") {
                    in_name_section = true;
                    continue;
                }
            } else if trimmed.starts_with("=cut") && in_name_section {
                break;
            } else if trimmed.starts_with('=') && in_name_section {
                // Any other POD directive ends the NAME section
                break;
            } else if in_name_section && !trimmed.is_empty() {
                name_lines.push(trimmed);
            }
        }

        if !name_lines.is_empty() {
            let name_doc = name_lines.join(" ");
            // Only return if the NAME section actually references this package
            if name_doc.contains(package_name)
                || name_doc.contains(&package_name.replace("::", "-"))
            {
                return Some(name_doc);
            }
        }

        None
    }

    /// Register signature parameters as implicit `my` variable declarations in the current scope.
    ///
    /// Handles `MandatoryParameter`, `OptionalParameter`, `SlurpyParameter`, and
    /// `NamedParameter` nodes by extracting the inner variable and registering it
    /// exactly as if the user had written `my $x` at the top of the subroutine body.
    fn register_signature_params(&mut self, sig: &Node) {
        let NodeKind::Signature { parameters } = &sig.kind else {
            return;
        };
        for param in parameters {
            let variable = match &param.kind {
                NodeKind::MandatoryParameter { variable } => variable.as_ref(),
                NodeKind::OptionalParameter { variable, .. } => variable.as_ref(),
                NodeKind::SlurpyParameter { variable } => variable.as_ref(),
                NodeKind::NamedParameter { variable } => variable.as_ref(),
                // Unexpected node kind inside a signature — skip gracefully
                _ => continue,
            };
            self.handle_variable_declaration("my", variable, &[], variable.location, None);
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

    fn try_extract_const_fast_declaration(&mut self, args: &[Node]) -> bool {
        let mut matched = false;

        for arg in args {
            match &arg.kind {
                NodeKind::VariableDeclaration { variable, .. } => {
                    if self.add_const_fast_symbol(variable, &[]) {
                        matched = true;
                    }
                }
                NodeKind::VariableListDeclaration { variables, .. } => {
                    let mut saw_decl = false;
                    for variable in variables {
                        if self.add_const_fast_symbol(variable, &[]) {
                            saw_decl = true;
                        }
                    }
                    matched |= saw_decl;
                }
                _ => self.visit_node(arg),
            }
        }

        matched
    }

    fn add_const_fast_symbol(&mut self, variable: &Node, attributes: &[String]) -> bool {
        match &variable.kind {
            NodeKind::Variable { name, .. } => {
                self.table.add_symbol(Symbol {
                    name: name.clone(),
                    qualified_name: name.clone(),
                    kind: SymbolKind::Constant,
                    location: variable.location,
                    scope_id: self.table.current_scope(),
                    declaration: Some("const".to_string()),
                    documentation: Some("Const::Fast read-only variable".to_string()),
                    attributes: attributes.to_vec(),
                });
                true
            }
            NodeKind::VariableWithAttributes { variable, attributes: inner_attributes } => {
                let mut merged = attributes.to_vec();
                merged.extend(inner_attributes.iter().cloned());
                self.add_const_fast_symbol(variable, &merged)
            }
            _ => false,
        }
    }

    fn register_catch_variable(&mut self, full_name: &str, catch_block_location: SourceLocation) {
        let (sigil, name) = split_variable_name(full_name);
        let kind = match sigil {
            "$" => SymbolKind::scalar(),
            "@" => SymbolKind::array(),
            "%" => SymbolKind::hash(),
            _ => return,
        };
        if name.is_empty() || name.contains("::") {
            return;
        }

        let location = self
            .find_catch_variable_location(catch_block_location.start, full_name)
            .unwrap_or(SourceLocation {
                start: catch_block_location.start,
                end: catch_block_location.start,
            });

        self.table.add_symbol(Symbol {
            name: name.to_string(),
            qualified_name: name.to_string(),
            kind,
            location,
            scope_id: self.table.current_scope(),
            declaration: Some("my".to_string()),
            documentation: Some("Exception variable bound by catch".to_string()),
            attributes: vec![],
        });
    }

    fn find_catch_variable_location(
        &self,
        catch_body_start: usize,
        full_name: &str,
    ) -> Option<SourceLocation> {
        if self.source.is_empty()
            || full_name.is_empty()
            || catch_body_start == 0
            || catch_body_start > self.source.len()
        {
            return None;
        }

        let window_start = catch_body_start.saturating_sub(256);
        let window = self.source.get(window_start..catch_body_start)?;
        let catch_start = window.rfind("catch")?;
        let search_start = catch_start + "catch".len();
        let var_offset = window[search_start..].rfind(full_name)? + search_start;
        let start = window_start + var_offset;
        let end = start + full_name.len();

        Some(SourceLocation { start, end })
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
        static SCALAR_RE: OnceLock<Result<Regex, regex::Error>> = OnceLock::new();

        // Simple regex to find scalar variables in strings
        // This handles $var, ${var}, but not arrays/hashes for now
        let scalar_re = match SCALAR_RE
            .get_or_init(|| Regex::new(r"\$([a-zA-Z_]\w*|\{[a-zA-Z_]\w*\})"))
            .as_ref()
        {
            Ok(re) => re,
            Err(_) => return, // Skip variable extraction if regex fails
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

fn split_variable_name(full_name: &str) -> (&str, &str) {
    full_name
        .char_indices()
        .next()
        .map(|(idx, ch)| (&full_name[idx..idx + ch.len_utf8()], &full_name[idx + ch.len_utf8()..]))
        .unwrap_or(("", ""))
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

    // ── Bug 3 test: NodeKind::Method uses SymbolKind::Method not Subroutine ──

    #[test]
    fn test_method_node_uses_symbol_kind_method() {
        let code = r#"
class MyClass {
    method greet {
        return "hello";
    }
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        assert!(table.symbols.contains_key("greet"), "expected 'greet' in symbol table");
        let greet_symbols = &table.symbols["greet"];
        assert_eq!(greet_symbols.len(), 1);
        assert_eq!(
            greet_symbols[0].kind,
            SymbolKind::Method,
            "NodeKind::Method should produce SymbolKind::Method, not Subroutine"
        );
        // Also verify the method attribute was pushed
        assert!(
            greet_symbols[0].attributes.contains(&"method".to_string()),
            "method symbol should have 'method' attribute"
        );
    }

    // ── Issue #3361: signature parameters added to symbol table ──

    #[test]
    fn test_subroutine_mandatory_params_in_symbol_table() {
        let code = r#"
sub foo ($x, $y) {
    return $x + $y;
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        assert!(
            table.symbols.contains_key("x"),
            "mandatory parameter $x should be in the symbol table"
        );
        assert!(
            table.symbols.contains_key("y"),
            "mandatory parameter $y should be in the symbol table"
        );

        let x_symbols = &table.symbols["x"];
        assert_eq!(x_symbols.len(), 1);
        assert_eq!(
            x_symbols[0].declaration,
            Some("my".to_string()),
            "$x should be declared as 'my'"
        );

        let y_symbols = &table.symbols["y"];
        assert_eq!(y_symbols.len(), 1);
        assert_eq!(
            y_symbols[0].declaration,
            Some("my".to_string()),
            "$y should be declared as 'my'"
        );
    }

    #[test]
    fn test_subroutine_optional_param_in_symbol_table() {
        let code = r#"
sub bar ($x, $y = 0) {
    return $x + $y;
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        assert!(
            table.symbols.contains_key("x"),
            "mandatory parameter $x should be in the symbol table"
        );
        assert!(
            table.symbols.contains_key("y"),
            "optional parameter $y should be in the symbol table"
        );
        assert_eq!(
            table.symbols["y"][0].declaration,
            Some("my".to_string()),
            "optional parameter $y should be declared as 'my'"
        );
    }

    #[test]
    fn test_subroutine_slurpy_param_in_symbol_table() {
        let code = r#"
sub baz ($x, @rest) {
    return scalar @rest;
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        assert!(
            table.symbols.contains_key("x"),
            "mandatory parameter $x should be in the symbol table"
        );
        assert!(
            table.symbols.contains_key("rest"),
            "slurpy parameter @rest should be in the symbol table"
        );
        assert_eq!(
            table.symbols["rest"][0].declaration,
            Some("my".to_string()),
            "slurpy parameter @rest should be declared as 'my'"
        );
    }

    #[test]
    fn test_method_signature_params_in_symbol_table() {
        let code = r#"
class Foo {
    method greet ($name) {
        return $name;
    }
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        assert!(
            table.symbols.contains_key("name"),
            "method signature parameter $name should be in the symbol table"
        );
        assert_eq!(
            table.symbols["name"][0].declaration,
            Some("my".to_string()),
            "method parameter $name should be declared as 'my'"
        );
    }

    #[test]
    fn test_empty_signature_no_crash() {
        // Edge case: empty signature `sub foo () { }` — should not crash and
        // should leave the symbol table with only the sub itself, not any param.
        let code = r#"
sub foo () {
    return 1;
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        // Sub `foo` is registered as a symbol
        assert!(table.symbols.contains_key("foo"), "sub foo should be in the symbol table");
        // No spurious variable symbols from an empty signature
        assert_eq!(
            table.symbols.len(),
            1,
            "only 'foo' should be in the symbol table for an empty-signature sub"
        );
    }

    #[test]
    fn test_hash_slurpy_param_in_symbol_table() {
        // Edge case: hash slurpy `%opts` — sigil % maps to SymbolKind::hash()
        let code = r#"
sub configure ($x, %opts) {
    return $opts{key};
}
"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        assert!(
            table.symbols.contains_key("opts"),
            "hash slurpy parameter %opts should be in the symbol table"
        );
        assert_eq!(
            table.symbols["opts"][0].declaration,
            Some("my".to_string()),
            "hash slurpy parameter %opts should be declared as 'my'"
        );
    }

    #[test]
    fn test_optional_param_location_is_variable_span() {
        // The symbol location for an optional param `$y = 0` should span just
        // the variable `$y`, not the entire `$y = 0` expression.  Callers like
        // go-to-definition use this span to highlight the declaration site.
        let code = "sub bar ($x, $y = 0) { $x + $y }";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(code);
        let table = extractor.extract(&ast);

        // `$y` starts at offset 13 in "sub bar ($x, $y = 0)"
        //                                            ^ offset 13
        let y_sym = &table.symbols["y"][0];
        let span_len = y_sym.location.end - y_sym.location.start;
        // The variable node "$y" is 2 bytes; the full param "$y = 0" is 6 bytes.
        assert_eq!(
            span_len, 2,
            "symbol location should cover just '$y' (2 chars), not the full '$y = 0' (6 chars)"
        );
    }
}
