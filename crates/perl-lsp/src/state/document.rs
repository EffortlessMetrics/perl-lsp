//! Document state management
//!
//! Manages document content with Rope-based storage for efficient
//! incremental updates and UTF-16 position mapping.

use perl_parser::declaration::ParentMap;
use perl_parser::position::LineStartsCache;
use std::borrow::Cow;
use std::sync::Arc;
use std::sync::atomic::{AtomicU32, Ordering};

/// Degradation tier for a document, indicating what level of LSP functionality
/// is available based on parse success.
///
/// Features should check the tier before attempting operations that require
/// a valid or partial AST. The tier is computed after each parse attempt and
/// stored alongside the document state.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum DegradationTier {
    /// Parse failed completely -- no AST available. Only basic text-based
    /// features are provided (word completion, bracket matching, folding
    /// via brace counting, text-based symbol extraction).
    Minimal,
    /// Parse produced errors but also produced a partial AST. Best-effort
    /// completions, navigation, and diagnostics are available from whatever
    /// parsed successfully.
    Partial,
    /// Parse succeeded without errors. All features are available.
    Full,
}

impl DegradationTier {
    /// Compute the degradation tier from parse results.
    ///
    /// - `Full`: AST present and no parse errors
    /// - `Partial`: AST present but parse errors exist
    /// - `Minimal`: No AST (parse failed completely)
    pub fn from_parse_result(
        ast: &Option<Arc<perl_parser::ast::Node>>,
        parse_errors: &[perl_parser::error::ParseError],
    ) -> Self {
        match ast {
            Some(_) if parse_errors.is_empty() => DegradationTier::Full,
            Some(_) => DegradationTier::Partial,
            None => DegradationTier::Minimal,
        }
    }

    /// Whether AST-based features (hover, go-to-definition, semantic
    /// tokens, etc.) should be attempted at this tier.
    pub fn has_ast(self) -> bool {
        matches!(self, DegradationTier::Full | DegradationTier::Partial)
    }

    /// Whether full semantic analysis (unused variable detection, type
    /// inference) should be attempted. Only reliable at `Full`.
    pub fn has_full_semantics(self) -> bool {
        matches!(self, DegradationTier::Full)
    }

    /// Human-readable label for diagnostics and logging.
    pub fn as_str(self) -> &'static str {
        match self {
            DegradationTier::Full => "full",
            DegradationTier::Partial => "partial",
            DegradationTier::Minimal => "minimal",
        }
    }
}

impl std::fmt::Display for DegradationTier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(self.as_str())
    }
}

/// Document state with Rope-based content management for efficient LSP operations
///
/// This structure maintains both a Rope for efficient edits and a cached String
/// representation for compatibility with subsystems that expect `&str`. The dual
/// representation ensures optimal performance for both incremental edits (Rope)
/// and parsing/analysis operations (String).
///
/// ## Performance Characteristics
/// - **Rope operations**: O(log n) for insertions, deletions, and slicing
/// - **String operations**: O(1) access for parsing and analysis
/// - **Position mapping**: O(log n) with line starts cache
/// - **Memory usage**: ~2x content size due to dual representation
#[derive(Clone)]
pub struct DocumentState {
    /// Rope-backed document content providing O(log n) edit performance
    ///
    /// The rope is the authoritative source for document content and supports
    /// efficient incremental updates from LSP TextDocumentContentChangeEvents.
    pub rope: ropey::Rope,

    /// Cached string representation synchronized with rope content
    ///
    /// This cached copy enables efficient access for parsing and analysis
    /// subsystems that operate on `&str`. Updated lazily when rope changes.
    pub text: String,

    /// LSP document version number for synchronization
    pub version: i32,

    /// Cached parsed AST for semantic analysis
    ///
    /// Rebuilt when document content changes, providing fast access to
    /// structured representation for LSP features like hover and completion.
    pub ast: Option<Arc<perl_parser::ast::Node>>,

    /// Parse errors from last AST generation attempt
    pub parse_errors: Vec<perl_parser::error::ParseError>,

    /// Parent map for O(1) scope traversal during semantic analysis
    ///
    /// Built once per AST generation, uses FxHashMap for faster pointer hashing
    /// enabling efficient parent lookups during symbol resolution.
    pub parent_map: ParentMap,

    /// Line starts cache for O(log n) LSP position conversion
    ///
    /// Enables fast conversion between byte offsets (rope operations) and
    /// line/column positions (LSP protocol) with UTF-16 encoding support.
    pub line_starts: LineStartsCache,

    /// Generation counter for race condition prevention in concurrent access
    pub generation: Arc<AtomicU32>,

    /// Current degradation tier based on the most recent parse attempt.
    ///
    /// Computed from `ast` and `parse_errors` after each parse. Feature
    /// providers should check this before attempting AST-dependent operations.
    pub degradation_tier: DegradationTier,

    /// Incremental document state for fast re-parsing on keystrokes.
    ///
    /// Initialized on didOpen and updated on every didChange. When `None`
    /// the LSP falls back to a full reparse. Only compiled when the
    /// `incremental` feature is enabled.
    #[cfg(feature = "incremental")]
    pub incremental_doc:
        Option<perl_incremental_parsing::incremental::incremental_document::IncrementalDocument>,

    /// Checkpoint-based incremental lexer state for the didChange fast path.
    ///
    /// On every ranged edit, the LSP server first attempts to apply the edit
    /// via `perl_incremental_parsing::incremental::apply_edits`, which resumes
    /// lexing from the nearest checkpoint before the edit site instead of
    /// re-lexing from offset 0. On success the updated AST replaces the full
    /// parse result. Falls back to a full `Parser::new` parse when:
    /// - The field is `None` (not yet initialized or previous apply failed).
    /// - The edit is a full-document replace (no range).
    /// - `apply_edits` returns `Err` (e.g. edit > 64 KB or > 10 changed lines).
    ///
    /// Only compiled when the `incremental` feature is enabled.
    #[cfg(feature = "incremental")]
    pub incremental_state: Option<perl_incremental_parsing::incremental::IncrementalState>,
}

impl DocumentState {
    /// Create a new document state from content
    pub fn new(content: &str, version: i32) -> Self {
        let rope = ropey::Rope::from_str(content);
        let text = content.to_string();
        let line_starts = LineStartsCache::new(content);

        Self {
            rope,
            text,
            version,
            ast: None,
            parse_errors: Vec::new(),
            parent_map: ParentMap::default(),
            line_starts,
            generation: Arc::new(AtomicU32::new(0)),
            degradation_tier: DegradationTier::Minimal,
            #[cfg(feature = "incremental")]
            incremental_doc: None,
            #[cfg(feature = "incremental")]
            incremental_state: None,
        }
    }

    /// Update document content and invalidate caches
    pub fn update_content(&mut self, content: &str, version: i32) {
        self.rope = ropey::Rope::from_str(content);
        self.text = content.to_string();
        self.version = version;
        self.ast = None;
        self.parse_errors.clear();
        self.parent_map = ParentMap::default();
        self.line_starts = LineStartsCache::new(content);
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.degradation_tier = DegradationTier::Minimal;
        #[cfg(feature = "incremental")]
        {
            self.incremental_doc = None;
            self.incremental_state = None;
        }
    }

    /// Get the current generation number
    pub fn current_generation(&self) -> u32 {
        self.generation.load(Ordering::SeqCst)
    }

    /// Apply a text change to the document
    pub fn apply_change(
        &mut self,
        start_line: usize,
        start_char: usize,
        end_line: usize,
        end_char: usize,
        new_text: &str,
        version: i32,
    ) {
        // Convert LSP positions to rope indices
        let start_idx = self.lsp_position_to_char_idx(start_line, start_char);
        let end_idx = self.lsp_position_to_char_idx(end_line, end_char);

        // Apply the change to the rope
        if start_idx < end_idx && end_idx <= self.rope.len_chars() {
            self.rope.remove(start_idx..end_idx);
        }
        if !new_text.is_empty() && start_idx <= self.rope.len_chars() {
            self.rope.insert(start_idx, new_text);
        }

        // Update cached string and caches
        self.text = self.rope.to_string();
        self.version = version;
        self.ast = None;
        self.parse_errors.clear();
        self.parent_map = ParentMap::default();
        self.line_starts = LineStartsCache::new(&self.text);
        self.generation.fetch_add(1, Ordering::SeqCst);
        self.degradation_tier = DegradationTier::Minimal;
    }

    /// Convert LSP position (line, character) to rope char index
    fn lsp_position_to_char_idx(&self, line: usize, character: usize) -> usize {
        if line >= self.rope.len_lines() {
            return self.rope.len_chars();
        }

        let line_start = self.rope.line_to_char(line);
        let line_text = self.rope.line(line);
        let line_len = line_text.len_chars();

        // UTF-16 character offset to char index
        let mut utf16_offset = 0;
        let mut char_idx = 0;

        for ch in line_text.chars() {
            if utf16_offset >= character {
                break;
            }
            utf16_offset += ch.len_utf16();
            char_idx += 1;
        }

        line_start + char_idx.min(line_len)
    }
}

/// Normalize legacy package separator ' to ::
pub fn normalize_package_separator(s: &str) -> Cow<'_, str> {
    perl_module_path::normalize_package_separator(s)
}

/// Client capabilities received during initialization
#[derive(Debug, Clone, Default)]
pub struct ClientCapabilities {
    /// Supports LocationLink for goto declaration
    pub declaration_link_support: bool,
    /// Supports LocationLink for goto definition
    pub definition_link_support: bool,
    /// Supports LocationLink for goto type definition
    pub type_definition_link_support: bool,
    /// Supports LocationLink for goto implementation
    pub implementation_link_support: bool,
    /// Supports dynamic registration for file watching
    pub dynamic_registration_support: bool,
    /// Supports snippet syntax in completion items
    pub snippet_support: bool,
    /// Supports markdown message content in diagnostics (LSP 3.18)
    ///
    /// When true, the server can provide rich markdown formatting in diagnostic
    /// messages via the `data.messageMarkup` field in pull diagnostics responses.
    pub markup_message_support: bool,
    /// Supports workspace/codeLens/refresh request
    pub code_lens_refresh_support: bool,
    /// Supports workspace/semanticTokens/refresh request
    pub semantic_tokens_refresh_support: bool,
    /// Supports workspace/inlayHint/refresh request
    pub inlay_hint_refresh_support: bool,
    /// Client declared textDocument/inlayHint capability
    pub inlay_hint_support: bool,
    /// Supports workspace/inlineValue/refresh request
    pub inline_value_refresh_support: bool,
    /// Supports workspace/diagnostic/refresh request
    pub diagnostic_refresh_support: bool,
    /// Supports workspace/foldingRange/refresh request
    pub folding_range_refresh_support: bool,
    /// Supports window/showDocument request
    pub show_document_support: bool,
    /// Supports window/workDoneProgress/create request
    pub work_done_progress_support: bool,
    /// Supports workspace/configuration reverse request
    pub workspace_configuration_support: bool,
    /// Properties the client can resolve via inlayHint/resolve
    ///
    /// Parsed from `capabilities.textDocument.inlayHint.resolveSupport.properties`.
    /// The server must only include a resolved property in the response when the
    /// client has declared that property here.  A `None` value means the client
    /// sent no `resolveSupport` entry at all (i.e. resolve is unsupported).
    pub inlay_hint_resolve_support: Option<std::collections::HashSet<String>>,
}
