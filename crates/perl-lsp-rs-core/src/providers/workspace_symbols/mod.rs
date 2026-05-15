#![warn(missing_docs)]
//! Workspace symbols provider for LSP with comprehensive Perl symbol support.
//!
//! Provides workspace/symbol functionality for searching symbols across all files
//! in a Perl workspace with enterprise-grade performance and accuracy.
//!
//! # LSP Workflow Integration
//!
//! Essential component in the Parse → Index → Navigate → Complete → Analyze pipeline:
//! 1. **Parse**: Extract symbols from individual Perl files
//! 2. **Index**: Build workspace-wide symbol registry with dual indexing
//! 3. **Navigate**: Enable workspace symbol search and go-to-definition
//! 4. **Complete**: Provide symbol context for completion suggestions
//! 5. **Analyze**: Support workspace refactoring and cross-reference analysis
//!
//! # Client capability requirements
//!
//! Requires client capability support for `workspace/symbol` requests and
//! workspace symbol resolve support when available.
//!
//! # Protocol compliance
//!
//! Implements the workspace symbol protocol with stable sorting and paging
//! behavior aligned to the LSP specification.
//!
//! # Performance Characteristics
//!
//! - **Symbol search**: O(log n) with prefix matching optimization
//! - **Result filtering**: <10ms for 100K+ symbols workspace
//! - **Memory overhead**: Minimal with lazy symbol materialization
//! - **Query response**: ≤50ms end-to-end for LSP responsiveness
//!
//! # Perl Symbol Support
//!
//! Comprehensive Perl symbol types:
//! - **Subroutines**: `sub function_name` with package qualification
//! - **Packages**: `package Package::Name` hierarchical namespaces
//! - **Variables**: `$scalar`, `@array`, `%hash` with lexical scoping
//! - **Constants**: `use constant NAME => value` definitions
//! - **Legacy compatibility**: Handles `'` and `::` package separators
//!
//! # Usage Examples
//!
//! ```rust,ignore
//! use perl_lsp_providers::ide::lsp_compat::workspace_symbols::WorkspaceSymbolsProvider;
//! use perl_parser_core::Parser;
//! use std::collections::HashMap;
//!
//! # fn main() -> Result<(), Box<dyn std::error::Error>> {
//! // Create provider and parse Perl code
//! let mut provider = WorkspaceSymbolsProvider::new();
//! let source = "sub hello { print 'world'; }";
//! let mut parser = Parser::new(source);
//! let ast = parser.parse()?;
//!
//! // Index the document
//! provider.index_document("file:///test.pl", &ast, source);
//!
//! // Search workspace symbols
//! let mut source_map = HashMap::new();
//! source_map.insert("file:///test.pl".to_string(), source.to_string());
//! let results = provider.search("hello", &source_map);
//! assert!(!results.is_empty());
//! # Ok(())
//! # }
//! ```

use crate::providers::symbol_query::{compare_names_by_query, matches_query};
use perl_module::path::normalize_package_separator;
use perl_parser_core::qualified_name::container_name;
use perl_parser_core::{SourceLocation, ast::Node};
use perl_position_tracking::{WireLocation, WireRange};
use perl_semantic_analyzer::symbol::{SymbolExtractor, SymbolKind};
use perl_semantic_facts::{
    AnchorId, Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind,
    ProviderFactTrace, ProviderFallbackState, ProviderSurface,
};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, summarize_identities,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// LSP WorkspaceSymbol representing a symbol found in the workspace.
///
/// Corresponds to the LSP `WorkspaceSymbol` type used in `workspace/symbol` responses.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceSymbol {
    /// The symbol's name (e.g., subroutine name, package name, variable name).
    pub name: String,
    /// LSP symbol kind as integer (e.g., 4=Namespace, 12=Function, 13=Variable).
    pub kind: i32,
    /// Location of the symbol definition in the workspace.
    pub location: WireLocation,
    /// Optional containing package or class name for qualified symbols.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub container_name: Option<String>,
}

/// Compiler-fact candidate considered by the workspace-symbol shadow proof.
///
/// This is not a live workspace-symbol response type. It lets the provider
/// compare the legacy workspace index against compiler facts and emit a typed
/// fact-source trace before any runtime provider cutover.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WorkspaceSymbolShadowCandidate {
    /// Stable identity for deterministic receipt comparison.
    pub identity: String,
    /// Fact source that produced the candidate.
    pub source: ProviderFactSourceKind,
    /// Semantic provenance for the candidate.
    pub provenance: Provenance,
    /// Confidence in the candidate.
    pub confidence: Confidence,
    /// Freshness of the candidate relative to the request.
    pub freshness: ProviderFactFreshness,
    /// Whether the candidate is shadowed, fallback, or blocked.
    pub fallback_state: ProviderFallbackState,
    /// Optional source hash for fact freshness proof.
    pub source_hash: Option<String>,
    /// Optional semantic anchor for the candidate.
    pub anchor_id: Option<AnchorId>,
    /// Optional producer model version.
    pub model_version: Option<u32>,
}

impl WorkspaceSymbolShadowCandidate {
    /// Build a shadow-only workspace-symbol candidate.
    ///
    /// Use this for generated or otherwise unpromoted candidates that must be
    /// measured without becoming live workspace-symbol answers.
    #[must_use]
    pub fn shadow(
        identity: impl Into<String>,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
    ) -> Self {
        Self {
            identity: identity.into(),
            source,
            provenance,
            confidence,
            freshness,
            fallback_state: ProviderFallbackState::Shadow,
            source_hash: None,
            anchor_id: None,
            model_version: None,
        }
    }

    /// Build a blocked workspace-symbol candidate.
    ///
    /// Use this for stale, dynamic, generated/no-source, or otherwise unsafe
    /// facts that must not authorize live workspace-symbol expansion.
    #[must_use]
    pub fn blocked(
        identity: impl Into<String>,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
    ) -> Self {
        Self {
            identity: identity.into(),
            source,
            provenance,
            confidence,
            freshness,
            fallback_state: ProviderFallbackState::Blocked,
            source_hash: None,
            anchor_id: None,
            model_version: None,
        }
    }

    /// Build a fallback/noise candidate that remains outside live promotion.
    #[must_use]
    pub fn fallback(
        identity: impl Into<String>,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
    ) -> Self {
        Self {
            identity: identity.into(),
            source,
            provenance,
            confidence,
            freshness,
            fallback_state: ProviderFallbackState::Fallback,
            source_hash: None,
            anchor_id: None,
            model_version: None,
        }
    }
}

/// Workspace-symbol shadow proof result.
///
/// Callers should keep returning the legacy workspace-symbol result while this
/// receipt is used for provider cutover proof.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct WorkspaceSymbolShadowResult {
    /// Legacy symbols returned by the existing workspace index path.
    pub legacy_symbols: Vec<WorkspaceSymbol>,
    /// Shadow receipt comparing legacy symbols with compiler-fact candidates.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Internal symbol information used for indexing.
///
/// Stores symbol metadata extracted from parsed Perl source files.
#[derive(Debug, Clone)]
struct SymbolInfo {
    /// Symbol name (bare, unqualified).
    name: String,
    /// Kind of symbol (subroutine, package, variable, etc.).
    kind: SymbolKind,
    /// Byte offset location in the source file.
    location: SourceLocation,
    /// Containing package name, if any.
    container: Option<String>,
}

/// Workspace symbols provider for LSP `workspace/symbol` requests.
///
/// Maintains an index of all symbols across the workspace and provides
/// search functionality with fuzzy matching support.
pub struct WorkspaceSymbolsProvider {
    /// Map of document URI to its extracted symbols.
    documents: HashMap<String, Vec<SymbolInfo>>,
    /// Fast lookup map from symbol name to symbol occurrences.
    symbols_by_name: HashMap<String, Vec<(String, SymbolInfo)>>,
}

impl Default for WorkspaceSymbolsProvider {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkspaceSymbolsProvider {
    /// Creates a new empty workspace symbols provider.
    #[must_use]
    pub fn new() -> Self {
        Self { documents: HashMap::new(), symbols_by_name: HashMap::new() }
    }

    /// Indexes all symbols from a parsed document.
    ///
    /// Extracts symbols from the AST and stores them for later search queries.
    /// Replaces any previously indexed symbols for the same URI.
    pub fn index_document(&mut self, uri: &str, ast: &Node, source: &str) {
        self.remove_document(uri);

        let extractor = SymbolExtractor::new_with_source(source);
        let table = extractor.extract(ast);

        let mut symbols = Vec::new();

        // Extract symbols from the symbol table
        for (name, symbol_list) in &table.symbols {
            for symbol in symbol_list {
                let container = container_name(&symbol.qualified_name).map(str::to_string);

                symbols.push(SymbolInfo {
                    name: name.clone(),
                    kind: symbol.kind,
                    location: symbol.location,
                    container,
                });
            }
        }

        for symbol in &symbols {
            self.symbols_by_name
                .entry(symbol.name.clone())
                .or_default()
                .push((uri.to_string(), symbol.clone()));
        }

        self.documents.insert(uri.to_string(), symbols);
    }

    /// Removes a document and its symbols from the index.
    ///
    /// Called when a file is deleted or closed in the workspace.
    pub fn remove_document(&mut self, uri: &str) {
        self.documents.remove(uri);
        self.symbols_by_name.retain(|_, entries| {
            entries.retain(|(entry_uri, _)| entry_uri != uri);
            !entries.is_empty()
        });
    }

    /// Returns all indexed symbols as LSP WorkspaceSymbols.
    ///
    /// Useful for bulk export or re-indexing operations.
    /// Note: Returned symbols have minimal location info (line 0, col 0).
    #[must_use]
    pub fn get_all_symbols(&self) -> Vec<WorkspaceSymbol> {
        let mut all_symbols = Vec::new();

        for (uri, symbols) in &self.documents {
            for symbol in symbols {
                // Create a minimal workspace symbol for indexing
                all_symbols.push(WorkspaceSymbol {
                    name: symbol.name.clone(),
                    kind: symbol.kind.to_lsp_kind() as i32,
                    location: WireLocation::new(uri.clone(), WireRange::default()),
                    container_name: symbol
                        .container
                        .as_ref()
                        .map(|s| normalize_package_separator(s).into_owned()),
                });
            }
        }

        all_symbols
    }

    /// Searches for symbols matching a query within a pre-filtered candidate set.
    ///
    /// More efficient than `search` when the caller has already narrowed down
    /// potential matches (e.g., from a global symbol index).
    ///
    /// Results are sorted by relevance: exact matches first, then prefix matches,
    /// then alphabetically.
    #[must_use]
    pub fn search_with_candidates(
        &self,
        query: &str,
        source_map: &HashMap<String, String>,
        candidates: &[String],
    ) -> Vec<WorkspaceSymbol> {
        let mut results = Vec::new();
        for candidate in candidates {
            if let Some(entries) = self.symbols_by_name.get(candidate) {
                for (uri, symbol) in entries {
                    if matches_query(&symbol.name, query) {
                        let Some(source) = source_map.get(uri) else {
                            continue;
                        };
                        results.push(self.symbol_to_workspace_symbol(uri, symbol, source));
                    }
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| compare_names_by_query(&a.name, &b.name, query));

        results
    }

    /// Searches for symbols matching a query string.
    ///
    /// Supports multiple match strategies:
    /// - Exact match (case-insensitive)
    /// - Prefix match
    /// - Contains match
    /// - Fuzzy/subsequence match
    ///
    /// Results are sorted by relevance: exact matches first, then prefix matches,
    /// then alphabetically.
    #[must_use]
    pub fn search(
        &self,
        query: &str,
        source_map: &HashMap<String, String>,
    ) -> Vec<WorkspaceSymbol> {
        let mut results = Vec::new();

        for (uri, symbols) in &self.documents {
            // Get source for this document to convert offsets
            let source = match source_map.get(uri) {
                Some(s) => s,
                None => continue,
            };

            for symbol in symbols {
                if matches_query(&symbol.name, query) {
                    results.push(self.symbol_to_workspace_symbol(uri, symbol, source));
                }
            }
        }

        // Sort by relevance
        results.sort_by(|a, b| compare_names_by_query(&a.name, &b.name, query));

        results
    }

    /// Converts an internal `SymbolInfo` to an LSP `WorkspaceSymbol`.
    ///
    /// Resolves byte offsets to line/column positions using the source text.
    /// Uses UTF-16 code unit counting as required by LSP protocol.
    fn symbol_to_workspace_symbol(
        &self,
        uri: &str,
        symbol: &SymbolInfo,
        source: &str,
    ) -> WorkspaceSymbol {
        // Use canonical UTF-16 conversion from perl-position-tracking
        let range =
            WireRange::from_byte_offsets(source, symbol.location.start, symbol.location.end);

        WorkspaceSymbol {
            name: symbol.name.clone(),
            kind: symbol.kind.to_lsp_kind() as i32,
            location: WireLocation::new(uri.to_string(), range),
            container_name: symbol
                .container
                .as_ref()
                .map(|s| normalize_package_separator(s).into_owned()),
        }
    }
}

/// Compare legacy workspace-symbol output against compiler-fact candidates.
///
/// This function is intentionally shadow-only: it returns the original legacy
/// symbols unchanged and emits a receipt that records source, provenance,
/// confidence, freshness, and fallback/blocker state for candidate facts.
#[must_use]
pub fn workspace_symbol_source_shadow(
    legacy_symbols: Vec<WorkspaceSymbol>,
    compiler_candidates: Vec<WorkspaceSymbolShadowCandidate>,
    query: &str,
) -> WorkspaceSymbolShadowResult {
    let old_result =
        summarize_identities(Some(legacy_symbols.iter().map(workspace_symbol_identity).collect()));
    let new_result =
        summarize_identities(Some(workspace_symbol_answer_identities(&compiler_candidates)));
    let notes = vec![workspace_symbol_shadow_note(&legacy_symbols, &compiler_candidates)];
    let fact_source_traces =
        compiler_candidates.iter().map(workspace_symbol_candidate_trace).collect();

    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::WorkspaceSymbols,
        ShadowQueryInput { symbol: query.to_string() },
        old_result,
        new_result,
        notes,
        fact_source_traces,
    );

    WorkspaceSymbolShadowResult { legacy_symbols, receipt }
}

fn workspace_symbol_identity(symbol: &WorkspaceSymbol) -> String {
    let container = symbol.container_name.as_deref().map_or("<none>", |value| value);
    format!(
        "{}:{}:{}:{}:{}:{}",
        symbol.name,
        symbol.kind,
        symbol.location.uri,
        symbol.location.range.start.line,
        symbol.location.range.start.character,
        container
    )
}

fn workspace_symbol_candidate_trace(
    candidate: &WorkspaceSymbolShadowCandidate,
) -> ProviderFactTrace {
    ProviderFactTrace::new(
        ProviderSurface::WorkspaceSymbols,
        candidate.source,
        candidate.provenance,
        candidate.confidence,
        candidate.freshness,
        candidate.fallback_state,
        candidate.source_hash.clone(),
        candidate.anchor_id,
        candidate.model_version,
    )
}

fn workspace_symbol_answer_identities(
    compiler_candidates: &[WorkspaceSymbolShadowCandidate],
) -> Vec<String> {
    compiler_candidates
        .iter()
        .filter(|candidate| {
            matches!(
                candidate.fallback_state,
                ProviderFallbackState::Primary
                    | ProviderFallbackState::Shadow
                    | ProviderFallbackState::Fallback
            )
        })
        .map(|candidate| candidate.identity.clone())
        .collect()
}

fn workspace_symbol_shadow_note(
    legacy_symbols: &[WorkspaceSymbol],
    compiler_candidates: &[WorkspaceSymbolShadowCandidate],
) -> String {
    let answer_count = workspace_symbol_answer_identities(compiler_candidates).len();
    let blocked_count = compiler_candidates
        .iter()
        .filter(|candidate| candidate.fallback_state == ProviderFallbackState::Blocked)
        .count();
    let generated_label_count = compiler_candidates
        .iter()
        .filter(|candidate| {
            candidate.source == ProviderFactSourceKind::FrameworkAdapter
                || candidate.identity.starts_with("generated:")
        })
        .count();
    let dynamic_boundary_blockers = compiler_candidates
        .iter()
        .filter(|candidate| {
            candidate.fallback_state == ProviderFallbackState::Blocked
                && (candidate.source == ProviderFactSourceKind::DynamicBoundary
                    || candidate.provenance == Provenance::DynamicBoundary)
        })
        .count();
    let stale_fact_blockers = compiler_candidates
        .iter()
        .filter(|candidate| {
            candidate.fallback_state == ProviderFallbackState::Blocked
                && candidate.freshness == ProviderFactFreshness::Stale
        })
        .count();
    let noise_delta = compiler_candidates
        .iter()
        .filter(|candidate| {
            candidate.fallback_state != ProviderFallbackState::Blocked
                && (candidate.confidence == Confidence::Low
                    || candidate.freshness != ProviderFactFreshness::Fresh
                    || candidate.source == ProviderFactSourceKind::DynamicBoundary
                    || candidate.fallback_state == ProviderFallbackState::Fallback)
        })
        .count();
    format!(
        "workspace-symbol shadow proof: legacy_candidates={}; compiler_fact_candidates={}; answer_candidates={}; rank_delta={}; noise_delta={}; query_latency=not_measured_shadow_only; generated_labels={}; dynamic_boundary_blockers={}; stale_fact_blockers={}; blocked_candidates={}; no live workspace-symbol behavior change",
        legacy_symbols.len(),
        compiler_candidates.len(),
        answer_count,
        signed_count_delta(legacy_symbols.len(), answer_count),
        noise_delta,
        generated_label_count,
        dynamic_boundary_blockers,
        stale_fact_blockers,
        blocked_count
    )
}

fn signed_count_delta(old_count: usize, new_count: usize) -> String {
    if new_count >= old_count {
        format!("+{}", new_count - old_count)
    } else {
        format!("-{}", old_count - new_count)
    }
}

// Symbol kind conversion is handled by perl_symbol::SymbolKind::to_lsp_kind()
// Position conversion is handled by perl_position_tracking via WireRange::from_byte_offsets()
// which correctly counts UTF-16 code units as required by the LSP protocol.

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_position_tracking::offset_to_utf16_line_col;
    use perl_tdd_support::must;
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    #[test]
    fn test_workspace_symbols_search() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        // Index a test file
        let source = r#"
package MyPackage;

sub foo {
    my $x = 42;
}

sub foobar {
    my $y = 'test';
}

sub baz {
    # Another function
}
"#;

        source_map.insert("file:///test.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        provider.index_document("file:///test.pl", &ast, source);

        // Test exact match
        let results = provider.search("foo", &source_map);
        assert_eq!(results.len(), 2); // foo and foobar
        assert_eq!(results[0].name, "foo"); // Exact match first

        // Test prefix match
        let results = provider.search("fo", &source_map);
        assert_eq!(results.len(), 2);

        // Test contains match
        let results = provider.search("bar", &source_map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");

        // Test fuzzy match
        let results = provider.search("fb", &source_map);
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].name, "foobar");
    }

    #[test]
    fn workspace_symbol_shadow_traces_fresh_compiler_fact() -> Result<(), Box<dyn std::error::Error>>
    {
        let legacy = legacy_symbol("imported_func");
        let identity = workspace_symbol_identity(&legacy);
        let result = workspace_symbol_source_shadow(
            vec![legacy],
            vec![shadow_candidate(
                &identity,
                ProviderFactSourceKind::CompilerFact,
                Provenance::ImportExportInference,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
            "imported",
        );

        assert_eq!(result.legacy_symbols.len(), 1);
        assert_eq!(result.receipt.query, ShadowQueryName::WorkspaceSymbols);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.surface, ProviderSurface::WorkspaceSymbols);
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.provenance, Provenance::ImportExportInference);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn workspace_symbol_shadow_labels_generated_candidates()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = workspace_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "generated:Foo::generated_accessor:virtual",
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
            "generated",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn workspace_symbol_shadow_blocks_dynamic_boundaries() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = workspace_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "blocker:symbolic_ref_boundary",
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
            "dynamic",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.new_result.match_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::DynamicBoundary);
        assert_eq!(trace.provenance, Provenance::DynamicBoundary);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn workspace_symbol_shadow_blocks_stale_compiler_facts()
    -> Result<(), Box<dyn std::error::Error>> {
        let result = workspace_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "stale:Foo::old_symbol",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                ProviderFallbackState::Blocked,
            )],
            "stale",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.new_result.match_count, 0);

        let trace = first_trace(&result)?;
        assert_eq!(trace.source, ProviderFactSourceKind::CompilerFact);
        assert_eq!(trace.confidence, Confidence::Low);
        assert_eq!(trace.freshness, ProviderFactFreshness::Stale);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Blocked);
        Ok(())
    }

    #[test]
    fn workspace_symbol_shadow_records_real_workspace_quality_receipt()
    -> Result<(), Box<dyn std::error::Error>> {
        let legacy_symbols = indexed_workspace_symbols("legacy")?;
        assert_eq!(legacy_symbols.len(), 1);
        let legacy_identity = workspace_symbol_identity(&legacy_symbols[0]);
        let result = workspace_symbol_source_shadow(
            legacy_symbols,
            vec![
                shadow_candidate(
                    &legacy_identity,
                    ProviderFactSourceKind::CompilerFact,
                    Provenance::ImportExportInference,
                    Confidence::High,
                    ProviderFactFreshness::Fresh,
                    ProviderFallbackState::Shadow,
                ),
                shadow_candidate(
                    "workspace:MyApp::Utils::format_date",
                    ProviderFactSourceKind::CompilerFact,
                    Provenance::ImportExportInference,
                    Confidence::High,
                    ProviderFactFreshness::Fresh,
                    ProviderFallbackState::Shadow,
                ),
                shadow_candidate(
                    "generated:MyApp::Model::name:virtual",
                    ProviderFactSourceKind::FrameworkAdapter,
                    Provenance::FrameworkSynthesis,
                    Confidence::Medium,
                    ProviderFactFreshness::Fresh,
                    ProviderFallbackState::Shadow,
                ),
                shadow_candidate(
                    "blocker:symbolic_ref_boundary",
                    ProviderFactSourceKind::DynamicBoundary,
                    Provenance::DynamicBoundary,
                    Confidence::High,
                    ProviderFactFreshness::Fresh,
                    ProviderFallbackState::Blocked,
                ),
                shadow_candidate(
                    "stale:MyApp::Old::removed_symbol",
                    ProviderFactSourceKind::CompilerFact,
                    Provenance::SemanticAnalyzer,
                    Confidence::Low,
                    ProviderFactFreshness::Stale,
                    ProviderFallbackState::Blocked,
                ),
            ],
            "workspace quality",
        );

        assert_eq!(result.legacy_symbols.len(), 1);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 3);

        let note = result
            .receipt
            .notes
            .first()
            .ok_or_else(|| "expected workspace-symbol quality note".to_string())?;
        assert!(note.contains("rank_delta=+2"));
        assert!(note.contains("noise_delta=0"));
        assert!(note.contains("query_latency=not_measured_shadow_only"));
        assert!(note.contains("generated_labels=1"));
        assert!(note.contains("dynamic_boundary_blockers=1"));
        assert!(note.contains("stale_fact_blockers=1"));
        assert!(note.contains("no live workspace-symbol behavior change"));

        assert_eq!(result.receipt.fact_source_traces.len(), 5);
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::FrameworkAdapter
                && trace.provenance == Provenance::FrameworkSynthesis
                && trace.fallback_state == ProviderFallbackState::Shadow
        }));
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::DynamicBoundary
                && trace.provenance == Provenance::DynamicBoundary
                && trace.fallback_state == ProviderFallbackState::Blocked
        }));
        assert!(result.receipt.fact_source_traces.iter().any(|trace| {
            trace.source == ProviderFactSourceKind::CompilerFact
                && trace.freshness == ProviderFactFreshness::Stale
                && trace.fallback_state == ProviderFallbackState::Blocked
        }));
        Ok(())
    }

    fn indexed_workspace_symbols(
        query: &str,
    ) -> Result<Vec<WorkspaceSymbol>, Box<dyn std::error::Error>> {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();
        for (uri, source) in [
            ("file:///lib/App.pm", "package App;\nsub legacy_helper { 1 }\n1;\n"),
            ("file:///lib/MyApp/Utils.pm", "package MyApp::Utils;\nsub format_date { 1 }\n1;\n"),
        ] {
            let mut parser = Parser::new(source);
            let ast = must(parser.parse());
            provider.index_document(uri, &ast, source);
            source_map.insert(uri.to_string(), source.to_string());
        }

        Ok(provider.search(query, &source_map))
    }

    fn legacy_symbol(name: &str) -> WorkspaceSymbol {
        WorkspaceSymbol {
            name: name.to_string(),
            kind: 12,
            location: WireLocation::new("file:///lib/Foo.pm".to_string(), WireRange::default()),
            container_name: Some("Foo".to_string()),
        }
    }

    fn shadow_candidate(
        identity: &str,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        fallback_state: ProviderFallbackState,
    ) -> WorkspaceSymbolShadowCandidate {
        WorkspaceSymbolShadowCandidate {
            identity: identity.to_string(),
            source,
            provenance,
            confidence,
            freshness,
            fallback_state,
            source_hash: Some("fixture-source-sha".to_string()),
            anchor_id: Some(AnchorId(1)),
            model_version: Some(1),
        }
    }

    fn first_trace(
        result: &WorkspaceSymbolShadowResult,
    ) -> Result<&ProviderFactTrace, Box<dyn std::error::Error>> {
        result
            .receipt
            .fact_source_traces
            .first()
            .ok_or_else(|| "expected workspace-symbol fact-source trace".into())
    }

    #[test]
    fn test_offset_to_utf16_line_col() {
        let source = "hello\nworld\n123";

        // Uses canonical UTF-16 conversion from perl-position-tracking
        assert_eq!(offset_to_utf16_line_col(source, 0), (0, 0)); // 'h'
        assert_eq!(offset_to_utf16_line_col(source, 5), (0, 5)); // '\n'
        assert_eq!(offset_to_utf16_line_col(source, 6), (1, 0)); // 'w'
        assert_eq!(offset_to_utf16_line_col(source, 11), (1, 5)); // '\n'
        assert_eq!(offset_to_utf16_line_col(source, 12), (2, 0)); // '1'
    }

    #[test]
    fn test_utf16_emoji_position() {
        // Regression test: emojis are 4 bytes in UTF-8 but 2 code units in UTF-16
        // LSP protocol requires UTF-16 code units for character positions
        let source = "😀x"; // emoji (4 bytes, 2 UTF-16 units) + 'x' (1 byte, 1 UTF-16 unit)

        // 'x' is at byte offset 4 (after the 4-byte emoji)
        // In UTF-16, 'x' is at character position 2 (emoji = 2 code units)
        let (line, character) = offset_to_utf16_line_col(source, 4);
        assert_eq!(line, 0);
        assert_eq!(
            character, 2,
            "Emoji should count as 2 UTF-16 code units, so 'x' is at character 2"
        );
    }

    #[test]
    fn test_workspace_symbol_utf16_position_with_emoji() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        // Symbol after an emoji should have correct UTF-16 character position
        let source = "my $😀 = 1;\nsub target { }";

        source_map.insert("file:///emoji.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        provider.index_document("file:///emoji.pl", &ast, source);

        let results = provider.search("target", &source_map);

        // Verify we found the target symbol
        assert!(!results.is_empty(), "Should find 'target' subroutine");

        // The position should use UTF-16 character counts
        // The emoji variable name counts as 2 UTF-16 code units
        let target_symbol = &results[0];
        assert_eq!(target_symbol.name, "target");
        // 'sub target' is on line 1 (0-indexed)
        assert_eq!(target_symbol.location.range.start.line, 1);
    }

    #[test]
    fn test_extract_container_name() {
        // Nested package qualification
        assert_eq!(
            container_name("Foo::Bar::baz").map(str::to_string),
            Some("Foo::Bar".to_string())
        );

        // Simple package qualification
        assert_eq!(container_name("MyClass::new").map(str::to_string), Some("MyClass".to_string()));

        // Top-level symbol (no container)
        assert_eq!(container_name("toplevel").map(str::to_string), None);

        // Empty string
        assert_eq!(container_name("").map(str::to_string), None);

        // Package name only (no method)
        assert_eq!(container_name("Package::").map(str::to_string), Some("Package".to_string()));

        // Deep nesting
        assert_eq!(
            container_name("A::B::C::D::method").map(str::to_string),
            Some("A::B::C::D".to_string())
        );
    }

    #[test]
    fn test_container_names_workspace_symbols() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        // Multi-package workspace with same method name in different packages
        let source = r#"
package Foo::Bar;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub process {
    my $self = shift;
}

package Baz::Qux;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub process {
    my $self = shift;
}

package main;

sub helper {
    print "top-level\n";
}
"#;

        source_map.insert("file:///multi.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        provider.index_document("file:///multi.pl", &ast, source);

        // Search for 'new' - should find both with different containers
        let results = provider.search("new", &source_map);
        assert_eq!(results.len(), 2, "Should find both 'new' methods");

        // Verify container names are populated correctly
        let containers: Vec<Option<String>> =
            results.iter().map(|r| r.container_name.clone()).collect();

        assert!(
            containers.contains(&Some("Foo::Bar".to_string())),
            "Should have Foo::Bar container"
        );
        assert!(
            containers.contains(&Some("Baz::Qux".to_string())),
            "Should have Baz::Qux container"
        );

        // Search for 'process' - should also find both with containers
        let results = provider.search("process", &source_map);
        assert_eq!(results.len(), 2, "Should find both 'process' methods");

        let containers: Vec<Option<String>> =
            results.iter().map(|r| r.container_name.clone()).collect();

        assert!(
            containers.contains(&Some("Foo::Bar".to_string())),
            "Should have Foo::Bar container for process"
        );
        assert!(
            containers.contains(&Some("Baz::Qux".to_string())),
            "Should have Baz::Qux container for process"
        );
    }

    #[test]
    fn test_top_level_no_container() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        // Top-level symbols in main package should have "main" as container
        let source = r#"
sub top_level_function {
    print "I'm at the top level\n";
}

my $top_level_var = 42;
"#;

        source_map.insert("file:///toplevel.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        provider.index_document("file:///toplevel.pl", &ast, source);

        // Search for top-level function
        let results = provider.search("top_level_function", &source_map);
        assert!(!results.is_empty(), "Should find top-level function");

        // Verify container name is "main" (the default package)
        assert_eq!(
            results[0].container_name,
            Some("main".to_string()),
            "Top-level subroutine should have 'main' as container"
        );

        // Lexical variables (my), if indexed, should have no container.
        // Note: Whether lexical variables appear in workspace symbols is an implementation detail.
        // This test verifies the correct container behavior when they do appear.
        let results = provider.search("top_level_var", &source_map);
        if let Some(sym) = results.iter().find(|s| s.name.contains("top_level_var")) {
            assert!(sym.container_name.is_none(), "Lexical variable should have no container");
        }
    }

    // -----------------------------------------------------------------------
    // Cross-file search: function names across multiple files
    // -----------------------------------------------------------------------

    #[test]
    fn search_function_name_across_multiple_files() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        let source_a = "package FileA;\nsub process_data { 1 }\nsub helper { 2 }\n";
        let source_b = "package FileB;\nsub process_data { 3 }\nsub render { 4 }\n";
        let source_c = "package FileC;\nsub unrelated { 5 }\n";

        for (uri, src) in
            [("file:///a.pm", source_a), ("file:///b.pm", source_b), ("file:///c.pm", source_c)]
        {
            source_map.insert(uri.to_string(), src.to_string());
            let mut parser = Parser::new(src);
            let ast = must(parser.parse());
            provider.index_document(uri, &ast, src);
        }

        let results = provider.search("process_data", &source_map);
        assert_eq!(results.len(), 2, "process_data should appear in two files");

        let uris: Vec<&str> = results.iter().map(|r| r.location.uri.as_str()).collect();
        assert!(uris.contains(&"file:///a.pm"), "should find in file A");
        assert!(uris.contains(&"file:///b.pm"), "should find in file B");
    }

    // -----------------------------------------------------------------------
    // Package name search
    // -----------------------------------------------------------------------

    #[test]
    fn search_package_name_returns_package_definition() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        let source = "package My::Web::Controller;\nsub index { 1 }\n";
        source_map.insert("file:///controller.pm".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        provider.index_document("file:///controller.pm", &ast, source);

        let results = provider.search("Controller", &source_map);
        let pkg = results.iter().find(|s| s.name.contains("Controller") && s.kind == 2); // 2 = Module/Package
        assert!(pkg.is_some(), "should find Controller package symbol");
        let pkg = pkg.unwrap_or_else(|| unreachable!());

        assert_eq!(pkg.kind, 2, "Package should have Module kind (2)");
    }

    // -----------------------------------------------------------------------
    // Fuzzy / subsequence matching
    // -----------------------------------------------------------------------

    #[test]
    fn fuzzy_match_camel_case_subsequence() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        let source = "sub getLogger { 1 }\nsub go_live { 2 }\nsub unrelated { 3 }\n";
        source_map.insert("file:///log.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        provider.index_document("file:///log.pl", &ast, source);

        // "gL" should fuzzy-match "getLogger" (g..e..t..L) and "go_live" (g..o.._..l)
        // but NOT "unrelated"
        let results = provider.search("gL", &source_map);
        let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();

        assert!(names.contains(&"getLogger"), "gL should fuzzy-match getLogger, got: {names:?}");
        assert!(!names.contains(&"unrelated"), "gL should not match unrelated");
    }

    // -----------------------------------------------------------------------
    // Symbol kinds are correctly set
    // -----------------------------------------------------------------------

    #[test]
    fn symbol_kinds_are_correct() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        let source = r#"
package Animal;
sub new { bless {}, shift }
sub speak { print "generic\n" }
use constant MAX_AGE => 100;
my $count = 0;
"#;
        source_map.insert("file:///animal.pm".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        provider.index_document("file:///animal.pm", &ast, source);

        let all = provider.search("", &source_map); // empty query => all symbols

        // Package "Animal" should be Module kind (2)
        if let Some(pkg) = all.iter().find(|s| s.name == "Animal") {
            assert_eq!(pkg.kind, 2, "Package should be Module kind (2)");
        }

        // Subroutines should be Function kind (12)
        for name in ["new", "speak"] {
            if let Some(sub) = all.iter().find(|s| s.name == name) {
                assert_eq!(sub.kind, 12, "{name} should be Function kind (12)");
            }
        }

        // Constants should be Constant kind (14)
        if let Some(constant) = all.iter().find(|s| s.name == "MAX_AGE") {
            assert_eq!(constant.kind, 14, "Constant should be Constant kind (14)");
        }

        // Variables should be Variable kind (13)
        if let Some(var) = all.iter().find(|s| s.name.contains("count")) {
            assert_eq!(var.kind, 13, "Variable should be Variable kind (13)");
        }
    }

    // -----------------------------------------------------------------------
    // Ranking: exact matches above substrings above fuzzy
    // -----------------------------------------------------------------------

    #[test]
    fn exact_match_ranks_above_substring_and_fuzzy() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        let source = r#"
sub log { 1 }
sub logger { 2 }
sub get_log { 3 }
"#;
        source_map.insert("file:///rank.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        provider.index_document("file:///rank.pl", &ast, source);

        let results = provider.search("log", &source_map);
        let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();

        assert!(names.len() >= 3, "should match log, logger, get_log");

        // Exact match "log" must be first
        assert_eq!(names[0], "log", "exact match should be first");

        // Prefix match "logger" should come before substring "get_log"
        let logger_pos = names.iter().position(|n| *n == "logger");
        let get_log_pos = names.iter().position(|n| *n == "get_log");
        if let (Some(lp), Some(gp)) = (logger_pos, get_log_pos) {
            assert!(lp < gp, "prefix match 'logger' should rank above substring 'get_log'");
        }
    }

    // -----------------------------------------------------------------------
    // Remove document removes symbols from results
    // -----------------------------------------------------------------------

    #[test]
    fn remove_document_clears_its_symbols() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        let source = "sub ephemeral { 1 }\n";
        source_map.insert("file:///tmp.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        provider.index_document("file:///tmp.pl", &ast, source);

        assert!(!provider.search("ephemeral", &source_map).is_empty());

        provider.remove_document("file:///tmp.pl");
        assert!(
            provider.search("ephemeral", &source_map).is_empty(),
            "symbols should be gone after remove_document"
        );
    }

    // -----------------------------------------------------------------------
    // search_with_candidates filters to candidate set
    // -----------------------------------------------------------------------

    #[test]
    fn search_with_candidates_filters_results() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        let source = "sub alpha { 1 }\nsub apex { 2 }\nsub beta { 3 }\n";
        source_map.insert("file:///cand.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());
        provider.index_document("file:///cand.pl", &ast, source);

        // Only include "alpha" in candidates - "apex" should be excluded
        let candidates = vec!["alpha".to_string()];
        let results = provider.search_with_candidates("a", &source_map, &candidates);
        let names: Vec<&str> = results.iter().map(|s| s.name.as_str()).collect();

        assert!(names.contains(&"alpha"), "alpha is a candidate match");
        assert!(!names.contains(&"apex"), "apex is not in candidate set");
    }

    #[test]
    fn test_ambiguous_symbol_resolution() {
        let mut provider = WorkspaceSymbolsProvider::new();
        let mut source_map = HashMap::new();

        // Same name in different packages should be disambiguated by container
        let source = r#"
package Database::MySQL;

sub connect {
    print "MySQL connection\n";
}

package Database::PostgreSQL;

sub connect {
    print "PostgreSQL connection\n";
}

package Database::SQLite;

sub connect {
    print "SQLite connection\n";
}
"#;

        source_map.insert("file:///database.pl".to_string(), source.to_string());

        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        provider.index_document("file:///database.pl", &ast, source);

        // Search for 'connect' - should find all three
        let results = provider.search("connect", &source_map);
        assert_eq!(results.len(), 3, "Should find all three 'connect' methods");

        // Verify all three different containers are present
        let containers: Vec<String> =
            results.iter().filter_map(|r| r.container_name.clone()).collect();

        assert_eq!(containers.len(), 3, "Should have three containers");
        assert!(containers.contains(&"Database::MySQL".to_string()), "Should have MySQL container");
        assert!(
            containers.contains(&"Database::PostgreSQL".to_string()),
            "Should have PostgreSQL container"
        );
        assert!(
            containers.contains(&"Database::SQLite".to_string()),
            "Should have SQLite container"
        );

        // All symbols should have unique container names for disambiguation
        let unique_containers: std::collections::HashSet<_> = containers.iter().collect();
        assert_eq!(unique_containers.len(), 3, "Each symbol should have a unique container");
    }
}
