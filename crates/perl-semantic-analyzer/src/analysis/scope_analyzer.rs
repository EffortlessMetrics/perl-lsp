//! Scope analysis and variable tracking for Perl parsing workflows
//!
//! This module provides comprehensive scope analysis for Perl scripts, tracking
//! variable declarations, usage patterns, and potential issues across different
//! scopes within the LSP workflow stages.
//!
//! # LSP Workflow Integration
//!
//! Scope analysis supports semantic validation across LSP workflow stages:
//! - **Parse**: Identify declarations and scopes during syntax analysis
//! - **Index**: Provide scope metadata for symbol indexing
//! - **Navigate**: Resolve references with scope-aware lookups
//! - **Complete**: Filter completion items based on visible bindings
//! - **Analyze**: Report unused, shadowed, and undeclared variables
//!
//! # Performance
//!
//! - **Time complexity**: O(n) over AST nodes with scoped hash lookups
//! - **Space complexity**: O(n) for scope tables and variable maps (memory bounded)
//! - **Optimizations**: Fast sigil indexing to keep performance stable
//! - **Benchmarks**: Typically <5ms for mid-sized files, low ms for large files
//! - **Large file scaling**: Designed to scale across large file sets in workspaces
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_parser::scope_analyzer::{ScopeAnalyzer, IssueKind};
//! use perl_parser::{Parser, ast::Node};
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Analyze Perl script for scope issues
//! let script = "my $var = 42; sub hello { print $var; }";
//! let mut parser = Parser::new(script);
//! let ast = parser.parse()?;
//!
//! let analyzer = ScopeAnalyzer::new();
//! let pragma_map = vec![];
//! let issues = analyzer.analyze(&ast, script, &pragma_map);
//!
//! // Check for common scope issues in Perl parsing code
//! for issue in &issues {
//!     match issue.kind {
//!         IssueKind::UnusedVariable => println!("Unused variable: {}", issue.variable_name),
//!         IssueKind::VariableShadowing => println!("Variable shadowing: {}", issue.variable_name),
//!         _ => {}
//!     }
//! }
//! # Ok(())
//! # }
//! ```

use crate::ast::{Node, NodeKind};
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use perl_parser_core::builtin_signatures_phf;
use rustc_hash::FxHashMap;
use std::cell::RefCell;
use std::ops::Range;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IssueKind {
    VariableShadowing,
    UnusedVariable,
    UndeclaredVariable,
    VariableRedeclaration,
    DuplicateParameter,
    ParameterShadowsGlobal,
    UnusedParameter,
    UnquotedBareword,
    UninitializedVariable,
}

#[derive(Debug, Clone)]
pub struct ScopeIssue {
    pub kind: IssueKind,
    pub variable_name: String,
    pub line: usize,
    pub range: (usize, usize),
    pub description: String,
}

#[derive(Debug)]
struct Variable {
    declaration_offset: usize,
    is_used: RefCell<bool>,
    is_our: bool,
    is_initialized: RefCell<bool>,
}

/// Convert a Perl sigil to an array index for fast variable lookup.
///
/// Sigil indices:
/// - `$` (scalar): 0
/// - `@` (array): 1
/// - `%` (hash): 2
/// - `&` (subroutine): 3
/// - `*` (glob): 4
/// - Other: 5 (fallback)
#[inline]
fn sigil_to_index(sigil: &str) -> usize {
    // Use first byte for fast comparison - sigils are always single ASCII chars
    match sigil.as_bytes().first() {
        Some(b'$') => 0,
        Some(b'@') => 1,
        Some(b'%') => 2,
        Some(b'&') => 3,
        Some(b'*') => 4,
        _ => 5,
    }
}

/// Convert an array index back to a Perl sigil.
#[inline]
fn index_to_sigil(index: usize) -> &'static str {
    match index {
        0 => "$",
        1 => "@",
        2 => "%",
        3 => "&",
        4 => "*",
        _ => "",
    }
}

#[derive(Debug)]
struct Scope {
    // Outer key: sigil index, Inner key: name
    variables: RefCell<[Option<FxHashMap<String, Rc<Variable>>>; 6]>,
    parent: Option<Rc<Scope>>,
}

impl Scope {
    fn new() -> Self {
        let vars = std::array::from_fn(|_| None);
        Self { variables: RefCell::new(vars), parent: None }
    }

    fn with_parent(parent: Rc<Scope>) -> Self {
        let vars = std::array::from_fn(|_| None);
        Self { variables: RefCell::new(vars), parent: Some(parent) }
    }

    fn declare_variable_parts(
        &self,
        sigil: &str,
        name: &str,
        offset: usize,
        is_our: bool,
        is_initialized: bool,
    ) -> Option<IssueKind> {
        let idx = sigil_to_index(sigil);

        // First check if already declared in this scope
        {
            let vars = self.variables.borrow();
            if let Some(map) = &vars[idx] {
                if map.contains_key(name) {
                    return Some(IssueKind::VariableRedeclaration);
                }
            }
        }

        // Check if it shadows a parent scope variable
        let shadows = if let Some(ref parent) = self.parent {
            parent.has_variable_parts(sigil, name)
        } else {
            false
        };

        // Now insert the variable
        let mut vars = self.variables.borrow_mut();
        let inner = vars[idx].get_or_insert_with(FxHashMap::default);

        inner.insert(
            name.to_string(),
            Rc::new(Variable {
                declaration_offset: offset,
                is_used: RefCell::new(is_our), // 'our' variables are considered used
                is_our,
                is_initialized: RefCell::new(is_initialized),
            }),
        );

        if shadows { Some(IssueKind::VariableShadowing) } else { None }
    }

    fn lookup_variable_parts(&self, sigil: &str, name: &str) -> Option<Rc<Variable>> {
        let idx = sigil_to_index(sigil);
        let mut current_scope = self;

        loop {
            if let Some(map) = &current_scope.variables.borrow()[idx] {
                if let Some(var) = map.get(name) {
                    return Some(var.clone());
                }
            }
            if let Some(ref parent) = current_scope.parent {
                current_scope = parent;
            } else {
                break;
            }
        }
        None
    }

    fn has_variable_parts(&self, sigil: &str, name: &str) -> bool {
        let idx = sigil_to_index(sigil);
        let mut current_scope = self;

        loop {
            {
                let vars = current_scope.variables.borrow();
                if let Some(map) = &vars[idx] {
                    if map.contains_key(name) {
                        return true;
                    }
                }
            }
            if let Some(ref parent) = current_scope.parent {
                current_scope = parent;
            } else {
                return false;
            }
        }
    }

    fn use_variable_parts(&self, sigil: &str, name: &str) -> (bool, bool) {
        let idx = sigil_to_index(sigil);
        let mut current_scope = self;

        loop {
            {
                let vars = current_scope.variables.borrow();
                if let Some(map) = &vars[idx] {
                    if let Some(var) = map.get(name) {
                        *var.is_used.borrow_mut() = true;
                        return (true, *var.is_initialized.borrow());
                    }
                }
            }

            if let Some(ref parent) = current_scope.parent {
                current_scope = parent;
            } else {
                return (false, false);
            }
        }
    }

    fn initialize_variable_parts(&self, sigil: &str, name: &str) {
        let idx = sigil_to_index(sigil);
        let mut current_scope = self;

        loop {
            {
                let vars = current_scope.variables.borrow();
                if let Some(map) = &vars[idx] {
                    if let Some(var) = map.get(name) {
                        *var.is_initialized.borrow_mut() = true;
                        return;
                    }
                }
            }

            if let Some(ref parent) = current_scope.parent {
                current_scope = parent;
            } else {
                return;
            }
        }
    }

    /// Optimized method to mark a variable as initialized AND used in one lookup.
    /// Returns true if the variable was found and updated.
    fn initialize_and_use_variable_parts(&self, sigil: &str, name: &str) -> bool {
        let idx = sigil_to_index(sigil);
        let mut current_scope = self;

        loop {
            {
                let vars = current_scope.variables.borrow();
                if let Some(map) = &vars[idx] {
                    if let Some(var) = map.get(name) {
                        *var.is_used.borrow_mut() = true;
                        *var.is_initialized.borrow_mut() = true;
                        return true;
                    }
                }
            }

            if let Some(ref parent) = current_scope.parent {
                current_scope = parent;
            } else {
                return false;
            }
        }
    }

    /// Iterate over unused variables that should be reported as diagnostics.
    /// Filters out underscore-prefixed variables (intentionally unused) before allocation.
    fn for_each_reportable_unused_variable<F>(&self, mut f: F)
    where
        F: FnMut(String, usize),
    {
        for (idx, inner_opt) in self.variables.borrow().iter().enumerate() {
            if let Some(inner) = inner_opt {
                for (name, var) in inner {
                    if !*var.is_used.borrow() && !var.is_our {
                        // Optimization: Check for underscore prefix before allocation
                        if name.starts_with('_') {
                            continue;
                        }
                        let full_name = format!("{}{}", index_to_sigil(idx), name);
                        f(full_name, var.declaration_offset);
                    }
                }
            }
        }
    }
}

/// Helper to split a full variable name into sigil and name parts.
fn split_variable_name(full_name: &str) -> (&str, &str) {
    if !full_name.is_empty() {
        let c = full_name.as_bytes()[0];
        if c == b'$' || c == b'@' || c == b'%' || c == b'&' || c == b'*' {
            return (&full_name[0..1], &full_name[1..]);
        }
    }
    ("", full_name)
}

enum ExtractedName<'a> {
    Parts(&'a str, &'a str),
    Full(String),
}

struct AnalysisContext<'a> {
    code: &'a str,
    pragma_map: &'a [(Range<usize>, PragmaState)],
    line_starts: RefCell<Option<Vec<usize>>>,
}

impl<'a> AnalysisContext<'a> {
    fn new(code: &'a str, pragma_map: &'a [(Range<usize>, PragmaState)]) -> Self {
        Self { code, pragma_map, line_starts: RefCell::new(None) }
    }

    fn get_line(&self, offset: usize) -> usize {
        let mut line_starts_guard = self.line_starts.borrow_mut();
        let starts = line_starts_guard.get_or_insert_with(|| {
            let mut indices = Vec::with_capacity(self.code.len() / 40); // Estimate
            indices.push(0);
            for (i, b) in self.code.bytes().enumerate() {
                if b == b'\n' {
                    indices.push(i + 1);
                }
            }
            indices
        });

        // Find the line that contains the offset
        match starts.binary_search(&offset) {
            Ok(idx) => idx + 1,
            Err(idx) => idx,
        }
    }
}

impl<'a> ExtractedName<'a> {
    fn as_string(&self) -> String {
        match self {
            ExtractedName::Parts(sigil, name) => format!("{}{}", sigil, name),
            ExtractedName::Full(s) => s.clone(),
        }
    }

    fn parts(&self) -> (&str, &str) {
        match self {
            ExtractedName::Parts(sigil, name) => (sigil, name),
            ExtractedName::Full(s) => split_variable_name(s),
        }
    }

    fn is_empty(&self) -> bool {
        match self {
            ExtractedName::Parts(sigil, name) => sigil.is_empty() && name.is_empty(),
            ExtractedName::Full(s) => s.is_empty(),
        }
    }
}

pub struct ScopeAnalyzer;

impl Default for ScopeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeAnalyzer {
    pub fn new() -> Self {
        Self
    }

    pub fn analyze(
        &self,
        ast: &Node,
        code: &str,
        pragma_map: &[(Range<usize>, PragmaState)],
    ) -> Vec<ScopeIssue> {
        let mut issues = Vec::new();
        let root_scope = Rc::new(Scope::new());

        // Use a vector as a stack for ancestors to avoid O(N) HashMap allocation
        let mut ancestors: Vec<&Node> = Vec::new();

        let context = AnalysisContext::new(code, pragma_map);

        self.analyze_node(ast, &root_scope, &mut ancestors, &mut issues, &context);

        // Collect all unused variables from all scopes
        self.collect_unused_variables(&root_scope, &mut issues, &context);

        issues
    }

    fn analyze_node<'a>(
        &self,
        node: &'a Node,
        scope: &Rc<Scope>,
        ancestors: &mut Vec<&'a Node>,
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'a>,
    ) {
        // Get effective pragma state at this node's location
        let pragma_state = PragmaTracker::state_for_offset(context.pragma_map, node.location.start);
        let strict_mode = pragma_state.strict_subs;
        match &node.kind {
            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                let extracted = self.extract_variable_name(variable);
                let (sigil, var_name_part) = extracted.parts();

                let is_our = declarator == "our";
                let is_initialized = initializer.is_some();

                // If checking initializer first (e.g. my $x = $x), we need to analyze initializer in
                // current scope BEFORE declaring the variable (standard Perl behavior)
                // Actually Perl evaluates RHS before LHS assignment, so usages in initializer refer to OUTER scope.
                // So we analyze initializer first.
                if let Some(init) = initializer {
                    self.analyze_node(init, scope, ancestors, issues, context);
                }

                if let Some(issue_kind) = scope.declare_variable_parts(
                    sigil,
                    var_name_part,
                    variable.location.start,
                    is_our,
                    is_initialized,
                ) {
                    let line = context.get_line(variable.location.start);
                    // Optimization: Only allocate full name string when we actually have an issue to report
                    let full_name = extracted.as_string();
                    // Build description first (borrows full_name), then move full_name into struct
                    let description = match issue_kind {
                        IssueKind::VariableShadowing => {
                            format!("Variable '{}' shadows a variable in outer scope", full_name)
                        }
                        IssueKind::VariableRedeclaration => {
                            format!("Variable '{}' is already declared in this scope", full_name)
                        }
                        _ => String::new(),
                    };
                    issues.push(ScopeIssue {
                        kind: issue_kind,
                        variable_name: full_name,
                        line,
                        range: (variable.location.start, variable.location.end),
                        description,
                    });
                }
            }

            NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
                let is_our = declarator == "our";
                let is_initialized = initializer.is_some();

                // Analyze initializer first
                if let Some(init) = initializer {
                    self.analyze_node(init, scope, ancestors, issues, context);
                }

                for variable in variables {
                    let extracted = self.extract_variable_name(variable);
                    let (sigil, var_name_part) = extracted.parts();

                    if let Some(issue_kind) = scope.declare_variable_parts(
                        sigil,
                        var_name_part,
                        variable.location.start,
                        is_our,
                        is_initialized,
                    ) {
                        let line = context.get_line(variable.location.start);
                        // Optimization: Only allocate full name string when we actually have an issue to report
                        let full_name = extracted.as_string();
                        // Build description first (borrows full_name), then move full_name into struct
                        let description = match issue_kind {
                            IssueKind::VariableShadowing => {
                                format!(
                                    "Variable '{}' shadows a variable in outer scope",
                                    full_name
                                )
                            }
                            IssueKind::VariableRedeclaration => {
                                format!(
                                    "Variable '{}' is already declared in this scope",
                                    full_name
                                )
                            }
                            _ => String::new(),
                        };
                        issues.push(ScopeIssue {
                            kind: issue_kind,
                            variable_name: full_name,
                            line,
                            range: (variable.location.start, variable.location.end),
                            description,
                        });
                    }
                }
            }

            NodeKind::Use { module, args, .. } => {
                // Handle 'use vars' pragma for global variable declarations
                if module == "vars" {
                    for arg in args {
                        // Parse qw() style arguments to extract individual variable names
                        if arg.starts_with("qw(") && arg.ends_with(")") {
                            let content = &arg[3..arg.len() - 1]; // Remove qw( and )
                            for var_name in content.split_whitespace() {
                                if !var_name.is_empty() {
                                    let (sigil, name) = split_variable_name(var_name);
                                    if !sigil.is_empty() {
                                        // Declare these variables as globals in the current scope
                                        scope.declare_variable_parts(
                                            sigil,
                                            name,
                                            node.location.start,
                                            true,
                                            true,
                                        ); // true = is_our (global), true = initialized (assumed)
                                    }
                                }
                            }
                        } else {
                            // Handle regular variable names (not in qw())
                            let var_name = arg.trim();
                            if !var_name.is_empty() {
                                let (sigil, name) = split_variable_name(var_name);
                                if !sigil.is_empty() {
                                    scope.declare_variable_parts(
                                        sigil,
                                        name,
                                        node.location.start,
                                        true,
                                        true,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            NodeKind::Variable { sigil, name } => {
                // Skip built-in global variables
                // Optimization: Check built-ins first to avoid string scan for "::" on common globals
                if is_builtin_global(sigil, name) {
                    return;
                }

                // Skip package-qualified variables
                if name.contains("::") {
                    return;
                }

                // Try to use the variable - allocation free!
                let (mut variable_used, mut is_initialized) = scope.use_variable_parts(sigil, name);

                // If not found as simple variable, check if this is part of a hash/array access pattern
                if !variable_used && (sigil == "$" || sigil == "@") {
                    // Check parent for hash/array access context
                    if let Some(parent) = ancestors.last() {
                        if let NodeKind::Binary { op, left, .. } = &parent.kind {
                            // Only check if this node is the LEFT side of the access
                            if std::ptr::eq(left.as_ref(), node) {
                                if op == "{}" {
                                    // Check if the corresponding hash exists
                                    let (hash_used, hash_init) =
                                        scope.use_variable_parts("%", name);
                                    if hash_used {
                                        variable_used = true;
                                        is_initialized = hash_init;
                                    }
                                } else if op == "[]" {
                                    // Check if the corresponding array exists
                                    let (array_used, array_init) =
                                        scope.use_variable_parts("@", name);
                                    if array_used {
                                        variable_used = true;
                                        is_initialized = array_init;
                                    }
                                }
                            }
                        }
                    }
                }

                // Variable not found - check if we should report it
                if !variable_used {
                    if strict_mode {
                        let full_name = format!("{}{}", sigil, name);
                        issues.push(ScopeIssue {
                            kind: IssueKind::UndeclaredVariable,
                            variable_name: full_name.clone(),
                            line: context.get_line(node.location.start),
                            range: (node.location.start, node.location.end),
                            description: format!(
                                "Variable '{}' is used but not declared",
                                full_name
                            ),
                        });
                    }
                } else if !is_initialized {
                    // Variable found but used before initialization
                    let full_name = format!("{}{}", sigil, name);
                    issues.push(ScopeIssue {
                        kind: IssueKind::UninitializedVariable,
                        variable_name: full_name.clone(),
                        line: context.get_line(node.location.start),
                        range: (node.location.start, node.location.end),
                        description: format!(
                            "Variable '{}' is used before being initialized",
                            full_name
                        ),
                    });
                }
            }
            NodeKind::Assignment { lhs, rhs, op: _ } => {
                // Handle assignment: LHS variable becomes initialized
                // First analyze RHS (usages)
                self.analyze_node(rhs, scope, ancestors, issues, context);

                // Optimization: Handle simple scalar assignment directly to avoid double lookup
                // (mark_initialized + analyze_node both perform lookups)
                if let NodeKind::Variable { sigil, name } = &lhs.kind {
                    if !name.contains("::") && !is_builtin_global(sigil, name) {
                        if scope.initialize_and_use_variable_parts(sigil, name) {
                            return;
                        }
                    }
                }

                // Then analyze LHS
                // We need to recursively mark variables as initialized in the LHS structure
                // This handles scalars ($x = 1) and lists (($x, $y) = (1, 2))
                self.mark_initialized(lhs, scope);

                // Recurse into LHS to trigger UndeclaredVariable checks
                // Note: 'use_variable' marks as used, which is technically correct for assignment too (write usage)
                self.analyze_node(lhs, scope, ancestors, issues, context);
            }

            NodeKind::Tie { variable, package, args } => {
                ancestors.push(node);
                // Analyze arguments first
                self.analyze_node(package, scope, ancestors, issues, context);
                for arg in args {
                    self.analyze_node(arg, scope, ancestors, issues, context);
                }

                if let NodeKind::VariableDeclaration { .. } = variable.kind {
                    // Must analyze declaration FIRST to declare it, then mark initialized
                    self.analyze_node(variable, scope, ancestors, issues, context);
                    self.mark_initialized(variable, scope);
                } else {
                    // For existing variables, mark initialized then analyze (usage)
                    self.mark_initialized(variable, scope);
                    self.analyze_node(variable, scope, ancestors, issues, context);
                }

                ancestors.pop();
            }

            NodeKind::Untie { variable } => {
                ancestors.push(node);
                self.analyze_node(variable, scope, ancestors, issues, context);
                ancestors.pop();
            }

            NodeKind::Identifier { name } => {
                // Check for barewords under strict mode, excluding hash keys
                // Hybrid check: Fast path for immediate hash keys (depth 1), then known functions, then deep check
                if strict_mode
                    && !self.is_in_hash_key_context(node, ancestors, 1)
                    && !is_known_function(name)
                    && !self.is_in_hash_key_context(node, ancestors, 10)
                {
                    issues.push(ScopeIssue {
                        kind: IssueKind::UnquotedBareword,
                        variable_name: name.clone(),
                        line: context.get_line(node.location.start),
                        range: (node.location.start, node.location.end),
                        description: format!("Bareword '{}' not allowed under 'use strict'", name),
                    });
                }
            }

            NodeKind::Binary { op: _, left, right } => {
                // All binary operations (including {} and [])
                // We don't need special handling for {} and [] here because NodeKind::Variable
                // will handle the context-sensitive lookup (checking ancestors).
                ancestors.push(node);
                self.analyze_node(left, scope, ancestors, issues, context);
                self.analyze_node(right, scope, ancestors, issues, context);
                ancestors.pop();
            }

            NodeKind::ArrayLiteral { elements } => {
                ancestors.push(node);
                for element in elements {
                    self.analyze_node(element, scope, ancestors, issues, context);
                }
                ancestors.pop();
            }

            NodeKind::Block { statements } => {
                let block_scope = Rc::new(Scope::with_parent(scope.clone()));
                ancestors.push(node);
                for stmt in statements {
                    self.analyze_node(stmt, &block_scope, ancestors, issues, context);
                }
                ancestors.pop();
                self.collect_unused_variables(&block_scope, issues, context);
            }

            NodeKind::For { init, condition, update, body, .. } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

                ancestors.push(node);

                if let Some(init_node) = init {
                    self.analyze_node(init_node, &loop_scope, ancestors, issues, context);
                }
                if let Some(cond) = condition {
                    self.analyze_node(cond, &loop_scope, ancestors, issues, context);
                }
                if let Some(upd) = update {
                    self.analyze_node(upd, &loop_scope, ancestors, issues, context);
                }
                self.analyze_node(body, &loop_scope, ancestors, issues, context);

                ancestors.pop();

                self.collect_unused_variables(&loop_scope, issues, context);
            }

            NodeKind::Foreach { variable, list, body } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

                ancestors.push(node);

                // Declare the loop variable
                self.analyze_node(variable, &loop_scope, ancestors, issues, context);
                self.analyze_node(list, &loop_scope, ancestors, issues, context);
                self.analyze_node(body, &loop_scope, ancestors, issues, context);

                ancestors.pop();

                self.collect_unused_variables(&loop_scope, issues, context);
            }

            NodeKind::Subroutine { signature, body, .. } => {
                let sub_scope = Rc::new(Scope::with_parent(scope.clone()));

                // Check for duplicate parameters and shadowing
                let mut param_names = std::collections::HashSet::new();

                // Extract parameters from signature if present
                let params_to_check = if let Some(sig) = signature {
                    match &sig.kind {
                        NodeKind::Signature { parameters } => parameters.clone(),
                        _ => Vec::new(),
                    }
                } else {
                    Vec::new()
                };

                for param in &params_to_check {
                    let extracted = self.extract_variable_name(param);
                    if !extracted.is_empty() {
                        let full_name = extracted.as_string();
                        let (sigil, name) = extracted.parts();

                        // Check for duplicate parameters
                        if !param_names.insert(full_name.clone()) {
                            issues.push(ScopeIssue {
                                kind: IssueKind::DuplicateParameter,
                                variable_name: full_name.clone(),
                                line: context.get_line(param.location.start),
                                range: (param.location.start, param.location.end),
                                description: format!(
                                    "Duplicate parameter '{}' in subroutine signature",
                                    full_name
                                ),
                            });
                        }

                        // Check if parameter shadows a global or parent scope variable
                        if scope.has_variable_parts(sigil, name) {
                            issues.push(ScopeIssue {
                                kind: IssueKind::ParameterShadowsGlobal,
                                variable_name: full_name.clone(),
                                line: context.get_line(param.location.start),
                                range: (param.location.start, param.location.end),
                                description: format!(
                                    "Parameter '{}' shadows a variable from outer scope",
                                    full_name
                                ),
                            });
                        }

                        // Declare the parameter in subroutine scope
                        sub_scope.declare_variable_parts(
                            sigil,
                            name,
                            param.location.start,
                            false,
                            true,
                        ); // Parameters are initialized
                        // Don't mark parameters as automatically used yet - track their actual usage
                    }
                }

                ancestors.push(node);
                self.analyze_node(body, &sub_scope, ancestors, issues, context);
                ancestors.pop();

                // Check for unused parameters
                if let Some(sig) = signature {
                    if let NodeKind::Signature { parameters } = &sig.kind {
                        for param in parameters {
                            let extracted = self.extract_variable_name(param);
                            if !extracted.is_empty() {
                                let (sigil, name) = extracted.parts();
                                let full_name = extracted.as_string();

                                // Skip parameters starting with underscore (intentionally unused)
                                if name.starts_with('_') {
                                    continue;
                                }
                                if let Some(var) = sub_scope.lookup_variable_parts(sigil, name) {
                                    if !*var.is_used.borrow() {
                                        issues.push(ScopeIssue {
                                            kind: IssueKind::UnusedParameter,
                                            variable_name: full_name.clone(),
                                            line: context.get_line(param.location.start),
                                            range: (param.location.start, param.location.end),
                                            description: format!(
                                                "Parameter '{}' is declared but never used",
                                                full_name
                                            ),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }

                self.collect_unused_variables(&sub_scope, issues, context);
            }

            NodeKind::FunctionCall { args, .. } => {
                // Handle function arguments, which may contain complex variable patterns
                ancestors.push(node);
                for arg in args {
                    self.analyze_node(arg, scope, ancestors, issues, context);
                }
                ancestors.pop();
            }

            _ => {
                // Recursively analyze children
                ancestors.push(node);
                for child in node.children() {
                    self.analyze_node(child, scope, ancestors, issues, context);
                }
                ancestors.pop();
            }
        }
    }

    /// Marks variables as initialized when they appear on the left-hand side of an assignment.
    /// Handles scalar variables, list assignments like `($x, $y) = ...`, and nested structures.
    fn mark_initialized(&self, node: &Node, scope: &Rc<Scope>) {
        match &node.kind {
            NodeKind::Variable { sigil, name } => {
                if !name.contains("::") {
                    scope.initialize_variable_parts(sigil, name);
                }
            }
            // For all other node types (parens, lists, etc.), recurse into children
            // to find any nested variables that should be marked as initialized
            _ => {
                for child in node.children() {
                    self.mark_initialized(child, scope);
                }
            }
        }
    }

    fn collect_unused_variables(
        &self,
        scope: &Rc<Scope>,
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'_>,
    ) {
        scope.for_each_reportable_unused_variable(|var_name, offset| {
            let start = offset.min(context.code.len());
            let end = (start + var_name.len()).min(context.code.len());

            // Optimization: Generate description using the string reference before moving it
            let description = format!("Variable '{}' is declared but never used", var_name);

            issues.push(ScopeIssue {
                kind: IssueKind::UnusedVariable,
                variable_name: var_name, // Move: Avoids cloning the string
                line: context.get_line(offset),
                range: (start, end),
                description,
            });
        });
    }

    fn extract_variable_name<'a>(&self, node: &'a Node) -> ExtractedName<'a> {
        match &node.kind {
            NodeKind::Variable { sigil, name } => ExtractedName::Parts(sigil, name),
            NodeKind::MandatoryParameter { variable } => self.extract_variable_name(variable),
            NodeKind::ArrayLiteral { elements } => {
                // Handle array reference patterns like @{$ref}
                if elements.len() == 1 {
                    if let Some(first) = elements.first() {
                        return self.extract_variable_name(first);
                    }
                }
                ExtractedName::Full(String::new())
            }
            NodeKind::Binary { op, left, .. } if op == "->" => {
                // Handle method call patterns on variables
                self.extract_variable_name(left)
            }
            _ => {
                if let Some(child) = node.first_child() {
                    self.extract_variable_name(child)
                } else {
                    ExtractedName::Full(String::new())
                }
            }
        }
    }

    /// Determines if a node is in a hash key context, where barewords are legitimate.
    ///
    /// This method efficiently detects various hash key contexts to avoid false positives
    /// in strict mode bareword detection. It handles:
    ///
    /// # Hash Key Contexts Detected:
    /// - **Hash subscripts**: `$hash{bareword_key}` or `%hash{bareword_key}`
    /// - **Hash literals**: `{ key => value, another_key => value2 }`
    /// - **Hash slices**: `@hash{key1, key2, key3}` where keys are in an array
    /// - **Nested hash structures**: Complex nested hash access patterns
    ///
    /// # Performance Characteristics:
    /// - Early termination on first positive match
    /// - Efficient pointer-based parent traversal
    /// - O(depth) complexity where depth is AST nesting level
    /// - Typical case: 1-3 parent checks for hash contexts
    ///
    /// # Examples:
    /// ```perl
    /// use strict;
    /// my %hash = (key1 => 'value1');        # key1 is in hash key context
    /// my $val = $hash{bareword_key};         # bareword_key is in hash key context  
    /// my @vals = @hash{key1, key2};          # key1, key2 are in hash key context
    /// print INVALID_BAREWORD;                # NOT in hash key context - should warn
    /// ```
    fn is_in_hash_key_context(&self, node: &Node, ancestors: &[&Node], max_depth: usize) -> bool {
        let mut current = node;

        // Traverse up the AST to find hash key contexts
        // Limit traversal depth to prevent excessive searching
        // Iterate ancestors in reverse (from immediate parent up)
        let len = ancestors.len();

        for i in (0..len).rev() {
            if len - i > max_depth {
                break;
            }

            let parent = ancestors[i];

            match &parent.kind {
                // Method call: Class->method (Class is bareword)
                NodeKind::Binary { op, left, right: _ } if op == "->" => {
                    // Check if current node is the class name (left side of the -> operation)
                    if std::ptr::eq(left.as_ref(), current) {
                        return true;
                    }
                }
                NodeKind::MethodCall { object, .. } => {
                    // Check if current node is the class name (object)
                    if std::ptr::eq(object.as_ref(), current) {
                        return true;
                    }
                }
                // Hash subscript: $hash{key} or %hash{key}
                NodeKind::Binary { op, left: _, right } if op == "{}" => {
                    // Check if current node is the key (right side of the {} operation)
                    if std::ptr::eq(right.as_ref(), current) {
                        return true;
                    }
                }
                NodeKind::HashLiteral { pairs } => {
                    // Check if current node is a key in any of the pairs
                    for (key, _value) in pairs {
                        if std::ptr::eq(key, current) {
                            return true;
                        }
                    }
                }
                NodeKind::ArrayLiteral { .. } => {
                    // Check grandparent
                    if i > 0 {
                        let grandparent = ancestors[i - 1];
                        if let NodeKind::Binary { op, right, .. } = &grandparent.kind {
                            if op == "{}" && std::ptr::eq(right.as_ref(), parent) {
                                return true;
                            }
                        }
                    }
                }
                // Handle IndirectCall which parser sometimes produces for $hash{key} in print statements
                NodeKind::IndirectCall { object, args, .. } => {
                    // Check if current is one of the arguments
                    for arg in args {
                        if std::ptr::eq(arg, current) {
                            // Check if object is a variable that looks like a hash
                            if let NodeKind::Variable { sigil, .. } = &object.kind {
                                if sigil == "$" {
                                    return true;
                                }
                            }
                        }
                    }
                }
                _ => {}
            }

            current = parent;
        }

        false
    }

    pub fn get_suggestions(&self, issues: &[ScopeIssue]) -> Vec<String> {
        issues
            .iter()
            .map(|issue| match issue.kind {
                IssueKind::VariableShadowing => {
                    format!("Consider rename '{}' to avoid shadowing", issue.variable_name)
                }
                IssueKind::UnusedVariable => {
                    format!(
                        "Remove unused variable '{}' or prefix with underscore",
                        issue.variable_name
                    )
                }
                IssueKind::UndeclaredVariable => {
                    format!("Declare '{}' with 'my', 'our', or 'local'", issue.variable_name)
                }
                IssueKind::VariableRedeclaration => {
                    format!("Remove duplicate declaration of '{}'", issue.variable_name)
                }
                IssueKind::DuplicateParameter => {
                    format!("Remove or rename duplicate parameter '{}'", issue.variable_name)
                }
                IssueKind::ParameterShadowsGlobal => {
                    format!("Rename parameter '{}' to avoid shadowing", issue.variable_name)
                }
                IssueKind::UnusedParameter => {
                    format!("Rename '{}' with underscore or add comment", issue.variable_name)
                }
                IssueKind::UnquotedBareword => {
                    format!("Quote bareword '{}' or declare as filehandle", issue.variable_name)
                }
                IssueKind::UninitializedVariable => {
                    format!("Initialize '{}' before use", issue.variable_name)
                }
            })
            .collect()
    }
}

/// Check if a variable is a built-in Perl global variable
fn is_builtin_global(sigil: &str, name: &str) -> bool {
    // Fast path: most user variables start with lowercase and are not built-ins
    // Exception: $a and $b are built-in sort variables
    if !name.is_empty() {
        let first = name.as_bytes()[0];
        if first.is_ascii_lowercase() {
            // Optimization: Combine length and byte check to avoid multiple comparisons
            if name.len() > 1 || (first != b'a' && first != b'b') {
                return false;
            }
        }
    }

    let sigil_byte = match sigil.as_bytes().first() {
        Some(b) => *b,
        None => {
            return match name {
                // Filehandles (no sigil)
                "STDIN" | "STDOUT" | "STDERR" | "DATA" | "ARGVOUT" => true,
                _ => false,
            };
        }
    };

    match sigil_byte {
        b'$' => match name {
            // Special variables
            "_" | "!" | "@" | "?" | "^" | "$" | "0" | "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8"
            | "9" | "." | "," | "/" | "\\" | "\"" | ";" | "%" | "=" | "-" | "~" | "|" | "&"
            | "`" | "'" | "+" | "[" | "]" | "^A" | "^C" | "^D" | "^E" | "^F" | "^H" | "^I" | "^L"
            | "^M" | "^N" | "^O" | "^P" | "^R" | "^S" | "^T" | "^V" | "^W" | "^X" |
            // Common globals
            "ARGV" | "VERSION" | "AUTOLOAD" |
            // Sort variables
            "a" | "b" |
            // Error variables
            "EVAL_ERROR" | "ERRNO" | "EXTENDED_OS_ERROR" | "CHILD_ERROR" |
            "PROCESS_ID" | "PROGRAM_NAME" |
            // Perl version variables
            "PERL_VERSION" | "OLD_PERL_VERSION" => true,
            _ => {
                // Check patterns
                // $^[A-Z] variables
                if name.starts_with('^') && name.len() == 2 {
                    // Optimization: access byte directly since we know len is 2 and it's ASCII range
                    let second = name.as_bytes()[1];
                    if second.is_ascii_uppercase() {
                        return true;
                    }
                }

                // Numbered capture variables ($1, $2, etc.)
                // Note: $0-$9 are already handled in the match above, but this covers $10+
                // Optimization: use byte check to avoid utf-8 decoding
                if !name.is_empty() && name.as_bytes().iter().all(|c| c.is_ascii_digit()) {
                    return true;
                }

                false
            }
        },
        b'@' => matches!(name, "_" | "+" | "INC" | "ARGV" | "EXPORT" | "EXPORT_OK" | "ISA"),
        b'%' => matches!(name, "_" | "+" | "ENV" | "INC" | "SIG" | "EXPORT_TAGS"),
        _ => false,
    }
}

/// Check if an identifier is a known Perl built-in function
fn is_known_function(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    // Optimization: All known functions are lowercase or start with non-uppercase chars
    if name.as_bytes()[0].is_ascii_uppercase() {
        return false;
    }

    // Optimization: Use Perfect Hash Function for O(1) lookup of standard builtins
    if builtin_signatures_phf::is_builtin(name) {
        return true;
    }

    // Fallback: Check for syntax keywords and operators not in BUILTIN_SIGS
    match name {
        // Quote-like operators
        "q" | "qq" | "qr" | "qw" | "qx" | "tr" |
        // Control flow
        "continue" | "break" | "given" | "when" | "default" |
        // Misc
        "import" | "package" | "sub" | "sysclose" => true,
        _ => false,
    }
}

/// Check if an identifier is a known filehandle
#[allow(dead_code)]
fn is_filehandle(name: &str) -> bool {
    match name {
        "STDIN" | "STDOUT" | "STDERR" | "ARGV" | "ARGVOUT" | "DATA" | "STDHANDLE"
        | "__PACKAGE__" | "__FILE__" | "__LINE__" | "__SUB__" | "__END__" | "__DATA__" => true,
        _ => {
            // Check if it's all uppercase (common convention for filehandles)
            name.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !name.is_empty()
        }
    }
}
