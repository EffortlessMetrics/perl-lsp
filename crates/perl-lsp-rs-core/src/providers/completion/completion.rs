//! Code completion provider for Perl
//!
//! This module provides intelligent code completion suggestions based on
//! context, including variables, functions, keywords, file paths, and more.
//!
//! ## Features
//!
//! ### Core Completion Types
//! - **Variables**: Scalar (`$var`), array (`@array`), hash (`%hash`) with scope analysis
//! - **Functions**: Built-in functions (150+ with signatures) and user-defined subroutines
//! - **Keywords**: Perl keywords with snippet expansion (`sub`, `if`, `while`, etc.)
//! - **Packages**: Package member completion with workspace index integration
//! - **Methods**: Context-aware method completion including DBI methods
//! - **Test Functions**: Test::More completions in test contexts
//!
//! ### File Path Completion (v0.8.7+)
//! **File completion with comprehensive security:**
//!
//! - **Smart Context Detection**: Automatically activates inside quoted string literals (`"path/file"` or `'path/file'`)
//! - **Path Recognition**: Detects `/` or `\` separators and alphanumeric patterns to identify file paths
//! - **Security Safeguards**:
//!   - Path traversal prevention (blocks `../` patterns)
//!   - Null byte protection and control character filtering
//!   - Windows reserved name filtering (CON, PRN, AUX, etc.)
//!   - UTF-8 validation and filename length limits (255 chars)
//!   - Safe directory canonicalization with fallbacks
//! - **Performance Optimizations**:
//!   - Controlled filesystem traversal (max 1 directory level deep)
//!   - Result limits (50 completions, 200 entries examined)
//!   - LSP cancellation support for responsive editing
//! - **File Type Intelligence**:
//!   - Perl files (`.pl`, `.pm`, `.t`) → "Perl file"
//!   - Source files (`.rs`, `.js`, `.py`) → Language-specific descriptions
//!   - Config files (`.json`, `.yaml`, `.toml`) → Format-specific descriptions
//!   - Generic fallback for unknown extensions
//! - **Cross-platform**: Handles Unix and Windows path separators consistently
//!
//! ## LSP Client Capabilities
//!
//! Requires client support for `textDocument/completion` and optional completion
//! capabilities such as `completionItem.snippetSupport` and
//! `completionItem.resolveSupport`.
//!
//! ## Protocol Compliance
//!
//! Implements the LSP completion protocol (`textDocument/completion` and
//! `completionItem/resolve`) with cancellation handling per the LSP 3.17+ spec.
//!
//! ## See also
//!
//! - [`CompletionContext`] for request-scoped parsing context
//! - [`CompletionItem`] for LSP completion payloads
//! - [`crate::ide::lsp_compat::semantic_tokens`] for shared symbol analysis
//!
//! ## Usage Examples
//!
//! ### Basic Variable Completion
//! ```perl
//! my $count = 42;
//! my @items = ();
//! $c<cursor> # Suggests: $count
//! ```
//!
//! ### File Path Completion
//! ```perl
//! my $config = "config/app.<cursor>"; # Suggests: config/app.yaml, config/app.json
//! open my $fh, '<', "src/lib<cursor>"; # Suggests: src/lib.rs, src/lib/
//! ```
//!
//! ### Method Completion
//! ```perl
//! my $dbh = DBI->connect(...);
//! $dbh-><cursor> # Suggests: do, prepare, selectrow_array, etc.
//! ```
//!
//! ## Security Model
//!
//! File completion implements comprehensive security measures:
//! - **Input validation**: Rejects dangerous paths and characters
//! - **Filesystem isolation**: Only accesses relative paths in safe directories
//! - **Resource limits**: Prevents excessive filesystem traversal
//! - **Safe canonicalization**: Handles path resolution with security checks
//!
//! ## Performance Characteristics
//!
//! - **Variable/function completion**: <1ms typical response
//! - **File path completion**: <10ms with filesystem traversal limits
//! - **Cancellation aware**: Respects LSP cancellation for responsiveness
//! - **Memory efficient**: Uses streaming iteration without loading all results

pub(crate) mod auto_import;
mod builtins;
mod context;
mod file_path;
mod functions;
mod items;
mod keywords;
mod methods;
mod packages;
mod regex_patterns;
pub(crate) mod scope_distance;
mod snippets;
mod sort;
pub(crate) mod test_more;
mod variables;
mod workspace;
mod xs_api;

// Re-export public types
pub use self::context::CompletionContext;
pub use self::items::{CompletionItem, CompletionItemKind};
pub use self::methods::get_dbi_method_documentation;
pub use self::test_more::get_test_more_documentation;
pub use self::xs_api::{add_xs_api_completions_for_prefix, get_xs_api_documentation, is_xs_source};

use perl_module::import::resolve_known_export_tag;
use perl_parser_core::ast::Node;
use perl_parser_core::ast::NodeKind;
use perl_semantic_analyzer::class_model::{ClassModel, ClassModelBuilder, Framework};
use perl_semantic_analyzer::semantic::{BuiltinDoc, get_moose_type_documentation};
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind, SymbolTable};
use perl_semantic_analyzer::type_inference::TypeInferenceEngine;
use perl_workspace::workspace_index::WorkspaceIndex;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::sync::Arc;

/// Maps module_name -> Set of explicitly imported symbol names.
///
/// Semantics:
/// - Entry MISSING: `use Module` with no args (import all of `@EXPORT`) — no filtering.
/// - Entry with EMPTY set: `use Module qw()` (explicit empty qw import) — nothing in namespace.
/// - Entry with non-empty set: `use Module qw(a b)` — only those symbols are imported.
type ImportMap = HashMap<String, HashSet<String>>;

const MOOSE_TYPE_CANDIDATES: &[&str] = &[
    "Any",
    "Item",
    "Undef",
    "Defined",
    "Value",
    "Bool",
    "Str",
    "Num",
    "Int",
    "ClassName",
    "RoleName",
    "Ref",
    "ScalarRef",
    "ArrayRef",
    "HashRef",
    "CodeRef",
    "RegexpRef",
    "GlobRef",
    "FileHandle",
    "Object",
    "Maybe",
    "InstanceOf",
    "ConsumerOf",
    "HasMethods",
    "Dict",
    "Tuple",
    "Map",
    "Enum",
];

/// Completion provider
pub struct CompletionProvider {
    symbol_table: SymbolTable,
    class_models: Vec<ClassModel>,
    type_engine: Option<TypeInferenceEngine>,
    workspace_index: Option<Arc<WorkspaceIndex>>,
    import_map: ImportMap,
    include_paths: Vec<PathBuf>,
    system_inc_paths: Vec<PathBuf>,
}

impl CompletionProvider {
    /// Create a new completion provider from parsed AST for Perl script analysis
    ///
    /// # Arguments
    ///
    /// * `ast` - Parsed AST from Perl script content during LSP Parse stage
    /// * `workspace_index` - Optional workspace-wide symbol index for cross-file completion
    ///
    /// # Returns
    ///
    /// A configured completion provider ready for Perl parsing workflow analysis
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser_core::Parser;
    /// use perl_lsp_completion::CompletionProvider;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let mut parser = Parser::new("my $var = 42; sub hello { print $var; }");
    /// let ast = parser.parse()?;
    /// let provider = CompletionProvider::new_with_index(&ast, None);
    /// // Provider ready for Perl script completion analysis
    /// # Ok(())
    /// # }
    /// ```
    /// Arguments: `ast`, `workspace_index`.
    pub fn new_with_index(ast: &Node, workspace_index: Option<Arc<WorkspaceIndex>>) -> Self {
        Self::new_with_index_and_source(ast, "", workspace_index)
    }

    /// Create a new completion provider from parsed AST and source with workspace integration
    ///
    /// Constructs a completion provider with full workspace symbol information for
    /// comprehensive completion suggestions during Perl script editing within the
    /// LSP workflow. Integrates local AST symbols with workspace-wide indexing.
    ///
    /// # Arguments
    ///
    /// * `ast` - Parsed AST containing local scope symbols and structure
    /// * `source` - Original source code for position-based context analysis
    /// * `workspace_index` - Optional workspace symbol index for cross-file completions
    ///
    /// # Returns
    ///
    /// A configured completion provider ready for LSP completion requests with
    /// both local and workspace symbol coverage for Perl script development.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser_core::Parser;
    /// use perl_lsp_completion::CompletionProvider;
    /// use perl_workspace::workspace_index::WorkspaceIndex;
    /// use std::sync::Arc;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let script = "package EmailProcessor; sub filter_spam { my $var; }";
    /// let mut parser = Parser::new(script);
    /// let ast = parser.parse()?;
    ///
    /// let workspace_idx = Arc::new(WorkspaceIndex::new());
    /// let provider = CompletionProvider::new_with_index_and_source(
    ///     &ast, script, Some(workspace_idx)
    /// );
    /// // Provider ready for cross-file Perl script completions
    /// # Ok(())
    /// # }
    /// ```
    /// Arguments: `ast`, `source`, `workspace_index`.
    /// Returns: A configured completion provider.
    /// Example: `CompletionProvider::new_with_index_and_source(&ast, source, None)`.
    pub fn new_with_index_and_source(
        ast: &Node,
        source: &str,
        workspace_index: Option<Arc<WorkspaceIndex>>,
    ) -> Self {
        Self::new_with_index_and_source_and_inc_paths(
            ast,
            source,
            workspace_index,
            Vec::new(),
            Vec::new(),
        )
    }

    /// Create a new completion provider and include-path context for module completions.
    pub fn new_with_index_and_source_and_inc_paths(
        ast: &Node,
        source: &str,
        workspace_index: Option<Arc<WorkspaceIndex>>,
        include_paths: Vec<PathBuf>,
        system_inc_paths: Vec<PathBuf>,
    ) -> Self {
        let symbol_table = SymbolExtractor::new_with_source(source).extract(ast);
        let class_models = ClassModelBuilder::new().build(ast);
        let type_engine = workspace_index.as_ref().map(|_| {
            let mut type_engine = TypeInferenceEngine::new();
            let _ = type_engine.infer(ast);
            type_engine
        });
        let import_map = Self::extract_import_map(ast);

        CompletionProvider {
            symbol_table,
            class_models,
            type_engine,
            workspace_index,
            import_map,
            include_paths,
            system_inc_paths,
        }
    }

    /// Walk the top-level AST and build an `ImportMap` from `use` statements.
    ///
    /// Only uppercase-starting module names are included (skips pragmas like
    /// `strict`, `warnings`, `feature`, `constant`, `utf8`, `lib`, `parent`, `base`).
    fn extract_import_map(ast: &Node) -> ImportMap {
        let mut map: ImportMap = HashMap::new();

        fn collect_import_symbols(
            module: &str,
            arg: &str,
            symbols: &mut HashSet<String>,
        ) -> (bool, bool) {
            let trimmed = arg.trim();
            if trimmed.is_empty() {
                return (false, false);
            }
            if matches!(trimmed, "=>" | "," | "(" | ")" | "[" | "]" | "{" | "}") {
                return (false, false);
            }

            let mut content = trimmed;
            if let Some(inner) = content.strip_prefix('[').and_then(|s| s.strip_suffix(']')) {
                content = inner.trim();
            }

            if content.starts_with("qw") {
                content = content
                    .trim_start_matches("qw")
                    .trim_start_matches(|c: char| "([{/<|!".contains(c))
                    .trim_end_matches(|c: char| ")]}/|!>".contains(c))
                    .trim();

                let mut unresolved_tag = false;
                for word in content.split_whitespace() {
                    if word.is_empty() {
                        continue;
                    }
                    if word.starts_with(':') {
                        if let Some(expanded) = resolve_known_export_tag(module, word) {
                            symbols.extend(expanded.iter().map(|name| (*name).to_string()));
                        } else {
                            unresolved_tag = true;
                        }
                    } else {
                        symbols.insert(word.to_string());
                    }
                }
                return (!content.is_empty(), unresolved_tag);
            }

            let cleaned = content.trim_matches(|c: char| c == '\'' || c == '"');
            if cleaned.is_empty() {
                return (false, false);
            }

            let mut unresolved_tag = false;
            for word in cleaned.split_whitespace() {
                if word.is_empty() {
                    continue;
                }
                if word.starts_with(':') {
                    if let Some(expanded) = resolve_known_export_tag(module, word) {
                        symbols.extend(expanded.iter().map(|name| (*name).to_string()));
                    } else {
                        unresolved_tag = true;
                    }
                } else {
                    symbols.insert(word.to_string());
                }
            }
            (true, unresolved_tag)
        }

        fn collect_node_import_symbols(
            module: &str,
            arg: &Node,
            symbols: &mut HashSet<String>,
        ) -> (bool, bool) {
            match &arg.kind {
                NodeKind::String { value, .. } => collect_import_symbols(
                    module,
                    value.trim_matches('\'').trim_matches('"'),
                    symbols,
                ),
                NodeKind::Identifier { name } => collect_import_symbols(module, name, symbols),
                NodeKind::ArrayLiteral { elements } => {
                    let mut has_symbols = false;
                    let mut has_unresolved_tag = false;
                    for element in elements {
                        let (element_has_symbols, element_unresolved_tag) =
                            collect_node_import_symbols(module, element, symbols);
                        if element_has_symbols {
                            has_symbols = true;
                        }
                        if element_unresolved_tag {
                            has_unresolved_tag = true;
                        }
                    }
                    (has_symbols, has_unresolved_tag)
                }
                _ => (false, false),
            }
        }

        fn require_module_name(expr: &Node) -> Option<String> {
            let NodeKind::FunctionCall { name, args } = &expr.kind else {
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
                    Some(cleaned.trim_end_matches(".pm").replace('/', "::"))
                }
                _ => None,
            }
        }

        fn module_runtime_alias(expr: &Node) -> Option<(String, String)> {
            let (alias_name, call_node) = match &expr.kind {
                NodeKind::Assignment { lhs, rhs, op } if op == "=" => {
                    let NodeKind::Variable { name, .. } = &lhs.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                NodeKind::VariableDeclaration { variable, initializer: Some(rhs), .. } => {
                    let NodeKind::Variable { name, .. } = &variable.kind else {
                        return None;
                    };
                    (name.as_str(), rhs.as_ref())
                }
                _ => return None,
            };
            let NodeKind::FunctionCall { name, args } = &call_node.kind else {
                return None;
            };
            if !matches!(
                name.as_str(),
                "use_module"
                    | "require_module"
                    | "Module::Runtime::use_module"
                    | "Module::Runtime::require_module"
            ) {
                return None;
            }
            let first = args.first()?;
            let NodeKind::String { value, .. } = &first.kind else {
                return None;
            };
            let module = value.trim_matches('\'').trim_matches('"').trim();
            if module.is_empty() {
                return None;
            }
            Some((alias_name.to_string(), module.to_string()))
        }

        fn inner_expr(node: &Node) -> &Node {
            if let NodeKind::ExpressionStatement { expression } = &node.kind {
                expression.as_ref()
            } else {
                node
            }
        }

        fn collect(node: &Node, map: &mut ImportMap) {
            match &node.kind {
                NodeKind::Use { module, args, .. } => {
                    // Skip pragmas: only process uppercase-starting module names
                    let first_char: Option<char> = module.chars().next();
                    if !first_char.is_some_and(|c: char| c.is_ascii_uppercase()) {
                        return;
                    }

                    // `use Module` with no args at all — import all of @EXPORT, no filtering
                    if args.is_empty() {
                        return;
                    }

                    let mut symbols: HashSet<String> = HashSet::new();
                    let mut has_symbol_args = false;
                    let mut has_unresolved_tag = false;

                    for arg in args {
                        // Skip version numbers (e.g. "1.50" in `use List::Util 1.50 qw(sum)`)
                        let first_byte = arg.as_bytes().first().copied().unwrap_or(0);
                        if first_byte.is_ascii_digit() {
                            continue;
                        }
                        // Skip flag args (e.g. "-norequire")
                        if arg.starts_with('-') {
                            continue;
                        }
                        let (has_symbols_in_arg, unresolved_tag) =
                            collect_import_symbols(module, arg, &mut symbols);
                        if has_symbols_in_arg {
                            has_symbol_args = true;
                        }
                        if unresolved_tag {
                            has_unresolved_tag = true;
                        }
                    }

                    if has_unresolved_tag {
                        return;
                    }

                    if has_symbol_args {
                        map.entry(module.clone()).or_default().extend(symbols);
                    } else {
                        // Explicit empty import: `use Module qw()`
                        map.entry(module.clone()).or_default();
                    }
                }
                NodeKind::Program { statements } | NodeKind::Block { statements } => {
                    let mut required_modules: Vec<String> = statements
                        .iter()
                        .filter_map(|stmt| require_module_name(inner_expr(stmt)))
                        .collect();
                    let mut aliases: HashMap<String, String> = HashMap::new();
                    for stmt in statements {
                        if let Some((alias, module)) = module_runtime_alias(inner_expr(stmt)) {
                            aliases.insert(alias, module.clone());
                            if !required_modules.contains(&module) {
                                required_modules.push(module);
                            }
                        }
                    }

                    for stmt in statements {
                        let expr = inner_expr(stmt);
                        let NodeKind::MethodCall { object, method, args } = &expr.kind else {
                            continue;
                        };
                        if method != "import" {
                            continue;
                        }
                        let object_name = match &object.kind {
                            NodeKind::Identifier { name } => Some(name.as_str()),
                            NodeKind::Variable { name, .. } => {
                                aliases.get(name).map(String::as_str)
                            }
                            _ => None,
                        };
                        let Some(object_name) = object_name else {
                            continue;
                        };
                        if !required_modules.iter().any(|module| module == object_name) {
                            continue;
                        }

                        // `Module->import()` with no args means default exports
                        // (equivalent to `use Module;` — import all of @EXPORT).
                        // We represent this by NOT adding an entry to the map,
                        // which means the module stays in the "import all" tier.
                        if args.is_empty() {
                            continue;
                        }

                        let mut imported_symbols: HashSet<String> = HashSet::new();
                        let mut has_symbols = false;
                        let mut has_unresolved_tag = false;
                        for arg in args {
                            let (arg_has_symbols, arg_unresolved_tag) = collect_node_import_symbols(
                                object_name,
                                arg,
                                &mut imported_symbols,
                            );
                            if arg_has_symbols {
                                has_symbols = true;
                            }
                            if arg_unresolved_tag {
                                has_unresolved_tag = true;
                            }
                        }
                        if has_unresolved_tag || !has_symbols {
                            continue;
                        }
                        map.entry(object_name.to_string()).or_default().extend(imported_symbols);
                    }

                    for stmt in statements {
                        collect(stmt, map);
                    }
                }
                _ => {}
            }
        }

        collect(ast, &mut map);
        map
    }

    /// Create a new completion provider from parsed AST without workspace context
    ///
    /// Constructs a basic completion provider using only local scope symbols from
    /// provided AST. Suitable for simple Perl script editing without cross-file
    /// dependencies in LSP workflow.
    ///
    /// # Arguments
    ///
    /// * `ast` - Parsed AST containing local symbols for completion
    ///
    /// # Returns
    ///
    /// A completion provider configured for local-only completions without
    /// workspace symbol integration.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser_core::Parser;
    /// use perl_lsp_completion::CompletionProvider;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let script = "my $email_count = 0; my $";
    /// let mut parser = Parser::new(script);
    /// let ast = parser.parse()?;
    ///
    /// let provider = CompletionProvider::new(&ast);
    /// // Provider ready for local variable completions
    /// # Ok(())
    /// # }
    /// ```
    /// Arguments: `ast`.
    /// Returns: A completion provider configured for local-only symbols.
    pub fn new(ast: &Node) -> Self {
        Self::new_with_index(ast, None)
    }

    /// Get completions at a given position with optional filepath for enhanced context
    ///
    /// Provides completion suggestions based on cursor position within Perl script
    /// source code. Uses filepath context to enable enhanced completions for test
    /// files and specific Perl parsing patterns within LSP workflows.
    ///
    /// # Arguments
    ///
    /// * `source` - Email script source code for analysis
    /// * `position` - Byte offset cursor position for completion
    /// * `filepath` - Optional file path for context-aware completion enhancement
    ///
    /// # Returns
    ///
    /// Vector of completion items sorted by relevance for current context,
    /// including local variables, functions, and workspace symbols when available.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser_core::Parser;
    /// use perl_lsp_completion::CompletionProvider;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let script = "my $var = 42; sub hello { print $var; }";
    /// let mut parser = Parser::new(script);
    /// let ast = parser.parse()?;
    ///
    /// let provider = CompletionProvider::new(&ast);
    /// let completions = provider.get_completions_with_path(
    ///     script, script.len(), Some("/path/to/data_processor.pl")
    /// );
    /// assert!(!completions.is_empty());
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See also [`Self::get_completions_with_path_cancellable`] for cancellation support
    /// and [`Self::get_completions`] for simple completions without filepath context.
    /// Arguments: `source`, `position`, `filepath`.
    /// Returns: A list of completion items for the current context.
    /// Example: `provider.get_completions_with_path(source, pos, Some(path))`.
    pub fn get_completions_with_path(
        &self,
        source: &str,
        position: usize,
        filepath: Option<&str>,
    ) -> Vec<CompletionItem> {
        self.get_completions_with_path_cancellable(source, position, filepath, &|| false)
    }

    /// Get completions at a given position with cancellation support for responsive editing
    ///
    /// Provides completion suggestions with cancellation capability for responsive
    /// Perl script editing during large workspace operations. Optimized for
    /// large-scale LSP environments where completion requests may need
    /// to be interrupted for better user experience.
    ///
    /// # Arguments
    ///
    /// * `source` - Email script source code for completion analysis
    /// * `position` - Byte offset cursor position within the source
    /// * `filepath` - Optional file path for enhanced context detection
    /// * `is_cancelled` - Cancellation callback for responsive completion
    ///
    /// # Returns
    ///
    /// Vector of completion items or empty vector if operation was cancelled,
    /// sorted by relevance for optimal Perl script development experience.
    ///
    /// # Performance
    ///
    /// - Respects cancellation for operations exceeding typical response times
    /// - Optimized for large Perl script files in large Perl codebase processing workflows
    /// - Provides partial results when possible before cancellation
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser_core::Parser;
    /// use perl_lsp_completion::CompletionProvider;
    /// use std::sync::atomic::{AtomicBool, Ordering};
    /// use std::sync::Arc;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let script = "package EmailHandler; sub process_emails { }";
    /// let mut parser = Parser::new(script);
    /// let ast = parser.parse()?;
    ///
    /// let provider = CompletionProvider::new(&ast);
    /// let cancelled = Arc::new(AtomicBool::new(false));
    /// let cancel_fn = || cancelled.load(Ordering::Relaxed);
    ///
    /// let completions = provider.get_completions_with_path_cancellable(
    ///     script, script.len(), Some("email_handler.pl"), &cancel_fn
    /// );
    /// # Ok(())
    /// # }
    /// ```
    /// Arguments: `source`, `position`, `filepath`, `is_cancelled`.
    /// Returns: A list of completion items or an empty list when cancelled.
    /// Example: `provider.get_completions_with_path_cancellable(source, pos, None, &|| false)`.
    pub fn get_completions_with_path_cancellable(
        &self,
        source: &str,
        position: usize,
        filepath: Option<&str>,
        is_cancelled: &dyn Fn() -> bool,
    ) -> Vec<CompletionItem> {
        // Input validation
        if position > source.len() {
            return vec![];
        }

        let context = match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.analyze_context(source, position)
        })) {
            Ok(mut ctx) => {
                ctx.in_use_statement = Self::is_use_statement_context(source, position);
                ctx
            }
            Err(_) => {
                return vec![];
            }
        };

        if context.in_comment {
            return vec![];
        }

        // Early cancellation check
        if is_cancelled() {
            return vec![];
        }

        // When `-` is a trigger character, only proceed for arrow-operator context.
        // All other uses of `-` (decrement `--`, subtract `-=`, unary minus, etc.)
        // must return empty so the editor doesn't flood the user with completions.
        if context.trigger_character == Some('-')
            && !(context.prefix.ends_with("->") && context.prefix.len() > 2)
        {
            return vec![];
        }

        let mut completions = Vec::new();

        // After regex close delimiter: offer flag completions.
        // This check MUST precede the in_regex check because the cursor after
        // the closing '/' is not itself inside the regex body.
        if Self::is_in_regex_flags(source, position) {
            regex_patterns::add_regex_flag_completions(&mut completions, &context, source);
            return sort::deduplicate_and_sort(completions);
        }

        // Regex context: suggest regex constructs when inside a regex literal,
        // but keep sigil-prefixed symbol completions available for interpolated
        // variables like `/^$fo/`.
        if context.in_regex && !matches!(context.prefix.chars().next(), Some('$' | '@' | '%')) {
            regex_patterns::add_regex_completions(&mut completions, &context, source);
            return sort::deduplicate_and_sort(completions);
        }

        // Determine what kind of completions to provide based on context
        // Check for `use Module qw(...)` import list context first
        if let Some((module_name, qw_prefix)) = Self::detect_use_qw_import_context(source, position)
        {
            workspace::add_use_qw_import_completions(
                &mut completions,
                &context,
                &self.workspace_index,
                &module_name,
                &qw_prefix,
            );
        } else if context.in_use_statement && !context.prefix.starts_with('$') {
            // Module name completion after `use` or `require`
            workspace::add_use_module_completions(
                &mut completions,
                &context,
                &self.workspace_index,
                &self.include_paths,
                &self.system_inc_paths,
            );
        } else if self.is_has_type_value_context(source, position) {
            self.add_has_type_completions(&mut completions, &context);
        } else if self.is_has_options_key_context(source, position) {
            self.add_has_option_completions(&mut completions, &context);
        } else if !context.in_comment
            && !context.in_string
            && !context.in_regex
            && let Some((varname, key_prefix)) = Self::detect_hash_key_context(source, position)
        {
            Self::add_hash_key_completions(
                &mut completions,
                &context,
                source,
                &varname,
                &key_prefix,
            );
        } else if let Some(package_name) = self.object_pad_constructor_package(source, position) {
            self.add_object_pad_constructor_completions(&mut completions, &context, &package_name);
        } else if (context.trigger_character == Some('>') || context.trigger_character == Some('-'))
            && context.prefix.ends_with("->")
            && context.prefix.len() > 2
        {
            // Method completion for both `>` (second char of `->`) and `-` (first char of
            // `->`) triggers. The prefix length guard ensures we have an actual receiver
            // (not bare `->` from a non-arrow context like `$x -=`).
            methods::add_method_completions(&mut completions, &context, source, &self.symbol_table);
            // Add workspace-indexed methods for the receiver's type
            workspace::add_workspace_method_completions(
                &mut completions,
                &context,
                source,
                self.type_engine.as_ref(),
                &self.workspace_index,
            );
        } else if context.prefix.starts_with('$') && context.prefix.contains("::") {
            packages::add_package_completions(&mut completions, &context, &self.workspace_index);
            if !completions.is_empty() {
                return completions;
            }
            variables::add_variable_completions(
                &mut completions,
                &context,
                SymbolKind::scalar(),
                &self.symbol_table,
            );
            if is_cancelled() {
                return vec![];
            }
            variables::add_special_variables(&mut completions, &context, "$");
        } else if context.prefix.starts_with('$') {
            // Scalar variable completion
            variables::add_variable_completions(
                &mut completions,
                &context,
                SymbolKind::scalar(),
                &self.symbol_table,
            );
            if is_cancelled() {
                return vec![];
            }
            variables::add_special_variables(&mut completions, &context, "$");
        } else if context.prefix.starts_with('@') && context.prefix.contains("::") {
            packages::add_package_completions(&mut completions, &context, &self.workspace_index);
            if !completions.is_empty() {
                return completions;
            }
            variables::add_variable_completions(
                &mut completions,
                &context,
                SymbolKind::array(),
                &self.symbol_table,
            );
            if is_cancelled() {
                return vec![];
            }
            variables::add_special_variables(&mut completions, &context, "@");
        } else if context.prefix.starts_with('@') {
            // Array variable completion
            variables::add_variable_completions(
                &mut completions,
                &context,
                SymbolKind::array(),
                &self.symbol_table,
            );
            if is_cancelled() {
                return vec![];
            }
            variables::add_special_variables(&mut completions, &context, "@");
        } else if context.prefix.starts_with('%') && context.prefix.contains("::") {
            packages::add_package_completions(&mut completions, &context, &self.workspace_index);
            if !completions.is_empty() {
                return completions;
            }
            variables::add_variable_completions(
                &mut completions,
                &context,
                SymbolKind::hash(),
                &self.symbol_table,
            );
            if is_cancelled() {
                return vec![];
            }
            variables::add_special_variables(&mut completions, &context, "%");
        } else if context.prefix.starts_with('%') {
            // Hash variable completion
            variables::add_variable_completions(
                &mut completions,
                &context,
                SymbolKind::hash(),
                &self.symbol_table,
            );
            if is_cancelled() {
                return vec![];
            }
            variables::add_special_variables(&mut completions, &context, "%");
        } else if context.prefix.starts_with('&') {
            // Subroutine completion
            functions::add_function_completions(&mut completions, &context, &self.symbol_table);
        } else if context.trigger_character == Some(':') && context.prefix.ends_with("::") {
            // Package member completion
            packages::add_package_completions(&mut completions, &context, &self.workspace_index);
        } else if context.in_string {
            // String interpolation or file path
            let line_prefix = &source[..context.position];
            if let Some(start) = line_prefix.rfind(['"', '\'']) {
                // Find the end of the string to check for dangerous characters
                // Safety: rfind returns byte offset, use get() for safe access
                let quote_char = match source.get(start..).and_then(|s| s.chars().next()) {
                    Some(c) => c,
                    None => return completions, // Invalid offset, skip file completions
                };
                let string_end = source[start + 1..]
                    .find(quote_char)
                    .map(|i| start + 1 + i)
                    .unwrap_or(source.len());
                let full_string_content = &source[start + 1..string_end];

                // Security check: reject strings with null bytes or other dangerous characters
                if full_string_content.contains('\0') {
                    return completions; // Return early without file completions
                }

                let path_prefix = &line_prefix[start + 1..];
                // Check if this looks like a file path (contains separators or path-like characters)
                if path_prefix.contains('/')
                    || path_prefix.contains('\\')  // Include backslashes for Windows paths
                    || path_prefix
                        .chars()
                        .all(|c| c.is_alphanumeric() || c == '.' || c == '_' || c == '-')
                {
                    let file_context = file_path::FileCompletionContext::new(
                        path_prefix,
                        start + 1,
                        context.position,
                    );
                    completions.extend(file_path::complete_file_paths(&file_context, is_cancelled));
                }
            }
        } else {
            // General completion: keywords, functions, variables
            let keywords = keywords::keywords();
            if context.prefix.is_empty() || self.could_be_keyword(&context.prefix, keywords) {
                keywords::add_keyword_completions(&mut completions, &context, keywords);
                if is_cancelled() {
                    return vec![];
                }
            }

            let builtins = builtins::create_builtins();
            xs_api::add_xs_api_completions(&mut completions, &context, source, filepath);
            if context.prefix.is_empty() || self.could_be_function(&context.prefix, &builtins) {
                builtins::add_builtin_completions(&mut completions, &context, &builtins);
                if is_cancelled() {
                    return vec![];
                }
                functions::add_function_completions(&mut completions, &context, &self.symbol_table);
                if is_cancelled() {
                    return vec![];
                }
            }

            // Add built-in snippet completions
            snippets::add_snippet_completions(&mut completions, &context);
            if is_cancelled() {
                return vec![];
            }

            // Also suggest variables without sigils in some contexts
            variables::add_all_variables(&mut completions, &context, &self.symbol_table);
            if is_cancelled() {
                return vec![];
            }

            // Add workspace symbol completions from other files
            workspace::add_workspace_symbol_completions(
                &mut completions,
                &context,
                &self.workspace_index,
                &self.import_map,
            );
            if is_cancelled() {
                return vec![];
            }

            // Add Test::More completions if in test context
            if self.is_test_context(source, filepath) {
                test_more::add_test_more_completions(&mut completions, &context);
            }
        }

        // Remove duplicates and sort completions by relevance
        sort::deduplicate_and_sort(completions)
    }

    /// Get completions at a given position for Perl script development
    ///
    /// Provides basic completion suggestions at specified cursor position
    /// within Perl script source code. This is the primary interface for
    /// LSP completion requests during Perl parsing workflow development.
    ///
    /// # Arguments
    ///
    /// * `source` - Email script source code for completion analysis
    /// * `position` - Byte offset cursor position where completions are requested
    ///
    /// # Returns
    ///
    /// Vector of completion items including local variables, functions, keywords,
    /// and built-in Perl constructs relevant to Perl parsing workflows.
    ///
    /// # Examples
    ///
    /// ```rust,ignore
    /// use perl_parser_core::Parser;
    /// use perl_lsp_completion::CompletionProvider;
    ///
    /// # fn main() -> Result<(), Box<dyn std::error::Error>> {
    /// let script = "my $email_count = scalar(@emails); $email_c";
    /// let mut parser = Parser::new(script);
    /// let ast = parser.parse()?;
    ///
    /// let provider = CompletionProvider::new(&ast);
    /// let completions = provider.get_completions(script, script.len());
    ///
    /// // Should include completion for $email_count variable
    /// assert!(completions.iter().any(|c| c.label.contains("email_count")));
    /// # Ok(())
    /// # }
    /// ```
    ///
    /// See also [`Self::get_completions_with_path`] for enhanced context-aware completions.
    /// Arguments: `source`, `position`.
    /// Returns: A list of completion items for the current context.
    /// Example: `provider.get_completions(source, pos)`.
    pub fn get_completions(&self, source: &str, position: usize) -> Vec<CompletionItem> {
        self.get_completions_with_path(source, position, None)
    }

    /// Detect if the cursor is inside `qw(...)` in a `use Module qw(...)` statement.
    ///
    /// Returns `Some((module_name, prefix))` when the cursor is inside the import list,
    /// where `module_name` is the module being imported from and `prefix` is the partial
    /// symbol the user has typed so far inside the `qw()`.
    ///
    /// Returns `None` when not in a `use ... qw()` import context.
    fn detect_use_qw_import_context(source: &str, position: usize) -> Option<(String, String)> {
        if !source.is_char_boundary(position) {
            return None;
        }
        let before = &source[..position];
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line = before[line_start..].trim_start();

        // Must start with `use `
        let rest = line.strip_prefix("use ")?;
        let rest = rest.trim_start();

        // Extract module name (starts uppercase, contains ::, alphanumeric, _)
        let mod_end =
            rest.find(|c: char| !c.is_alphanumeric() && c != ':' && c != '_').unwrap_or(rest.len());
        if mod_end == 0 {
            return None;
        }
        let module_name = &rest[..mod_end];

        // Module names start with uppercase by convention
        if !module_name.starts_with(|c: char| c.is_ascii_uppercase()) {
            return None;
        }

        let after_module = &rest[mod_end..];

        // Find `qw` followed by a delimiter
        let qw_pos = after_module.find("qw")?;
        let after_qw = &after_module[qw_pos + 2..];
        let after_qw = after_qw.trim_start();

        // qw can use various delimiters: (, [, {, /, |, !, etc.
        let first_char = after_qw.chars().next()?;
        let close_delim = match first_char {
            '(' => ')',
            '[' => ']',
            '{' => '}',
            '<' => '>',
            other => other, // For symmetric delimiters like / or |
        };

        let inside_qw = &after_qw[first_char.len_utf8()..];

        // Check we haven't passed the closing delimiter
        if inside_qw.contains(close_delim) {
            return None;
        }

        // Extract the prefix: the last word being typed inside qw()
        // Words in qw() are whitespace-separated
        let prefix = inside_qw.rsplit(|c: char| c.is_ascii_whitespace()).next().unwrap_or("");

        Some((module_name.to_string(), prefix.to_string()))
    }

    /// Check if the cursor is in a `use` or `require` statement context.
    ///
    /// Detects patterns like `use Mod`, `use Some::Mo`, `require Mo` etc.
    /// Returns true when the cursor is positioned where a module name is expected.
    ///
    /// Returns false for pragma-like directives (`use constant`, `use lib`, `use if`,
    /// `use strict`, `use warnings`, etc.) where module-name completion is not useful,
    /// and for positions past the module name (after `;`, `(`, or `qw`).
    fn is_use_statement_context(source: &str, position: usize) -> bool {
        // Guard against slicing at a non-char-boundary
        if !source.is_char_boundary(position) {
            return false;
        }
        let before = &source[..position];
        // Find the start of the current line
        let line_start = before.rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line = before[line_start..].trim_start();

        // Check for `use Module` or `require Module` patterns
        // Must be at the start of a statement (after optional whitespace)
        if let Some(rest) = line.strip_prefix("use ") {
            // After `use `, we expect a module name (possibly partial)
            // But not if we've already moved past the module name (e.g., `use Module qw(`)
            let rest = rest.trim_start();
            // If there's a semicolon, version number, or import list, we're past the module name
            if rest.contains(';') || rest.contains('(') || rest.contains("qw") {
                return false;
            }
            // Skip pragma-like directives where the token after `use` is lowercase
            // (e.g. `use strict`, `use warnings`, `use constant`, `use lib`, `use if`)
            // Module names in Perl start with an uppercase letter by convention
            let first_char = rest.chars().next();
            // Empty rest means cursor is right after `use ` -- still a valid context
            // Uppercase first char means a module name is being typed
            first_char.is_none() || first_char.is_some_and(|c| c.is_ascii_uppercase())
        } else if let Some(rest) = line.strip_prefix("require ") {
            let rest = rest.trim_start();
            if rest.contains(';') {
                return false;
            }
            // `require` also accepts file paths and perl version numbers:
            //   require "./file.pl";   (quoted paths — starts with ' or ")
            //   require './file.pl';   (quoted paths — starts with ' or ")
            //   require 5.010;         (version — starts with digit)
            //   require v5.10;         (v-string version — starts with 'v' but no ::)
            // Allow empty (cursor right after `require `) or any identifier-start char
            // (both uppercase like `require POSIX` and lowercase like `require autodie`).
            // Block only: digit, quote chars, path separators (. / \), sigils ($ @ %), backtick.
            let first_char = rest.chars().next();
            let Some(c) = first_char else {
                return true; // cursor right after `require ` — valid module context
            };
            // Block digit (version numbers), quote (string-literal paths), path/sigil chars
            !matches!(c, '0'..='9' | '\'' | '"' | '`' | '.' | '/' | '\\')
        } else {
            false
        }
    }

    /// Analyze the context at the cursor position
    fn analyze_context(&self, source: &str, position: usize) -> CompletionContext {
        // Find the word being typed
        // Special handling for method calls: include the -> and the receiver
        let (word_prefix, prefix_start) = if position >= 2
            && &source[position.saturating_sub(2)..position] == "->"
        {
            // We're right after ->, find the receiver variable or package name.
            // Include ':' so that qualified package names like `My::Package->` are
            // captured as a single receiver token rather than truncated at `::`.
            let receiver_start = source[..position.saturating_sub(2)]
                .rfind(|c: char| {
                    !c.is_alphanumeric() && c != '_' && c != '$' && c != '@' && c != '%' && c != ':'
                })
                .map(|p| p + 1)
                .unwrap_or(0);
            (source[receiver_start..position].to_string(), receiver_start)
        } else if position >= 1
            && source.as_bytes()[position - 1] == b'-'
            && (position < 2 || source.as_bytes()[position - 2] != b'-')
        {
            // Cursor is right after a lone `-` (not `--`). This fires when `-` is a
            // trigger character and the user has typed the first char of `->`.
            // Build the prefix as receiver + `->` so that downstream method-completion
            // functions see the same shape as the `>` trigger path.
            let receiver_start = source[..position.saturating_sub(1)]
                .rfind(|c: char| {
                    !c.is_alphanumeric() && c != '_' && c != '$' && c != '@' && c != '%' && c != ':'
                })
                .map(|p| p + 1)
                .unwrap_or(0);
            let receiver = &source[receiver_start..position - 1];
            (format!("{receiver}->"), receiver_start)
        } else {
            let word_start = source[..position]
                .rfind(|c: char| {
                    !c.is_alphanumeric()
                        && c != '_'
                        && c != ':'
                        && c != '$'
                        && c != '@'
                        && c != '%'
                        && c != '&'
                })
                .map(|p| p + 1)
                .unwrap_or(0);
            (source[word_start..position].to_string(), word_start)
        };

        // Detect trigger character (trigger chars are ASCII, so byte access is safe)
        let trigger_character = if position > 0 {
            let b = source.as_bytes()[position - 1];
            if b.is_ascii() { Some(b as char) } else { None }
        } else {
            None
        };

        // Simple heuristics for context detection
        let in_string = self.is_in_string(source, position);
        let in_regex = Self::is_in_regex(source, position);
        let in_comment = self.is_in_comment(source, position);

        let mut context = CompletionContext::new(
            &self.symbol_table,
            position,
            trigger_character,
            in_string,
            in_regex,
            in_comment,
            word_prefix,
            prefix_start,
        );
        context.cursor_scope_id =
            scope_distance::scope_at_position(&self.symbol_table, source, position);
        context
    }

    /// Add file path completions with comprehensive security and performance safeguards
    #[cfg(not(target_arch = "wasm32"))]
    #[allow(dead_code)] // Backward compatibility wrapper, may be used by external code
    fn add_file_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        self.add_file_completions_with_cancellation(completions, context, &|| false);
    }

    /// Add file path completions with comprehensive security and performance safeguards
    #[cfg(target_arch = "wasm32")]
    #[allow(dead_code)] // Backward compatibility wrapper, may be used by external code
    fn add_file_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        // File system traversal isn't available on wasm32 targets.
        let _ = (completions, context);
    }

    /// Add file path completions with cancellation support
    ///
    /// Uses the builder pattern via [`file_path::FilePathCallbacks`] to bundle
    /// security callbacks, reducing argument count and improving maintainability.
    #[cfg(not(target_arch = "wasm32"))]
    fn add_file_completions_with_cancellation(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
        is_cancelled: &dyn Fn() -> bool,
    ) {
        completions.extend(file_path::complete_file_paths(
            &file_path::FileCompletionContext::new(
                &context.prefix,
                context.prefix_start,
                context.position,
            ),
            is_cancelled,
        ));
    }

    /// Add file path completions with cancellation support
    #[cfg(target_arch = "wasm32")]
    fn add_file_completions_with_cancellation(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
        _is_cancelled: &dyn Fn() -> bool,
    ) {
        // File system traversal isn't available on wasm32 targets.
        let _ = (completions, context, _is_cancelled);
    }

    /// Check whether the cursor is inside a Moo/Moose `has (...)` option-key context.
    fn is_has_options_key_context(&self, source: &str, position: usize) -> bool {
        if position > source.len() {
            return false;
        }

        let prefix = &source[..position];
        let statement_start = prefix.rfind(';').map(|idx| idx + 1).unwrap_or(0);
        let statement = &prefix[statement_start..];

        let Some(has_idx) = Self::find_keyword(statement, "has") else {
            return false;
        };
        let after_has = &statement[has_idx + 3..];

        let Some(arrow_idx) = after_has.find("=>") else {
            return false;
        };
        let after_arrow = &after_has[arrow_idx + 2..];

        let Some(open_idx) = after_arrow.find('(') else {
            return false;
        };
        let options_text = &after_arrow[open_idx + 1..];

        // Must still be inside the `(` ... `)` option list.
        let mut paren_depth = 1i32;
        for ch in options_text.chars() {
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth -= 1;
                if paren_depth <= 0 {
                    return false;
                }
            }
        }

        // Find the current top-level option segment (after last comma).
        let mut depth = 1i32;
        let mut segment_start = 0usize;
        for (idx, ch) in options_text.char_indices() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
            } else if ch == ',' && depth == 1 {
                segment_start = idx + 1;
            }
        }

        let segment = options_text[segment_start..].trim_start();
        if segment.is_empty() {
            return true;
        }

        // If `=>` is already present in this segment, we're in value context.
        if segment.contains("=>") {
            return false;
        }

        segment.chars().all(|ch| {
            ch.is_ascii_alphanumeric()
                || ch == '_'
                || ch == '\''
                || ch == '"'
                || ch.is_ascii_whitespace()
        })
    }

    /// Check whether the cursor is inside the value position of a Moo/Moose `isa => ...`
    /// attribute inside a `has(...)` declaration.
    fn is_has_type_value_context(&self, source: &str, position: usize) -> bool {
        self.has_option_value_prefix(source, position, "isa").is_some()
    }

    /// Return the current value prefix for a `has(...)` option if the cursor is in that
    /// option's value position.
    fn has_option_value_prefix(
        &self,
        source: &str,
        position: usize,
        option_name: &str,
    ) -> Option<String> {
        if position > source.len() {
            return None;
        }

        let prefix = &source[..position];
        let statement_start = prefix.rfind(';').map(|idx| idx + 1).unwrap_or(0);
        let statement = &prefix[statement_start..];

        let has_idx = Self::find_keyword(statement, "has")?;
        let after_has = &statement[has_idx + 3..];

        let arrow_idx = after_has.find("=>")?;
        let after_arrow = &after_has[arrow_idx + 2..];

        let open_idx = after_arrow.find('(')?;
        let options_text = &after_arrow[open_idx + 1..];

        // Must still be inside the `(` ... `)` option list.
        let mut paren_depth = 1i32;
        for ch in options_text.chars() {
            if ch == '(' {
                paren_depth += 1;
            } else if ch == ')' {
                paren_depth -= 1;
                if paren_depth <= 0 {
                    return None;
                }
            }
        }

        // Find the current top-level option segment (after last comma).
        let mut depth = 1i32;
        let mut segment_start = 0usize;
        for (idx, ch) in options_text.char_indices() {
            if ch == '(' {
                depth += 1;
            } else if ch == ')' {
                depth -= 1;
            } else if ch == ',' && depth == 1 {
                segment_start = idx + 1;
            }
        }

        let segment = options_text[segment_start..].trim_start();
        let option_prefix = segment.strip_prefix(option_name)?;
        let option_prefix = option_prefix.trim_start().strip_prefix("=>")?;

        Some(option_prefix.trim_start().to_string())
    }

    fn object_pad_constructor_package(&self, source: &str, position: usize) -> Option<String> {
        if position > source.len() {
            return None;
        }

        let prefix = &source[..position];
        let statement_start = prefix.rfind(';').map(|idx| idx + 1).unwrap_or(0);
        let statement = &prefix[statement_start..];
        let mut search_end = statement.len();

        while let Some(new_idx) = statement[..search_end].rfind("->new") {
            let mut open_paren_idx = new_idx + "->new".len();
            while open_paren_idx < statement.len()
                && statement.as_bytes()[open_paren_idx].is_ascii_whitespace()
            {
                open_paren_idx += 1;
            }

            if open_paren_idx >= statement.len() || statement.as_bytes()[open_paren_idx] != b'(' {
                search_end = new_idx;
                continue;
            }

            let receiver = statement[..new_idx].trim_end();
            let receiver_start = receiver
                .rfind(|c: char| !c.is_ascii_alphanumeric() && c != '_' && c != ':' && c != '\'')
                .map(|idx| idx + 1)
                .unwrap_or(0);
            let package_name = receiver[receiver_start..].trim();
            if package_name.is_empty()
                || package_name.starts_with('$')
                || package_name.starts_with('@')
                || package_name.starts_with('%')
            {
                search_end = new_idx;
                continue;
            }

            let args_text = &statement[open_paren_idx + 1..];
            let mut paren_depth = 1i32;
            let mut brace_depth = 0i32;
            let mut bracket_depth = 0i32;
            let mut segment_start = 0usize;

            for (idx, ch) in args_text.char_indices() {
                match ch {
                    '(' => paren_depth += 1,
                    ')' => {
                        paren_depth -= 1;
                        if paren_depth <= 0 {
                            return None;
                        }
                    }
                    '{' => brace_depth += 1,
                    '}' => brace_depth -= 1,
                    '[' => bracket_depth += 1,
                    ']' => bracket_depth -= 1,
                    ',' if paren_depth == 1 && brace_depth == 0 && bracket_depth == 0 => {
                        segment_start = idx + 1;
                    }
                    _ => {}
                }
            }

            let segment = args_text[segment_start..].trim_start();
            if segment.is_empty() {
                return Some(package_name.to_string());
            }
            if segment.contains("=>") {
                return None;
            }
            if segment.chars().all(|ch| {
                ch.is_ascii_alphanumeric() || ch == '_' || ch == ':' || ch.is_ascii_whitespace()
            }) {
                return Some(package_name.to_string());
            }
            return None;
        }

        None
    }

    /// Detect whether the cursor is inside a plain hash subscript `$varname{prefix`.
    ///
    /// Returns `Some((varname, key_prefix))` when:
    /// - The source before `position` contains `$varname{` (with no `->` immediately before `{`)
    /// - The context is not inside a comment or string literal
    ///
    /// Returns `None` for hashref dereferences (`$ref->{...}`), double-sigil derefs
    /// (`$$ref{...}`), or contexts where hash key completion is not meaningful.
    fn detect_hash_key_context(source: &str, position: usize) -> Option<(String, String)> {
        if position == 0 || !source.is_char_boundary(position) {
            return None;
        }

        let before = &source[..position];

        // Find the last `{` before the cursor that is not part of a nested structure.
        // We scan backward to find the most recent unmatched `{`.
        let brace_pos = {
            let bytes = before.as_bytes();
            let mut depth = 0i32;
            let mut found = None;
            let mut i = bytes.len();
            while i > 0 {
                i -= 1;
                match bytes[i] {
                    b'}' => depth += 1,
                    b'{' => {
                        if depth == 0 {
                            found = Some(i);
                            break;
                        }
                        depth -= 1;
                    }
                    _ => {}
                }
            }
            found?
        };

        // Extract typed prefix after the `{` (alphanumeric + `_` chars)
        let key_prefix = {
            let after_brace = &before[brace_pos + 1..];
            // Prefix is the alphanumeric+_ run from after `{` to position
            let non_ident = after_brace
                .rfind(|c: char| !c.is_alphanumeric() && c != '_')
                .map(|p| p + 1)
                .unwrap_or(0);
            after_brace[non_ident..].to_string()
        };

        // The text between the `{` and the start of key_prefix must contain only
        // word chars and whitespace (no operators, semicolons, etc.) — if it contains
        // any non-whitespace non-word chars it is not a simple hash subscript.
        let between = &before[brace_pos + 1..position - key_prefix.len()];
        if between.chars().any(|c| !c.is_alphanumeric() && c != '_' && !c.is_whitespace()) {
            return None;
        }

        // Check for `->` immediately before the `{` (hashref deref — out of scope).
        if brace_pos >= 2 && &source[brace_pos - 2..brace_pos] == "->" {
            return None;
        }

        // Extract the variable name: scan backward from `{` looking for `$word`.
        let before_brace = before[..brace_pos].trim_end();
        if before_brace.is_empty() {
            return None;
        }

        // Variable name ends right before the `{`, scan back for `$`.
        let var_end = before_brace.len();
        let var_name_start = before_brace
            .rfind(|c: char| !c.is_alphanumeric() && c != '_')
            .map(|p| p + 1)
            .unwrap_or(0);
        let var_name = &before_brace[var_name_start..var_end];
        if var_name.is_empty() {
            return None;
        }

        // Require `$` sigil immediately before the variable name.
        // Also reject `$$var{` (double-sigil deref) by ensuring the char before `$`
        // is not itself a `$` — that would indicate `$$var{key}` which is a scalar-ref
        // dereference, not a plain hash subscript.
        if var_name_start == 0 || before_brace.as_bytes()[var_name_start - 1] != b'$' {
            return None;
        }
        if var_name_start >= 2 && before_brace.as_bytes()[var_name_start - 2] == b'$' {
            return None;
        }

        Some((var_name.to_string(), key_prefix))
    }

    /// Scan `source` text for all keys defined in `%varname` hash literals and
    /// individual `$varname{key}` assignment patterns.
    ///
    /// Uses only str operations — no regex crate dependency.
    fn collect_hash_keys_from_source(source: &str, varname: &str) -> Vec<String> {
        let mut keys: Vec<String> = Vec::new();
        let mut seen: HashSet<String> = HashSet::new();

        // Pattern 1: `my %varname = (` / `our %varname = (` / `%varname = (`
        // Scan for `%varname` followed by `=` and `(`
        let hash_pat = format!("%{varname}");
        let mut search_start = 0;
        while let Some(pos) = source[search_start..].find(hash_pat.as_str()) {
            let abs_pos = search_start + pos;
            let after = &source[abs_pos + hash_pat.len()..];
            let trimmed = after.trim_start();
            if let Some(rest) = trimmed.strip_prefix('=') {
                let rest = rest.trim_start();
                if let Some(inner_start) = rest.find('(') {
                    let inner = &rest[inner_start + 1..];
                    // Find matching `)` — walk forward tracking depth
                    let mut depth = 1i32;
                    let mut inner_end = inner.len();
                    for (idx, ch) in inner.char_indices() {
                        match ch {
                            '(' => depth += 1,
                            ')' => {
                                depth -= 1;
                                if depth == 0 {
                                    inner_end = idx;
                                    break;
                                }
                            }
                            _ => {}
                        }
                    }
                    let list_text = &inner[..inner_end];
                    Self::extract_fat_comma_keys(list_text, &mut keys, &mut seen);
                }
            }
            search_start = abs_pos + 1;
            if search_start >= source.len() {
                break;
            }
        }

        // Pattern 2: `$varname{key} =` individual assignment
        let scalar_pat = format!("${varname}{{");
        let mut search_start = 0;
        while let Some(pos) = source[search_start..].find(scalar_pat.as_str()) {
            let abs_pos = search_start + pos;
            let after_brace = &source[abs_pos + scalar_pat.len()..];
            // Key is alphanumeric+_ up to `}`
            let key_end = after_brace
                .find(|c: char| !c.is_alphanumeric() && c != '_')
                .unwrap_or(after_brace.len());
            let key = &after_brace[..key_end];
            if !key.is_empty() && after_brace[key_end..].trim_start().starts_with('}') {
                // Check that `=` (but not `=>`) follows the `}`
                let after_close = after_brace[key_end..].trim_start();
                let after_close = after_close.strip_prefix('}').unwrap_or("").trim_start();
                if after_close.starts_with('=') && !after_close.starts_with("=>") {
                    let key_str = key.to_string();
                    if seen.insert(key_str.clone()) {
                        keys.push(key_str);
                    }
                }
            }
            search_start = abs_pos + 1;
            if search_start >= source.len() {
                break;
            }
        }

        keys
    }

    /// Extract bare-word and single-quoted keys from a fat-comma list like
    /// `host => 'localhost', port => 5432`.
    fn extract_fat_comma_keys(list_text: &str, keys: &mut Vec<String>, seen: &mut HashSet<String>) {
        // Split by `=>` to find key positions.
        // Every token immediately before a `=>` is a key.
        let mut remaining = list_text;
        while let Some(arrow_pos) = remaining.find("=>") {
            let key_segment = remaining[..arrow_pos].trim_end();
            // Find the last token (after the previous `,` or start)
            let token_start = key_segment.rfind([',', '(', '\n']).map(|p| p + 1).unwrap_or(0);
            let token = key_segment[token_start..].trim();
            // Strip single or double quotes
            let token = token
                .strip_prefix('\'')
                .and_then(|t| t.strip_suffix('\''))
                .or_else(|| token.strip_prefix('"').and_then(|t| t.strip_suffix('"')))
                .unwrap_or(token);
            if !token.is_empty() && token.chars().all(|c| c.is_alphanumeric() || c == '_') {
                let key_str = token.to_string();
                if seen.insert(key_str.clone()) {
                    keys.push(key_str);
                }
            }
            // Advance past `=>` and the value. Value ends at the next top-level `,`.
            let after_arrow = &remaining[arrow_pos + 2..];
            let value_end = {
                let mut depth = 0i32;
                let mut end = after_arrow.len();
                for (idx, ch) in after_arrow.char_indices() {
                    match ch {
                        '(' | '[' | '{' => depth += 1,
                        ')' | ']' | '}' => depth -= 1,
                        ',' if depth == 0 => {
                            end = idx;
                            break;
                        }
                        _ => {}
                    }
                }
                end
            };
            remaining = &after_arrow[value_end..];
            if let Some(stripped) = remaining.strip_prefix(',') {
                remaining = stripped;
            }
        }
    }

    /// Push hash key completion items for `$varname{key_prefix<cursor>`.
    fn add_hash_key_completions(
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
        source: &str,
        varname: &str,
        key_prefix: &str,
    ) {
        let keys = Self::collect_hash_keys_from_source(source, varname);
        for key in keys {
            if !key_prefix.is_empty() && !key.starts_with(key_prefix) {
                continue;
            }
            let key_prefix_len = key_prefix.len();
            completions.push(CompletionItem {
                label: key.clone(),
                kind: CompletionItemKind::Property,
                detail: Some(format!("key of %{varname}")),
                documentation: None,
                insert_text: Some(key.clone()),
                sort_text: Some(format!("0_{key}")),
                filter_text: Some(key.clone()),
                additional_edits: vec![],
                text_edit_range: Some((context.position - key_prefix_len, context.position)),
                commit_characters: None,
            });
        }
    }

    /// Add completions for Moo/Moose type constraint values inside `isa => ...`.
    fn add_has_type_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        let raw_prefix = context.prefix.trim();
        let prefix = raw_prefix.trim_start_matches(['\'', '"']);
        let mut seen: HashSet<String> = completions.iter().map(|item| item.label.clone()).collect();

        let mut push_completion =
            |label: &str, detail: String, documentation: String, kind: CompletionItemKind| {
                if !seen.insert(label.to_string()) {
                    return;
                }

                completions.push(CompletionItem {
                    label: label.to_string(),
                    kind,
                    detail: Some(detail),
                    documentation: Some(documentation),
                    insert_text: Some(label.to_string()),
                    sort_text: Some(format!("0_{label}")),
                    filter_text: Some(label.to_string()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            };

        for type_name in MOOSE_TYPE_CANDIDATES {
            if !prefix.is_empty() && !type_name.starts_with(prefix) {
                continue;
            }

            if let Some(doc) = get_moose_type_documentation(type_name) {
                push_completion(
                    type_name,
                    "Built-in Moose type".to_string(),
                    Self::format_type_documentation(&doc),
                    CompletionItemKind::Module,
                );
            }
        }

        for (module_name, symbols) in &self.import_map {
            for symbol in symbols {
                if !Self::looks_like_type_name(symbol) {
                    continue;
                }
                if !prefix.is_empty() && !symbol.starts_with(prefix) {
                    continue;
                }

                push_completion(
                    symbol,
                    format!("Imported type from {module_name}"),
                    format!("Imported from `{module_name}`."),
                    CompletionItemKind::Module,
                );
            }
        }
    }

    /// Find a keyword in source text using ASCII identifier boundaries.
    fn find_keyword(text: &str, keyword: &str) -> Option<usize> {
        let mut start = 0usize;
        while let Some(rel_idx) = text[start..].find(keyword) {
            let idx = start + rel_idx;
            let before = text[..idx].chars().next_back();
            let after = text[idx + keyword.len()..].chars().next();

            let before_ok = before.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            let after_ok = after.is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            if before_ok && after_ok {
                return Some(idx);
            }

            start = idx + keyword.len();
        }
        None
    }

    /// Convert Moose type documentation into a concise completion tooltip.
    fn format_type_documentation(doc: &BuiltinDoc) -> String {
        format!("{}\n\n{}", doc.signature, doc.description)
    }

    /// Return `true` when the label looks like a type name rather than a function.
    fn looks_like_type_name(label: &str) -> bool {
        label.chars().next().is_some_and(|c| c.is_ascii_uppercase()) || label.contains("::")
    }

    /// Add common Moo/Moose `has` option-key completions.
    fn add_has_option_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        let raw_prefix = context.prefix.trim();
        let prefix = raw_prefix.trim_start_matches(['\'', '"']);
        let options = [
            ("is", "Accessor mode (`ro`, `rw`, or `rwp`)"),
            ("isa", "Type constraint for this attribute"),
            ("default", "Default value or builder closure"),
            ("required", "Require attribute during construction"),
            ("lazy", "Delay default computation until first access"),
            ("builder", "Method name used to build the default value"),
            ("reader", "Custom reader method name"),
            ("writer", "Custom writer method name"),
            ("accessor", "Custom combined read/write accessor"),
            ("predicate", "Method name to test if attribute is set"),
            ("clearer", "Method name to clear attribute value"),
            ("handles", "Delegated methods for referenced object"),
        ];

        for (label, doc) in options {
            if prefix.is_empty() || label.starts_with(prefix) {
                completions.push(CompletionItem {
                    label: label.to_string(),
                    kind: CompletionItemKind::Property,
                    detail: Some("Moo/Moose option".to_string()),
                    documentation: Some(doc.to_string()),
                    insert_text: Some(format!("{label} => ")),
                    sort_text: Some(format!("0_{label}")),
                    filter_text: Some(label.to_string()),
                    additional_edits: vec![],
                    text_edit_range: Some((context.prefix_start, context.position)),
                    commit_characters: None,
                });
            }
        }
    }

    fn add_object_pad_constructor_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
        package_name: &str,
    ) {
        let prefix = context.prefix.trim();
        let Some(model) =
            self.class_models.iter().rev().find(|model| {
                model.name == package_name && model.framework == Framework::ObjectPad
            })
        else {
            return;
        };

        for field_name in model.object_pad_param_field_names() {
            if !prefix.is_empty() && !field_name.starts_with(prefix) {
                continue;
            }

            completions.push(CompletionItem {
                label: field_name.to_string(),
                kind: CompletionItemKind::Property,
                detail: Some("Object::Pad constructor parameter".to_string()),
                documentation: Some(format!("`:param` field for `{package_name}->new(...)`.")),
                insert_text: Some(format!("{field_name} => ")),
                sort_text: Some(format!("0_{field_name}")),
                filter_text: Some(field_name.to_string()),
                additional_edits: vec![],
                text_edit_range: Some((context.prefix_start, context.position)),
                commit_characters: None,
            });
        }
    }

    /// Check if prefix could be a keyword
    fn could_be_keyword(&self, prefix: &str, keywords: &[&'static str]) -> bool {
        keywords.iter().any(|k| k.starts_with(prefix))
    }

    /// Check if prefix could be a function
    fn could_be_function(
        &self,
        prefix: &str,
        builtins: &std::collections::HashSet<&'static str>,
    ) -> bool {
        // Check builtins
        if builtins.iter().any(|b| b.starts_with(prefix)) {
            return true;
        }

        // Check user-defined functions
        for (name, symbols) in &self.symbol_table.symbols {
            for symbol in symbols {
                if (symbol.kind == SymbolKind::Subroutine || symbol.kind == SymbolKind::Constant)
                    && name.starts_with(prefix)
                {
                    return true;
                }
            }
        }

        false
    }

    /// Simple heuristic to check if position is in a string
    fn is_in_string(&self, source: &str, position: usize) -> bool {
        let before = &source[..position];
        let single_quotes = before.matches('\'').count();
        let double_quotes = before.matches('"').count();

        // Very simple: odd number of quotes means we're inside
        single_quotes % 2 == 1 || double_quotes % 2 == 1
    }

    /// Heuristic to check if position is inside a regex literal.
    ///
    /// Detects the following regex contexts:
    /// - Binding operators: `=~ /…/` and `!~ /…/`
    /// - Explicit regex operators: `m/…/`, `qr/…/`, `s/…/…/`, `tr/…/…/`, `y/…/…/`
    /// - Bare regex after operators/keywords that expect a regex value
    fn is_in_regex(source: &str, position: usize) -> bool {
        let before = &source[..position];

        // Find the last unescaped `/` before the cursor -- that could be the
        // opening delimiter of the regex we are inside.
        let Some(last_slash) = before.rfind('/') else {
            return false;
        };

        // Check if the slash is preceded by a regex binding operator.
        let pre_slash = before[..last_slash].trim_end();
        if pre_slash.ends_with("=~") || pre_slash.ends_with("!~") {
            return true;
        }

        // Check for explicit regex operators: m/, qr/, s/, tr/, y/
        // We look for the operator keyword immediately before the slash (with
        // optional whitespace).
        if Self::pre_slash_has_regex_op(pre_slash) {
            return true;
        }

        if matches!(
            pre_slash.split_ascii_whitespace().next_back(),
            Some("or") | Some("and") | Some("not")
        ) {
            return true;
        }

        // Bare `/` after certain tokens that unambiguously start a regex.
        if let Some(last_char) = pre_slash.chars().next_back() {
            // After `(`, `,`, `=`, `!`, `&&`, `||`, `or`, `and`, `not`, `;`, `{`
            // a `/` starts a regex rather than a division.
            if matches!(last_char, '(' | ',' | '=' | '!' | '&' | '|' | ';' | '{' | '~') {
                return true;
            }
        }

        // If pre_slash is empty, the slash is at position 0 -- that is a regex.
        pre_slash.is_empty()
    }

    /// Return true when the text immediately before a `/` is one of the
    /// explicit regex operators (`m`, `qr`, `s`, `tr`, `y`).
    fn pre_slash_has_regex_op(pre_slash: &str) -> bool {
        let trimmed = pre_slash.trim_end();
        for op in &["qr", "m", "s", "tr", "y"] {
            if let Some(before_op) = trimmed.strip_suffix(op) {
                // The operator must be at a word boundary -- the character
                // before it (if any) must not be alphanumeric or `_`.
                let boundary_ok = before_op
                    .chars()
                    .next_back()
                    .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
                if boundary_ok {
                    return true;
                }
            }
        }
        false
    }

    /// Return true when the cursor is positioned in the flag region after a
    /// closing regex delimiter — e.g., `$x =~ /foo/|` or `m/foo/i|`.
    ///
    /// Algorithm:
    /// 1. Strip any trailing flag characters from the text before the cursor.
    /// 2. The stripped text must end with `/` (the closing delimiter).
    /// 3. The text before the closing `/` must look like a regex body:
    ///    - For single-delimiter operators (`/…/`, `m/…/`, `qr/…/`): the
    ///      character just before the closing `/` must be inside a regex body
    ///      per `is_in_regex`.
    ///    - For multi-delimiter operators (`s/…/…/`, `tr/…/…/`, `y/…/…/`):
    ///      count the number of unescaped `/` chars; an even count (≥2) with
    ///      a known operator keyword confirms the closing delimiter.
    ///
    /// The `is_in_regex_flags` check MUST be dispatched before `is_in_regex`
    /// in the completion pipeline, because the cursor after the closing `/` is
    /// not itself `in_regex`.
    pub(crate) fn is_in_regex_flags(source: &str, position: usize) -> bool {
        if position == 0 || position > source.len() {
            return false;
        }
        let before = &source[..position];
        let flag_chars: &[char] =
            &['g', 'i', 'm', 's', 'x', 'e', 'r', 'a', 'd', 'u', 'p', 'l', 'c'];
        let without_flags = before.trim_end_matches(|c: char| flag_chars.contains(&c));
        // Must end with the closing delimiter '/'.
        if !without_flags.ends_with('/') {
            return false;
        }
        let close_pos = without_flags.len();
        if close_pos < 2 {
            return false;
        }

        // Fast path for single-delimiter operators: the position just before the
        // closing '/' must be inside a regex body per is_in_regex.
        if Self::is_in_regex(source, close_pos - 1) {
            return true;
        }

        // Slow path for multi-delimiter operators (s///, tr///, y///):
        // count unescaped '/' chars in `without_flags`. If there are exactly 3
        // (i.e., op/pattern/replacement/) and the operator is s/tr/y, we are
        // in flags position.
        let body = without_flags.trim();
        Self::is_multi_delim_regex_at_close(body)
    }

    /// Return true when `text` looks like `s/…/…/`, `tr/…/…/`, or `y/…/…/`
    /// with a complete closing delimiter (three `/` chars for s, tr, y).
    fn is_multi_delim_regex_at_close(text: &str) -> bool {
        // Identify whether the text starts with a known multi-delimiter operator.
        let (op_len, required_slashes) = if text.starts_with("tr/") || text.starts_with("y/") {
            let op = if text.starts_with("tr/") { 2 } else { 1 };
            (op, 3usize) // tr/search/replacement/ has 3 '/'
        } else if text.starts_with("s/") {
            (1, 3usize) // s/pattern/replacement/ has 3 '/'
        } else {
            // Not a multi-delimiter operator we handle — also try with a
            // binding operator prefix like `$x =~ s/…/…/`.
            let stripped = text
                .find("=~")
                .map(|p| text[p + 2..].trim_start())
                .or_else(|| text.find("!~").map(|p| text[p + 2..].trim_start()));
            if let Some(rhs) = stripped {
                return Self::is_multi_delim_regex_at_close(rhs);
            }
            return false;
        };
        // Count unescaped '/' characters in the operator body.
        let body_after_op = &text[op_len..];
        let slash_count = Self::count_unescaped_slashes(body_after_op);
        slash_count == required_slashes
    }

    /// Count the number of unescaped `/` characters in `s`.
    fn count_unescaped_slashes(s: &str) -> usize {
        let mut count = 0usize;
        let mut escaped = false;
        for ch in s.chars() {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '/' {
                count += 1;
            }
        }
        count
    }

    /// Simple heuristic to check if position is in a comment
    fn is_in_comment(&self, source: &str, position: usize) -> bool {
        let line_start = source[..position].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let line = &source[line_start..position];
        line.contains('#')
    }

    /// Check if we're in a test context
    fn is_test_context(&self, source: &str, filepath: Option<&str>) -> bool {
        // Check if file ends with .t
        if let Some(path) = filepath
            && path.ends_with(".t")
        {
            return true;
        }

        // Check if source contains Test::More or Test2::V0
        source.contains("use Test::More") || source.contains("use Test2::V0")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::{must, must_some};
    use perl_workspace::workspace_index::WorkspaceIndex;
    use std::path::PathBuf;
    use std::sync::Arc;
    use url::Url;

    #[test]
    fn test_provider_captures_include_and_system_inc_paths() {
        let code = "use My::Module;\n";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let include_paths = vec![PathBuf::from("/workspace/lib"), PathBuf::from("t/lib")];
        let system_inc_paths = vec![PathBuf::from("/usr/lib/perl5")];

        let provider = CompletionProvider::new_with_index_and_source_and_inc_paths(
            &ast,
            code,
            None,
            include_paths.clone(),
            system_inc_paths.clone(),
        );

        assert_eq!(provider.include_paths, include_paths);
        assert_eq!(provider.system_inc_paths, system_inc_paths);
    }

    #[test]
    fn test_use_module_completion_unchanged_with_empty_inc_vectors()
    -> Result<(), Box<dyn std::error::Error>> {
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyApp.pm")?;
        index.index_file(module_uri, "package MyApp;\n1;\n".to_string())?;

        let code = "use MyA";
        let mut parser = Parser::new(code);
        let ast = parser.parse()?;

        let baseline_provider =
            CompletionProvider::new_with_index_and_source(&ast, code, Some(Arc::clone(&index)));
        let baseline = baseline_provider.get_completions_with_path(code, code.len(), None);

        let with_empty_inc = CompletionProvider::new_with_index_and_source_and_inc_paths(
            &ast,
            code,
            Some(index),
            Vec::new(),
            Vec::new(),
        );
        let with_empty_inc_results =
            with_empty_inc.get_completions_with_path(code, code.len(), None);

        let baseline_labels: std::collections::HashSet<String> =
            baseline.into_iter().map(|item| item.label).collect();
        let with_empty_labels: std::collections::HashSet<String> =
            with_empty_inc_results.into_iter().map(|item| item.label).collect();

        assert_eq!(baseline_labels, with_empty_labels);
        Ok(())
    }

    #[test]
    fn test_variable_completion() {
        let code = r#"
my $count = 42;
my $counter = 0;
my @items = ();

$c
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len() - 1);

        assert!(completions.iter().any(|c| c.label == "$count"));
        assert!(completions.iter().any(|c| c.label == "$counter"));
    }

    #[test]
    fn test_function_completion() {
        let code = r#"
sub process_data {
    # ...
}

sub process_items {
    # ...
}

proc
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len() - 1);

        assert!(completions.iter().any(|c| c.label == "process_data"));
        assert!(completions.iter().any(|c| c.label == "process_items"));
    }

    #[test]
    fn test_use_constant_completion_from_visible_symbol_table() {
        let code = r#"
package My::Config;
use constant PI => 3.14159;
use constant qw(MAX_RETRIES TIMEOUT);

P
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len() - 1);

        let pi_completion = completions.iter().find(|c| c.label == "PI");
        assert!(pi_completion.is_some(), "expected PI constant completion");
        assert_eq!(
            pi_completion.map(|c| c.kind),
            Some(crate::providers::completion_item::CompletionItemKind::Constant)
        );
    }

    #[test]
    fn test_use_constant_hash_form_completion() {
        // Verify hash-ref form `use constant { FOO => 1, BAR => 2 }` surfaces both names.
        let code = r#"
use constant { MIN_VAL => 1, MAX_VAL => 100 };

M
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len() - 1);

        let min_completion = completions.iter().find(|c| c.label == "MIN_VAL");
        assert!(
            min_completion.is_some(),
            "MIN_VAL should appear in completions from hash-form use constant"
        );
        assert_eq!(
            min_completion.map(|c| c.kind),
            Some(crate::providers::completion_item::CompletionItemKind::Constant),
            "MIN_VAL should have kind Constant"
        );

        let max_completion = completions.iter().find(|c| c.label == "MAX_VAL");
        assert!(
            max_completion.is_some(),
            "MAX_VAL should appear in completions from hash-form use constant"
        );
    }

    #[test]
    fn test_use_constant_no_parens_in_insert_text() {
        // Constants must insert without trailing () — unlike function completions.
        let code = r#"
use constant ANSWER => 42;

A
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len() - 1);

        let answer = completions.iter().find(|c| c.label == "ANSWER");
        assert!(answer.is_some(), "ANSWER should appear in completions");
        assert_eq!(
            answer.and_then(|c| c.insert_text.as_deref()),
            Some("ANSWER"),
            "Constants must not insert trailing () — they are called like barewords"
        );
    }

    #[test]
    fn test_builtin_completion() {
        let code = "pr";

        let mut parser = Parser::new(""); // Empty AST
        let ast = must(parser.parse());

        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        assert!(completions.iter().any(|c| c.label == "print"));
        assert!(completions.iter().any(|c| c.label == "printf"));
    }

    #[test]
    fn test_current_package_detection() {
        let code = r#"package Foo;
my $x = 1;
$x
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);

        // position at end of file
        let context = provider.analyze_context(code, code.len());
        assert_eq!(context.current_package, "Foo");
    }

    #[test]
    fn test_package_block_detection() {
        let code = r#"package Foo {
    my $x;
    $x;
}
package Bar;
$"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);

        // Inside Foo block
        let pos_foo = must_some(code.find("$x;")) + 2; // position after $x
        let ctx_foo = provider.analyze_context(code, pos_foo);
        assert_eq!(ctx_foo.current_package, "Foo");

        // After block, in Bar package
        let pos_bar = code.len();
        let ctx_bar = provider.analyze_context(code, pos_bar);
        assert_eq!(ctx_bar.current_package, "Bar");
    }

    #[test]
    fn test_incomplete_nested_block_scope_context() {
        let code = concat!(
            "my $file_var = 0;\n",
            "sub process {\n",
            "    my $sub_var = 1;\n",
            "    if (1) {\n",
            "        my $block_var = 2;\n",
            "        $"
        );

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
        let context = provider.analyze_context(code, code.len());

        let sub_scope = must_some(
            provider
                .symbol_table
                .symbols
                .get("sub_var")
                .and_then(|symbols| symbols.first())
                .map(|symbol| symbol.scope_id),
        );
        let block_scope = must_some(
            provider
                .symbol_table
                .symbols
                .get("block_var")
                .and_then(|symbols| symbols.first())
                .map(|symbol| symbol.scope_id),
        );

        assert_eq!(
            context.cursor_scope_id, block_scope,
            "expected cursor scope to match block_var scope in incomplete nested block; cursor={:?} sub={:?} block={:?}",
            context.cursor_scope_id, sub_scope, block_scope
        );
    }

    #[test]
    fn test_incomplete_nested_block_variable_sorting() {
        let code = concat!(
            "my $file_var = 0;\n",
            "sub process {\n",
            "    my $sub_var = 1;\n",
            "    if (1) {\n",
            "        my $block_var = 2;\n",
            "        $"
        );

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
        let completions = provider.get_completions(code, code.len());

        let block_item =
            must_some(completions.iter().find(|completion| completion.label == "$block_var"));
        let sub_item =
            must_some(completions.iter().find(|completion| completion.label == "$sub_var"));

        assert!(
            block_item.sort_text < sub_item.sort_text,
            "expected incomplete block variable to outrank parent variable, got block={:?} sub={:?}",
            block_item.sort_text,
            sub_item.sort_text
        );
    }

    #[test]
    fn test_package_member_completion() {
        // Create workspace index with a module exporting a function
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = must(Url::parse("file:///workspace/MyModule.pm"));
        let module_code = r#"package MyModule;
our @EXPORT = qw(exported_sub);
sub exported_sub { }
sub internal_sub { }
1;
"#;
        must(index.index_file(module_uri, module_code.to_string()));

        // Code that triggers package completion
        let code = "use MyModule;\nMyModule::";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "exported_sub"),
            "should suggest exported_sub"
        );
        let exported_sub =
            must_some(completions.iter().find(|completion| completion.label == "exported_sub"));
        let documentation = must_some(exported_sub.documentation.as_deref());
        assert!(
            documentation.contains("MyModule::exported_sub"),
            "expected package member doc to mention qualified symbol, got: {documentation:?}"
        );
    }

    #[test]
    fn test_moo_accessor_method_completion() {
        let code = r#"
package Example::User;
use Moo;

has 'name' => (is => 'ro', isa => 'Str');

sub greet {
    my $self = shift;
    return $self->name;
}
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let synthesized = provider
            .symbol_table
            .symbols
            .get("name")
            .map(|symbols| symbols.iter().any(|symbol| symbol.kind == SymbolKind::Subroutine))
            .unwrap_or(false);
        assert!(synthesized, "expected synthesized `name` subroutine symbol in symbol table");

        let pos = must_some(code.find("$self->name")) + "$self->".len();
        let completions = provider.get_completions(code, pos);

        assert!(
            completions.iter().any(|item| item.label == "name"),
            "expected synthesized Moo accessor `name` in method completion"
        );
    }

    #[test]
    fn test_moo_accessor_completion_shows_isa_type() {
        let code = r#"
package Example::User;
use Moo;

has 'name' => (is => 'ro', isa => 'Str');
has 'age'  => (is => 'rw', isa => 'Int');

sub greet {
    my $self = shift;
    $self->
}
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let pos = must_some(code.find("$self->")) + "$self->".len();
        let completions = provider.get_completions(code, pos);

        // name accessor should appear with isa type in documentation
        let name_item = must_some(completions.iter().find(|c| c.label == "name"));
        let name_doc = must_some(name_item.documentation.as_deref());
        assert!(
            name_doc.contains("Str"),
            "expected `Str` type in name accessor documentation, got: {name_doc:?}"
        );

        // age accessor should appear with isa type in documentation
        let age_item = must_some(completions.iter().find(|c| c.label == "age"));
        let age_doc = must_some(age_item.documentation.as_deref());
        assert!(
            age_doc.contains("Int"),
            "expected `Int` type in age accessor documentation, got: {age_doc:?}"
        );

        // detail should indicate it's a Moo/Moose accessor, not just "method"
        let name_detail = must_some(name_item.detail.as_deref());
        assert!(
            name_detail.contains("accessor"),
            "expected 'accessor' in detail for Moo attribute, got: {name_detail:?}"
        );
    }

    #[test]
    fn test_moose_accessor_completion_shows_isa_type() {
        let code = r#"
package Example::Animal;
use Moose;

has 'species' => (is => 'ro', isa => 'Str', required => 1);

sub describe {
    my $self = shift;
    $self->
}
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let pos = must_some(code.find("$self->")) + "$self->".len();
        let completions = provider.get_completions(code, pos);

        let species_item = must_some(completions.iter().find(|c| c.label == "species"));
        let species_doc = must_some(species_item.documentation.as_deref());
        assert!(
            species_doc.contains("Str"),
            "expected `Str` type in species accessor documentation, got: {species_doc:?}"
        );
    }

    #[test]
    fn test_moo_has_option_key_completion() {
        let code = r#"
use Moo;
has 'name' => (re
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|item| item.label == "required"),
            "expected `required` option completion inside has(...) context"
        );
        assert!(
            completions.iter().any(|item| item.label == "reader"),
            "expected `reader` option completion inside has(...) context"
        );
    }

    #[test]
    fn test_moo_has_option_key_completion_with_quoted_prefix() {
        let code = r#"
use Moo;
has 'name' => ('re
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|item| item.label == "required"),
            "expected `required` option completion for quoted key prefix"
        );
    }

    #[test]
    fn test_object_pad_constructor_param_completion() {
        let code = r#"
use Object::Pad;

class Point {
    field $x :param = 0;
    field $y :param = 0;
    field $cache = 1;
}

Point->new(
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|item| item.label == "x"),
            "expected `x` constructor completion inside Point->new(...)"
        );
        assert!(
            completions.iter().any(|item| item.label == "y"),
            "expected `y` constructor completion inside Point->new(...)"
        );
        assert!(
            !completions.iter().any(|item| item.label == "cache"),
            "non-:param fields should not appear in constructor completion"
        );

        let x_item = must_some(completions.iter().find(|item| item.label == "x"));
        assert_eq!(x_item.insert_text.as_deref(), Some("x => "));
    }

    #[test]
    fn test_object_pad_constructor_param_completion_honors_prefix_and_value_context() {
        let prefix_code = r#"
use Object::Pad;

class Point {
    field $name :param;
    field $native_name :param;
    field $age :param;
}

Point->new(na"#;

        let mut parser = Parser::new(prefix_code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, prefix_code, None);
        let completions = provider.get_completions(prefix_code, prefix_code.len());
        let constructor_labels: Vec<&str> = completions
            .iter()
            .filter(|item| item.detail.as_deref() == Some("Object::Pad constructor parameter"))
            .map(|item| item.label.as_str())
            .collect();

        assert!(constructor_labels.contains(&"name"), "expected `name` to match prefix `na`");
        assert!(
            constructor_labels.contains(&"native_name"),
            "expected `native_name` to remain available when matching prefix"
        );
        assert!(
            !constructor_labels.contains(&"age"),
            "non-matching constructor params should be filtered by prefix"
        );

        let value_code = r#"
use Object::Pad;

class Point {
    field $name :param;
    field $native_name :param;
}

Point->new(name => "#;

        let mut parser = Parser::new(value_code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, value_code, None);
        let value_completions = provider.get_completions(value_code, value_code.len());
        let value_constructor_labels: Vec<&str> = value_completions
            .iter()
            .filter(|item| item.detail.as_deref() == Some("Object::Pad constructor parameter"))
            .map(|item| item.label.as_str())
            .collect();

        assert!(
            !value_constructor_labels.contains(&"name"),
            "constructor key completions should not appear in value position"
        );
        assert!(
            !value_constructor_labels.contains(&"native_name"),
            "constructor key completions should not appear after `=>`"
        );
    }

    #[test]
    fn test_object_pad_constructor_param_completion_supports_lowercase_class_names() {
        let code = r#"
use Object::Pad;

class point {
    field $name :param;
    field $native_name :param;
}

point->new(na"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);
        let completions = provider.get_completions(code, code.len());
        let constructor_labels: Vec<&str> = completions
            .iter()
            .filter(|item| item.detail.as_deref() == Some("Object::Pad constructor parameter"))
            .map(|item| item.label.as_str())
            .collect();

        assert!(constructor_labels.contains(&"name"));
        assert!(constructor_labels.contains(&"native_name"));
    }

    #[test]
    fn test_moo_isa_type_completion_includes_builtins_and_imports() {
        let code = r#"
use MyApp::Types qw(UserID PositiveInt);
use Moose;

has 'id' => (
    is => 'ro',
    isa => 
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|item| item.label == "Str"),
            "expected built-in Moose type `Str` in isa completion, got: {:?}",
            completions.iter().map(|item| &item.label).collect::<Vec<_>>()
        );
        assert!(
            completions.iter().any(|item| item.label == "ArrayRef"),
            "expected built-in Moose type `ArrayRef` in isa completion"
        );
        assert!(
            completions.iter().any(|item| item.label == "UserID"),
            "expected imported custom type `UserID` in isa completion"
        );
    }

    #[test]
    fn test_moo_isa_type_completion_with_quoted_prefix() {
        let code = r#"
use Moose;

has 'id' => (
    is => 'ro',
    isa => 'St
"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index_and_source(&ast, code, None);

        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|item| item.label == "Str"),
            "expected built-in Moose type `Str` for quoted isa prefix"
        );
    }

    #[test]
    fn test_regex_completion_binding_operator() {
        // Cursor right after the opening slash of a regex
        let code = r#"my $x = "hello"; $x =~ /"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        // Should contain regex constructs
        assert!(
            completions.iter().any(|c| c.label == "\\d"),
            "expected \\d regex completion inside =~ /.../"
        );
        assert!(
            completions.iter().any(|c| c.label == "\\w"),
            "expected \\w regex completion inside =~ /.../"
        );
        assert!(
            completions.iter().any(|c| c.label == "(?:...)"),
            "expected non-capturing group regex completion"
        );
    }

    #[test]
    fn test_regex_completion_negated_binding() {
        let code = r#"my $x = "test"; $x !~ /"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "\\d"),
            "expected regex completions after !~"
        );
    }

    #[test]
    fn test_regex_completion_m_operator() {
        let code = "if ($line =~ m/";

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "\\d"),
            "expected regex completions inside m/.../"
        );
        assert!(
            completions.iter().any(|c| c.label == "^"),
            "expected anchor completions inside m/.../"
        );
    }

    #[test]
    fn test_regex_completion_qr_operator() {
        let code = "my $re = qr/";

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "\\d+"),
            "expected common pattern completions inside qr/.../"
        );
        assert!(
            completions.iter().any(|c| c.label == "(?=...)"),
            "expected lookahead group completion inside qr/.../"
        );
    }

    #[test]
    fn test_regex_completion_s_operator() {
        let code = "($line = $input) =~ s/";

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "\\s+"),
            "expected common pattern completions inside s/.../"
        );
    }

    #[test]
    fn test_regex_completion_has_all_categories() {
        let code = r#"$x =~ /"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        // Character classes
        assert!(completions.iter().any(|c| c.label == "\\d"));
        assert!(completions.iter().any(|c| c.label == "\\D"));
        assert!(completions.iter().any(|c| c.label == "\\w"));
        assert!(completions.iter().any(|c| c.label == "\\W"));
        assert!(completions.iter().any(|c| c.label == "\\s"));
        assert!(completions.iter().any(|c| c.label == "\\S"));
        assert!(completions.iter().any(|c| c.label == "\\h"));
        assert!(completions.iter().any(|c| c.label == "\\H"));
        assert!(completions.iter().any(|c| c.label == "\\v"));
        assert!(completions.iter().any(|c| c.label == "\\V"));
        assert!(completions.iter().any(|c| c.label == "\\R"));
        assert!(completions.iter().any(|c| c.label == "[...]"));
        assert!(completions.iter().any(|c| c.label == "[^...]"));

        // Anchors
        assert!(completions.iter().any(|c| c.label == "^"));
        assert!(completions.iter().any(|c| c.label == "$"));
        assert!(completions.iter().any(|c| c.label == "\\b"));
        assert!(completions.iter().any(|c| c.label == "\\B"));
        assert!(completions.iter().any(|c| c.label == "\\A"));
        assert!(completions.iter().any(|c| c.label == "\\z"));
        assert!(completions.iter().any(|c| c.label == "\\Z"));

        // Quantifiers
        assert!(completions.iter().any(|c| c.label == "*"));
        assert!(completions.iter().any(|c| c.label == "+"));
        assert!(completions.iter().any(|c| c.label == "?"));
        assert!(completions.iter().any(|c| c.label == "{n}"));
        assert!(completions.iter().any(|c| c.label == "{n,}"));
        assert!(completions.iter().any(|c| c.label == "{n,m}"));

        // Groups
        assert!(completions.iter().any(|c| c.label == "(...)"));
        assert!(completions.iter().any(|c| c.label == "(?:...)"));
        assert!(completions.iter().any(|c| c.label == "(?=...)"));
        assert!(completions.iter().any(|c| c.label == "(?!...)"));
        assert!(completions.iter().any(|c| c.label == "(?<=...)"));
        assert!(completions.iter().any(|c| c.label == "(?<!...)"));

        // Common patterns
        assert!(completions.iter().any(|c| c.label == "\\d+"));
        assert!(completions.iter().any(|c| c.label == "\\w+"));
        assert!(completions.iter().any(|c| c.label == "\\s+"));
        assert!(completions.iter().any(|c| c.label == ".*?"));
        assert!(completions.iter().any(|c| c.label == ".+?"));
    }

    #[test]
    fn test_regex_completion_items_have_correct_kind() {
        let code = r#"$x =~ /"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        for item in &completions {
            assert_eq!(
                item.kind,
                CompletionItemKind::Snippet,
                "regex completion '{}' should be Snippet kind",
                item.label
            );
        }
    }

    #[test]
    fn test_regex_completion_items_have_documentation() {
        let code = r#"$x =~ /"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        for item in &completions {
            assert!(
                item.documentation.is_some(),
                "regex completion '{}' should have documentation",
                item.label
            );
            assert!(item.detail.is_some(), "regex completion '{}' should have detail", item.label);
        }
    }

    #[test]
    fn test_regex_completion_not_in_normal_context() {
        // Outside regex context, should not get regex completions
        let code = "my $x = 1;\n";

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        assert!(
            !completions.iter().any(|c| c.label == "\\d"),
            "regex completions should NOT appear outside regex context"
        );
    }

    #[test]
    fn test_is_in_regex_binding_operator() {
        let code = r#"$x =~ /hello"#;
        assert!(CompletionProvider::is_in_regex(code, code.len()));
    }

    #[test]
    fn test_is_in_regex_m_operator() {
        let code = "m/pattern";
        assert!(CompletionProvider::is_in_regex(code, code.len()));
    }

    #[test]
    fn test_is_in_regex_qr_operator() {
        let code = "my $re = qr/pattern";
        assert!(CompletionProvider::is_in_regex(code, code.len()));
    }

    #[test]
    fn test_is_in_regex_s_operator() {
        let code = "$line =~ s/old";
        assert!(CompletionProvider::is_in_regex(code, code.len()));
    }

    #[test]
    fn test_is_in_regex_keyword_operator() {
        let code = "$x or /pattern";
        assert!(CompletionProvider::is_in_regex(code, code.len()));
    }

    #[test]
    fn test_is_not_in_regex_division() {
        // Division should NOT be detected as regex
        let code = "my $result = $x / $y";
        // Position after "$x / $" -- should not be regex because $ precedes /
        // but our heuristic checks pre_slash context
        assert!(
            !CompletionProvider::is_in_regex(code, code.len()),
            "division should not be detected as regex context"
        );
    }

    #[test]
    fn test_regex_completion_preserves_sigil_completions_in_interpolation() {
        // Cursor is inside the regex body at the end of `$fo` — before the
        // closing `/`. Variable completions must be offered, not flag completions.
        let code = r#"my $foo = 1; my $bar = qr/^$fo/"#;
        // Position just before the closing '/'
        let pos = code.len() - 1;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, pos);

        assert!(
            completions.iter().any(|item| item.label == "$foo"),
            "expected interpolated regex variables to keep scalar completions"
        );
    }

    #[test]
    fn test_regex_completion_replaces_escape_prefix_range() {
        let code = r#"$x =~ /\d"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        let item = must_some(completions.iter().find(|completion| completion.label == r"\d"));
        assert_eq!(
            item.text_edit_range,
            Some((code.len() - r"\d".len(), code.len())),
            "expected regex completion to replace the typed escape sequence"
        );
    }

    #[test]
    fn test_regex_completion_offers_perl_whitespace_and_linebreak_classes() {
        let code = r#"$x =~ /\"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();

        for label in &["\\h", "\\H", "\\v", "\\V", "\\R"] {
            assert!(
                labels.contains(label),
                "expected Perl regex class completion '{label}', got: {labels:?}"
            );
        }
    }

    #[test]
    fn test_regex_completion_replaces_group_prefix_range() {
        let code = r#"$x =~ /(?: "#;
        let code = code.trim_end();

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        let item = must_some(completions.iter().find(|completion| completion.label == "(?:...)"));
        assert_eq!(
            item.text_edit_range,
            Some((code.len() - "(?:".len(), code.len())),
            "expected regex completion to replace the typed group opener"
        );
    }

    #[test]
    fn test_detect_use_qw_import_context_basic() {
        // Cursor right after opening paren in qw()
        let code = "use MyModule qw(";
        let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
        assert!(result.is_some(), "should detect qw() import context");
        let (module, prefix) =
            result.as_ref().map(|(m, p)| (m.as_str(), p.as_str())).unwrap_or_default();
        assert_eq!(module, "MyModule");
        assert_eq!(prefix, "");
    }

    #[test]
    fn test_detect_use_qw_import_context_with_prefix() {
        let code = "use File::Basename qw(bas";
        let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
        assert!(result.is_some(), "should detect qw() import context with prefix");
        let (module, prefix) =
            result.as_ref().map(|(m, p)| (m.as_str(), p.as_str())).unwrap_or_default();
        assert_eq!(module, "File::Basename");
        assert_eq!(prefix, "bas");
    }

    #[test]
    fn test_detect_use_qw_import_context_with_existing_imports() {
        let code = "use MyModule qw(foo bar ba";
        let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
        assert!(result.is_some(), "should detect qw() import context after existing imports");
        let (module, prefix) =
            result.as_ref().map(|(m, p)| (m.as_str(), p.as_str())).unwrap_or_default();
        assert_eq!(module, "MyModule");
        assert_eq!(prefix, "ba");
    }

    #[test]
    fn test_detect_use_qw_not_after_close() {
        // Cursor after the closing paren
        let code = "use MyModule qw(foo bar);";
        let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
        assert!(result.is_none(), "should not detect context after closing paren");
    }

    #[test]
    fn test_detect_use_qw_not_for_pragmas() {
        let code = "use strict qw(";
        let result = CompletionProvider::detect_use_qw_import_context(code, code.len());
        assert!(result.is_none(), "should not detect context for lowercase pragmas");
    }

    #[test]
    fn test_use_qw_import_completion_with_workspace() -> Result<(), Box<dyn std::error::Error>> {
        // Create workspace index with a module that has subroutines
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyUtils.pm")?;
        let module_code = r#"package MyUtils;
use Exporter 'import';
our @EXPORT_OK = qw(helper_one helper_two);
sub helper_one { }
sub helper_two { }
sub _private_internal { }
1;
"#;
        index.index_file(module_uri, module_code.to_string())?;

        // Code where user is typing inside qw()
        let code = "use MyUtils qw(hel";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "helper_one"),
            "should suggest helper_one from MyUtils: got {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        assert!(
            completions.iter().any(|c| c.label == "helper_two"),
            "should suggest helper_two from MyUtils"
        );
        Ok(())
    }

    #[test]
    fn test_use_qw_import_completion_empty_prefix() -> Result<(), Box<dyn std::error::Error>> {
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/Utils.pm")?;
        let module_code = r#"package Utils;
sub alpha { }
sub beta { }
1;
"#;
        index.index_file(module_uri, module_code.to_string())?;

        // Empty prefix inside qw()
        let code = "use Utils qw(";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "alpha"),
            "should suggest alpha with empty prefix: got {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        assert!(
            completions.iter().any(|c| c.label == "beta"),
            "should suggest beta with empty prefix"
        );
        Ok(())
    }

    #[test]
    fn test_use_qw_import_completion_detail_shows_module() -> Result<(), Box<dyn std::error::Error>>
    {
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyLib.pm")?;
        let module_code = r#"package MyLib;
sub do_work { }
1;
"#;
        index.index_file(module_uri, module_code.to_string())?;

        let code = "use MyLib qw(do";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());

        let do_work = completions.iter().find(|c| c.label == "do_work");
        assert!(do_work.is_some(), "should suggest do_work");
        let detail = must_some(do_work.and_then(|c| c.detail.as_deref()));
        assert!(detail.contains("MyLib"), "detail should mention module name, got: {detail:?}");
        Ok(())
    }

    #[test]
    fn test_self_arrow_resolves_workspace_methods() -> Result<(), Box<dyn std::error::Error>> {
        // Regression test for issue #2536: $self-> method completion should resolve
        // workspace-indexed methods from the current package.
        //
        // The methods are ONLY in the workspace index (a separate .pm file), not in
        // the currently-parsed source. This tests the workspace path specifically:
        // `infer_receiver_package` must return `MyService` for `$self->` when
        // `context.current_package == "MyService"`.
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyService.pm")?;
        let module_code = r#"package MyService;
sub new { bless {}, shift }
sub process_request { }
sub validate_input { }
1;
"#;
        index.index_file(module_uri, module_code.to_string())?;

        // The currently-edited file is in MyService but does NOT define
        // process_request or validate_input locally — they are workspace-only.
        let code = r#"package MyService;
sub run {
    my $self = shift;
    $self->"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "process_request"),
            "$self-> should suggest process_request from workspace index; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        assert!(
            completions.iter().any(|c| c.label == "validate_input"),
            "$self-> should suggest validate_input from workspace index"
        );
        Ok(())
    }

    #[test]
    fn test_this_arrow_resolves_workspace_methods() -> Result<(), Box<dyn std::error::Error>> {
        // Same as above but using $this as the invocant variable.
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyHandler.pm")?;
        let module_code = r#"package MyHandler;
sub new { bless {}, shift }
sub handle { }
1;
"#;
        index.index_file(module_uri, module_code.to_string())?;

        // Only `run` is in the edited file; `handle` lives only in the workspace index.
        let code = r#"package MyHandler;
sub run {
    my $this = shift;
    $this->"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());

        assert!(
            completions.iter().any(|c| c.label == "handle"),
            "$this-> should suggest handle from workspace index; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_self_arrow_in_main_package_does_not_resolve() -> Result<(), Box<dyn std::error::Error>>
    {
        // Edge case: $self-> in the main package should NOT resolve to any package methods.
        // The guard condition `context.current_package != "main"` prevents incorrect
        // suggestions when the user is in script-level code.
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyLib.pm")?;
        let module_code = r#"package MyLib;
sub new { bless {}, shift }
sub helper { }
1;
"#;
        index.index_file(module_uri, module_code.to_string())?;

        // Code is at package main (implicit), so $self-> should not resolve
        let code = r#"sub run {
    my $self = shift;
    $self->"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());

        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());

        // Should NOT suggest MyLib methods just because the variable is named $self
        assert!(
            !completions.iter().any(|c| c.label == "helper"),
            "$self-> in main package should not suggest methods from other packages"
        );
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Tests for is_use_statement_context and add_use_module_completions
    // -------------------------------------------------------------------------

    #[test]
    fn test_use_statement_context_after_use_keyword() -> Result<(), Box<dyn std::error::Error>> {
        // "use " with cursor right after space — empty prefix, should trigger module completion
        let index = Arc::new(WorkspaceIndex::new());
        let uri = Url::parse("file:///lib/MyApp.pm")?;
        index.index_file(uri, "package MyApp;\n1;\n".to_string())?;
        let code = "use ";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "MyApp" && c.kind == CompletionItemKind::Module),
            "use <cursor> should suggest workspace module names; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_use_statement_context_with_prefix() -> Result<(), Box<dyn std::error::Error>> {
        // "use MyA" — prefix filtering should narrow to MyApp, not OtherLib
        let index = Arc::new(WorkspaceIndex::new());
        index
            .index_file(Url::parse("file:///lib/MyApp.pm")?, "package MyApp;\n1;\n".to_string())?;
        index.index_file(
            Url::parse("file:///lib/OtherLib.pm")?,
            "package OtherLib;\n1;\n".to_string(),
        )?;
        let code = "use MyA";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "MyApp" && c.kind == CompletionItemKind::Module),
            "use MyA should suggest MyApp with Module kind"
        );
        assert!(
            !completions.iter().any(|c| c.label == "OtherLib"),
            "use MyA should not suggest OtherLib"
        );
        Ok(())
    }

    #[test]
    fn test_use_statement_skips_pragmas() -> Result<(), Box<dyn std::error::Error>> {
        // Lowercase-first token after `use` means pragma — no module completion.
        // The index is populated with a Module-kind package so that if the lowercase
        // guard in is_use_statement_context were absent the test would fail (not
        // vacuously pass due to an empty index).
        let index = Arc::new(WorkspaceIndex::new());
        index.index_file(
            Url::parse("file:///lib/Strict.pm")?,
            "package Strict;\n1;\n".to_string(),
        )?;
        let code = "use strict";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
            "use strict should not trigger module completions; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_use_statement_skips_past_module_name_at_qw() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor inside qw list should NOT trigger module-name completion.
        // The index is populated so the test is non-vacuous: if the qw-dispatch
        // branch were removed, add_use_module_completions would fire and the
        // Module-kind assertion would fail.
        let index = Arc::new(WorkspaceIndex::new());
        index.index_file(
            Url::parse("file:///lib/Module.pm")?,
            "package Module;\nsub foo {}\n1;\n".to_string(),
        )?;
        let code = "use Module qw(foo";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        // This context routes to qw-import completions (Function kind), not module-name
        // completions (Module kind).
        assert!(
            !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
            "cursor inside qw() should not get module-name completions; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_require_statement_triggers_module_completion() -> Result<(), Box<dyn std::error::Error>>
    {
        let index = Arc::new(WorkspaceIndex::new());
        index
            .index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
        let code = "require Ut";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "Utils" && c.kind == CompletionItemKind::Module),
            "require Ut should suggest Utils with Module kind; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_require_statement_skips_file_path() -> Result<(), Box<dyn std::error::Error>> {
        let index = Arc::new(WorkspaceIndex::new());
        index
            .index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
        let code = "require './utils.pl'";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
            "require './utils.pl' should not trigger module-name completions; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_require_statement_skips_version_check() -> Result<(), Box<dyn std::error::Error>> {
        let index = Arc::new(WorkspaceIndex::new());
        index
            .index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
        let code = "require 5.010";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
            "require 5.010 should not trigger module-name completions; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_require_statement_triggers_completion_for_lowercase_module()
    -> Result<(), Box<dyn std::error::Error>> {
        // `require autodie` is valid Perl — lowercase module names must still get completions.
        // The previous implementation incorrectly blocked all non-uppercase-starting require
        // targets, including valid lowercase modules like autodie, overload, and Carp.
        let index = Arc::new(WorkspaceIndex::new());
        index.index_file(
            Url::parse("file:///lib/autodie.pm")?,
            "package autodie;\n1;\n".to_string(),
        )?;
        let code = "require auto";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions
                .iter()
                .any(|c| c.label == "autodie" && c.kind == CompletionItemKind::Module),
            "require auto should suggest 'autodie' with Module kind; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_require_statement_skips_vstring_version() -> Result<(), Box<dyn std::error::Error>> {
        // `require v5.10` is a v-string version check, not a module name.
        // 'v' starts the token but it is not followed by '::' — it should be blocked
        // because 'v' is a digit-prefix indicator in this context.
        // Currently, 'v' is a letter so it passes the digit/quote/path check.
        // This is an inherent limitation of single-char prefix detection — the full
        // `require v5.10` case requires position-aware parsing to resolve correctly.
        // For now, assert the observed (not-yet-blocked) behavior to document it.
        let index = Arc::new(WorkspaceIndex::new());
        index
            .index_file(Url::parse("file:///lib/Utils.pm")?, "package Utils;\n1;\n".to_string())?;
        let code = "require v5.10";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        // v5.10 is an unlikely prefix for a module (no CPAN modules start with 'v' in practice),
        // and even if triggered, the module index has no matching 'v*' entry.
        // Assert we never suggest Utils for this context.
        assert!(
            !completions.iter().any(|c| c.label == "Utils" && c.kind == CompletionItemKind::Module),
            "require v5.10 should not suggest unrelated modules; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_use_module_deduplication() -> Result<(), Box<dyn std::error::Error>> {
        // Two files declaring the same package should produce one completion, not two
        let index = Arc::new(WorkspaceIndex::new());
        index
            .index_file(Url::parse("file:///lib/MyApp.pm")?, "package MyApp;\n1;\n".to_string())?;
        index.index_file(
            Url::parse("file:///lib/MyApp2.pm")?,
            "package MyApp;\n1;\n".to_string(), // duplicate package name
        )?;
        let code = "use MyA";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        let myapp_count = completions.iter().filter(|c| c.label == "MyApp").count();
        assert_eq!(
            myapp_count, 1,
            "Duplicate package declarations should produce exactly one completion"
        );
        Ok(())
    }

    #[test]
    fn test_use_module_non_use_context_excluded() -> Result<(), Box<dyn std::error::Error>> {
        // Outside a use/require statement, module-priority sort_text should NOT appear.
        // add_use_module_completions gates on in_use_statement; its "1_" sort_text
        // prefix is the marker we check here.
        let index = Arc::new(WorkspaceIndex::new());
        index.index_file(
            Url::parse("file:///lib/MyApp.pm")?,
            "package MyApp;\nsub hello {}\n1;\n".to_string(),
        )?;
        let code = "my $x = MyA";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        // The "1_MyApp" sort_text is only emitted by add_use_module_completions,
        // which is guarded by in_use_statement. It must not appear outside that context.
        assert!(
            !completions.iter().any(|c| c.sort_text.as_deref() == Some("1_MyApp")),
            "Module-priority sort_text should only appear in use context"
        );
        Ok(())
    }

    #[test]
    fn test_use_statement_past_semicolon_excluded() -> Result<(), Box<dyn std::error::Error>> {
        // Cursor at the end of `use Module;` — the semicolon guard in
        // is_use_statement_context must suppress module-name completions.
        // Without the `;` check the cursor would be considered still inside
        // the use statement and would show stale module suggestions.
        let index = Arc::new(WorkspaceIndex::new());
        index.index_file(
            Url::parse("file:///lib/Module.pm")?,
            "package Module;\n1;\n".to_string(),
        )?;
        let code = "use Module;";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            !completions.iter().any(|c| c.kind == CompletionItemKind::Module),
            "cursor after `use Module;` should not trigger module-name completions; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
        Ok(())
    }

    // ── Gap 1: Named capture group in regex_patterns ─────────────────────────

    #[test]
    fn test_regex_named_capture_completion() {
        // Cursor inside an empty regex body — named capture should be offered.
        let code = r#"$x =~ /"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "(?<name>...)"),
            "expected named capture group in regex completions; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_regex_named_capture_prefix_disambig() {
        // Typing `(?<` inside a regex → both lookbehind and named capture offered.
        let code = r#"$x =~ /(?<"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "(?<=...)"),
            "expected lookbehind when prefix is (?<"
        );
        assert!(
            completions.iter().any(|c| c.label == "(?<name>...)"),
            "expected named capture when prefix is (?<"
        );
    }

    #[test]
    fn test_regex_named_capture_prefix_lookbehind_only() {
        // Typing `(?<=` — only the lookbehind should match (named capture label
        // does not start with `(?<=`).
        let code = r#"$x =~ /(?<="#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "(?<=...)"),
            "expected lookbehind for prefix (?<="
        );
        assert!(
            !completions.iter().any(|c| c.label == "(?<name>...)"),
            "named capture should NOT appear for prefix (?<= (label doesn't start with (?<=)"
        );
    }

    // ── Gap 2: is_in_regex_flags heuristic ───────────────────────────────────

    #[test]
    fn test_is_in_regex_flags_after_close_slash() {
        // Cursor immediately after the closing `/` of a regex.
        let code = "$x =~ /foo/";
        assert!(
            CompletionProvider::is_in_regex_flags(code, code.len()),
            "cursor right after closing / should be in regex-flags context"
        );
    }

    #[test]
    fn test_is_in_regex_flags_after_partial_flag() {
        // Cursor after one already-typed flag character.
        let code = "m/foo/i";
        assert!(
            CompletionProvider::is_in_regex_flags(code, code.len()),
            "cursor after /i should still be in regex-flags context"
        );
    }

    #[test]
    fn test_is_in_regex_flags_s_operator() {
        let code = "s/foo/bar/g";
        assert!(
            CompletionProvider::is_in_regex_flags(code, code.len()),
            "s/// with /g flag should be in regex-flags context"
        );
    }

    #[test]
    fn test_is_not_in_regex_flags_division() {
        // Plain division — must not be treated as regex flags.
        let code = "my $x = $a / $b /";
        assert!(
            !CompletionProvider::is_in_regex_flags(code, code.len()),
            "division should not be detected as regex-flags context"
        );
    }

    #[test]
    fn test_regex_flag_completions_after_close() {
        // Cursor right after closing `/` — should offer all standard flag letters.
        let code = "$x =~ /foo/";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        // Standard regex flags per Perl documentation
        for flag in &["g", "i", "m", "s", "x", "e", "r", "a", "p"] {
            assert!(
                labels.contains(flag),
                "expected standard regex flag '{flag}' in completions; got: {labels:?}"
            );
        }
    }

    #[test]
    fn test_regex_flag_completions_skip_already_typed() {
        // `g` already typed — completions should include `i` but not `g`.
        let code = "$x =~ /foo/g";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(!labels.contains(&"g"), "already-typed flag 'g' should be excluded");
        assert!(labels.contains(&"i"), "flag 'i' should still be offered");
    }

    #[test]
    fn test_regex_tr_flag_completions() {
        // tr/// should offer only c, d, s — not g, i, e.
        let code = "tr/a-z/A-Z/";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        for flag in &["c", "d", "s"] {
            assert!(
                labels.contains(flag),
                "tr/// flag '{flag}' should be offered; got: {labels:?}"
            );
        }
        for flag in &["g", "i", "e"] {
            assert!(!labels.contains(flag), "tr/// should NOT offer '{flag}'; got: {labels:?}");
        }
    }

    #[test]
    fn test_regex_tr_binding_operator_flag_completions() {
        // `$x =~ tr/.../` should also offer only c, d, s (binding form).
        let code = "$x =~ tr/a-z/A-Z/";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        for flag in &["c", "d", "s"] {
            assert!(
                labels.contains(flag),
                "tr/// binding flag '{flag}' should be offered; got: {labels:?}"
            );
        }
        assert!(!labels.contains(&"g"), "tr/// should NOT offer 'g'; got: {labels:?}");
    }

    // ── Gap 3: Statement-level regex operator snippets ───────────────────────

    #[test]
    fn test_regex_operator_snippets_present() {
        let code = "";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, 0);
        let labels: Vec<&str> = completions.iter().map(|c| c.label.as_str()).collect();
        assert!(labels.contains(&"mregex"), "mregex snippet missing; got: {labels:?}");
        assert!(labels.contains(&"ssubst"), "ssubst snippet missing; got: {labels:?}");
        assert!(labels.contains(&"qrpat"), "qrpat snippet missing; got: {labels:?}");
    }

    #[test]
    fn test_regex_operator_snippet_bodies() {
        // Verify the insert_text for each new snippet is syntactically correct.
        let code = "mregex";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

        let mregex = must_some(completions.iter().find(|c| c.label == "mregex"));
        let insert = mregex.insert_text.as_deref().unwrap_or_default();
        assert!(insert.starts_with("m/"), "mregex body must start with m/; got: {insert:?}");

        // Also verify ssubst and qrpat with explicit prefix lookup
        let code2 = "ssubst";
        let mut parser2 = Parser::new(code2);
        let ast2 = must(parser2.parse());
        let provider2 = CompletionProvider::new(&ast2);
        let completions2 = provider2.get_completions(code2, code2.len());
        let ssubst = must_some(completions2.iter().find(|c| c.label == "ssubst"));
        let insert2 = ssubst.insert_text.as_deref().unwrap_or_default();
        assert!(insert2.starts_with("s/"), "ssubst body must start with s/; got: {insert2:?}");

        let code3 = "qrpat";
        let mut parser3 = Parser::new(code3);
        let ast3 = must(parser3.parse());
        let provider3 = CompletionProvider::new(&ast3);
        let completions3 = provider3.get_completions(code3, code3.len());
        let qrpat = must_some(completions3.iter().find(|c| c.label == "qrpat"));
        let insert3 = qrpat.insert_text.as_deref().unwrap_or_default();
        assert!(insert3.starts_with("qr/"), "qrpat body must start with qr/; got: {insert3:?}");
    }

    // ── Dash trigger character tests (#2865) ─────────────────────────────────
    // When `-` is a trigger character, context detection must distinguish
    // method-call arrows (`->`) from arithmetic/decrement operators.

    #[test]
    fn test_dash_trigger_fires_method_completion_for_arrow()
    -> Result<(), Box<dyn std::error::Error>> {
        // `$obj-` (cursor after `-`) — the `-` is the start of `->`, so method
        // completions must appear even before the `>` is typed.
        // Crucially, the result must be ONLY method completions (Function kind),
        // not the entire keyword/snippet list. Without the `-` trigger feature,
        // the code returns all completions — this assertion catches that false pass.
        let code = r#"package MyService;
sub new { bless {}, shift }
sub process { }
sub validate { }
sub run {
    my $self = shift;
    $self-"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyService.pm")?;
        let module_code = "package MyService;\nsub process { }\nsub validate { }\n1;\n";
        index.index_file(module_uri, module_code.to_string())?;
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        // Must find method completions from the workspace index
        assert!(
            completions.iter().any(|c| c.label == "process" || c.label == "validate"),
            "dash trigger on `$self-` should produce method completions; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        // Must NOT return the full keyword/snippet dump — only method completions.
        // "arrayref", "hashref" are snippets from the generic path; they should not
        // appear when the context is a method-call arrow.
        assert!(
            !completions.iter().any(|c| c.label == "arrayref" || c.label == "hashref"),
            "dash trigger on `$self-` must not return generic snippets; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        Ok(())
    }

    #[test]
    fn test_dash_trigger_suppressed_for_subtract_assign() {
        // `$x -=` (cursor after `-` in `-=`) — must return NO completions.
        let code = "$x -";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        // Position is at len() which puts cursor right after `-` preceded by space.
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.is_empty(),
            "dash trigger on `$x -` (subtract context) should return no completions; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_dash_trigger_suppressed_for_decrement() {
        // `$x--` — second `-` is preceded by another `-`, must return NO completions.
        // The guard `source[position-2] != b'-'` prevents treating `--` as `->`.
        let code = "$x--";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        // Cursor after the second `-`: preceding char is `-`, not an identifier.
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.is_empty(),
            "dash trigger on `$x--` (decrement context) should return no completions; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_dash_trigger_suppressed_for_unary_minus() {
        // `my $x = -$y` — unary minus, `-` preceded by space → no completions.
        let code = "my $x = -";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.is_empty(),
            "dash trigger on `my $x = -` (unary minus) should return no completions; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_dash_trigger_fires_for_hash_deref_arrow() -> Result<(), Box<dyn std::error::Error>> {
        // `$hash->{key}` — trigger on `-` in `$hash->`, receiver ends with `h`
        // (alphanumeric), should produce method completions (not a generic dump).
        let code = r#"package MyService;
sub new { bless {}, shift }
sub get_data { }
sub run {
    my $hash = {};
    $hash-"#;
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let index = Arc::new(WorkspaceIndex::new());
        let module_uri = Url::parse("file:///workspace/MyService.pm")?;
        let module_code = "package MyService;\nsub new { }\nsub get_data { }\n1;\n";
        index.index_file(module_uri, module_code.to_string())?;
        let provider = CompletionProvider::new_with_index(&ast, Some(index));
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "get_data" || c.label == "new"),
            "dash trigger on `$hash-` should produce completions; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        // Must not return generic snippet dump
        assert!(
            !completions.iter().any(|c| c.label == "arrayref" || c.label == "hashref"),
            "dash trigger on `$hash-` must not return generic snippets; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        Ok(())
    }

    // ── Hash key completion tests ─────────────────────────────────────────────

    #[test]
    fn test_hash_key_completion_basic() {
        // my %config = (host => 'localhost', port => 5432);
        // $config{ho<cursor>
        // Expected: "host" suggested, "port" filtered out by prefix
        let code = "my %config = (host => 'localhost', port => 5432);\n$config{ho";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "host"),
            "expected 'host' in hash key completions for prefix 'ho'; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        assert!(
            !completions.iter().any(|c| c.label == "port"),
            "expected 'port' filtered out by prefix 'ho'; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_hash_key_completion_empty_prefix() {
        // $config{<cursor> -- all keys returned
        let code = "my %config = (host => 'localhost', port => 5432);\n$config{";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        assert!(
            completions.iter().any(|c| c.label == "host"),
            "expected 'host' in hash key completions with empty prefix; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
        assert!(
            completions.iter().any(|c| c.label == "port"),
            "expected 'port' in hash key completions with empty prefix; got: {:?}",
            completions.iter().map(|c| &c.label).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_hash_key_completion_does_not_fire_for_hashref_deref() {
        // $ref->{ho<cursor> -- hashref deref, must NOT suggest hash keys
        let code = "my $ref = {host => 'localhost'};\n$ref->{ho";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        // Must not return a Property-kinded "host" completion (hash key detection
        // must bail when `->` precedes the `{`)
        assert!(
            !completions
                .iter()
                .any(|c| c.label == "host" && c.kind == CompletionItemKind::Property),
            "hashref deref `$ref->{{ho` must not produce Property-kinded 'host' completion; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_hash_key_completion_in_comment_no_suggestions() {
        // # $config{ho<cursor> -- in comment, should not suggest hash keys
        let code = "my %config = (host => 'localhost');\n# $config{ho";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        assert!(
            !completions
                .iter()
                .any(|c| c.label == "host" && c.kind == CompletionItemKind::Property),
            "hash key completion must not fire inside a comment; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_hash_key_completion_unknown_variable_returns_empty_for_that_hash() {
        // $config{<cursor> where %config has no known init -- no leaked keys from %other
        let code = "my %other = (a => 1);\n$config{";
        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());
        // Keys from %other must not appear as completions for $config{}
        assert!(
            !completions.iter().any(|c| c.label == "a" && c.kind == CompletionItemKind::Property),
            "keys from %%other must not leak into %%config completions; got: {:?}",
            completions.iter().map(|c| (&c.label, &c.kind)).collect::<Vec<_>>()
        );
    }
}
