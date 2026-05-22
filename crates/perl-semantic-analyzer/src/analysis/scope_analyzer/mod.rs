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

mod calls_and_exprs;
mod declarations;
mod interpolation;
mod scope_constructs;
mod uses;

use crate::ast::{Node, NodeKind};
use crate::pragma_tracker::{PragmaQueryCursor, PragmaState};
use perl_module::import::resolve_known_export_tag;
use rustc_hash::FxHashMap;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
use std::ops::Range;
use std::rc::Rc;

/// Category of scope-related issue detected during analysis.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IssueKind {
    /// A variable declared in an inner scope shadows one in an outer scope.
    VariableShadowing,
    /// A declared variable is never read.
    UnusedVariable,
    /// A variable is used without a prior declaration (`my`/`our`/`local`).
    UndeclaredVariable,
    /// The same variable name is declared twice in the same scope.
    VariableRedeclaration,
    /// A subroutine parameter name appears more than once in the signature.
    DuplicateParameter,
    /// A parameter name shadows a package-level (`our`) variable.
    ParameterShadowsGlobal,
    /// A subroutine parameter is never used inside the body.
    UnusedParameter,
    /// A bareword was used where a string or identifier was expected.
    UnquotedBareword,
    /// A variable was accessed before any initializing assignment.
    UninitializedVariable,
    /// Capture variable (`$1`, `$2`, etc.) used with no preceding regex match in scope.
    CaptureVarWithoutRegexMatch,
}

/// A single scope-analysis finding with location and human-readable description.
#[derive(Debug, Clone)]
pub struct ScopeIssue {
    /// The category of scope problem detected.
    pub kind: IssueKind,
    /// The bare variable name (without sigil) involved in the issue.
    pub variable_name: String,
    /// Zero-based line number of the first token of the offending construct.
    pub line: usize,
    /// Byte offset range `(start, end)` of the offending construct.
    pub range: (usize, usize),
    /// Human-readable explanation of the issue.
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
pub(super) fn sigil_to_index(sigil: &str) -> usize {
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
pub(super) struct Scope {
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
pub(super) fn split_variable_name(full_name: &str) -> (&str, &str) {
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

pub(super) enum ExtractedName<'a> {
    Parts(&'a str, &'a str),
    Full(String),
}

pub(super) struct AnalysisContext<'a> {
    code: &'a str,
    pragma_map: &'a [(Range<usize>, PragmaState)],
    pragma_cursor: RefCell<PragmaQueryCursor>,
    imported_barewords: HashSet<String>,
    line_starts: RefCell<Option<Vec<usize>>>,
    /// Current package name, updated as `package` statements are traversed.
    current_package: RefCell<String>,
}

impl<'a> AnalysisContext<'a> {
    fn new(ast: &Node, code: &'a str, pragma_map: &'a [(Range<usize>, PragmaState)]) -> Self {
        Self {
            code,
            pragma_map,
            pragma_cursor: RefCell::new(PragmaQueryCursor::new()),
            imported_barewords: collect_imported_barewords(ast),
            line_starts: RefCell::new(None),
            current_package: RefCell::new("main".to_string()),
        }
    }

    fn pragma_state_for_offset(&self, offset: usize) -> PragmaState {
        self.pragma_cursor.borrow_mut().state_for_offset(self.pragma_map, offset)
    }

    fn has_imported_bareword(&self, name: &str) -> bool {
        self.imported_barewords.contains(name)
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

/// Analyzes an AST for scope-related issues such as unused variables and shadowing.
///
/// Produces a list of [`ScopeIssue`]s that can be surfaced as LSP diagnostics
/// or used by the refactoring engine.  The analyzer is stateless and may be
/// reused across multiple invocations.
pub struct ScopeAnalyzer;

impl Default for ScopeAnalyzer {
    fn default() -> Self {
        Self::new()
    }
}

impl ScopeAnalyzer {
    /// Create a new scope analyzer instance.
    pub fn new() -> Self {
        Self
    }

    pub(super) fn package_variable_name(
        &self,
        name: &str,
        context: &AnalysisContext<'_>,
    ) -> Option<String> {
        if name.is_empty() || name.contains("::") {
            return None;
        }

        let current_package = context.current_package.borrow();
        Some(format!("{}::{}", current_package.as_str(), name))
    }

    pub(super) fn declare_variable_parts_in_context(
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

    pub(super) fn has_variable_parts_in_context(
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

    pub(super) fn use_variable_parts_in_context(
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

    pub(super) fn initialize_variable_parts_in_context(
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

    pub(super) fn initialize_and_use_variable_parts_in_context(
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

    /// Analyze `ast` for scope issues, using `pragma_map` to honour `use strict` regions.
    ///
    /// Returns all detected issues sorted by byte offset.
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

        let context = AnalysisContext::new(ast, code, pragma_map);

        self.analyze_node(ast, &root_scope, &mut ancestors, &mut issues, &context);

        // Collect all unused variables from all scopes
        self.collect_unused_variables(&root_scope, &mut issues, &context);

        issues
    }

    pub(super) fn analyze_node<'a>(
        &self,
        node: &'a Node,
        scope: &Rc<Scope>,
        ancestors: &mut Vec<&'a Node>,
        issues: &mut Vec<ScopeIssue>,
        context: &AnalysisContext<'a>,
    ) {
        // Get effective pragma state at this node's location
        let pragma_state = context.pragma_state_for_offset(node.location.start);
        let strict_vars_mode = pragma_state.strict_vars || pragma_state.signatures_strict;
        let strict_subs_mode = pragma_state.strict_subs || pragma_state.signatures_strict;
        match &node.kind {
            NodeKind::VariableDeclaration { declarator, variable, initializer, .. } => {
                let _ = declarations::handle_variable_declaration(
                    self,
                    node,
                    declarator,
                    variable,
                    initializer.as_deref(),
                    scope,
                    ancestors,
                    issues,
                    context,
                );
            }

            NodeKind::VariableListDeclaration { declarator, variables, initializer, .. } => {
                declarations::handle_variable_list_declaration(
                    self,
                    initializer.as_deref(),
                    declarator,
                    variables,
                    scope,
                    ancestors,
                    issues,
                    context,
                );
            }

            NodeKind::Use { module, args, .. } => {
                declarations::handle_use(self, node, module, args, scope, context);
            }
            NodeKind::Variable { sigil, name } => {
                let _ = uses::handle_variable(
                    self,
                    node,
                    sigil,
                    name,
                    scope,
                    ancestors,
                    issues,
                    context,
                    strict_vars_mode,
                );
            }
            NodeKind::Typeglob { name } => {
                uses::handle_typeglob(self, node, name, scope, issues, context, strict_vars_mode);
            }
            NodeKind::Readline { filehandle: Some(filehandle) } => {
                uses::handle_readline(
                    self,
                    node,
                    filehandle,
                    scope,
                    issues,
                    context,
                    strict_vars_mode,
                );
            }
            NodeKind::FunctionCall { name, args } => {
                calls_and_exprs::handle_function_call(
                    self,
                    node,
                    name,
                    args,
                    scope,
                    ancestors,
                    issues,
                    context,
                    strict_vars_mode,
                );
            }
            NodeKind::MethodCall { object, method, args } => {
                calls_and_exprs::handle_method_call(
                    self,
                    node,
                    object,
                    method,
                    args,
                    scope,
                    ancestors,
                    issues,
                    context,
                    strict_vars_mode,
                );
            }
            NodeKind::Unary { op: _, operand } => {
                calls_and_exprs::handle_unary(
                    self, node, operand, scope, ancestors, issues, context,
                );
            }
            NodeKind::String { value, interpolated } => {
                interpolation::handle_string(self, value, *interpolated, scope, context);
            }
            NodeKind::Heredoc { content, interpolated, .. } => {
                interpolation::handle_heredoc(self, content, *interpolated, scope, context);
            }
            NodeKind::Assignment { lhs, rhs, op: _ } => {
                let _ = uses::handle_assignment(
                    self, node, lhs, rhs, scope, ancestors, issues, context,
                );
            }

            NodeKind::Tie { variable, package, args } => {
                uses::handle_tie(
                    self, node, variable, package, args, scope, ancestors, issues, context,
                );
            }

            NodeKind::Untie { variable } => {
                uses::handle_untie(self, node, variable, scope, ancestors, issues, context);
            }

            NodeKind::Identifier { name } => {
                uses::handle_identifier(
                    self,
                    node,
                    name,
                    issues,
                    context,
                    ancestors,
                    &pragma_state,
                    strict_subs_mode,
                );
            }

            NodeKind::Binary { op: _, left, right } => {
                // All binary operations (including {} and [])
                // We don't need special handling for {} and [] here because NodeKind::Variable
                // will handle the context-sensitive lookup (checking ancestors).
                calls_and_exprs::handle_binary(
                    self, node, left, right, scope, ancestors, issues, context,
                );
            }

            NodeKind::ArrayLiteral { elements } => {
                calls_and_exprs::handle_array_literal(
                    self, node, elements, scope, ancestors, issues, context,
                );
            }

            NodeKind::Block { statements } => {
                scope_constructs::handle_block(
                    self, node, statements, scope, ancestors, issues, context,
                );
            }

            NodeKind::PhaseBlock { block, .. } => {
                scope_constructs::handle_phase_block(
                    self, node, block, scope, ancestors, issues, context,
                );
            }

            NodeKind::For { init, condition, update, body, .. } => {
                scope_constructs::handle_for(
                    self,
                    node,
                    init.as_deref(),
                    condition.as_deref(),
                    update.as_deref(),
                    body,
                    scope,
                    ancestors,
                    issues,
                    context,
                );
            }

            NodeKind::Foreach { variable, list, body, continue_block } => {
                scope_constructs::handle_foreach(
                    self,
                    node,
                    variable,
                    list,
                    body,
                    continue_block.as_deref(),
                    scope,
                    ancestors,
                    issues,
                    context,
                );
            }

            NodeKind::Subroutine { signature, body, .. } => {
                scope_constructs::handle_subroutine(
                    self,
                    node,
                    signature.as_deref(),
                    body,
                    scope,
                    ancestors,
                    issues,
                    context,
                );
            }

            NodeKind::Try { body, catch_blocks, finally_block } => {
                scope_constructs::handle_try(
                    self,
                    node,
                    body,
                    catch_blocks,
                    finally_block.as_deref(),
                    scope,
                    ancestors,
                    issues,
                    context,
                );
            }

            NodeKind::Package { name, block, .. } => {
                scope_constructs::handle_package(
                    self,
                    node,
                    name,
                    block.as_deref(),
                    scope,
                    ancestors,
                    issues,
                    context,
                );
            }

            // Regex match operations set capture variables ($1, $2, ...) in the current scope.
            NodeKind::Match { expr, .. } => {
                interpolation::handle_match(self, node, expr, scope, ancestors, issues, context);
            }

            NodeKind::Substitution { expr, .. } => {
                interpolation::handle_substitution(
                    self, node, expr, scope, ancestors, issues, context,
                );
            }

            // Standalone regex (m// matching against $_) also sets capture variables.
            NodeKind::Regex { .. } => {
                interpolation::handle_regex(scope);
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
    pub(super) fn resolve_variable_use_target<'a>(
        &self,
        node: &'a Node,
        ancestors: &[&'a Node],
        context: &AnalysisContext<'_>,
    ) -> Option<(&'a str, &'a str)> {
        let NodeKind::Variable { sigil, name } = &node.kind else {
            return None;
        };

        // Explicit scalar-reference dereference forms should count as uses of the
        // underlying scalar lexical (`$ref`) rather than a container lexical of the
        // same bare name. This covers compact and braced syntaxes such as:
        // - `@$ref`, `%$ref`, `$$ref`
        // - `@{$ref}`, `%{$ref}`, `${$ref}`
        if (sigil == "@" || sigil == "%" || sigil == "$")
            && context
                .code
                .get(node.location.start..node.location.end)
                .is_some_and(is_explicit_scalar_reference_deref)
        {
            return Some(("$", normalize_scalar_deref_base_name(name)));
        }

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

        // Hash slice syntax (`@hash{...}`) reads from `%hash`, not a lexical `@hash`.
        // Bridge this so strict-vars and usage tracking resolve against the declared hash.
        if sigil == "@"
            && let Some(parent) = ancestors.last()
            && let NodeKind::Binary { op, left, .. } = &parent.kind
            && op == "{}"
            && std::ptr::eq(left.as_ref(), node)
        {
            return Some(("%", name));
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

    pub(super) fn extract_name_like_variable<'a>(
        &self,
        name: &'a str,
    ) -> Option<(&'a str, &'a str)> {
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

    pub(super) fn extract_method_name_variable<'a>(
        &self,
        method: &'a str,
    ) -> Option<(&'a str, &'a str)> {
        self.extract_name_like_variable(method).or_else(|| {
            let inner = method.strip_prefix("${")?.strip_suffix('}')?;
            if inner.contains("::") || !self.looks_like_variable_name(inner) {
                return None;
            }
            Some(("$", inner))
        })
    }

    pub(super) fn looks_like_variable_name(&self, name: &str) -> bool {
        matches!(
            name.chars().next(),
            Some('A'..='Z' | 'a'..='z' | '_' | '$' | '@' | '%' | '&' | '*' | '^' | '#' | '!' | '?')
        )
    }

    pub(super) fn is_dynamic_method_deref_rhs(&self, node: &Node) -> bool {
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

    pub(super) fn is_dynamic_method_deref_context<'a>(
        &self,
        node: &'a Node,
        ancestors: &[&'a Node],
    ) -> bool {
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

    pub(super) fn is_braced_dynamic_method_call(
        &self,
        node: &Node,
        context: &AnalysisContext<'_>,
    ) -> bool {
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

    pub(super) fn record_variable_use(
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

    pub(super) fn push_undeclared_variable_issue(
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

    pub(super) fn push_uninitialized_variable_issue(
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
    pub(super) fn mark_initialized(
        &self,
        node: &Node,
        scope: &Rc<Scope>,
        context: &AnalysisContext<'_>,
    ) {
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

    pub(super) fn analyze_block_with_scope<'a>(
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

    pub(super) fn mark_builtin_declaration_arg_consumed(
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

    pub(super) fn mark_interpolated_variables_used(
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

    pub(super) fn collect_unused_variables(
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

    pub(super) fn extract_variable_name<'a>(&self, node: &'a Node) -> ExtractedName<'a> {
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
    pub(super) fn is_in_hash_key_context(
        &self,
        node: &Node,
        ancestors: &[&Node],
        max_depth: usize,
    ) -> bool {
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

    /// Return one human-readable fix suggestion per issue.
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

fn collect_imported_barewords(ast: &Node) -> HashSet<String> {
    fn push_symbol(imported: &mut HashSet<String>, module: &str, token: &str) {
        let symbol = token.trim().trim_matches('\'').trim_matches('"').trim();
        if symbol.is_empty() || symbol == "," {
            return;
        }

        if symbol.starts_with(':') {
            if let Some(expanded) = resolve_known_export_tag(module, symbol) {
                imported.extend(expanded.iter().map(|name| (*name).to_string()));
            }
            return;
        }

        let is_bareword = symbol.bytes().all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
            && symbol
                .as_bytes()
                .first()
                .is_some_and(|first| first.is_ascii_alphabetic() || *first == b'_');
        if is_bareword {
            imported.insert(symbol.to_string());
        }
    }

    fn require_module_name(node: &Node) -> Option<String> {
        let NodeKind::FunctionCall { name, args } = &node.kind else {
            return None;
        };
        if name != "require" {
            return None;
        }
        let first = args.first()?;
        match &first.kind {
            NodeKind::Identifier { name } => Some(name.clone()),
            NodeKind::String { value, .. } => {
                let cleaned = value.trim_matches('\'').trim_matches('"').trim();
                if cleaned.is_empty() {
                    return None;
                }
                Some(cleaned.trim_end_matches(".pm").replace('/', "::"))
            }
            _ => None,
        }
    }

    fn require_variable_name(node: &Node) -> Option<String> {
        let NodeKind::FunctionCall { name, args } = &node.kind else {
            return None;
        };
        if name != "require" {
            return None;
        }
        let first = args.first()?;
        let NodeKind::Variable { sigil, name } = &first.kind else {
            return None;
        };
        (sigil == "$" && !name.contains("::")).then(|| name.clone())
    }

    fn maybe_record_manual_imports(
        node: &Node,
        required_modules: &HashSet<String>,
        imported: &mut HashSet<String>,
    ) {
        let NodeKind::MethodCall { object, method, args } = &node.kind else {
            return;
        };
        if method != "import" {
            return;
        }
        let NodeKind::Identifier { name: module } = &object.kind else {
            return;
        };
        if !required_modules.contains(module) {
            return;
        }
        for arg in args {
            match &arg.kind {
                NodeKind::String { value, .. } => push_symbol(imported, module, value),
                NodeKind::Identifier { name } => {
                    if name.starts_with("qw") {
                        let content = name
                            .trim_start_matches("qw")
                            .trim_start_matches(|c: char| "([{/<|!".contains(c))
                            .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                        for token in content.split_whitespace() {
                            push_symbol(imported, module, token);
                        }
                    } else {
                        push_symbol(imported, module, name);
                    }
                }
                NodeKind::ArrayLiteral { elements } => {
                    for el in elements {
                        if let NodeKind::String { value, .. } = &el.kind {
                            push_symbol(imported, module, value);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    fn maybe_record_dynamic_manual_imports(
        node: &Node,
        dynamic_require_vars: &HashSet<String>,
        imported: &mut HashSet<String>,
    ) {
        let NodeKind::MethodCall { object, method, args } = &node.kind else {
            return;
        };
        if method != "import" {
            return;
        }
        let NodeKind::Variable { sigil, name } = &object.kind else {
            return;
        };
        if sigil != "$" || !dynamic_require_vars.contains(name) {
            return;
        }

        for arg in args {
            match &arg.kind {
                NodeKind::String { value, .. } => push_symbol(imported, "", value),
                NodeKind::Identifier { name } => {
                    if name.starts_with("qw") {
                        let content = name
                            .trim_start_matches("qw")
                            .trim_start_matches(|c: char| "([{/<|!".contains(c))
                            .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                        for token in content.split_whitespace() {
                            push_symbol(imported, "", token);
                        }
                    } else {
                        push_symbol(imported, "", name);
                    }
                }
                NodeKind::ArrayLiteral { elements } => {
                    for el in elements {
                        if let NodeKind::String { value, .. } = &el.kind {
                            push_symbol(imported, "", value);
                        }
                    }
                }
                _ => {}
            }
        }
    }

    /// Unwrap an `ExpressionStatement` node to its inner expression, or return
    /// the node itself if it is not an expression statement.
    fn inner_node(stmt: &Node) -> &Node {
        if let NodeKind::ExpressionStatement { expression } = &stmt.kind {
            expression.as_ref()
        } else {
            stmt
        }
    }

    // `in_eval` — when true we are inside a runtime `eval { }` block and
    // `require` statements are no longer static; skip the require+import
    // suppression analysis for the current block.
    fn visit(node: &Node, imported: &mut HashSet<String>, in_eval: bool) {
        if let NodeKind::Use { module, args, .. } = &node.kind {
            for arg in args {
                if arg.starts_with("qw") {
                    let content = arg
                        .trim_start_matches("qw")
                        .trim_start_matches(|c: char| "([{/<|!".contains(c))
                        .trim_end_matches(|c: char| ")]}/|!>".contains(c));
                    for token in content.split_whitespace() {
                        push_symbol(imported, module, token);
                    }
                } else {
                    push_symbol(imported, module, arg);
                }
            }
        } else if !in_eval {
            if let NodeKind::Program { statements } | NodeKind::Block { statements } = &node.kind {
                let required_modules: HashSet<String> = statements
                    .iter()
                    .filter_map(|stmt| require_module_name(inner_node(stmt)))
                    .collect();
                let dynamic_require_vars: HashSet<String> = statements
                    .iter()
                    .filter_map(|stmt| require_variable_name(inner_node(stmt)))
                    .collect();
                if !required_modules.is_empty() || !dynamic_require_vars.is_empty() {
                    for stmt in statements {
                        let inner = inner_node(stmt);
                        maybe_record_manual_imports(inner, &required_modules, imported);
                        maybe_record_dynamic_manual_imports(inner, &dynamic_require_vars, imported);
                    }
                }
            }
        }

        // Propagate eval context: children of an Eval block are runtime.
        let child_in_eval = in_eval || matches!(&node.kind, NodeKind::Eval { .. });
        for child in node.children() {
            visit(child, imported, child_in_eval);
        }
    }

    let mut imported = HashSet::new();
    visit(ast, &mut imported, false);
    imported
}

/// Returns true if `name` (without sigil) is a numbered capture variable.
///
/// Capture variables are `$1`, `$2`, ..., `$9`, `$10`, `$11`, etc.
/// `$0` is the program name and is NOT a capture variable.
#[inline]
pub(super) fn is_capture_variable(name: &str) -> bool {
    // Must be non-empty, all digits, and not "0" (which is $0 = program name)
    !name.is_empty() && name != "0" && name.as_bytes().iter().all(|c| c.is_ascii_digit())
}

/// Check if a variable is a built-in Perl global variable
pub(super) fn is_builtin_global(sigil: &str, name: &str) -> bool {
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
        b'%' => matches!(name, "_" | "+" | "-" | "!" | "ENV" | "INC" | "SIG" | "EXPORT_TAGS"),
        _ => false,
    }
}

/// Check if an identifier is a known Perl built-in function
pub(super) fn is_known_function(name: &str) -> bool {
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
pub(super) fn builtin_declaration_arg_positions(name: &str) -> &'static [usize] {
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
pub(super) fn is_topic_defaulting_builtin(name: &str) -> bool {
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

/// Topic-defaulting builtins that also modify `$_` when called without args.
pub(super) fn is_topic_modifying_builtin(name: &str) -> bool {
    matches!(name, "chomp" | "chop")
}

fn is_explicit_scalar_reference_deref(source: &str) -> bool {
    source.starts_with("@$")
        || source.starts_with("%$")
        || source.starts_with("$$")
        || source.starts_with("@{$")
        || source.starts_with("%{$")
        || source.starts_with("${$")
}

fn normalize_scalar_deref_base_name(name: &str) -> &str {
    let unwrapped =
        name.strip_prefix('{').and_then(|inner| inner.strip_suffix('}')).unwrap_or(name);

    unwrapped.strip_prefix('$').unwrap_or(unwrapped)
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
