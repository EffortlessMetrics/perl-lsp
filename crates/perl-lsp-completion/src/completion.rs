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
//! **Production-ready file completion with enterprise-grade security:**
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

mod builtins;
mod context;
mod file_path;
mod functions;
mod items;
mod keywords;
mod methods;
mod packages;
mod sort;
mod test_more;
mod variables;
mod workspace;

// Re-export public types
pub use self::context::CompletionContext;
pub use self::items::{CompletionItem, CompletionItemKind};

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
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider;
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
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider;
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
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider;
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
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider;
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
    /// enterprise-scale LSP environments where completion requests may need
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
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider;
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
            Ok(ctx) => ctx,
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

        // Determine what kind of completions to provide based on context
        if self.is_has_options_key_context(source, position) {
            self.add_has_option_completions(&mut completions, &context);
        } else if context.trigger_character == Some('>') && context.prefix.ends_with("->") {
            // Method completion must run before sigil-prefixed variable completion.
            methods::add_method_completions(&mut completions, &context, source, &self.symbol_table);
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
                    let mut file_context = context.clone();
                    file_context.prefix = path_prefix.to_string();
                    file_context.prefix_start = start + 1;
                    self.add_file_completions_with_cancellation(
                        &mut completions,
                        &file_context,
                        is_cancelled,
                    );
                }
            }
        } else {
            // General completion: keywords, functions, variables
            let keywords = keywords::create_keywords();
            if context.prefix.is_empty() || self.could_be_keyword(&context.prefix, &keywords) {
                keywords::add_keyword_completions(&mut completions, &context, &keywords);
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
        match std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            sort::deduplicate_and_sort(completions.clone())
        })) {
            Ok(sorted_completions) => sorted_completions,
            Err(_) => {
                completions // Return unsorted completions as fallback
            }
        }
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
    /// ```rust
    /// use perl_parser_core::Parser;
    /// use perl_lsp_providers::ide::lsp_compat::completion::CompletionProvider;
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

    /// Analyze the context at the cursor position
    fn analyze_context(&self, source: &str, position: usize) -> CompletionContext {
        // Find the prefix (text before cursor on the same line)
        let line_start = source[..position].rfind('\n').map(|p| p + 1).unwrap_or(0);
        let _prefix = source[line_start..position].to_string();

        // Find the word being typed
        // Special handling for method calls: include the -> and the receiver
        let (word_prefix, prefix_start) =
            if position >= 2 && &source[position.saturating_sub(2)..position] == "->" {
                // We're right after ->, find the receiver variable
                let receiver_start = source[..position.saturating_sub(2)]
                    .rfind(|c: char| {
                        !c.is_alphanumeric() && c != '_' && c != '$' && c != '@' && c != '%'
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

        // Detect trigger character
        let trigger_character = if position > 0 { source.chars().nth(position - 1) } else { None };

        // Simple heuristics for context detection
        let in_string = self.is_in_string(source, position);
        let in_regex = self.is_in_regex(source, position);
        let in_comment = self.is_in_comment(source, position);

        CompletionContext::new(
            &self.symbol_table,
            position,
            trigger_character,
            in_string,
            in_regex,
            in_comment,
            word_prefix,
            prefix_start,
        )
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
        let callbacks = file_path::FilePathCallbacks::default();
        file_path::add_file_completions_with_callbacks(
            completions,
            context,
            &callbacks,
            is_cancelled,
        );
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
    fn could_be_keyword(
        &self,
        prefix: &str,
        keywords: &std::collections::HashSet<&'static str>,
    ) -> bool {
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

    /// Simple heuristic to check if position is in a regex
    fn is_in_regex(&self, source: &str, position: usize) -> bool {
        // Look for regex patterns before position
        let before = &source[..position];
        if let Some(last_slash) = before.rfind('/') {
            if last_slash > 0 {
                let prev_char = before.chars().nth(last_slash - 1);
                matches!(prev_char, Some('~' | '!'))
            } else {
                false
            }
        } else {
            false
        }
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
}
