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
mod import_map;
mod items;
mod keywords;
mod lexical_context;
mod methods;
mod packages;
mod regex_patterns;
mod request;
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
pub use self::workspace::collect_module_names_from_roots_with_cache;
pub use self::xs_api::{add_xs_api_completions_for_prefix, get_xs_api_documentation, is_xs_source};

use crate::providers::completion::module_scan_cache::ModuleCompletionScanCache;
use perl_parser_core::ast::Node;
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
    /// Modules referenced by `use` statements in the buffer, regardless
    /// of explicit symbol lists. Used by the bounded Unknown-receiver
    /// method-completion fallback (#7929) — bare `use Foo;` *is*
    /// captured here, while `import_map` only tracks explicit symbol
    /// lists.
    used_modules: HashSet<String>,
    include_paths: Vec<PathBuf>,
    system_inc_paths: Vec<PathBuf>,
    include_system_inc: bool,
    /// Optional runtime-owned scan cache (issue #8514).
    ///
    /// When `Some`, repeated `use Module::Prefix|` completions within 1 second
    /// avoid re-scanning the same include root subdirectory. The cache is owned
    /// by `LspServer` and survives across requests; this field holds a cheap
    /// `Arc` clone valid for the lifetime of this provider instance.
    scan_cache: Option<Arc<ModuleCompletionScanCache>>,
}

fn method_receiver_start(source: &str, arrow_start: usize) -> usize {
    let mut brace_depth = 0usize;
    let mut bracket_depth = 0usize;
    let mut paren_depth = 0usize;

    for (idx, ch) in source[..arrow_start].char_indices().rev() {
        match ch {
            '}' => {
                brace_depth += 1;
                continue;
            }
            '{' if brace_depth > 0 => {
                brace_depth -= 1;
                continue;
            }
            ']' => {
                bracket_depth += 1;
                continue;
            }
            '[' if bracket_depth > 0 => {
                bracket_depth -= 1;
                continue;
            }
            ')' => {
                paren_depth += 1;
                continue;
            }
            '(' if paren_depth > 0 => {
                paren_depth -= 1;
                continue;
            }
            _ => {}
        }

        if brace_depth > 0 || bracket_depth > 0 || paren_depth > 0 {
            continue;
        }

        if !is_method_receiver_char(ch) {
            return idx + ch.len_utf8();
        }
    }

    0
}

fn is_method_receiver_char(ch: char) -> bool {
    ch.is_alphanumeric()
        || matches!(ch, '_' | '$' | '@' | '%' | ':' | '-' | '>' | '{' | '}' | '[' | ']')
}

fn next_char_boundary_after(source: &str, index: usize) -> usize {
    source[index..].chars().next().map_or(source.len(), |ch| index + ch.len_utf8())
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
        Self::new_with_index_and_source_and_paths(
            ast,
            "",
            workspace_index,
            Vec::new(),
            Vec::new(),
            false,
        )
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
        Self::new_with_index_and_source_and_paths(
            ast,
            source,
            workspace_index,
            Vec::new(),
            Vec::new(),
            false,
        )
    }

    /// Create a completion provider with explicit module completion search roots.
    pub fn new_with_index_and_source_and_paths(
        ast: &Node,
        source: &str,
        workspace_index: Option<Arc<WorkspaceIndex>>,
        include_paths: Vec<PathBuf>,
        system_inc_paths: Vec<PathBuf>,
        include_system_inc: bool,
    ) -> Self {
        let symbol_table = Self::extract_symbol_table(ast, source);
        let class_models = Self::build_class_models(ast);
        let type_engine = Self::build_type_engine(ast, workspace_index.is_some());
        let import_map = import_map::extract_import_map(ast);
        let used_modules = import_map::collect_used_module_names(ast);

        CompletionProvider {
            symbol_table,
            class_models,
            type_engine,
            workspace_index,
            import_map,
            used_modules,
            include_paths,
            system_inc_paths,
            include_system_inc,
            scan_cache: None,
        }
    }

    /// Attach a runtime-owned scan cache to this provider.
    ///
    /// Called by `LspServer` after construction to wire the server-level cache
    /// into the per-request provider without changing the public constructor API.
    pub fn with_scan_cache(mut self, cache: Arc<ModuleCompletionScanCache>) -> Self {
        self.scan_cache = Some(cache);
        self
    }

    fn extract_symbol_table(ast: &Node, source: &str) -> SymbolTable {
        SymbolExtractor::new_with_source(source).extract(ast)
    }

    fn build_class_models(ast: &Node) -> Vec<ClassModel> {
        ClassModelBuilder::new().build(ast)
    }

    fn build_type_engine(ast: &Node, has_workspace_index: bool) -> Option<TypeInferenceEngine> {
        has_workspace_index.then(|| {
            let mut type_engine = TypeInferenceEngine::new();
            let _ = type_engine.infer(ast);
            type_engine
        })
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
        request::complete(self, source, position, filepath, is_cancelled)
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
        let (word_prefix, prefix_start) = if source[..position].ends_with("->") {
            // We're right after ->, find the receiver variable or package name.
            let receiver_start = method_receiver_start(source, position.saturating_sub(2));
            (source[receiver_start..position].to_string(), receiver_start)
        } else if position >= 1
            && source.as_bytes()[position - 1] == b'-'
            && (position < 2 || source.as_bytes()[position - 2] != b'-')
        {
            // Cursor is right after a lone `-` (not `--`). This fires when `-` is a
            // trigger character and the user has typed the first char of `->`.
            // Build the prefix as receiver + `->` so that downstream method-completion
            // functions see the same shape as the `>` trigger path.
            let receiver_start = method_receiver_start(source, position.saturating_sub(1));
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
                .map(|p| next_char_boundary_after(source, p))
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
                .map(|idx| next_char_boundary_after(receiver, idx))
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
                .char_indices()
                .rev()
                .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
                .map(|(p, c)| p + c.len_utf8())
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
        // Use byte comparison to avoid slicing at a non-char-boundary when a multi-byte
        // character immediately precedes `{`.
        if brace_pos >= 2 && source.as_bytes().get(brace_pos - 2..brace_pos) == Some(b"->") {
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
            .char_indices()
            .rev()
            .find(|(_, c)| !c.is_alphanumeric() && *c != '_')
            .map(|(p, c)| p + c.len_utf8())
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
                label_details: None,
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
                    label_details: None,
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
                    label_details: None,
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
                label_details: None,
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

    fn is_in_string(&self, source: &str, position: usize) -> bool {
        lexical_context::is_in_string(source, position)
    }

    fn is_in_regex(source: &str, position: usize) -> bool {
        lexical_context::is_in_regex(source, position)
    }

    pub(crate) fn is_in_regex_flags(source: &str, position: usize) -> bool {
        lexical_context::is_in_regex_flags(source, position)
    }

    fn is_in_comment(&self, source: &str, position: usize) -> bool {
        lexical_context::is_in_comment(source, position)
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
mod tests;
