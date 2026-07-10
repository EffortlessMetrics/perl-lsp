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
    /// Parse produced blocking errors but also produced a partial AST.
    /// Best-effort completions, navigation, and diagnostics are available from
    /// whatever parsed successfully. Advisory-only diagnostics do not
    /// downgrade to this tier.
    Partial,
    /// Parse succeeded without blocking errors. All features are available.
    /// Advisory-only diagnostics retain this tier rather than demoting to
    /// [`Self::Partial`].
    Full,
}

impl DegradationTier {
    /// Compute the degradation tier from parse results.
    ///
    /// - `Full`: AST present and no blocking parse errors
    /// - `Partial`: AST present with blocking parse errors
    /// - `Minimal`: No AST (parse failed completely)
    pub fn from_parse_result(
        ast: &Option<Arc<perl_parser::ast::Node>>,
        parse_errors: &[perl_parser::error::ParseError],
    ) -> Self {
        match ast {
            Some(_) if parse_errors.iter().all(|error| !error.blocks_clean_parse()) => {
                DegradationTier::Full
            }
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

/// A single, immutable, generation-tagged parse result.
///
/// Constructed once per parse attempt (successful, partial, or fully failed)
/// and published onto a [`DocumentState`] via
/// [`DocumentState::publish_parsed_if_current`]. Once published a snapshot
/// never changes -- a new parse produces a brand-new `ParsedSnapshot`, never
/// a mutation of an existing one.
///
/// This is the single source of truth for parsed state: `DocumentState` holds
/// at most one snapshot (`Option<Arc<ParsedSnapshot>>`) rather than the four
/// separate `ast` / `parse_errors` / `parent_map` / `degradation_tier` fields
/// it used to carry, so there is no dual-write and no way for those fields to
/// disagree with each other or with the generation they were parsed from.
///
/// ## Construction is guarded, not free
///
/// Every field is private. The only way to build a `ParsedSnapshot` is
/// [`Self::from_parse_result`], which derives `content_hash` from the exact
/// source text passed in, builds `parent_map` from the exact `ast` passed in
/// (never a different AST), and derives `degradation_tier` from the `(ast,
/// parse_errors)` pair -- so a caller cannot assemble a snapshot with a
/// parent map from one parse paired with an AST from another, a
/// `content_hash` that doesn't describe the parsed source, or a
/// degradation tier that disagrees with `ast`/`parse_errors`. Read fields
/// through the accessor methods below.
#[derive(Debug, Clone)]
pub struct ParsedSnapshot {
    /// The document generation this snapshot was parsed from.
    generation: u32,
    /// Hash of the document text this snapshot was parsed from.
    content_hash: u64,
    /// Parsed AST, or `None` when the parse failed completely.
    ast: Option<Arc<perl_parser::ast::Node>>,
    /// Parse errors from this parse attempt.
    parse_errors: Arc<[perl_parser::error::ParseError]>,
    /// Parent map built from `ast`, for O(1) scope traversal.
    parent_map: Arc<ParentMap>,
    /// Degradation tier computed from `ast` and `parse_errors`.
    degradation_tier: DegradationTier,
}

impl ParsedSnapshot {
    /// Build a `ParsedSnapshot` from a parse attempt's raw outputs.
    ///
    /// This is the *only* way to construct a `ParsedSnapshot` -- it derives
    /// every dependent field from `source`/`ast`/`parse_errors` so the
    /// result is internally consistent by construction:
    /// - `content_hash` is `hash_content(source)` (the same
    ///   `DefaultHasher`-over-text scheme the semantic-analyzer /
    ///   type-inference caches already use --
    ///   `perl_lsp_rs_core::tooling::perl_critic::hash_content`, reused
    ///   here rather than a second hashing scheme). Callers must pass the
    ///   exact source text `ast` was parsed from.
    /// - `parent_map` is built from `ast` via
    ///   `DeclarationProvider::build_parent_map`, so it can never be paired
    ///   with an AST from a different parse.
    /// - `degradation_tier` is [`DegradationTier::from_parse_result`] over
    ///   `(&ast, &parse_errors)`.
    ///
    /// `generation` is the caller's responsibility (it's the document
    /// generation this parse was performed for, which the constructor has
    /// no way to derive on its own) -- pair it with the same value passed
    /// to [`DocumentState::publish_parsed_if_current`] as
    /// `expected_generation`, which will reject a mismatch.
    pub(crate) fn from_parse_result(
        generation: u32,
        source: &str,
        ast: Option<Arc<perl_parser::ast::Node>>,
        parse_errors: Vec<perl_parser::error::ParseError>,
    ) -> Self {
        let content_hash = perl_lsp_rs_core::tooling::perl_critic::hash_content(source);
        let mut parent_map = ParentMap::default();
        if let Some(ref arc) = ast {
            crate::declaration::DeclarationProvider::build_parent_map(arc, &mut parent_map, None);
        }
        let degradation_tier = DegradationTier::from_parse_result(&ast, &parse_errors);
        Self {
            generation,
            content_hash,
            ast,
            parse_errors: Arc::from(parse_errors),
            parent_map: Arc::new(parent_map),
            degradation_tier,
        }
    }

    /// The document generation this snapshot was parsed from.
    pub fn generation(&self) -> u32 {
        self.generation
    }

    /// Hash of the document text this snapshot was parsed from. See
    /// [`Self::from_parse_result`] for the hashing scheme.
    pub fn content_hash(&self) -> u64 {
        self.content_hash
    }

    /// Parsed AST, or `None` when the parse failed completely.
    pub fn ast(&self) -> Option<&Arc<perl_parser::ast::Node>> {
        self.ast.as_ref()
    }

    /// Parse errors from this parse attempt.
    pub fn parse_errors(&self) -> &[perl_parser::error::ParseError] {
        &self.parse_errors
    }

    /// Parse errors from this parse attempt, as a cheaply-clonable `Arc<[_]>`.
    ///
    /// For callers that need to carry the error list past the lifetime of
    /// this `ParsedSnapshot` reference (e.g. escaping a `documents` map lock
    /// by snapshotting into an owned tuple) without the allocation
    /// `Arc::from(self.parse_errors())` would incur -- a refcount bump
    /// against the same backing allocation `self.parse_errors()` borrows
    /// from. Prefer [`Self::parse_errors`] when a borrow suffices.
    pub(crate) fn parse_errors_arc(&self) -> Arc<[perl_parser::error::ParseError]> {
        Arc::clone(&self.parse_errors)
    }

    /// Parent map built from [`Self::ast`], for O(1) scope traversal.
    pub fn parent_map(&self) -> &ParentMap {
        &self.parent_map
    }

    /// Degradation tier computed from `ast` and `parse_errors` for this
    /// parse attempt. See [`DegradationTier::from_parse_result`].
    pub fn degradation_tier(&self) -> DegradationTier {
        self.degradation_tier
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
///
/// ## Parsed state
///
/// Parsed state (AST, parse errors, parent map, degradation tier) lives
/// behind a single `parsed: Option<Arc<ParsedSnapshot>>` field, not as
/// separate `DocumentState` fields. This is deliberate: it is the load-
/// bearing seam for the async parse worker (a later change) -- once parsing
/// moves off the mutation lock, a read can no longer assume the latest
/// snapshot matches the current text generation, and every call site must
/// make an explicit choice between "give me the current-generation parse or
/// nothing" ([`Self::current_parsed`]) and "give me whatever was last
/// published, even if stale" ([`Self::latest_parsed`]). Access the field only
/// through these accessors and [`Self::publish_parsed_if_current`] -- never
/// add the four fields back directly.
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

    /// Latest published parse result, if any.
    ///
    /// Private by design -- read it only through [`Self::latest_parsed`],
    /// [`Self::current_parsed`], or write it only through
    /// [`Self::publish_parsed_if_current`]. This keeps the freshness
    /// invariant (`current_parsed().is_some()` iff `parsed.generation ==
    /// generation.load()`) from being bypassed by a scattered direct read.
    parsed: Option<Arc<ParsedSnapshot>>,

    /// Line starts cache for O(log n) LSP position conversion
    ///
    /// Enables fast conversion between byte offsets (rope operations) and
    /// line/column positions (LSP protocol) with UTF-16 encoding support.
    pub line_starts: LineStartsCache,

    /// Generation counter for race condition prevention in concurrent access
    pub generation: Arc<AtomicU32>,

    /// Incremental document state for the (dormant) keystroke fast-path.
    ///
    /// Off by default (#3396): the committed AST that every provider reads is
    /// always produced by the full `Parser::new` parse, and nothing on the read
    /// path consumes this field, so it is `None` unless
    /// `LspServer::set_incremental_eager(true)` opts in. When opted in it is
    /// initialized on didOpen and updated on every didChange. Only compiled when
    /// the `incremental` feature is enabled.
    #[cfg(feature = "incremental")]
    pub incremental_doc:
        Option<perl_parser::incremental::incremental_document::IncrementalDocument>,

    /// Checkpoint-based incremental lexer state for the (dormant) didChange
    /// fast path.
    ///
    /// Off by default (#3396) for the same reason as `incremental_doc`: it feeds
    /// nothing on the read path, so it is only maintained when
    /// `LspServer::set_incremental_eager(true)` opts in. When opted in, each
    /// ranged edit attempts `perl_parser::incremental::apply_edits`, which
    /// resumes lexing from the nearest checkpoint before the edit site instead
    /// of re-lexing from offset 0. This accelerates the lexer pass only — the
    /// committed AST still comes from the full parse. Falls back to a full
    /// reinitialization when:
    /// - The field is `None` (not yet initialized or previous apply failed).
    /// - The edit is a full-document replace (no range).
    /// - `apply_edits` returns `Err` (e.g. edit > 64 KB or > 10 changed lines).
    ///
    /// Only compiled when the `incremental` feature is enabled.
    #[cfg(feature = "incremental")]
    pub incremental_state: Option<perl_parser::incremental::IncrementalState>,
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
            parsed: None,
            line_starts,
            generation: Arc::new(AtomicU32::new(0)),
            #[cfg(feature = "incremental")]
            incremental_doc: None,
            #[cfg(feature = "incremental")]
            incremental_state: None,
        }
    }

    /// Construct a document state from raw rope/text/version parts while
    /// preserving an existing generation counter.
    ///
    /// Used by `didChange` handling: when a change reuses a document's
    /// generation `Arc` (rather than starting a fresh one, as `new` does),
    /// callers outside this module cannot build a `DocumentState` via struct
    /// literal syntax because `parsed` is private -- this is the sanctioned
    /// construction path. `parsed` starts `None`; publish a snapshot via
    /// [`Self::publish_parsed_if_current`] afterward.
    pub(crate) fn from_parts(
        rope: ropey::Rope,
        text: String,
        version: i32,
        generation: Arc<AtomicU32>,
    ) -> Self {
        let line_starts = LineStartsCache::new_rope(&rope);
        Self {
            rope,
            text,
            version,
            parsed: None,
            line_starts,
            generation,
            #[cfg(feature = "incremental")]
            incremental_doc: None,
            #[cfg(feature = "incremental")]
            incremental_state: None,
        }
    }

    /// Update document content and invalidate caches
    ///
    /// `parsed` is deliberately **preserved**, not cleared: bumping
    /// `generation` already makes [`Self::current_parsed`] correctly report
    /// staleness (the snapshot's `generation` no longer matches), while
    /// [`Self::latest_parsed`] keeps exposing the previous snapshot until a
    /// fresh parse republishes for the new generation -- see
    /// [`Self::replace_text_state`] for the invariant this preserves and why
    /// clearing `parsed` on every edit would violate it.
    pub fn update_content(&mut self, content: &str, version: i32) {
        self.rope = ropey::Rope::from_str(content);
        self.text = content.to_string();
        self.version = version;
        self.line_starts = LineStartsCache::new(content);
        self.generation.fetch_add(1, Ordering::SeqCst);
        #[cfg(feature = "incremental")]
        {
            self.incremental_doc = None;
            self.incremental_state = None;
        }
    }

    /// Replace this document's text state (rope/text/version/line_starts)
    /// in place, without touching `parsed` or `generation`.
    ///
    /// Unlike [`Self::new`]/[`Self::from_parts`] (which always start with
    /// `parsed: None`), this preserves whatever was previously published.
    /// That is the production invariant [`Self::latest_parsed`] promises --
    /// "the last publication, even if stale" -- and it only holds if a real
    /// text edit doesn't silently discard the previous snapshot by
    /// reconstructing a fresh `DocumentState`. After calling this:
    /// - `current_parsed()` correctly becomes `None` once `generation` no
    ///   longer matches the preserved snapshot's generation (see below).
    /// - `latest_parsed()` keeps returning the previous snapshot until a
    ///   fresh parse publishes one for the new generation via
    ///   [`Self::publish_parsed_if_current`].
    ///
    /// **Does not bump `generation` itself** -- that is the caller's
    /// responsibility, and deliberately decoupled from the text-state
    /// update: production callers (see the `didChange` handler in
    /// `runtime/text_sync.rs`) bump the generation counter *before* this is
    /// called, as part of the same edit, for a reason unrelated to this
    /// method (stale-request detection while the parse is in flight -- a
    /// concurrent edit must be detectable before the slow parse even
    /// starts). Calling `replace_text_state` without an intervening
    /// generation bump leaves `current_parsed()` re-validating a snapshot
    /// that predates this edit, which is almost never what a caller wants;
    /// bump `generation` (e.g. via `self.generation.fetch_add(1, ..)`)
    /// first.
    pub(crate) fn replace_text_state(&mut self, rope: ropey::Rope, text: String, version: i32) {
        self.line_starts = LineStartsCache::new_rope(&rope);
        self.rope = rope;
        self.text = text;
        self.version = version;
    }

    /// Get the current generation number
    pub fn current_generation(&self) -> u32 {
        self.generation.load(Ordering::SeqCst)
    }

    /// The last published parse result, regardless of whether it matches the
    /// current document generation.
    ///
    /// Returns an **owned** `Arc<ParsedSnapshot>` (a refcount bump, not a
    /// data clone -- the backing field is already `Option<Arc<ParsedSnapshot>>`)
    /// rather than a borrow tied to `&self`. This is deliberate: a borrowed
    /// return would force every caller to hold the `documents` map lock for
    /// the full lifetime of its analysis, cementing exactly the lock
    /// boundary this seam exists to eventually break. An owned return lets a
    /// caller grab the snapshot, drop the lock, and analyze off-lock later
    /// (the async parse worker's read path will need this).
    ///
    /// Use this only when the caller deliberately wants to tolerate
    /// staleness (e.g. keep showing the previous parse's results while a
    /// newer parse is in flight). Most callers should prefer
    /// [`Self::current_parsed`].
    pub fn latest_parsed(&self) -> Option<Arc<ParsedSnapshot>> {
        self.parsed.clone()
    }

    /// The published parse result, but only if it was parsed from the
    /// document's *current* generation.
    ///
    /// Returns an **owned** `Arc<ParsedSnapshot>` for the same reason as
    /// [`Self::latest_parsed`] -- see that method's doc comment.
    ///
    /// Returns `None` when no snapshot has ever been published, or when the
    /// last published snapshot is stale (parsed from an older generation
    /// than the document is now at). This is the freshness-correct default:
    /// once an async parse worker can publish out of order, a stale
    /// `Some` here would let a provider silently answer from an outdated
    /// AST. In today's fully synchronous parse-under-the-lock world, a
    /// published snapshot's generation always equals the current generation
    /// immediately after commit, so this is always `Some` right after a
    /// `didOpen`/`didChange` completes -- behavior-identical to reading the
    /// old `ast`/`parse_errors`/`parent_map`/`degradation_tier` fields
    /// directly.
    pub fn current_parsed(&self) -> Option<Arc<ParsedSnapshot>> {
        let snapshot = self.parsed.clone()?;
        (snapshot.generation == self.current_generation()).then_some(snapshot)
    }

    /// Publish a parse result, but only if `expected_generation` still
    /// matches the document's current generation *and* the snapshot itself
    /// was parsed from that generation.
    ///
    /// Returns `true` and stores `snapshot` when the publication is
    /// accepted; returns `false` and leaves the existing `parsed` value
    /// untouched when either:
    /// - a newer generation has already superseded `expected_generation`
    ///   (a stale parse result publishes nothing), or
    /// - `snapshot.generation != expected_generation` (a mismatched caller
    ///   passing a snapshot parsed from a different generation than the one
    ///   it claims to be publishing for -- every current call site
    ///   constructs the two to match, so this only guards against a future
    ///   caller mistake, not a live path).
    pub fn publish_parsed_if_current(
        &mut self,
        expected_generation: u32,
        snapshot: Arc<ParsedSnapshot>,
    ) -> bool {
        if self.current_generation() != expected_generation
            || snapshot.generation != expected_generation
        {
            return false;
        }
        self.parsed = Some(snapshot);
        true
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

        // Update cached string and caches. `parsed` is deliberately
        // preserved -- see the doc comment on `update_content`.
        self.text = self.rope.to_string();
        self.version = version;
        self.line_starts = LineStartsCache::new(&self.text);
        self.generation.fetch_add(1, Ordering::SeqCst);
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

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    /// Build a `ParsedSnapshot` at `generation` from real parse output for
    /// `source` via [`ParsedSnapshot::from_parse_result`] -- exercises the
    /// same guarded construction path the publication site
    /// (`runtime/text_sync.rs`) uses, rather than hand-rolling fixture data.
    fn snapshot_for(source: &str, generation: u32) -> ParsedSnapshot {
        let mut parser = perl_parser::Parser::new(source);
        let (ast, errors) = match parser.parse() {
            Ok(ast) => (Some(ast), parser.errors().to_vec()),
            Err(e) => (None, vec![e]),
        };
        let ast_arc = ast.map(Arc::new);
        ParsedSnapshot::from_parse_result(generation, source, ast_arc, errors)
    }

    #[test]
    fn current_parsed_matches_generation() {
        let mut doc = DocumentState::new("my $x = 1;", 1);
        let doc_gen = doc.current_generation();
        let snapshot = Arc::new(snapshot_for("my $x = 1;", doc_gen));
        assert!(doc.publish_parsed_if_current(doc_gen, snapshot));
        let current = must_some(doc.current_parsed());
        assert_eq!(current.generation(), doc_gen);
    }

    #[test]
    fn current_parsed_none_when_generation_differs() {
        let mut doc = DocumentState::new("my $x = 1;", 1);
        let doc_gen = doc.current_generation();
        let snapshot = Arc::new(snapshot_for("my $x = 1;", doc_gen));
        assert!(doc.publish_parsed_if_current(doc_gen, snapshot));

        // Advance the generation without publishing a new snapshot -- the
        // previously published one is now stale relative to `current_parsed`.
        doc.generation.fetch_add(1, Ordering::SeqCst);
        assert!(
            doc.current_parsed().is_none(),
            "current_parsed must be None once the generation has moved past the snapshot"
        );
    }

    #[test]
    fn latest_parsed_survives_a_real_text_edit() {
        // Real-edit-path proof, not an artificial generation bump: a
        // published snapshot must survive `apply_change` -- the actual
        // production text-mutation method -- not just a bare
        // `generation.fetch_add`. Before `replace_text_state` existed, every
        // text-mutating method (`apply_change`, `update_content`, and
        // `DocumentState::from_parts` reconstruction as used by
        // `runtime/text_sync.rs`'s didChange) unconditionally cleared
        // `parsed`, so `latest_parsed()` silently returned `None`
        // immediately after any real edit in production -- the invariant
        // only ever passed in tests that bypassed the real edit path by
        // bumping the atomic directly.
        let mut doc = DocumentState::new("my $x = 1;", 1);
        let doc_gen = doc.current_generation();
        let snapshot = Arc::new(snapshot_for("my $x = 1;", doc_gen));
        assert!(doc.publish_parsed_if_current(doc_gen, snapshot));
        let old = must_some(doc.current_parsed());

        // A real ranged edit: replace the "1" in "my $x = 1;" (0-indexed
        // char 8, one char wide) with "2".
        doc.apply_change(0, 8, 0, 9, "2", 2);

        assert!(
            doc.current_parsed().is_none(),
            "generation must have advanced past the pre-edit snapshot with no fresh parse yet"
        );
        let latest = must_some(doc.latest_parsed());
        assert!(
            Arc::ptr_eq(&old, &latest),
            "apply_change must preserve the exact pre-edit snapshot object, not just its generation value"
        );
        assert_eq!(doc.text, "my $x = 2;", "the edit itself must still take effect");
    }

    #[test]
    fn publish_parsed_if_current_rejects_stale_expected_generation() {
        let mut doc = DocumentState::new("my $x = 1;", 1);
        let doc_gen = doc.current_generation();
        // Simulate another writer bumping the generation before this
        // publication lands.
        doc.generation.fetch_add(1, Ordering::SeqCst);

        let stale_snapshot = Arc::new(snapshot_for("my $x = 1;", doc_gen));
        assert!(
            !doc.publish_parsed_if_current(doc_gen, stale_snapshot),
            "a stale expected_generation must be rejected"
        );
        assert!(doc.latest_parsed().is_none(), "rejected publication must not be stored");
    }

    #[test]
    fn publish_parsed_if_current_accepts_matching_generation() {
        let mut doc = DocumentState::new("my $x = 1;", 1);
        let doc_gen = doc.current_generation();
        let snapshot = Arc::new(snapshot_for("my $x = 1;", doc_gen));
        assert!(doc.publish_parsed_if_current(doc_gen, snapshot));
        assert!(doc.current_parsed().is_some());
    }

    #[test]
    fn publish_parsed_if_current_rejects_mismatched_snapshot_generation() {
        // A malformed caller: `expected_generation` matches the document's
        // current generation, but the snapshot itself carries a different
        // `generation` value than the one it claims to publish for. No
        // current call site can produce this (every publication site builds
        // `expected_generation` and `snapshot.generation` from the same
        // value), but the guard must still reject it rather than let a
        // mismatched snapshot report itself as fresh via `current_parsed`.
        let mut doc = DocumentState::new("my $x = 1;", 1);
        let doc_gen = doc.current_generation();
        let mismatched_snapshot = Arc::new(snapshot_for("my $x = 1;", doc_gen + 1));
        assert!(
            !doc.publish_parsed_if_current(doc_gen, mismatched_snapshot),
            "a snapshot whose generation disagrees with expected_generation must be rejected"
        );
        assert!(doc.latest_parsed().is_none(), "rejected publication must not be stored");
    }

    #[test]
    fn failed_parse_yields_minimal_snapshot() {
        // Deliberately malformed Perl that fails to produce any AST at all.
        let source = "my $x = ";
        let snapshot = snapshot_for(source, 0);
        // Whatever the parser does with this input, the invariant under test
        // is the tier/ast/errors relationship computed by
        // `DegradationTier::from_parse_result`, not this specific input's
        // exact recovery behavior.
        if snapshot.ast().is_none() {
            assert_eq!(snapshot.degradation_tier(), DegradationTier::Minimal);
            assert!(!snapshot.parse_errors().is_empty(), "a failed parse must carry errors");
        }
    }

    #[test]
    fn partial_parse_retains_ast_and_errors() {
        // Malformed but recoverable: parser produces a partial AST plus errors.
        let source = "sub foo { my $x = 1; ";
        let snapshot = snapshot_for(source, 0);
        if snapshot.ast().is_some() && !snapshot.parse_errors().is_empty() {
            assert_eq!(snapshot.degradation_tier(), DegradationTier::Partial);
        }
    }

    #[test]
    fn advisory_only_parse_retains_full_semantics() {
        let snapshot = snapshot_for("my $pattern = qr/^(a+)+$/;", 0);

        assert!(snapshot.ast().is_some(), "valid regex must retain its AST");
        assert!(
            !snapshot.parse_errors().is_empty(),
            "nested quantifier must remain visible as an advisory"
        );
        assert!(
            snapshot.parse_errors().iter().all(|error| !error.blocks_clean_parse()),
            "the valid nested-quantifier diagnostic must be advisory-only"
        );
        assert_eq!(snapshot.degradation_tier(), DegradationTier::Full);
        assert!(snapshot.degradation_tier().has_full_semantics());
    }

    #[test]
    fn snapshot_is_immutable_and_shared_across_clone() {
        let mut doc = DocumentState::new("my $x = 1;", 1);
        let doc_gen = doc.current_generation();
        let snapshot = Arc::new(snapshot_for("my $x = 1;", doc_gen));
        assert!(doc.publish_parsed_if_current(doc_gen, snapshot));

        let cloned = doc.clone();
        let original_arc = must_some(doc.latest_parsed());
        let cloned_arc = must_some(cloned.latest_parsed());
        // Both point at the *same* allocation -- cloning DocumentState is a
        // refcount bump on the Arc<ParsedSnapshot>, not a deep copy.
        assert!(
            Arc::ptr_eq(&original_arc, &cloned_arc),
            "clone must share the same Arc<ParsedSnapshot> allocation"
        );

        // A subsequent edit on the original must not mutate the snapshot the
        // clone (or the original, before republishing) still observes.
        doc.update_content("my $y = 2;", 2);
        assert!(
            doc.current_parsed().is_none(),
            "current_parsed must be None immediately after an edit with no new snapshot published yet"
        );
        let cloned_snapshot = must_some(cloned.latest_parsed());
        assert_eq!(
            cloned_snapshot.generation(),
            doc_gen,
            "the clone's snapshot must be unaffected by edits on the original"
        );
    }

    #[test]
    fn ratchet_no_direct_field_access_to_removed_document_state_fields() {
        // Guardrail against regressing back to scattered direct reads of the
        // fields `ParsedSnapshot` replaced. The `parsed` field on
        // `DocumentState` is module-private (compiler-enforced already);
        // this ratchet additionally guards against a future PR widening its
        // visibility (e.g. to `pub(crate)`) and then bypassing the
        // accessors with `doc.parsed.as_ref()...` from elsewhere in the
        // crate, which would defeat the freshness invariant.
        let manifest_dir = must_some(std::env::var("CARGO_MANIFEST_DIR").ok());
        let src_dir = std::path::Path::new(&manifest_dir).join("src");
        let this_file = std::path::Path::new(file!())
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("document.rs");

        let mut offenders = Vec::new();
        for entry in walkdir::WalkDir::new(&src_dir).into_iter().filter_map(Result::ok) {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("rs") {
                continue;
            }
            // document.rs (this file) defines `parsed` and is exempt.
            let is_this_file = path.file_name().and_then(|n| n.to_str()) == Some(this_file)
                && path.parent().and_then(|p| p.file_name()).and_then(|n| n.to_str())
                    == Some("state");
            if is_this_file {
                continue;
            }
            let Ok(contents) = std::fs::read_to_string(path) else {
                continue;
            };
            for (idx, line) in contents.lines().enumerate() {
                // `.parsed` is the only field DocumentState now has for
                // parsed state; direct access to it outside this module
                // would only compile if visibility were widened, but a
                // textual match here catches that at review time even
                // before such a widening would need to happen.
                let mentions_parsed = line.contains(".parsed") && !line.contains(".parsed_range");
                let via_accessor = line.contains("current_parsed")
                    || line.contains("latest_parsed")
                    || line.contains("publish_parsed_if_current");
                if mentions_parsed && !via_accessor {
                    offenders.push(format!("{}:{}: {}", path.display(), idx + 1, line.trim()));
                }
            }
        }

        assert!(
            offenders.is_empty(),
            "found direct `.parsed` field access outside DocumentState's accessors:\n{}",
            offenders.join("\n")
        );
    }
}

/// Normalize legacy package separator ' to ::
pub fn normalize_package_separator(s: &str) -> Cow<'_, str> {
    perl_module::path::normalize_package_separator(s)
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
    /// Supports dynamic registration for file watching only.
    ///
    /// Other dynamically registered LSP features must use their own client
    /// capability fields; this flag is deliberately forced off for some
    /// clients without disabling unrelated dynamic registrations.
    pub dynamic_registration_support: bool,
    /// Supports `RelativePattern` objects in dynamic file watcher registrations.
    ///
    /// Parsed from `capabilities.workspace.didChangeWatchedFiles.relativePatternSupport`.
    /// When false, watcher registrations must keep the existing string glob shape.
    pub file_watcher_relative_pattern_support: bool,
    /// Client declared textDocument/inlineCompletion capability
    pub inline_completion_support: bool,
    /// Supports dynamic registration for textDocument/inlineCompletion
    pub inline_completion_dynamic_registration_support: bool,
    /// Supports `workspace/configuration` reverse requests from server.
    pub workspace_configuration_support: bool,
    /// Supports server-originated `workspace/applyEdit` requests.
    ///
    /// Parsed from `capabilities.workspace.applyEdit`.
    pub workspace_apply_edit_support: bool,
    /// Supports `workspaceFolders` capability negotiation/events.
    pub workspace_folders_support: bool,
    /// Supports snippet syntax in completion items
    pub snippet_support: bool,
    /// Supports versioned document edits in `WorkspaceEdit`s.
    ///
    /// Parsed from `capabilities.workspace.workspaceEdit.documentChanges`.
    /// `SnippetTextEdit` can only be emitted inside document changes.
    pub workspace_edit_document_changes_support: bool,
    /// Supports `SnippetTextEdit` workspace edits (LSP 3.18).
    ///
    /// Parsed from `capabilities.workspace.workspaceEdit.snippetEditSupport`.
    /// Unsupported clients must keep receiving plain `TextEdit`s.
    pub workspace_edit_snippet_edit_support: bool,
    /// Supports `ApplyWorkspaceEditParams.metadata` on server-originated edits (LSP 3.18).
    ///
    /// Parsed from `capabilities.workspace.workspaceEdit.metadataSupport`.
    /// Metadata must not be emitted on ordinary `WorkspaceEdit` responses.
    pub workspace_edit_metadata_support: bool,
    /// Supports `completionItem.commitCharacters` in completion results
    pub completion_commit_characters_support: bool,
    /// Supports markup message content in pull diagnostics (LSP 3.18)
    ///
    /// When true, the server can provide `Diagnostic.message` as
    /// `MarkupContent` in pull diagnostics responses.
    pub markup_message_support: bool,
    /// Supports static documentation for classes of code actions (LSP 3.18).
    ///
    /// Parsed from `capabilities.textDocument.codeAction.documentationSupport`.
    /// When true, the server may advertise `CodeActionOptions.documentation` in
    /// `codeActionProvider`.
    pub code_action_documentation_support: bool,
    /// Supports the LSP 3.18 `CodeActionTag.LLMGenerated` tag.
    ///
    /// Parsed from `capabilities.textDocument.codeAction.tagSupport.valueSet`.
    /// Deterministic actions must remain untagged; generated actions may only
    /// emit `tags: [1]` when this is true.
    pub code_action_llm_generated_tag_support: bool,
    /// Supports workspace/codeLens/refresh request
    pub code_lens_refresh_support: bool,
    /// Supports workspace/semanticTokens/refresh request
    pub semantic_tokens_refresh_support: bool,
    /// Supports workspace/inlayHint/refresh request
    pub inlay_hint_refresh_support: bool,
    /// Client declared textDocument/inlayHint capability
    pub inlay_hint_support: bool,
    /// Properties the client can resolve via codeLens/resolve
    ///
    /// Parsed from `capabilities.textDocument.codeLens.resolveSupport.properties`.
    /// The server must only defer CodeLens properties that appear here. A `None`
    /// value means the client sent no CodeLens `resolveSupport` entry.
    pub code_lens_resolve_support: Option<std::collections::HashSet<String>>,
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
    /// Properties the client can resolve via inlayHint/resolve
    ///
    /// Parsed from `capabilities.textDocument.inlayHint.resolveSupport.properties`.
    /// The server must only include a resolved property in the response when the
    /// client has declared that property here.  A `None` value means the client
    /// sent no `resolveSupport` entry at all (i.e. resolve is unsupported).
    pub inlay_hint_resolve_support: Option<std::collections::HashSet<String>>,
    /// Client supports `completionItem.labelDetails` (LSP 3.17+).
    ///
    /// When true the server may include a `labelDetails` object in completion
    /// items and in `completionItem/resolve` responses.
    pub label_details_support: bool,
    /// Client supports `CompletionList.itemDefaults.data` (LSP 3.18).
    ///
    /// Parsed from `capabilities.textDocument.completion.completionList.itemDefaults`.
    /// When true, completion responses may include shared `itemDefaults.data`
    /// for clients that understand completion-list default item data.
    pub completion_list_item_defaults_data_support: bool,
    /// Client supports `CompletionList.applyKind` (LSP 3.18).
    ///
    /// Parsed from `capabilities.textDocument.completion.completionList.applyKindSupport`.
    /// When true, completion responses may describe how supported item defaults
    /// combine with per-item fields.
    pub completion_list_apply_kind_support: bool,
    /// Negotiated position encoding per LSP 3.17 spec.
    ///
    /// Parsed from `capabilities.general.positionEncodings` - the server picks
    /// the first encoding from the client's list that it supports, or defaults
    /// to UTF-16 if the list is empty or missing.
    pub position_encoding: crate::textdoc::PosEnc,
}
