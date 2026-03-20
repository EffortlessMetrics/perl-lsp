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

// Re-export public types
pub use self::context::CompletionContext;
pub use self::items::{CompletionItem, CompletionItemKind};
pub use self::test_more::get_test_more_documentation;

use perl_parser_core::ast::Node;
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind, SymbolTable};
use perl_workspace_index::workspace_index::WorkspaceIndex;
use std::sync::Arc;

/// Completion provider
pub struct CompletionProvider {
    symbol_table: SymbolTable,
    workspace_index: Option<Arc<WorkspaceIndex>>,
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
    /// use perl_workspace_index::workspace_index::WorkspaceIndex;
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
        let symbol_table = SymbolExtractor::new_with_source(source).extract(ast);

        CompletionProvider { symbol_table, workspace_index }
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

        let mut completions = Vec::new();

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
            );
        } else if self.is_has_options_key_context(source, position) {
            self.add_has_option_completions(&mut completions, &context);
        } else if context.trigger_character == Some('>') && context.prefix.ends_with("->") {
            // Method completion must run before sigil-prefixed variable completion.
            methods::add_method_completions(&mut completions, &context, source, &self.symbol_table);
            // Add workspace-indexed methods for the receiver's type
            workspace::add_workspace_method_completions(
                &mut completions,
                &context,
                source,
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
            !rest.contains(';')
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

        segment
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch.is_ascii_whitespace())
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

    /// Add common Moo/Moose `has` option-key completions.
    fn add_has_option_completions(
        &self,
        completions: &mut Vec<CompletionItem>,
        context: &CompletionContext,
    ) {
        let prefix = context.prefix.trim();
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
                });
            }
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
                if symbol.kind == SymbolKind::Subroutine && name.starts_with(prefix) {
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
    use perl_workspace_index::workspace_index::WorkspaceIndex;
    use std::sync::Arc;
    use url::Url;

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
        let code = r#"my $foo = 1; my $bar = qr/^$fo/"#;

        let mut parser = Parser::new(code);
        let ast = must(parser.parse());
        let provider = CompletionProvider::new(&ast);
        let completions = provider.get_completions(code, code.len());

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
}
