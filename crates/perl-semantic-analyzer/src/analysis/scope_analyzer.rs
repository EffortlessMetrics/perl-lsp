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
//! ```rust,ignore
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
use rustc_hash::FxHashMap;
use std::cell::{Cell, RefCell};
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
    /// Capture variable (`$1`, `$2`, etc.) used with no preceding regex match in scope.
    CaptureVarWithoutRegexMatch,
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
    /// Whether a regex match operation (`=~`, `m//`, `s///`) has been seen in this scope.
    has_regex_match: Cell<bool>,
}

impl Scope {
    fn new() -> Self {
        let vars = std::array::from_fn(|_| None);
        Self { variables: RefCell::new(vars), parent: None, has_regex_match: Cell::new(false) }
    }

    fn with_parent(parent: Rc<Scope>) -> Self {
        let vars = std::array::from_fn(|_| None);
        Self {
            variables: RefCell::new(vars),
            parent: Some(parent),
            has_regex_match: Cell::new(false),
        }
    }

    /// Returns true if this scope or any ancestor scope has seen a regex match operation.
    fn regex_match_in_scope(&self) -> bool {
        if self.has_regex_match.get() {
            return true;
        }
        if let Some(ref parent) = self.parent { parent.regex_match_in_scope() } else { false }
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

fn is_interpolated_var_start(byte: u8) -> bool {
    byte.is_ascii_alphabetic() || byte == b'_'
}

fn is_interpolated_var_continue(byte: u8) -> bool {
    byte.is_ascii_alphanumeric() || byte == b'_' || byte == b':'
}

fn has_escaped_interpolation_marker(bytes: &[u8], index: usize) -> bool {
    if index == 0 {
        return false;
    }

    let mut backslashes = 0usize;
    let mut cursor = index;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        backslashes += 1;
        cursor -= 1;
    }

    backslashes % 2 == 1
}

enum ExtractedName<'a> {
    Parts(&'a str, &'a str),
    Full(String),
}

struct AnalysisContext<'a> {
    code: &'a str,
    pragma_map: &'a [(Range<usize>, PragmaState)],
    line_starts: RefCell<Option<Vec<usize>>>,
    /// Current package name, updated as `package` statements are traversed.
    current_package: RefCell<String>,
}

impl<'a> AnalysisContext<'a> {
    fn new(code: &'a str, pragma_map: &'a [(Range<usize>, PragmaState)]) -> Self {
        Self {
            code,
            pragma_map,
            line_starts: RefCell::new(None),
            current_package: RefCell::new("main".to_string()),
        }
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

    fn find_catch_variable_range(
        &self,
        catch_body_start: usize,
        full_name: &str,
    ) -> Option<(usize, usize)> {
        if full_name.is_empty() || catch_body_start == 0 || catch_body_start > self.code.len() {
            return None;
        }

        let window_start = catch_body_start.saturating_sub(256);
        let window = self.code.get(window_start..catch_body_start)?;
        let catch_start = window.rfind("catch")?;
        let search_start = catch_start + "catch".len();
        let var_offset = window[search_start..].rfind(full_name)? + search_start;
        let start = window_start + var_offset;
        let end = start + full_name.len();

        Some((start, end))
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

    fn package_variable_name(&self, name: &str, context: &AnalysisContext<'_>) -> Option<String> {
        if name.is_empty() || name.contains("::") {
            return None;
        }

        let current_package = context.current_package.borrow();
        Some(format!("{}::{}", current_package.as_str(), name))
    }

    fn declare_variable_parts_in_context(
        &self,
        scope: &Rc<Scope>,
        sigil: &str,
        name: &str,
        offset: usize,
        is_our: bool,
        is_initialized: bool,
        context: &AnalysisContext<'_>,
    ) -> Option<IssueKind> {
        if is_our && let Some(qualified_name) = self.package_variable_name(name, context) {
            return scope.declare_variable_parts(
                sigil,
                &qualified_name,
                offset,
                is_our,
                is_initialized,
            );
        }

        scope.declare_variable_parts(sigil, name, offset, is_our, is_initialized)
    }

    fn has_variable_parts_in_context(
        &self,
        scope: &Rc<Scope>,
        sigil: &str,
        name: &str,
        context: &AnalysisContext<'_>,
    ) -> bool {
        if scope.has_variable_parts(sigil, name) {
            return true;
        }

        self.package_variable_name(name, context)
            .is_some_and(|qualified_name| scope.has_variable_parts(sigil, &qualified_name))
    }

    fn use_variable_parts_in_context(
        &self,
        scope: &Rc<Scope>,
        sigil: &str,
        name: &str,
        context: &AnalysisContext<'_>,
    ) -> (bool, bool) {
        let (found, initialized) = scope.use_variable_parts(sigil, name);
        if found {
            return (found, initialized);
        }

        self.package_variable_name(name, context).map_or((false, false), |qualified_name| {
            scope.use_variable_parts(sigil, &qualified_name)
        })
    }

    fn initialize_variable_parts_in_context(
        &self,
        scope: &Rc<Scope>,
        sigil: &str,
        name: &str,
        context: &AnalysisContext<'_>,
    ) {
        if scope.has_variable_parts(sigil, name) {
            scope.initialize_variable_parts(sigil, name);
            return;
        }

        if let Some(qualified_name) = self.package_variable_name(name, context) {
            scope.initialize_variable_parts(sigil, &qualified_name);
        }
    }

    fn initialize_and_use_variable_parts_in_context(
        &self,
        scope: &Rc<Scope>,
        sigil: &str,
        name: &str,
        context: &AnalysisContext<'_>,
    ) -> bool {
        if scope.initialize_and_use_variable_parts(sigil, name) {
            return true;
        }

        self.package_variable_name(name, context).is_some_and(|qualified_name| {
            scope.initialize_and_use_variable_parts(sigil, &qualified_name)
        })
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
        let strict_vars_mode = pragma_state.strict_vars;
        let strict_subs_mode = pragma_state.strict_subs;
        match &node.kind {
            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                let extracted = self.extract_variable_name(variable);
                let (sigil, var_name_part) = extracted.parts();

                let is_our = declarator == "our";
                let is_initialized = initializer.is_some();

                // `local` of a builtin special variable (e.g. `local $/`, `local $,`) temporarily
                // modifies the global; it does not create a new lexical binding.  Declaring it in
                // the lexical scope would cause a spurious UnusedVariable diagnostic because all
                // later uses of `$/` etc. are recognised by is_builtin_global and never counted as
                // uses of the scope entry.  Skip the declaration entirely and only analyse any
                // initialiser expression that may be present.
                if declarator == "local" && is_builtin_global(sigil, var_name_part) {
                    // For `local $special = expr`, the parser embeds the assignment inside
                    // `variable` as an Assignment node rather than in `initializer`.  Walk the
                    // variable node's children to pick up any RHS expressions.
                    if let Some(init) = initializer {
                        self.analyze_node(init, scope, ancestors, issues, context);
                    }
                    if let NodeKind::Assignment { rhs, .. } = &variable.kind {
                        self.analyze_node(rhs, scope, ancestors, issues, context);
                    }
                    return;
                }

                // If checking initializer first (e.g. my $x = $x), we need to analyze initializer in
                // current scope BEFORE declaring the variable (standard Perl behavior)
                // Actually Perl evaluates RHS before LHS assignment, so usages in initializer refer to OUTER scope.
                // So we analyze initializer first.
                if let Some(init) = initializer {
                    self.analyze_node(init, scope, ancestors, issues, context);
                }

                if let Some(issue_kind) = self.declare_variable_parts_in_context(
                    scope,
                    sigil,
                    var_name_part,
                    variable.location.start,
                    is_our,
                    is_initialized,
                    context,
                ) {
                    // `our` re-declares a package global — valid Perl idiom when switching
                    // packages (`package Foo; our $x; package Bar; our $x;`).  Never report
                    // VariableRedeclaration for `our` declarations.
                    if is_our && issue_kind == IssueKind::VariableRedeclaration {
                        // Silently accept: different-package re-use of the same bare name.
                    } else {
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

                    if let Some(issue_kind) = self.declare_variable_parts_in_context(
                        scope,
                        sigil,
                        var_name_part,
                        variable.location.start,
                        is_our,
                        is_initialized,
                        context,
                    ) {
                        // `our` redeclaration is always valid — see VariableDeclaration handler.
                        if is_our && issue_kind == IssueKind::VariableRedeclaration {
                            // Silently accept.
                        } else {
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
                                        self.declare_variable_parts_in_context(
                                            scope,
                                            sigil,
                                            name,
                                            node.location.start,
                                            true,
                                            true,
                                            context,
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
                                    self.declare_variable_parts_in_context(
                                        scope,
                                        sigil,
                                        name,
                                        node.location.start,
                                        true,
                                        true,
                                        context,
                                    );
                                }
                            }
                        }
                    }
                }
            }
            NodeKind::Variable { sigil, name } => {
                // Capture variables ($1, $2, ...) are built-in globals but require a preceding
                // regex match in scope to be meaningful. Check before the general builtin skip.
                if sigil == "$" && is_capture_variable(name) {
                    if !scope.regex_match_in_scope() {
                        let full_name = format!("{}{}", sigil, name);
                        issues.push(ScopeIssue {
                            kind: IssueKind::CaptureVarWithoutRegexMatch,
                            variable_name: full_name.clone(),
                            line: context.get_line(node.location.start),
                            range: (node.location.start, node.location.end),
                            description: format!(
                                "Capture variable '{}' used without a preceding regex match in scope",
                                full_name
                            ),
                        });
                    }
                    return;
                }

                // Skip built-in global variables — but only when no lexical declaration shadows
                // them.  Variables like $a and $b are sort globals, but `my ($a, $b) = @_`
                // creates a lexical shadow that must be tracked as used.
                if is_builtin_global(sigil, name) && !scope.has_variable_parts(sigil, name) {
                    return;
                }

                // Skip package-qualified variables
                if name.contains("::") {
                    return;
                }

                // Normalize explicit dereference/container syntax before lookup so that
                // `@$ref` resolves to `$ref`, while direct subscripting keeps using the
                // container sigil that the syntax implies.
                let (lookup_sigil, lookup_name) = self
                    .resolve_variable_use_target(node, ancestors, context)
                    .unwrap_or((sigil, name));
                let (variable_used, is_initialized) =
                    self.use_variable_parts_in_context(scope, lookup_sigil, lookup_name, context);

                // Variable not found - check if we should report it
                if !variable_used {
                    if strict_vars_mode {
                        self.push_undeclared_variable_issue(issues, context, node, sigil, name);
                    }
                } else if !is_initialized {
                    self.push_uninitialized_variable_issue(issues, context, node, sigil, name);
                }
            }
            NodeKind::Typeglob { name } => {
                let (sigil, var_name) = split_variable_name(name);
                if !sigil.is_empty() && !var_name.is_empty() && !var_name.contains("::") {
                    self.record_variable_use(
                        scope,
                        strict_vars_mode,
                        context,
                        issues,
                        node,
                        sigil,
                        var_name,
                    );
                }
            }
            NodeKind::FunctionCall { name, args } => {
                if let Some((sigil, var_name)) = self.extract_name_like_variable(name) {
                    self.record_variable_use(
                        scope,
                        strict_vars_mode,
                        context,
                        issues,
                        node,
                        sigil,
                        var_name,
                    );
                }

                // Handle function arguments, which may contain complex variable patterns.
                // Some builtins consume declaration-capable filehandle arguments directly,
                // e.g. `open my $fh, ...` or `pipe my $r, my $w;`. Those declarations should
                // count as used and initialized by the builtin itself.
                //
                // Builtins that default to $_ when called with zero arguments implicitly
                // read (and in some cases modify) $_. Mark it as used so that any lexically-
                // scoped `my $_` in scope is not reported as unused or uninitialized.
                if args.is_empty() && is_topic_defaulting_builtin(name) {
                    let _ = scope.use_variable_parts("$", "_");
                }
                ancestors.push(node);
                let declaration_arg_positions = builtin_declaration_arg_positions(name);
                for (arg_index, arg) in args.iter().enumerate() {
                    self.analyze_node(arg, scope, ancestors, issues, context);
                    if declaration_arg_positions.contains(&arg_index) {
                        self.mark_builtin_declaration_arg_consumed(arg, scope, context);
                    }
                }
                ancestors.pop();
            }
            NodeKind::MethodCall { object, method, args } => {
                ancestors.push(node);
                self.analyze_node(object, scope, ancestors, issues, context);
                if let Some((sigil, var_name)) = self.extract_method_name_variable(method) {
                    self.record_variable_use(
                        scope,
                        strict_vars_mode,
                        context,
                        issues,
                        node,
                        sigil,
                        var_name,
                    );
                }
                for arg in args {
                    self.analyze_node(arg, scope, ancestors, issues, context);
                }
                ancestors.pop();
            }
            NodeKind::Unary { op: _, operand } => {
                ancestors.push(node);
                self.analyze_node(operand, scope, ancestors, issues, context);
                ancestors.pop();
            }
            NodeKind::String { value, interpolated } => {
                if *interpolated
                    || value.starts_with('"')
                    || value.starts_with('`')
                    || value.starts_with("qq")
                    || value.starts_with("qx")
                {
                    self.mark_interpolated_variables_used(value, scope, context);
                }
            }
            NodeKind::Heredoc { content, interpolated, .. } => {
                if *interpolated {
                    self.mark_interpolated_variables_used(content, scope, context);
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
                        if self.initialize_and_use_variable_parts_in_context(
                            scope, sigil, name, context,
                        ) {
                            return;
                        }
                    }
                }

                // Then analyze LHS
                // We need to recursively mark variables as initialized in the LHS structure
                // This handles scalars ($x = 1) and lists (($x, $y) = (1, 2))
                self.mark_initialized(lhs, scope, context);

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
                    self.mark_initialized(variable, scope, context);
                } else {
                    // For existing variables, mark initialized then analyze (usage)
                    self.mark_initialized(variable, scope, context);
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
                if strict_subs_mode
                    && !self.is_in_hash_key_context(node, ancestors, 1)
                    && !is_known_function(name)
                    && !pragma_state.has_builtin_import(name)
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

            NodeKind::PhaseBlock { block, .. } => {
                let phase_scope = Rc::new(Scope::with_parent(scope.clone()));
                ancestors.push(node);
                self.analyze_node(block, &phase_scope, ancestors, issues, context);
                ancestors.pop();
                self.collect_unused_variables(&phase_scope, issues, context);
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

            NodeKind::Foreach { variable, list, body, continue_block } => {
                let loop_scope = Rc::new(Scope::with_parent(scope.clone()));

                ancestors.push(node);

                // Declare the loop variable and immediately mark it initialized — the list
                // provides its value at runtime so there is no uninitialized window.
                self.analyze_node(variable, &loop_scope, ancestors, issues, context);
                self.mark_initialized(variable, &loop_scope, context);
                self.analyze_node(list, &loop_scope, ancestors, issues, context);
                self.analyze_node(body, &loop_scope, ancestors, issues, context);
                if let Some(cb) = continue_block {
                    self.analyze_node(cb, &loop_scope, ancestors, issues, context);
                }

                ancestors.pop();

                self.collect_unused_variables(&loop_scope, issues, context);
            }

            NodeKind::Subroutine { signature, body, .. } => {
                let sub_scope = Rc::new(Scope::with_parent(scope.clone()));

                // Check for duplicate parameters and shadowing
                let mut param_names = std::collections::HashSet::new();

                // Extract parameters from signature if present
                // Optimization: Use slice to avoid cloning the parameters vector (deep copy of AST nodes)
                let params_to_check: &[Node] = if let Some(sig) = signature {
                    match &sig.kind {
                        NodeKind::Signature { parameters } => parameters.as_slice(),
                        _ => &[],
                    }
                } else {
                    &[]
                };

                for param in params_to_check {
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
                        if self.has_variable_parts_in_context(scope, sigil, name, context) {
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
                        self.declare_variable_parts_in_context(
                            &sub_scope,
                            sigil,
                            name,
                            param.location.start,
                            false,
                            true,
                            context,
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

                                // Optimization: Access variable directly from current scope to avoid Rc clone
                                let idx = sigil_to_index(sigil);
                                let vars = sub_scope.variables.borrow();
                                if let Some(map) = vars[idx].as_ref() {
                                    if let Some(var) = map.get(name) {
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
                                            // Mark as used to prevent double reporting
                                            *var.is_used.borrow_mut() = true;
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                self.collect_unused_variables(&sub_scope, issues, context);
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                ancestors.push(node);
                self.analyze_node(body, scope, ancestors, issues, context);

                for (catch_var, catch_body) in catch_blocks {
                    let catch_scope = Rc::new(Scope::with_parent(scope.clone()));

                    if let Some(full_name) = catch_var.as_deref() {
                        let catch_var_range = context
                            .find_catch_variable_range(catch_body.location.start, full_name)
                            .unwrap_or((catch_body.location.start, catch_body.location.start));
                        let (sigil, name) = split_variable_name(full_name);
                        if !sigil.is_empty() && !name.is_empty() && !name.contains("::") {
                            if let Some(issue_kind) = catch_scope.declare_variable_parts(
                                sigil,
                                name,
                                catch_var_range.0,
                                false,
                                true,
                            ) {
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
                                    variable_name: full_name.to_string(),
                                    line: context.get_line(catch_var_range.0),
                                    range: catch_var_range,
                                    description,
                                });
                            }
                        }
                    }

                    self.analyze_block_with_scope(
                        catch_body,
                        &catch_scope,
                        ancestors,
                        issues,
                        context,
                    );
                    self.collect_unused_variables(&catch_scope, issues, context);
                }

                if let Some(finally) = finally_block {
                    self.analyze_node(finally, scope, ancestors, issues, context);
                }

                ancestors.pop();
            }
            NodeKind::Package { name, block, .. } => {
                // Track the active package so that `our` variable declarations can be
                // correctly namespaced.  Two packages that each declare `our $VAR` are
                // declaring *different* package-global variables (`Alpha::VAR` vs
                // `Beta::VAR`) and must not be reported as redeclarations.
                if let Some(block_node) = block {
                    // Block form: `package Foo { ... }` — scope is limited to the block.
                    // Save the previous package name and restore it after the block.
                    let saved_package = context.current_package.borrow().clone();
                    *context.current_package.borrow_mut() = name.clone();

                    let pkg_scope = Rc::new(Scope::with_parent(scope.clone()));
                    ancestors.push(node);
                    self.analyze_node(block_node, &pkg_scope, ancestors, issues, context);
                    ancestors.pop();
                    self.collect_unused_variables(&pkg_scope, issues, context);

                    *context.current_package.borrow_mut() = saved_package;
                } else {
                    // Statement form: `package Foo;` — affects the rest of the file.
                    // No scope boundary is created; the current scope continues.
                    *context.current_package.borrow_mut() = name.clone();
                }
            }

            // Regex match operations set capture variables ($1, $2, ...) in the current scope.
            NodeKind::Match { expr, .. } => {
                scope.has_regex_match.set(true);
                ancestors.push(node);
                self.analyze_node(expr, scope, ancestors, issues, context);
                ancestors.pop();
            }

            NodeKind::Substitution { expr, .. } => {
                scope.has_regex_match.set(true);
                ancestors.push(node);
                self.analyze_node(expr, scope, ancestors, issues, context);
                ancestors.pop();
            }

            // Standalone regex (m// matching against $_) also sets capture variables.
            NodeKind::Regex { .. } => {
                scope.has_regex_match.set(true);
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

    /// Resolve the variable symbol that a syntax form should count as a use.
    ///
    /// This keeps explicit dereference syntax precise:
    /// - `@$ref` and `%$ref` count as uses of `$ref`
    /// - `$arr[0]` counts as a use of `@arr`
    /// - `$hash{k}` counts as a use of `%hash`
    /// - Arrow dereference forms stay on the scalar reference itself
    fn resolve_variable_use_target<'a>(
        &self,
        node: &'a Node,
        ancestors: &[&'a Node],
        context: &AnalysisContext<'_>,
    ) -> Option<(&'a str, &'a str)> {
        let NodeKind::Variable { sigil, name } = &node.kind else {
            return None;
        };

        if (sigil == "@" || sigil == "%" || sigil == "$") && name.starts_with('$') && name.len() > 1
        {
            return Some(("$", &name[1..]));
        }

        if sigil == "$"
            && let Some(parent) = ancestors.last()
            && let NodeKind::Binary { op, left, right } = &parent.kind
            && std::ptr::eq(left.as_ref(), node)
        {
            match op.as_str() {
                "[]" => return Some(("@", name)),
                "->[]" | "->{}" => return Some(("$", name)),
                "{}" if self.is_dynamic_method_deref_rhs(right)
                    || self.is_dynamic_method_deref_context(parent, ancestors)
                    || self.is_braced_dynamic_method_call(parent, context) =>
                {
                    return Some(("$", name));
                }
                "{}" => return Some(("%", name)),
                _ => {}
            }
        }

        // When the parser interprets `print $arr[0]` as indirect-object syntax, it produces
        // `IndirectCall { object: Variable($, "arr"), args: [ArrayLiteral([0])] }`.
        // Similarly, `print $hash{a}` produces
        // `IndirectCall { object: Variable($, "hash"), args: [Block([a])] }`.
        // Bridge the sigil so that `@arr` / `%hash` are marked as used, not `$arr` / `$hash`.
        if sigil == "$"
            && let Some(parent) = ancestors.last()
            && let NodeKind::IndirectCall { object, args, .. } = &parent.kind
            && std::ptr::eq(object.as_ref(), node)
        {
            if let Some(first_arg) = args.first() {
                match &first_arg.kind {
                    NodeKind::ArrayLiteral { .. } => return Some(("@", name)),
                    NodeKind::Block { .. } => return Some(("%", name)),
                    _ => {}
                }
            }
        }

        Some((sigil, name))
    }

    fn extract_name_like_variable<'a>(&self, name: &'a str) -> Option<(&'a str, &'a str)> {
        let (sigil, var_name) = split_variable_name(name);
        if sigil.is_empty()
            || var_name.is_empty()
            || var_name.contains("::")
            || !self.looks_like_variable_name(var_name)
        {
            return None;
        }
        Some((sigil, var_name))
    }

    fn extract_method_name_variable<'a>(&self, method: &'a str) -> Option<(&'a str, &'a str)> {
        self.extract_name_like_variable(method).or_else(|| {
            let inner = method.strip_prefix("${")?.strip_suffix('}')?;
            if inner.contains("::") || !self.looks_like_variable_name(inner) {
                return None;
            }
            Some(("$", inner))
        })
    }

    fn looks_like_variable_name(&self, name: &str) -> bool {
        matches!(
            name.chars().next(),
            Some('A'..='Z' | 'a'..='z' | '_' | '$' | '@' | '%' | '&' | '*' | '^' | '#' | '!' | '?')
        )
    }

    fn is_dynamic_method_deref_rhs(&self, node: &Node) -> bool {
        matches!(
            &node.kind,
            NodeKind::Unary { op, operand }
                if op == "\\"
                    && matches!(
                        &operand.kind,
                        NodeKind::String { .. } | NodeKind::Identifier { .. }
                    )
        )
    }

    fn is_dynamic_method_deref_context<'a>(&self, node: &'a Node, ancestors: &[&'a Node]) -> bool {
        let Some(grandparent) = ancestors.iter().rev().nth(1).copied() else {
            return false;
        };

        match &grandparent.kind {
            NodeKind::MethodCall { object, .. } => std::ptr::eq(object.as_ref(), node),
            NodeKind::FunctionCall { name, args } if name == "->()" => {
                args.first().is_some_and(|arg| std::ptr::eq(arg, node))
            }
            _ => false,
        }
    }

    fn is_braced_dynamic_method_call(&self, node: &Node, context: &AnalysisContext<'_>) -> bool {
        let Some(selector_text) = context.code.get(node.location.start..node.location.end) else {
            return false;
        };
        if !selector_text.contains("->${") {
            return false;
        }

        let Some(suffix) = context.code.get(node.location.end..) else {
            return false;
        };
        suffix.trim_start().starts_with("()")
    }

    fn record_variable_use(
        &self,
        scope: &Rc<Scope>,
        strict_vars_mode: bool,
        context: &AnalysisContext<'_>,
        issues: &mut Vec<ScopeIssue>,
        node: &Node,
        sigil: &str,
        name: &str,
    ) {
        let (variable_used, is_initialized) =
            self.use_variable_parts_in_context(scope, sigil, name, context);
        if !variable_used {
            if strict_vars_mode {
                self.push_undeclared_variable_issue(issues, context, node, sigil, name);
            }
        } else if !is_initialized {
            self.push_uninitialized_variable_issue(issues, context, node, sigil, name);
        }
    }

    fn push_undeclared_variable_issue(
        &self,
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'_>,
        node: &Node,
        sigil: &str,
        name: &str,
    ) {
        let full_name = format!("{}{}", sigil, name);
        issues.push(ScopeIssue {
            kind: IssueKind::UndeclaredVariable,
            variable_name: full_name.clone(),
            line: context.get_line(node.location.start),
            range: (node.location.start, node.location.end),
            description: format!("Variable '{}' is used but not declared", full_name),
        });
    }

    fn push_uninitialized_variable_issue(
        &self,
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'_>,
        node: &Node,
        sigil: &str,
        name: &str,
    ) {
        let full_name = format!("{}{}", sigil, name);
        issues.push(ScopeIssue {
            kind: IssueKind::UninitializedVariable,
            variable_name: full_name.clone(),
            line: context.get_line(node.location.start),
            range: (node.location.start, node.location.end),
            description: format!("Variable '{}' is used before being initialized", full_name),
        });
    }

    /// Marks variables as initialized when they appear on the left-hand side of an assignment.
    /// Handles scalar variables, list assignments like `($x, $y) = ...`, and nested structures.
    fn mark_initialized(&self, node: &Node, scope: &Rc<Scope>, context: &AnalysisContext<'_>) {
        match &node.kind {
            NodeKind::Variable { sigil, name } => {
                if !name.contains("::") {
                    self.initialize_variable_parts_in_context(scope, sigil, name, context);
                }
            }
            // For all other node types (parens, lists, etc.), recurse into children
            // to find any nested variables that should be marked as initialized
            _ => {
                for child in node.children() {
                    self.mark_initialized(child, scope, context);
                }
            }
        }
    }

    fn analyze_block_with_scope<'a>(
        &self,
        node: &'a Node,
        scope: &Rc<Scope>,
        ancestors: &mut Vec<&'a Node>,
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'a>,
    ) {
        if let NodeKind::Block { statements } = &node.kind {
            ancestors.push(node);
            for stmt in statements {
                self.analyze_node(stmt, scope, ancestors, issues, context);
            }
            ancestors.pop();
        } else {
            self.analyze_node(node, scope, ancestors, issues, context);
        }
    }

    fn mark_builtin_declaration_arg_consumed(
        &self,
        node: &Node,
        scope: &Rc<Scope>,
        context: &AnalysisContext<'_>,
    ) {
        match &node.kind {
            NodeKind::VariableDeclaration { variable, .. } => {
                let extracted = self.extract_variable_name(variable);
                let (sigil, name) = extracted.parts();
                if !sigil.is_empty() && !name.is_empty() && !name.contains("::") {
                    let _ = self
                        .initialize_and_use_variable_parts_in_context(scope, sigil, name, context);
                }
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                for variable in variables {
                    self.mark_builtin_declaration_arg_consumed(variable, scope, context);
                }
            }
            NodeKind::VariableWithAttributes { variable, .. } => {
                self.mark_builtin_declaration_arg_consumed(variable, scope, context);
            }
            _ => {}
        }
    }

    fn mark_interpolated_variables_used(
        &self,
        content: &str,
        scope: &Rc<Scope>,
        context: &AnalysisContext<'_>,
    ) {
        let bytes = content.as_bytes();
        let mut index = 0;

        while index < bytes.len() {
            let sigil = match bytes[index] {
                b'$' => "$",
                b'@' => "@",
                _ => {
                    index += 1;
                    continue;
                }
            };

            if has_escaped_interpolation_marker(bytes, index) {
                index += 1;
                continue;
            }

            if index + 1 >= bytes.len() {
                break;
            }

            let (start, requires_closing_brace) =
                if bytes[index + 1] == b'{' { (index + 2, true) } else { (index + 1, false) };

            if start >= bytes.len() || !is_interpolated_var_start(bytes[start]) {
                index += 1;
                continue;
            }

            let mut end = start + 1;
            while end < bytes.len() && is_interpolated_var_continue(bytes[end]) {
                end += 1;
            }

            if requires_closing_brace && (end >= bytes.len() || bytes[end] != b'}') {
                index += 1;
                continue;
            }

            if let Some(name) = content.get(start..end) {
                if !name.contains("::") {
                    let _ = self.use_variable_parts_in_context(scope, sigil, name, context);
                }
            }

            index = if requires_closing_brace { end + 1 } else { end };
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
            NodeKind::MandatoryParameter { variable }
            | NodeKind::OptionalParameter { variable, .. }
            | NodeKind::SlurpyParameter { variable }
            | NodeKind::NamedParameter { variable } => self.extract_variable_name(variable),
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
                IssueKind::CaptureVarWithoutRegexMatch => {
                    format!(
                        "Perform a regex match (=~ /.../) before using capture variable '{}'",
                        issue.variable_name
                    )
                }
            })
            .collect()
    }
}

/// Returns true if `name` (without sigil) is a numbered capture variable.
///
/// Capture variables are `$1`, `$2`, ..., `$9`, `$10`, `$11`, etc.
/// `$0` is the program name and is NOT a capture variable.
#[inline]
fn is_capture_variable(name: &str) -> bool {
    // Must be non-empty, all digits, and not "0" (which is $0 = program name)
    !name.is_empty() && name != "0" && name.as_bytes().iter().all(|c| c.is_ascii_digit())
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
            "PERL_VERSION" | "OLD_PERL_VERSION" |
            // Perl internal special values (perlguts/perlapi) — used in XS and introspection code
            "PL_sv_yes" | "PL_sv_no" | "PL_sv_undef" => true,
            _ => {
                // Check patterns
                // $^X (single-char) control variables — lexer produces name `^X`.
                // ${^NAME} (multi-char) control variables — lexer produces name `{^NAME}`.
                // Both should be treated as built-ins.
                //
                // Form 1: `^` followed by one or more ASCII uppercase letters or underscores.
                //   Examples: `^A`, `^W`, `^MATCH`, `^PREMATCH`, `^POSTMATCH`.
                // Form 2: `{^NAME}` — same but wrapped in braces by the lexer.
                //   Examples: `{^MATCH}`, `{^PREMATCH}`, `{^POSTMATCH}`.
                let caret_name = if let Some(inner) = name
                    .strip_prefix('{')
                    .and_then(|s| s.strip_suffix('}'))
                {
                    inner
                } else {
                    name
                };
                if let Some(rest) = caret_name.strip_prefix('^') {
                    if !rest.is_empty()
                        && rest
                            .as_bytes()
                            .iter()
                            .all(|c| c.is_ascii_uppercase() || *c == b'_')
                    {
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
        b'@' => matches!(name, "_" | "+" | "-" | "INC" | "ARGV" | "EXPORT" | "EXPORT_OK" | "ISA"),
        b'%' => matches!(name, "_" | "+" | "ENV" | "INC" | "SIG" | "EXPORT_TAGS"),
        _ => false,
    }
}

/// Check if an identifier is a known Perl built-in function
fn is_known_function(name: &str) -> bool {
    if name.is_empty() {
        return false;
    }
    if matches!(name, "PL_sv_yes" | "PL_sv_no" | "PL_sv_undef") {
        return true;
    }
    // Optimization: All known functions are lowercase or start with non-uppercase chars
    if name.as_bytes()[0].is_ascii_uppercase() {
        return false;
    }

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

/// Builtins whose declaration-capable arguments are all consumed by the builtin itself.
///
/// Keep this list explicit and conservative. Only include builtins where the parser already
/// emits declaration nodes for the relevant argument, and where treating that declaration as
/// used avoids false diagnostics after the call.
///
/// Position semantics:
/// - Position 0: `open`, `opendir`, `sysopen`, `socket`, `accept`, `dbmopen`
/// - Position 1: `read`, `sysread`, `recv`, `shmread`
/// - Positions 0 and 1: `pipe`, `socketpair`
fn builtin_declaration_arg_positions(name: &str) -> &'static [usize] {
    match name {
        // Position 0: the first argument is the new handle/socket
        "open" | "opendir" | "sysopen" | "socket" | "accept" | "dbmopen" => &[0],
        // Position 1: the second argument is the buffer (first is an existing handle)
        "read" | "sysread" | "recv" | "shmread" => &[1],
        // pipe: both first arguments are new handles
        "pipe" => &[0, 1],
        // socketpair: both first arguments are new sockets
        "socketpair" => &[0, 1],
        _ => &[],
    }
}

/// Builtins that operate on `$_` by default when called with zero arguments.
///
/// When any of these is invoked as a bare call (no args), Perl implicitly reads
/// (and in some cases modifies) `$_`. Marking `$_` as used at call sites prevents
/// false "unused" or "uninitialized" diagnostics for lexically-scoped `my $_`.
fn is_topic_defaulting_builtin(name: &str) -> bool {
    matches!(
        name,
        "chomp"
            | "chop"
            | "chr"
            | "hex"
            | "lc"
            | "lcfirst"
            | "length"
            | "oct"
            | "ord"
            | "uc"
            | "ucfirst"
            | "abs"
            | "int"
            | "log"
            | "sqrt"
            | "cos"
            | "sin"
            | "exp"
            | "print"
            | "say"
    )
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
