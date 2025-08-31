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
}

#[derive(Debug, Clone)]
pub struct ScopeIssue {
    pub kind: IssueKind,
    pub variable_name: String,
    pub line: usize,
    pub description: String,
}

#[derive(Debug)]
struct Variable {
    name: String,
    line: usize,
    is_used: RefCell<bool>,
    is_our: bool,
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

    fn declare_variable(&self, name: &str, line: usize, is_our: bool) -> Option<IssueKind> {
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

    fn use_variable(&self, name: &str) -> bool {
        if let Some(var) = self.lookup_variable(name) {
            *var.is_used.borrow_mut() = true;
            true
        } else {
            false
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

        // Build a parent map so we can walk ancestor relationships
        let mut parent_map: HashMap<*const Node, &Node> = HashMap::new();
        self.build_parent_map(ast, None, &mut parent_map);

        self.analyze_node(ast, &root_scope, &parent_map, &mut issues, code, pragma_map);

        // Collect all unused variables from all scopes
        self.collect_unused_variables(&root_scope, &mut issues, code);

        issues
    }

    fn analyze_node(
        &self,
        node: &Node,
        scope: &Rc<Scope>,
        parent_map: &HashMap<*const Node, &Node>,
        issues: &mut Vec<ScopeIssue>,
        code: &str,
        pragma_map: &[(Range<usize>, PragmaState)],
    ) {
        // Get effective pragma state at this node's location
        let pragma_state = PragmaTracker::state_for_offset(pragma_map, node.location.start);
        let strict_mode = pragma_state.strict_subs;
        match &node.kind {
            NodeKind::VariableDeclaration { declarator, variable, .. } => {
                let var_name = self.extract_variable_name(variable);
                let line = self.get_line_from_node(variable, code);
                let is_our = declarator == "our";

                if let Some(issue_kind) =
                    scope.declare_variable(&var_name, variable.location.start, is_our)
                {
                    issues.push(ScopeIssue {
                        kind: issue_kind,
                        variable_name: var_name.clone(),
                        line,
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

            NodeKind::VariableListDeclaration { declarator, variables, .. } => {
                let is_our = declarator == "our";
                for variable in variables {
                    let var_name = self.extract_variable_name(variable);
                    let line = self.get_line_from_node(variable, code);

                    if let Some(issue_kind) =
                        scope.declare_variable(&var_name, variable.location.start, is_our)
                    {
                        issues.push(ScopeIssue {
                            kind: issue_kind,
                            variable_name: var_name.clone(),
                            line,
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
                                    scope.declare_variable(var_name, node.location.start, true); // true = is_our (global)
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
                                scope.declare_variable(var_name, node.location.start, true);
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
                if !scope.use_variable(&full_name) {
                    // Variable not found - check if we should report it
                    if strict_mode {
                        issues.push(ScopeIssue {
                            kind: IssueKind::UndeclaredVariable,
                            variable_name: full_name.clone(),
                            line: self.get_line_from_node(node, code),
                            description: format!(
                                "Variable '{}' is used but not declared",
                                full_name
                            ),
                        });
                    }
                }
            }

            NodeKind::Identifier { name } => {
                // Check for barewords under strict mode, excluding hash keys
                if strict_mode
                    && !self.is_in_hash_key_context(node, parent_map)
                    && !is_known_function(name)
                {
                    issues.push(ScopeIssue {
                        kind: IssueKind::UnquotedBareword,
                        variable_name: name.clone(),
                        line: self.get_line_from_node(node, code),
                        description: format!("Bareword '{}' not allowed under 'use strict'", name),
                    });
                }
            }

            NodeKind::Binary { left, right, .. } => {
                self.analyze_node(left, scope, parent_map, issues, code, pragma_map);
                self.analyze_node(right, scope, parent_map, issues, code, pragma_map);
            }

            NodeKind::ArrayLiteral { elements } => {
                for element in elements {
                    self.analyze_node(element, scope, parent_map, issues, code, pragma_map);
                }
            }

            NodeKind::Block { statements } => {
                let block_scope = Rc::new(Scope::with_parent(scope.clone()));
                for stmt in statements {
                    self.analyze_node(stmt, &block_scope, parent_map, issues, code, pragma_map);
                }
                self.collect_unused_variables(&block_scope, issues, code);
            }

            NodeKind::For { init, condition, update, body, .. } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

                if let Some(init_node) = init {
                    self.analyze_node(init_node, &loop_scope, parent_map, issues, code, pragma_map);
                }
                if let Some(cond) = condition {
                    self.analyze_node(cond, &loop_scope, parent_map, issues, code, pragma_map);
                }
                if let Some(upd) = update {
                    self.analyze_node(upd, &loop_scope, parent_map, issues, code, pragma_map);
                }
                self.analyze_node(body, &loop_scope, parent_map, issues, code, pragma_map);

                self.collect_unused_variables(&loop_scope, issues, code);
            }

            NodeKind::Foreach { variable, list, body } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

                // Declare the loop variable
                self.analyze_node(variable, &loop_scope, parent_map, issues, code, pragma_map);
                self.analyze_node(list, &loop_scope, parent_map, issues, code, pragma_map);
                self.analyze_node(body, &loop_scope, parent_map, issues, code, pragma_map);

                self.collect_unused_variables(&loop_scope, issues, code);
            }

            NodeKind::Subroutine { params, body, .. } => {
                let sub_scope = Rc::new(Scope::with_parent(scope.clone()));

                // Check for duplicate parameters and shadowing
                let mut param_names = std::collections::HashSet::new();
                for param in params {
                    if let NodeKind::Variable { sigil, name } = &param.kind {
                        let full_name = format!("{}{}", sigil, name);

                        // Check for duplicate parameters
                        if !param_names.insert(full_name.clone()) {
                            issues.push(ScopeIssue {
                                kind: IssueKind::DuplicateParameter,
                                variable_name: full_name.clone(),
                                line: self.get_line_from_node(param, code),
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
                                description: format!(
                                    "Parameter '{}' shadows a variable from outer scope",
                                    full_name
                                ),
                            });
                        }

                        // Declare the parameter in subroutine scope
                        sub_scope.declare_variable(&full_name, param.location.start, false);
                        // Don't mark parameters as automatically used yet - track their actual usage
                    }
                }

                self.analyze_node(body, &sub_scope, parent_map, issues, code, pragma_map);

                // Check for unused parameters
                for param in params {
                    if let NodeKind::Variable { sigil, name } = &param.kind {
                        let full_name = format!("{}{}", sigil, name);
                        // Skip parameters starting with underscore (intentionally unused)
                        if name.starts_with('_') {
                            continue;
                        }
                        if let Some(var) = sub_scope.lookup_variable(&full_name) {
                            if !*var.is_used.borrow() {
                                issues.push(ScopeIssue {
                                    kind: IssueKind::UnusedParameter,
                                    variable_name: full_name.clone(),
                                    line: self.get_line_from_node(param, code),
                                    description: format!(
                                        "Parameter '{}' is declared but never used",
                                        full_name
                                    ),
                                });
                            }
                        }
                    }
                }

                self.collect_unused_variables(&sub_scope, issues, code);
            }

            _ => {
                // Recursively analyze children
                for child in node.children() {
                    self.analyze_node(child, scope, parent_map, issues, code, pragma_map);
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
            issues.push(ScopeIssue {
                kind: IssueKind::UnusedVariable,
                variable_name: var_name.clone(),
                line: self.get_line_number(code, offset),
                description: format!("Variable '{}' is declared but never used", var_name),
            });
        }
    }

    fn extract_variable_name(&self, node: &Node) -> String {
        match &node.kind {
            NodeKind::Variable { sigil, name } => format!("{}{}", sigil, name),
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
    fn is_in_hash_key_context(
        &self,
        node: &Node,
        parent_map: &HashMap<*const Node, &Node>,
    ) -> bool {
        let mut current = node as *const Node;

        // Traverse up the AST to find hash key contexts
        // Limit traversal depth to prevent excessive searching
        let mut depth = 0;
        const MAX_TRAVERSAL_DEPTH: usize = 10;

        while let Some(parent) = parent_map.get(&current) {
            if depth > MAX_TRAVERSAL_DEPTH {
                break; // Safety limit for deeply nested structures
            }

            match &parent.kind {
                // Hash subscript: $hash{key} or %hash{key}
                NodeKind::Binary { op, left: _, right } if op == "{}" => {
                    // Check if current node is the key (right side of the {} operation)
                    if std::ptr::eq(right.as_ref(), current) {
                        return true;
                    }
                }

                // Hash literal: { key => value }
                NodeKind::HashLiteral { pairs } => {
                    // Check if current node is a key in any of the pairs
                    for (key, _value) in pairs {
                        if std::ptr::eq(key, current) {
                            return true;
                        }
                    }
                }
                // Array literal containing hash keys (for hash slices @hash{key1, key2})
                NodeKind::ArrayLiteral { elements: _ } => {
                    // Check if the parent of this array literal is a hash subscript
                    if let Some(grandparent) = parent_map.get(&(*parent as *const _)) {
                        if let NodeKind::Binary { op, right, .. } = &grandparent.kind {
                            if op == "{}" && std::ptr::eq(right.as_ref(), *parent) {
                                // This array literal is the key part of a hash slice
                                return true;
                            }
                        }
                    }
                }

                // Handle nested hash contexts (hash of hashes)
                // This covers cases like $hash{key1}{key2} where we might be in key2 context
                // Note: These cases are already handled by cases above, but this
                // documents that we explicitly support nested hash structures
                _ => {} // Continue traversing for other node types
            }

            current = *parent as *const _;
            depth += 1;
        }
        false
    }

    fn build_parent_map<'a>(
        &self,
        node: &'a Node,
        parent: Option<&'a Node>,
        map: &mut HashMap<*const Node, &'a Node>,
    ) {
        if let Some(p) = parent {
            map.insert(node as *const _, p);
        }
        for child in node.children() {
            self.build_parent_map(child, Some(node), map);
        }
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
            })
            .collect()
    }
}

impl Node {
    pub fn children(&self) -> Vec<&Node> {
        match &self.kind {
            NodeKind::Program { statements } => statements.iter().collect(),
            NodeKind::Block { statements } => statements.iter().collect(),
            NodeKind::Binary { left, right, .. } => vec![left.as_ref(), right.as_ref()],
            NodeKind::Unary { operand, .. } => vec![operand.as_ref()],
            NodeKind::If { condition, then_branch, else_branch, .. } => {
                let mut children = vec![condition.as_ref(), then_branch.as_ref()];
                if let Some(else_b) = else_branch {
                    children.push(else_b.as_ref());
                }
                children
            }
            NodeKind::FunctionCall { args, .. } => args.iter().collect(),
            NodeKind::MethodCall { object, args, .. } => {
                let mut children = vec![object.as_ref()];
                children.extend(args.iter());
                children
            }
            NodeKind::Assignment { lhs, rhs, .. } => vec![lhs.as_ref(), rhs.as_ref()],
            NodeKind::Return { value } => {
                if let Some(val) = value {
                    vec![val.as_ref()]
                } else {
                    vec![]
                }
            }
            NodeKind::Subroutine { body, .. } => {
                vec![body.as_ref()]
            }
            NodeKind::For { init, condition, update, body, .. } => {
                let mut children = vec![body.as_ref()];
                if let Some(i) = init {
                    children.push(i.as_ref());
                }
                if let Some(c) = condition {
                    children.push(c.as_ref());
                }
                if let Some(u) = update {
                    children.push(u.as_ref());
                }
                children
            }
            NodeKind::While { condition, body, .. } => {
                vec![condition.as_ref(), body.as_ref()]
            }
            NodeKind::ExpressionStatement { expression } => {
                vec![expression.as_ref()]
            }
            NodeKind::VariableDeclaration { variable, initializer, .. } => {
                let mut children = vec![variable.as_ref()];
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                children
            }
            NodeKind::VariableListDeclaration { variables, initializer, .. } => {
                let mut children: Vec<&Node> = variables.iter().collect();
                if let Some(init) = initializer {
                    children.push(init.as_ref());
                }
                children
            }
            _ => vec![],
        }
    }
}

/// Check if a variable is a built-in Perl global variable
fn is_builtin_global(name: &str) -> bool {
    // Standard global variables
    const BUILTIN_GLOBALS: &[&str] = &[
        // Special variables
        "$_",
        "@_",
        "%_",
        "$!",
        "$@",
        "$?",
        "$^",
        "$$",
        "$0",
        "$1",
        "$2",
        "$3",
        "$4",
        "$5",
        "$6",
        "$7",
        "$8",
        "$9",
        "$.",
        "$,",
        "$/",
        "$\\",
        "$\"",
        "$;",
        "$%",
        "$=",
        "$-",
        "$~",
        "$|",
        "$&",
        "$`",
        "$'",
        "$+",
        "@+",
        "%+",
        "$[",
        "$]",
        "$^A",
        "$^C",
        "$^D",
        "$^E",
        "$^F",
        "$^H",
        "$^I",
        "$^L",
        "$^M",
        "$^N",
        "$^O",
        "$^P",
        "$^R",
        "$^S",
        "$^T",
        "$^V",
        "$^W",
        "$^X",
        // Common globals
        "%ENV",
        "@INC",
        "%INC",
        "@ARGV",
        "%SIG",
        "$ARGV",
        "@EXPORT",
        "@EXPORT_OK",
        "%EXPORT_TAGS",
        "@ISA",
        "$VERSION",
        "$AUTOLOAD",
        // Filehandles
        "STDIN",
        "STDOUT",
        "STDERR",
        "DATA",
        "ARGVOUT",
        // Sort variables
        "$a",
        "$b",
        // Error variables
        "$EVAL_ERROR",
        "$ERRNO",
        "$EXTENDED_OS_ERROR",
        "$CHILD_ERROR",
        "$PROCESS_ID",
        "$PROGRAM_NAME",
        // Perl version variables
        "$PERL_VERSION",
        "$OLD_PERL_VERSION",
    ];

    // Check exact matches
    if BUILTIN_GLOBALS.contains(&name) {
        return true;
    }

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
    if name.starts_with('$') && name.len() > 1 && name[1..].chars().all(|c| c.is_ascii_digit()) {
        return true;
    }

    false
}

/// Check if an identifier is a known Perl built-in function
fn is_known_function(name: &str) -> bool {
    const KNOWN_FUNCTIONS: &[&str] = &[
        // I/O functions
        "print",
        "printf",
        "say",
        "open",
        "close",
        "read",
        "write",
        "seek",
        "tell",
        "eof",
        "fileno",
        "binmode",
        "sysopen",
        "sysread",
        "syswrite",
        "sysclose",
        "select",
        // String functions
        "chomp",
        "chop",
        "chr",
        "crypt",
        "fc",
        "hex",
        "index",
        "lc",
        "lcfirst",
        "length",
        "oct",
        "ord",
        "pack",
        "q",
        "qq",
        "qr",
        "quotemeta",
        "qw",
        "qx",
        "reverse",
        "rindex",
        "sprintf",
        "substr",
        "tr",
        "uc",
        "ucfirst",
        "unpack",
        // Array/List functions
        "pop",
        "push",
        "shift",
        "unshift",
        "splice",
        "split",
        "join",
        "grep",
        "map",
        "sort",
        "reverse",
        // Hash functions
        "delete",
        "each",
        "exists",
        "keys",
        "values",
        // Control flow
        "die",
        "exit",
        "return",
        "goto",
        "last",
        "next",
        "redo",
        "continue",
        "break",
        "given",
        "when",
        "default",
        // File test operators
        "stat",
        "lstat",
        "-r",
        "-w",
        "-x",
        "-o",
        "-R",
        "-W",
        "-X",
        "-O",
        "-e",
        "-z",
        "-s",
        "-f",
        "-d",
        "-l",
        "-p",
        "-S",
        "-b",
        "-c",
        "-t",
        "-u",
        "-g",
        "-k",
        "-T",
        "-B",
        "-M",
        "-A",
        "-C",
        // System functions
        "system",
        "exec",
        "fork",
        "wait",
        "waitpid",
        "kill",
        "sleep",
        "alarm",
        "getpgrp",
        "getppid",
        "getpriority",
        "setpgrp",
        "setpriority",
        "time",
        "times",
        "localtime",
        "gmtime",
        // Math functions
        "abs",
        "atan2",
        "cos",
        "exp",
        "int",
        "log",
        "rand",
        "sin",
        "sqrt",
        "srand",
        // Misc functions
        "defined",
        "undef",
        "ref",
        "bless",
        "tie",
        "tied",
        "untie",
        "eval",
        "caller",
        "import",
        "require",
        "use",
        "do",
        "package",
        "sub",
        "my",
        "our",
        "local",
        "state",
        "scalar",
        "wantarray",
        "warn",
    ];

    KNOWN_FUNCTIONS.contains(&name)
}

/// Check if an identifier is a known filehandle
#[allow(dead_code)]
fn is_filehandle(name: &str) -> bool {
    const KNOWN_FILEHANDLES: &[&str] = &[
        "STDIN",
        "STDOUT",
        "STDERR",
        "ARGV",
        "ARGVOUT",
        "DATA",
        "STDHANDLE",
        "__PACKAGE__",
        "__FILE__",
        "__LINE__",
        "__SUB__",
        "__END__",
        "__DATA__",
    ];

    // Check standard filehandles
    KNOWN_FILEHANDLES.contains(&name) ||
    // Check if it's all uppercase (common convention for filehandles)
    (name.chars().all(|c| c.is_ascii_uppercase() || c == '_') && !name.is_empty())
}
