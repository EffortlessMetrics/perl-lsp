//! Scope analysis and variable tracking for Perl parsing workflow pipeline
//!
//! This module provides comprehensive scope analysis for Perl scripts, tracking
//! variable declarations, usage patterns, and potential issues across different
//! scopes within the LSP workflow stages.
//!
//! # LSP Workflow Integration
//!
//! Scope analysis supports Perl parsing across all LSP stages:
//! - **Extract**: Analyze variable scope in embedded Perl scripts
//! - **Normalize**: Track variable usage during standardization transforms
//! - **Thread**: Analyze control flow variable dependencies
//! - **Render**: Validate variable scope during output generation
//! - **Index**: Extract scope information for symbol indexing
//!
//! # Usage Examples
//!
//! ```rust
//! use perl_parser::scope_analyzer::{ScopeAnalyzer, IssueKind};
//! use perl_parser::{Parser, ast::Node};
//!
//! // Analyze Perl script for scope issues
//! let script = "my $var = 42; sub hello { print $var; }";
//! let mut parser = Parser::new(script);
//! let ast = parser.parse().unwrap();
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
//! ```

use crate::ast::{Node, NodeKind};
use crate::pragma_tracker::{PragmaState, PragmaTracker};
use std::cell::RefCell;
use std::collections::HashMap;
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
    name: String,
    line: usize,
    is_used: RefCell<bool>,
    is_our: bool,
    is_initialized: RefCell<bool>,
}

#[derive(Debug)]
struct Scope {
    variables: RefCell<HashMap<String, Rc<Variable>>>,
    parent: Option<Rc<Scope>>,
}

impl Scope {
    fn new() -> Self {
        Self { variables: RefCell::new(HashMap::new()), parent: None }
    }

    fn with_parent(parent: Rc<Scope>) -> Self {
        Self { variables: RefCell::new(HashMap::new()), parent: Some(parent) }
    }

    fn declare_variable(
        &self,
        name: &str,
        line: usize,
        is_our: bool,
        is_initialized: bool,
    ) -> Option<IssueKind> {
        // First check if already declared in this scope
        {
            let vars = self.variables.borrow();
            if vars.contains_key(name) {
                return Some(IssueKind::VariableRedeclaration);
            }
        }

        // Check if it shadows a parent scope variable
        let shadows = if let Some(ref parent) = self.parent {
            parent.lookup_variable(name).is_some()
        } else {
            false
        };

        // Now insert the variable
        let mut vars = self.variables.borrow_mut();
        vars.insert(
            name.to_string(),
            Rc::new(Variable {
                name: name.to_string(),
                line,
                is_used: RefCell::new(is_our), // 'our' variables are considered used
                is_our,
                is_initialized: RefCell::new(is_initialized),
            }),
        );

        if shadows { Some(IssueKind::VariableShadowing) } else { None }
    }

    fn lookup_variable(&self, name: &str) -> Option<Rc<Variable>> {
        self.variables
            .borrow()
            .get(name)
            .cloned()
            .or_else(|| self.parent.as_ref()?.lookup_variable(name))
    }

    fn use_variable(&self, name: &str) -> (bool, bool) {
        if let Some(var) = self.lookup_variable(name) {
            *var.is_used.borrow_mut() = true;
            (true, *var.is_initialized.borrow())
        } else {
            (false, false)
        }
    }

    fn initialize_variable(&self, name: &str) {
        if let Some(var) = self.lookup_variable(name) {
            *var.is_initialized.borrow_mut() = true;
        }
    }

    fn get_unused_variables(&self) -> Vec<(String, usize)> {
        let mut unused = Vec::new();

        for var in self.variables.borrow().values() {
            if !*var.is_used.borrow() && !var.is_our {
                unused.push((var.name.clone(), var.line));
            }
        }

        // Recursively collect from parent scopes if needed
        // (Not needed for our current use case)

        unused
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

        self.analyze_node(ast, &root_scope, &mut ancestors, &mut issues, code, pragma_map);

        // Collect all unused variables from all scopes
        self.collect_unused_variables(&root_scope, &mut issues, code);

        issues
    }

    fn analyze_node<'a>(
        &self,
        node: &'a Node,
        scope: &Rc<Scope>,
        ancestors: &mut Vec<&'a Node>,
        issues: &mut Vec<ScopeIssue>,
        code: &str,
        pragma_map: &[(Range<usize>, PragmaState)],
    ) {
        // Get effective pragma state at this node's location
        let pragma_state = PragmaTracker::state_for_offset(pragma_map, node.location.start);
        let strict_mode = pragma_state.strict_subs;
        match &node.kind {
            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                let var_name = self.extract_variable_name(variable);
                let line = self.get_line_from_node(variable, code);
                let is_our = declarator == "our";
                let is_initialized = initializer.is_some();

                // If checking initializer first (e.g. my $x = $x), we need to analyze initializer in
                // current scope BEFORE declaring the variable (standard Perl behavior)
                // Actually Perl evaluates RHS before LHS assignment, so usages in initializer refer to OUTER scope.
                // So we analyze initializer first.
                if let Some(init) = initializer {
                    self.analyze_node(init, scope, parent_map, issues, code, pragma_map);
                }

                if let Some(issue_kind) = scope.declare_variable(
                    &var_name,
                    variable.location.start,
                    is_our,
                    is_initialized,
                ) {
                    issues.push(ScopeIssue {
                        kind: issue_kind,
                        variable_name: var_name.clone(),
                        line,
                        range: (variable.location.start, variable.location.end),
                        description: match issue_kind {
                            IssueKind::VariableShadowing => {
                                format!("Variable '{}' shadows a variable in outer scope", var_name)
                            }
                            IssueKind::VariableRedeclaration => {
                                format!("Variable '{}' is already declared in this scope", var_name)
                            }
                            _ => String::new(),
                        },
                    });
                }
            }

            NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
                let is_our = declarator == "our";
                let is_initialized = initializer.is_some();

                // Analyze initializer first
                if let Some(init) = initializer {
                    self.analyze_node(init, scope, parent_map, issues, code, pragma_map);
                }

                for variable in variables {
                    let var_name = self.extract_variable_name(variable);
                    let line = self.get_line_from_node(variable, code);

                    if let Some(issue_kind) = scope.declare_variable(
                        &var_name,
                        variable.location.start,
                        is_our,
                        is_initialized,
                    ) {
                        issues.push(ScopeIssue {
                            kind: issue_kind,
                            variable_name: var_name.clone(),
                            line,
                            range: (variable.location.start, variable.location.end),
                            description: match issue_kind {
                                IssueKind::VariableShadowing => {
                                    format!(
                                        "Variable '{}' shadows a variable in outer scope",
                                        var_name
                                    )
                                }
                                IssueKind::VariableRedeclaration => {
                                    format!(
                                        "Variable '{}' is already declared in this scope",
                                        var_name
                                    )
                                }
                                _ => String::new(),
                            },
                        });
                    }
                }
            }

            NodeKind::Use { module, args } => {
                // Handle 'use vars' pragma for global variable declarations
                if module == "vars" {
                    for arg in args {
                        // Parse qw() style arguments to extract individual variable names
                        if arg.starts_with("qw(") && arg.ends_with(")") {
                            let content = &arg[3..arg.len() - 1]; // Remove qw( and )
                            for var_name in content.split_whitespace() {
                                if !var_name.is_empty()
                                    && (var_name.starts_with('$')
                                        || var_name.starts_with('@')
                                        || var_name.starts_with('%'))
                                {
                                    // Declare these variables as globals in the current scope
                                    scope.declare_variable(
                                        var_name,
                                        node.location.start,
                                        true,
                                        true,
                                    ); // true = is_our (global), true = initialized (assumed)
                                }
                            }
                        } else {
                            // Handle regular variable names (not in qw())
                            let var_name = arg.trim();
                            if !var_name.is_empty()
                                && (var_name.starts_with('$')
                                    || var_name.starts_with('@')
                                    || var_name.starts_with('%'))
                            {
                                scope.declare_variable(var_name, node.location.start, true, true);
                            }
                        }
                    }
                }
            }
            NodeKind::Variable { sigil, name } => {
                let full_name = format!("{}{}", sigil, name);

                // Skip package-qualified variables
                if full_name.contains("::") {
                    return;
                }

                // Skip built-in global variables
                if is_builtin_global(&full_name) {
                    return;
                }

                // Try to use the variable
                let (mut variable_used, mut is_initialized) = scope.use_variable(&full_name);

                // If not found as simple variable, check if this is part of a hash/array access pattern
                if !variable_used && sigil == "$" {
                    // Check if the corresponding hash or array exists
                    let hash_name = format!("%{}", name);
                    let array_name = format!("@{}", name);

                    let (hash_used, hash_init) = scope.use_variable(&hash_name);
                    let (array_used, array_init) = scope.use_variable(&array_name);

                    if hash_used || array_used {
                        variable_used = true;
                        is_initialized = hash_init || array_init;
                    }
                }

                // Variable not found - check if we should report it
                if !variable_used {
                    if strict_mode {
                        issues.push(ScopeIssue {
                            kind: IssueKind::UndeclaredVariable,
                            variable_name: full_name.clone(),
                            line: self.get_line_from_node(node, code),
                            range: (node.location.start, node.location.end),
                            description: format!(
                                "Variable '{}' is used but not declared",
                                full_name
                            ),
                        });
                    }
                } else if !is_initialized {
                    // Variable found but used before initialization
                    // Note: This check is rudimentary and doesn't account for flow control
                    // It only catches: my $x; print $x;
                    issues.push(ScopeIssue {
                        kind: IssueKind::UninitializedVariable,
                        variable_name: full_name.clone(),
                        line: self.get_line_from_node(node, code),
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
                self.analyze_node(rhs, scope, parent_map, issues, code, pragma_map);

                // Then analyze LHS
                // We need to recursively mark variables as initialized in the LHS structure
                // This handles scalars ($x = 1) and lists (($x, $y) = (1, 2))
                self.mark_initialized(lhs, scope);

                // Recurse into LHS to trigger UndeclaredVariable checks
                // Note: 'use_variable' marks as used, which is technically correct for assignment too (write usage)
                self.analyze_node(lhs, scope, parent_map, issues, code, pragma_map);
            }

            NodeKind::Identifier { name } => {
                // Check for barewords under strict mode, excluding hash keys
                if strict_mode
                    && !self.is_in_hash_key_context(node, ancestors)
                    && !is_known_function(name)
                {
                    issues.push(ScopeIssue {
                        kind: IssueKind::UnquotedBareword,
                        variable_name: name.clone(),
                        line: self.get_line_from_node(node, code),
                        range: (node.location.start, node.location.end),
                        description: format!("Bareword '{}' not allowed under 'use strict'", name),
                    });
                }
            }

            NodeKind::Binary { op, left, right } => {
                match op.as_str() {
                    "{}" => {
                        // Hash access: $hash{key} -> mark %hash as used if it exists
                        if let NodeKind::Variable { sigil, name } = &left.kind {
                            if sigil == "$" {
                                let hash_name = format!("%{}", name);
                                // Only mark as used if the hash actually exists
                                if scope.lookup_variable(&hash_name).is_some() {
                                    scope.use_variable(&hash_name);
                                }
                            }
                        }
                        // Always process both children to ensure undefined variables are caught
                        ancestors.push(node);
                        self.analyze_node(left, scope, ancestors, issues, code, pragma_map);
                        self.analyze_node(right, scope, ancestors, issues, code, pragma_map);
                        ancestors.pop();
                    }
                    "[]" => {
                        // Array access: $array[index] -> mark @array as used if it exists
                        if let NodeKind::Variable { sigil, name } = &left.kind {
                            if sigil == "$" {
                                let array_name = format!("@{}", name);
                                // Only mark as used if the array actually exists
                                if scope.lookup_variable(&array_name).is_some() {
                                    scope.use_variable(&array_name);
                                }
                            }
                        }
                        // Always process both children to ensure undefined variables are caught
                        ancestors.push(node);
                        self.analyze_node(left, scope, ancestors, issues, code, pragma_map);
                        self.analyze_node(right, scope, ancestors, issues, code, pragma_map);
                        ancestors.pop();
                    }
                    _ => {
                        // Other binary operations
                        ancestors.push(node);
                        self.analyze_node(left, scope, ancestors, issues, code, pragma_map);
                        self.analyze_node(right, scope, ancestors, issues, code, pragma_map);
                        ancestors.pop();
                    }
                }
            }

            NodeKind::ArrayLiteral { elements } => {
                ancestors.push(node);
                for element in elements {
                    self.analyze_node(element, scope, ancestors, issues, code, pragma_map);
                }
                ancestors.pop();
            }

            NodeKind::Block { statements } => {
                let block_scope = Rc::new(Scope::with_parent(scope.clone()));
                ancestors.push(node);
                for stmt in statements {
                    self.analyze_node(stmt, &block_scope, ancestors, issues, code, pragma_map);
                }
                ancestors.pop();
                self.collect_unused_variables(&block_scope, issues, code);
            }

            NodeKind::For { init, condition, update, body, .. } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

                ancestors.push(node);

                if let Some(init_node) = init {
                    self.analyze_node(init_node, &loop_scope, ancestors, issues, code, pragma_map);
                }
                if let Some(cond) = condition {
                    self.analyze_node(cond, &loop_scope, ancestors, issues, code, pragma_map);
                }
                if let Some(upd) = update {
                    self.analyze_node(upd, &loop_scope, ancestors, issues, code, pragma_map);
                }
                self.analyze_node(body, &loop_scope, ancestors, issues, code, pragma_map);

                ancestors.pop();

                self.collect_unused_variables(&loop_scope, issues, code);
            }

            NodeKind::Foreach { variable, list, body } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

                ancestors.push(node);

                // Declare the loop variable
                self.analyze_node(variable, &loop_scope, ancestors, issues, code, pragma_map);
                self.analyze_node(list, &loop_scope, ancestors, issues, code, pragma_map);
                self.analyze_node(body, &loop_scope, ancestors, issues, code, pragma_map);

                ancestors.pop();

                self.collect_unused_variables(&loop_scope, issues, code);
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
                    let full_name = self.extract_variable_name(param);
                    if !full_name.is_empty() {
                        // Check for duplicate parameters
                        if !param_names.insert(full_name.clone()) {
                            issues.push(ScopeIssue {
                                kind: IssueKind::DuplicateParameter,
                                variable_name: full_name.clone(),
                                line: self.get_line_from_node(param, code),
                                range: (param.location.start, param.location.end),
                                description: format!(
                                    "Duplicate parameter '{}' in subroutine signature",
                                    full_name
                                ),
                            });
                        }

                        // Check if parameter shadows a global or parent scope variable
                        if scope.lookup_variable(&full_name).is_some() {
                            issues.push(ScopeIssue {
                                kind: IssueKind::ParameterShadowsGlobal,
                                variable_name: full_name.clone(),
                                line: self.get_line_from_node(param, code),
                                range: (param.location.start, param.location.end),
                                description: format!(
                                    "Parameter '{}' shadows a variable from outer scope",
                                    full_name
                                ),
                            });
                        }

                        // Declare the parameter in subroutine scope
                        sub_scope.declare_variable(&full_name, param.location.start, false, true); // Parameters are initialized
                        // Don't mark parameters as automatically used yet - track their actual usage
                    }
                }

                ancestors.push(node);
                self.analyze_node(body, &sub_scope, ancestors, issues, code, pragma_map);
                ancestors.pop();

                // Check for unused parameters
                if let Some(sig) = signature {
                    if let NodeKind::Signature { parameters } = &sig.kind {
                        for param in parameters {
                            let full_name = self.extract_variable_name(param);
                            if !full_name.is_empty() {
                                // Skip parameters starting with underscore (intentionally unused)
                                let name_part = &full_name[1..]; // Remove sigil
                                if name_part.starts_with('_') {
                                    continue;
                                }
                                if let Some(var) = sub_scope.lookup_variable(&full_name) {
                                    if !*var.is_used.borrow() {
                                        issues.push(ScopeIssue {
                                            kind: IssueKind::UnusedParameter,
                                            variable_name: full_name.clone(),
                                            line: self.get_line_from_node(param, code),
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

                self.collect_unused_variables(&sub_scope, issues, code);
            }

            NodeKind::FunctionCall { args, .. } => {
                // Handle function arguments, which may contain complex variable patterns
                ancestors.push(node);
                for arg in args {
                    self.analyze_node(arg, scope, ancestors, issues, code, pragma_map);
                }
                ancestors.pop();
            }

            _ => {
                // Recursively analyze children
                ancestors.push(node);
                for child in node.children() {
                    self.analyze_node(child, scope, ancestors, issues, code, pragma_map);
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
                let full_name = format!("{}{}", sigil, name);
                // Skip package-qualified variables (e.g., $Foo::bar)
                if !full_name.contains("::") {
                    scope.initialize_variable(&full_name);
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
        code: &str,
    ) {
        for (var_name, offset) in scope.get_unused_variables() {
            // Skip variables starting with underscore (intentionally unused)
            if var_name.len() > 1 && var_name.chars().nth(1) == Some('_') {
                continue;
            }
            let start = offset.min(code.len());
            let end = (start + var_name.len()).min(code.len());
            issues.push(ScopeIssue {
                kind: IssueKind::UnusedVariable,
                variable_name: var_name.clone(),
                line: self.get_line_number(code, offset),
                range: (start, end),
                description: format!("Variable '{}' is declared but never used", var_name),
            });
        }
    }

    fn extract_variable_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Variable { sigil, name } => format!("{}{}", sigil, name),
            NodeKind::MandatoryParameter { variable } => self.extract_variable_name(variable),
            NodeKind::ArrayLiteral { elements } => {
                // Handle array reference patterns like @{$ref}
                if elements.len() == 1 {
                    if let Some(first) = elements.first() {
                        return self.extract_variable_name(first);
                    }
                }
                String::new()
            }
            NodeKind::Binary { op, left, .. } if op == "->" => {
                // Handle method call patterns on variables
                self.extract_variable_name(left)
            }
            _ => {
                if let Some(child) = node.children().first() {
                    self.extract_variable_name(child)
                } else {
                    String::new()
                }
            }
        }
    }

    fn get_line_from_node(&self, node: &Node, code: &str) -> usize {
        self.get_line_number(code, node.location.start)
    }

    #[allow(dead_code)]
    fn get_line_from_position(&self, offset: usize, code: &str) -> usize {
        self.get_line_number(code, offset)
    }

    fn get_line_number(&self, code: &str, offset: usize) -> usize {
        code[..offset.min(code.len())].chars().filter(|&c| c == '\n').count() + 1
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
    fn is_in_hash_key_context(&self, node: &Node, ancestors: &[&Node]) -> bool {
        let mut current = node;

        // Traverse up the AST to find hash key contexts
        // Limit traversal depth to prevent excessive searching
        // Iterate ancestors in reverse (from immediate parent up)
        let len = ancestors.len();

        // We limit depth to 10.
        for i in (0..len).rev() {
            if len - i > 10 {
                break;
            }

            let parent = ancestors[i];

            match &parent.kind {
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
fn is_builtin_global(name: &str) -> bool {
    match name {
        // Special variables
        "$_" | "@_" | "%_" | "$!" | "$@" | "$?" | "$^" | "$$" | "$0" | "$1" | "$2" | "$3"
        | "$4" | "$5" | "$6" | "$7" | "$8" | "$9" | "$." | "$," | "$/" | "$\\" | "$\"" | "$;"
        | "$%" | "$=" | "$-" | "$~" | "$|" | "$&" | "$`" | "$'" | "$+" | "@+" | "%+" | "$["
        | "$]" | "$^A" | "$^C" | "$^D" | "$^E" | "$^F" | "$^H" | "$^I" | "$^L" | "$^M" | "$^N"
        | "$^O" | "$^P" | "$^R" | "$^S" | "$^T" | "$^V" | "$^W" | "$^X" |
        // Common globals
        "%ENV" | "@INC" | "%INC" | "@ARGV" | "%SIG" | "$ARGV" | "@EXPORT" | "@EXPORT_OK"
        | "%EXPORT_TAGS" | "@ISA" | "$VERSION" | "$AUTOLOAD" |
        // Filehandles
        "STDIN" | "STDOUT" | "STDERR" | "DATA" | "ARGVOUT" |
        // Sort variables
        "$a" | "$b" |
        // Error variables
        "$EVAL_ERROR" | "$ERRNO" | "$EXTENDED_OS_ERROR" | "$CHILD_ERROR" | "$PROCESS_ID"
        | "$PROGRAM_NAME" |
        // Perl version variables
        "$PERL_VERSION" | "$OLD_PERL_VERSION" => true,
        _ => {
            // Check patterns
            // $^[A-Z] variables
            if name.starts_with("$^") && name.len() == 3 {
                if let Some(ch) = name.chars().nth(2) {
                    if ch.is_ascii_uppercase() {
                        return true;
                    }
                }
            }

            // Numbered capture variables ($1, $2, etc.)
            // Note: $0-$9 are already handled in the match above
            if name.starts_with('$') && name.len() > 1 && name[1..].chars().all(|c| c.is_ascii_digit()) {
                return true;
            }

            false
        }
    }
}

/// Check if an identifier is a known Perl built-in function
fn is_known_function(name: &str) -> bool {
    match name {
        // I/O functions
        "print" | "printf" | "say" | "open" | "close" | "read" | "write" | "seek" | "tell"
        | "eof" | "fileno" | "binmode" | "sysopen" | "sysread" | "syswrite" | "sysclose"
        | "select" |
        // String functions
        "chomp" | "chop" | "chr" | "crypt" | "fc" | "hex" | "index" | "lc" | "lcfirst" | "length"
        | "oct" | "ord" | "pack" | "q" | "qq" | "qr" | "quotemeta" | "qw" | "qx" | "reverse"
        | "rindex" | "sprintf" | "substr" | "tr" | "uc" | "ucfirst" | "unpack" |
        // Array/List functions
        "pop" | "push" | "shift" | "unshift" | "splice" | "split" | "join" | "grep" | "map"
        | "sort" |
        // Hash functions
        "delete" | "each" | "exists" | "keys" | "values" |
        // Control flow
        "die" | "exit" | "return" | "goto" | "last" | "next" | "redo" | "continue" | "break"
        | "given" | "when" | "default" |
        // File test operators
        "stat" | "lstat" | "-r" | "-w" | "-x" | "-o" | "-R" | "-W" | "-X" | "-O" | "-e" | "-z"
        | "-s" | "-f" | "-d" | "-l" | "-p" | "-S" | "-b" | "-c" | "-t" | "-u" | "-g" | "-k"
        | "-T" | "-B" | "-M" | "-A" | "-C" |
        // System functions
        "system" | "exec" | "fork" | "wait" | "waitpid" | "kill" | "sleep" | "alarm"
        | "getpgrp" | "getppid" | "getpriority" | "setpgrp" | "setpriority" | "time" | "times"
        | "localtime" | "gmtime" |
        // Math functions
        "abs" | "atan2" | "cos" | "exp" | "int" | "log" | "rand" | "sin" | "sqrt" | "srand" |
        // Misc functions
        "defined" | "undef" | "ref" | "bless" | "tie" | "tied" | "untie" | "eval" | "caller"
        | "import" | "require" | "use" | "do" | "package" | "sub" | "my" | "our" | "local"
        | "state" | "scalar" | "wantarray" | "warn" => true,
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
