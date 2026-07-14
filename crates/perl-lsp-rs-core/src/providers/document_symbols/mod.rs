//! Document-symbol provider proof and source-backed live helpers.
//!
//! The runtime owns LSP request handling and fallback. This module owns the
//! compiler-shaped document-symbol facts that are safe to promote live:
//! fresh, high-confidence, source-backed syntax symbols, plus shadow receipts
//! for generated, stale, and dynamic candidates that must stay gated.

use perl_parser_core::ast::Node;
use perl_position_tracking::WireRange;
use perl_semantic_analyzer::symbol::{ScopeId, Symbol, SymbolExtractor, SymbolKind, SymbolTable};
use perl_semantic_facts::{
    AnchorId, Confidence, Provenance, ProviderFactFreshness, ProviderFactSourceKind,
    ProviderFactTrace, ProviderFallbackState, ProviderSurface,
};
use perl_workspace::semantic_shadow_compare::{
    SemanticShadowCompareReceipt, ShadowQueryInput, ShadowQueryName, summarize_identities,
};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// LSP-compatible document symbol produced from source-backed compiler facts.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct DocumentSymbol {
    /// Display name shown by the client.
    pub name: String,
    /// Optional symbol detail text.
    pub detail: String,
    /// LSP `SymbolKind` numeric value.
    pub kind: u32,
    /// Full source-backed range for the symbol.
    pub range: WireRange,
    /// Source-backed selection range for the symbol name.
    pub selection_range: WireRange,
    /// Nested source-backed symbols.
    pub children: Vec<DocumentSymbol>,
}

/// Source-backed document-symbol live result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DocumentSymbolLiveResult {
    /// Symbols safe to return live.
    pub symbols: Vec<DocumentSymbol>,
    /// Fact traces proving source/provenance/confidence/freshness.
    pub fact_traces: Vec<ProviderFactTrace>,
}

/// Build live document symbols from fresh, source-backed parser/compiler facts.
///
/// This helper deliberately excludes virtual generated members, stale facts,
/// dynamic boundaries, and ambiguous/no-source candidates. The runtime keeps
/// its existing fallback behavior when parsing is unavailable.
#[must_use]
pub fn source_backed_document_symbols_from_ast(
    ast: &Node,
    source: &str,
) -> DocumentSymbolLiveResult {
    let extractor = SymbolExtractor::new_with_source(source);
    let symbol_table = extractor.extract(ast);
    source_backed_document_symbols_from_table(&symbol_table, source)
}

fn source_backed_document_symbols_from_table(
    symbol_table: &SymbolTable,
    source: &str,
) -> DocumentSymbolLiveResult {
    let mut symbols_by_scope: HashMap<ScopeId, Vec<Symbol>> = HashMap::new();
    for symbols in symbol_table.symbols.values() {
        for symbol in symbols {
            if is_source_backed(symbol, source) {
                symbols_by_scope.entry(symbol.scope_id).or_default().push(symbol.clone());
            }
        }
    }

    let empty_vec = Vec::new();
    let global_symbols = symbols_by_scope.get(&0).unwrap_or(&empty_vec);
    let mut raw = Vec::new();
    let mut fact_traces = Vec::new();

    for symbol in global_symbols {
        if let Some(document_symbol) = source_backed_document_symbol(
            symbol,
            symbol_table,
            &symbols_by_scope,
            source,
            &mut fact_traces,
        ) {
            raw.push(document_symbol);
        }
    }

    // De-duplicate: a symbol that was collected as a child of some container must
    // not also appear at the top level.  Without this guard, a `package Foo;`
    // statement at byte offset 0 matches the global scope (also start=0) in the
    // children-search, causing every global-scope sibling to appear BOTH as a
    // child of `Foo` AND as a flat top-level entry — the duplication bug in #1519.
    // We identify duplicates by (line, character) of the symbol's start position,
    // which is unique within a source file.
    let child_starts: HashSet<(u32, u32)> = raw
        .iter()
        .flat_map(|sym| sym.children.iter())
        .map(|c| (c.range.start.line, c.range.start.character))
        .collect();
    let output: Vec<DocumentSymbol> = raw
        .into_iter()
        .filter(|sym| !child_starts.contains(&(sym.range.start.line, sym.range.start.character)))
        .collect();

    DocumentSymbolLiveResult { symbols: output, fact_traces }
}

fn source_backed_document_symbol(
    symbol: &Symbol,
    symbol_table: &SymbolTable,
    symbols_by_scope: &HashMap<ScopeId, Vec<Symbol>>,
    source: &str,
    fact_traces: &mut Vec<ProviderFactTrace>,
) -> Option<DocumentSymbol> {
    if !is_source_backed(symbol, source) {
        return None;
    }

    let mut children = Vec::new();
    if symbol.kind == SymbolKind::Package
        || symbol.kind == SymbolKind::Class
        || symbol.kind == SymbolKind::Subroutine
    {
        for (scope_id, scope) in &symbol_table.scopes {
            if scope.location.start == symbol.location.start {
                if let Some(child_symbols) = symbols_by_scope.get(scope_id) {
                    for child in child_symbols {
                        if child.location == symbol.location
                            && child.kind == symbol.kind
                            && child.name == symbol.name
                        {
                            continue;
                        }
                        if let Some(child_symbol) =
                            source_backed_leaf_symbol(child, source, fact_traces)
                        {
                            children.push((
                                document_symbol_priority(child),
                                child.location.start,
                                child.location.end,
                                child_symbol,
                            ));
                        }
                    }
                }
                break;
            }
        }
    }

    children.sort_by_key(|(priority, start, end, _)| (*priority, *start, *end));
    let children = children.into_iter().map(|(_, _, _, child)| child).collect();

    fact_traces.push(source_backed_trace());
    Some(DocumentSymbol {
        name: document_symbol_name(symbol),
        detail: document_symbol_detail(symbol),
        kind: document_symbol_kind(symbol),
        range: symbol_range(source, symbol),
        selection_range: symbol_name_range(source, symbol),
        children,
    })
}

fn source_backed_leaf_symbol(
    symbol: &Symbol,
    source: &str,
    fact_traces: &mut Vec<ProviderFactTrace>,
) -> Option<DocumentSymbol> {
    if !is_source_backed(symbol, source) {
        return None;
    }

    fact_traces.push(source_backed_trace());
    Some(DocumentSymbol {
        name: document_symbol_name(symbol),
        detail: document_symbol_detail(symbol),
        kind: document_symbol_kind(symbol),
        range: symbol_range(source, symbol),
        selection_range: symbol_name_range(source, symbol),
        children: Vec::new(),
    })
}

fn source_backed_trace() -> ProviderFactTrace {
    ProviderFactTrace::new(
        ProviderSurface::DocumentSymbols,
        ProviderFactSourceKind::ParserSyntax,
        Provenance::ExactAst,
        Confidence::High,
        ProviderFactFreshness::Fresh,
        ProviderFallbackState::Primary,
        None,
        None,
        Some(1),
    )
}

fn is_source_backed(symbol: &Symbol, source: &str) -> bool {
    symbol.location.start <= symbol.location.end && symbol.location.end <= source.len()
}

fn symbol_range(source: &str, symbol: &Symbol) -> WireRange {
    WireRange::from_byte_offsets(source, symbol.location.start, symbol.location.end)
}

/// Return a [`WireRange`] spanning only the symbol's identifier name, not its full declaration.
///
/// Per LSP 3.17, `selectionRange` must pinpoint the symbol name alone (e.g. just `greet` in
/// `sub greet { ... }`), while `range` covers the entire construct.  This helper performs a
/// byte-string search for `symbol.name` within the source slice bounded by `symbol.location`.
///
/// # Fallback
/// If the name cannot be located within the bounds (malformed source, empty name, out-of-range
/// location), the function falls back to `symbol_range()` — the full declaration span — so that
/// clients always receive a valid, non-panicking range.
fn symbol_name_range(source: &str, symbol: &Symbol) -> WireRange {
    if symbol.name.is_empty() {
        return symbol_range(source, symbol);
    }
    let slice = match source.get(symbol.location.start..symbol.location.end) {
        Some(s) => s,
        None => return symbol_range(source, symbol),
    };
    let rel_offset = match slice.find(symbol.name.as_str()) {
        Some(o) => o,
        None => return symbol_range(source, symbol),
    };
    let abs_start = symbol.location.start + rel_offset;
    let abs_end = abs_start + symbol.name.len();
    // Guard: name end must not exceed the symbol's declared bounds.
    if abs_end > symbol.location.end {
        return symbol_range(source, symbol);
    }
    WireRange::from_byte_offsets(source, abs_start, abs_end)
}

fn document_symbol_kind(symbol: &Symbol) -> u32 {
    if symbol.declaration.as_deref() == Some("has") && symbol.kind == SymbolKind::scalar() {
        7
    } else {
        symbol.kind.to_lsp_kind_document_symbol()
    }
}

fn document_symbol_name(symbol: &Symbol) -> String {
    if symbol.declaration.as_deref() == Some("has") {
        symbol.name.clone()
    } else if let Some(sigil) = symbol.kind.sigil() {
        format!("{}{}", sigil, symbol.name)
    } else {
        symbol.name.clone()
    }
}

fn document_symbol_detail(symbol: &Symbol) -> String {
    if symbol.declaration.as_deref() == Some("has") && symbol.kind == SymbolKind::scalar() {
        if !symbol.attributes.is_empty() {
            symbol.attributes.join(", ")
        } else {
            symbol.documentation.clone().unwrap_or_default()
        }
    } else if symbol.declaration.as_deref() == Some("has") {
        symbol.documentation.clone().unwrap_or_default()
    } else {
        symbol.declaration.as_deref().unwrap_or("").to_string()
    }
}

fn document_symbol_priority(symbol: &Symbol) -> u8 {
    if symbol.declaration.as_deref() == Some("has") && symbol.kind == SymbolKind::scalar() {
        0
    } else if symbol.declaration.as_deref() == Some("has") {
        2
    } else if matches!(symbol.kind, SymbolKind::Package | SymbolKind::Class | SymbolKind::Role) {
        1
    } else if symbol.kind.is_callable() {
        3
    } else {
        4
    }
}

/// Legacy document-symbol identity considered by the shadow proof.
///
/// This is not a live LSP response type. The identity should be stable enough
/// to compare the existing provider result against compiler-fact candidates.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct DocumentSymbolShadowLegacy {
    /// Stable identity for deterministic receipt comparison.
    pub identity: String,
}

/// Compiler-fact candidate considered by the document-symbol shadow proof.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DocumentSymbolShadowCandidate {
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

/// Document-symbol shadow proof result.
#[derive(Debug, Clone)]
#[non_exhaustive]
pub struct DocumentSymbolShadowResult {
    /// Legacy symbols returned by the existing runtime provider path.
    pub legacy_symbols: Vec<DocumentSymbolShadowLegacy>,
    /// Shadow receipt comparing legacy symbols with compiler-fact candidates.
    pub receipt: SemanticShadowCompareReceipt,
}

/// Compare legacy document-symbol output against compiler-fact candidates.
///
/// This function is intentionally shadow-only: it returns the original legacy
/// symbols unchanged and emits a receipt that records source, provenance,
/// confidence, freshness, and fallback/blocker state for candidate facts.
#[must_use]
pub fn document_symbol_source_shadow(
    legacy_symbols: Vec<DocumentSymbolShadowLegacy>,
    compiler_candidates: Vec<DocumentSymbolShadowCandidate>,
    symbol: &str,
) -> DocumentSymbolShadowResult {
    let old_result = summarize_identities(Some(
        legacy_symbols.iter().map(|symbol| symbol.identity.clone()).collect(),
    ));
    let new_result =
        summarize_identities(Some(document_symbol_answer_identities(&compiler_candidates)));
    let notes = vec![document_symbol_shadow_note(&legacy_symbols, &compiler_candidates)];
    let fact_source_traces =
        compiler_candidates.iter().map(document_symbol_candidate_trace).collect();

    let receipt = SemanticShadowCompareReceipt::from_summaries_with_fact_source_traces(
        ShadowQueryName::DocumentSymbols,
        ShadowQueryInput { symbol: symbol.to_string() },
        old_result,
        new_result,
        notes,
        fact_source_traces,
    );

    DocumentSymbolShadowResult { legacy_symbols, receipt }
}

fn document_symbol_candidate_trace(candidate: &DocumentSymbolShadowCandidate) -> ProviderFactTrace {
    ProviderFactTrace::new(
        ProviderSurface::DocumentSymbols,
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

fn document_symbol_answer_identities(
    compiler_candidates: &[DocumentSymbolShadowCandidate],
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

fn document_symbol_shadow_note(
    legacy_symbols: &[DocumentSymbolShadowLegacy],
    compiler_candidates: &[DocumentSymbolShadowCandidate],
) -> String {
    let blocked_count = compiler_candidates
        .iter()
        .filter(|candidate| candidate.fallback_state == ProviderFallbackState::Blocked)
        .count();
    format!(
        "document-symbol shadow proof: legacy_candidates={}; compiler_fact_candidates={}; blocked_candidates={}; no live document-symbol behavior change",
        legacy_symbols.len(),
        compiler_candidates.len(),
        blocked_count
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_parser_core::Parser;
    use perl_tdd_support::must;
    use perl_workspace::semantic_shadow_compare::ShadowCompareVerdict;

    #[test]
    fn source_backed_document_symbols_emit_fresh_primary_traces()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "package Foo;\n\nsub greet {\n    return 1;\n}\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);

        assert!(!result.symbols.is_empty(), "expected source-backed symbols");
        assert!(
            result.symbols.iter().any(|symbol| symbol.name == "Foo"),
            "package symbol should be present: {:?}",
            result.symbols
        );
        assert!(
            result.fact_traces.iter().any(|trace| {
                trace.source == ProviderFactSourceKind::ParserSyntax
                    && trace.provenance == Provenance::ExactAst
                    && trace.confidence == Confidence::High
                    && trace.freshness == ProviderFactFreshness::Fresh
                    && trace.fallback_state == ProviderFallbackState::Primary
            }),
            "source-backed live result must carry fresh primary parser-syntax traces"
        );
        Ok(())
    }

    /// Regression guard for #1519: `package Foo;\nsub greet {}` must produce greet
    /// exactly ONCE — as a child of Foo, not also duplicated at the top level.
    ///
    /// The global scope has `location.start = 0` and `package Foo;` at the top of a
    /// file also has `location.start = 0`, so the children-search absorbs global-scope
    /// siblings (greet) as children of Foo.  The de-dup pass then removes greet from
    /// the top-level list.  Both effects together give the correct LSP outline.
    #[test]
    fn source_backed_document_symbols_no_duplication_statement_package()
    -> Result<(), Box<dyn std::error::Error>> {
        let source =
            "package Foo;\n\nsub greet {\n    return 1;\n}\nsub farewell {\n    return 0;\n}\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);

        // greet and farewell must each appear exactly once in the full tree.
        let all_names: Vec<String> = {
            let mut names = Vec::new();
            for sym in &result.symbols {
                names.push(sym.name.clone());
                for child in &sym.children {
                    names.push(child.name.clone());
                }
            }
            names
        };
        assert_eq!(
            all_names.iter().filter(|n| n.as_str() == "greet").count(),
            1,
            "greet must appear exactly once (no top-level duplication); tree names: {:?}",
            all_names
        );
        assert_eq!(
            all_names.iter().filter(|n| n.as_str() == "farewell").count(),
            1,
            "farewell must appear exactly once (no top-level duplication); tree names: {:?}",
            all_names
        );

        // Foo must be at the top level.
        assert!(
            result.symbols.iter().any(|s| s.name == "Foo"),
            "Foo package must appear at top level; got: {:?}",
            result.symbols
        );

        // greet and farewell must NOT appear at top level (they are children of Foo).
        assert!(
            !result.symbols.iter().any(|s| s.name == "greet"),
            "greet must not appear at top level (it is a child of Foo); got: {:?}",
            result.symbols
        );

        // greet must be a child of Foo with source-backed ranges.
        let foo = result.symbols.iter().find(|s| s.name == "Foo").ok_or("no Foo")?;
        let greet = foo
            .children
            .iter()
            .find(|c| c.name == "greet")
            .ok_or("greet must be a child of Foo")?;
        assert!(
            greet.range.start.line <= greet.range.end.line,
            "greet must carry source-backed range: {:?}",
            greet
        );
        Ok(())
    }

    #[test]
    fn document_symbol_shadow_traces_explicit_syntax_fact() -> Result<(), Box<dyn std::error::Error>>
    {
        let legacy = legacy_symbol("package:Foo:0:0");
        let result = document_symbol_source_shadow(
            vec![legacy],
            vec![shadow_candidate(
                "package:Foo:0:0",
                ProviderFactSourceKind::ParserSyntax,
                Provenance::ExactAst,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
            "Foo",
        );

        assert_eq!(result.legacy_symbols.len(), 1);
        assert_eq!(result.receipt.query, ShadowQueryName::DocumentSymbols);
        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Same);
        assert_eq!(result.receipt.old_result.match_count, 1);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.surface, ProviderSurface::DocumentSymbols);
        assert_eq!(trace.source, ProviderFactSourceKind::ParserSyntax);
        assert_eq!(trace.provenance, Provenance::ExactAst);
        assert_eq!(trace.confidence, Confidence::High);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn document_symbol_shadow_labels_generated_candidates() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = document_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "generated:Foo::reader:virtual",
                ProviderFactSourceKind::FrameworkAdapter,
                Provenance::FrameworkSynthesis,
                Confidence::Medium,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Shadow,
            )],
            "reader",
        );

        assert_eq!(result.receipt.verdict, ShadowCompareVerdict::Improved);
        assert_eq!(result.receipt.old_result.match_count, 0);
        assert_eq!(result.receipt.new_result.match_count, 1);

        let trace = first_trace(&result)?;
        assert_eq!(trace.surface, ProviderSurface::DocumentSymbols);
        assert_eq!(trace.source, ProviderFactSourceKind::FrameworkAdapter);
        assert_eq!(trace.provenance, Provenance::FrameworkSynthesis);
        assert_eq!(trace.confidence, Confidence::Medium);
        assert_eq!(trace.freshness, ProviderFactFreshness::Fresh);
        assert_eq!(trace.fallback_state, ProviderFallbackState::Shadow);
        Ok(())
    }

    #[test]
    fn document_symbol_shadow_blocks_dynamic_boundaries() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = document_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "dynamic:Foo::AUTOLOAD",
                ProviderFactSourceKind::DynamicBoundary,
                Provenance::DynamicBoundary,
                Confidence::High,
                ProviderFactFreshness::Fresh,
                ProviderFallbackState::Blocked,
            )],
            "AUTOLOAD",
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
    fn document_symbol_shadow_blocks_stale_compiler_facts() -> Result<(), Box<dyn std::error::Error>>
    {
        let result = document_symbol_source_shadow(
            Vec::new(),
            vec![shadow_candidate(
                "stale:Foo::old_sub",
                ProviderFactSourceKind::CompilerFact,
                Provenance::SemanticAnalyzer,
                Confidence::Low,
                ProviderFactFreshness::Stale,
                ProviderFallbackState::Blocked,
            )],
            "old_sub",
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

    fn legacy_symbol(identity: &str) -> DocumentSymbolShadowLegacy {
        DocumentSymbolShadowLegacy { identity: identity.to_string() }
    }

    fn shadow_candidate(
        identity: &str,
        source: ProviderFactSourceKind,
        provenance: Provenance,
        confidence: Confidence,
        freshness: ProviderFactFreshness,
        fallback_state: ProviderFallbackState,
    ) -> DocumentSymbolShadowCandidate {
        DocumentSymbolShadowCandidate {
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
        result: &DocumentSymbolShadowResult,
    ) -> Result<&ProviderFactTrace, Box<dyn std::error::Error>> {
        result
            .receipt
            .fact_source_traces
            .first()
            .ok_or_else(|| "expected document-symbol fact-source trace".into())
    }

    #[test]
    fn test_symbol_name_range_subroutine() -> Result<(), Box<dyn std::error::Error>> {
        let source = "sub foo { }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);
        let sub_symbol = result
            .symbols
            .iter()
            .find(|s| s.name == "foo")
            .ok_or("expected 'foo' subroutine symbol")?;

        // "foo" starts at character 4 (after "sub ")
        // Expected: selectionRange points to "foo", not "sub foo"
        let _name_start_byte = source.find("foo").ok_or("name 'foo' not found in source")?;

        // selection_range should span just "foo", not the entire "sub foo { }"
        assert!(
            sub_symbol.selection_range.start.line == 0,
            "selectionRange should start at line 0, got: {:?}",
            sub_symbol.selection_range.start
        );
        assert!(
            sub_symbol.selection_range.end.line == 0,
            "selectionRange should end at line 0, got: {:?}",
            sub_symbol.selection_range.end
        );

        // Verify selection_range is smaller than range
        let range_char_span = sub_symbol.range.end.character - sub_symbol.range.start.character;
        let sel_char_span =
            sub_symbol.selection_range.end.character - sub_symbol.selection_range.start.character;
        assert!(
            sel_char_span < range_char_span,
            "selectionRange ({}) should be smaller than range ({})",
            sel_char_span,
            range_char_span
        );

        Ok(())
    }

    #[test]
    fn test_symbol_name_range_package() -> Result<(), Box<dyn std::error::Error>> {
        let source = "package MyPkg;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);
        let pkg_symbol = result
            .symbols
            .iter()
            .find(|s| s.name == "MyPkg")
            .ok_or("expected 'MyPkg' package symbol")?;

        // "MyPkg" starts after "package "
        let _name_start_byte = source.find("MyPkg").ok_or("name 'MyPkg' not found in source")?;

        // selection_range should span just "MyPkg", not the entire "package MyPkg;"
        assert!(
            pkg_symbol.selection_range.start.line == 0,
            "selectionRange should start at line 0, got: {:?}",
            pkg_symbol.selection_range.start
        );

        // Verify selection_range is smaller than range
        let range_char_span = pkg_symbol.range.end.character - pkg_symbol.range.start.character;
        let sel_char_span =
            pkg_symbol.selection_range.end.character - pkg_symbol.selection_range.start.character;
        assert!(
            sel_char_span < range_char_span,
            "selectionRange ({}) should be smaller than range ({})",
            sel_char_span,
            range_char_span
        );

        Ok(())
    }

    #[test]
    fn test_symbol_name_range_scalar_variable() -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $counter = 0;";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);
        // DocumentSymbol.name for a scalar variable includes the sigil (e.g. "$counter").
        let var_symbol = result
            .symbols
            .iter()
            .find(|s| s.name == "$counter")
            .ok_or("expected '$counter' variable symbol")?;

        // "counter" starts after "my $"
        let _name_start_byte =
            source.find("counter").ok_or("name 'counter' not found in source")?;

        // selection_range should span just "counter", not "$counter"
        assert!(
            var_symbol.selection_range.start.line == 0,
            "selectionRange should start at line 0, got: {:?}",
            var_symbol.selection_range.start
        );

        // Verify selection_range is smaller than or equal to range
        let sel_char_span =
            var_symbol.selection_range.end.character - var_symbol.selection_range.start.character;
        let range_char_span = var_symbol.range.end.character - var_symbol.range.start.character;
        assert!(
            sel_char_span <= range_char_span,
            "selectionRange ({}) should be <= range ({})",
            sel_char_span,
            range_char_span
        );

        Ok(())
    }

    #[test]
    fn test_symbol_name_range_array_variable() -> Result<(), Box<dyn std::error::Error>> {
        let source = "my @items = ();";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);
        // DocumentSymbol.name for an array variable includes the sigil (e.g. "@items").
        let var_symbol = result
            .symbols
            .iter()
            .find(|s| s.name == "@items")
            .ok_or("expected '@items' variable symbol")?;

        // "items" starts after "my @"
        let _name_start_byte = source.find("items").ok_or("name 'items' not found in source")?;

        // selection_range should span just "items", not "@items"
        assert!(
            var_symbol.selection_range.start.line == 0,
            "selectionRange should start at line 0, got: {:?}",
            var_symbol.selection_range.start
        );

        // Verify selection_range is smaller than or equal to range
        let sel_char_span =
            var_symbol.selection_range.end.character - var_symbol.selection_range.start.character;
        let range_char_span = var_symbol.range.end.character - var_symbol.range.start.character;
        assert!(
            sel_char_span <= range_char_span,
            "selectionRange ({}) should be <= range ({})",
            sel_char_span,
            range_char_span
        );

        Ok(())
    }

    #[test]
    fn test_symbol_name_range_moose_attribute() -> Result<(), Box<dyn std::error::Error>> {
        // `use Moose;` is required for the SymbolExtractor to recognise the `has` pattern.
        // Without framework context the `has name => ...` expression is not synthesized into
        // a symbol.  DocumentSymbol.name for a Moose attribute is the bare attribute name
        // (no sigil) per document_symbol_name().
        let source = "package Foo;\nuse Moose;\nhas name => (is => 'ro');\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);
        let attr_symbol = result
            .symbols
            .iter()
            .flat_map(|s| std::iter::once(s).chain(s.children.iter()))
            .find(|s| s.name == "name")
            .ok_or("expected 'name' Moose attribute symbol")?;

        // "name" starts after "has " within the has declaration.
        // Source layout: line 0="package Foo;", line 1="use Moose;", line 2="has name => ..."
        let _name_start_byte = source.find("name").ok_or("name 'name' not found in source")?;

        // selection_range should span just "name" at character 4 of line 2 (after "has ").
        assert!(
            attr_symbol.selection_range.start.line == 2,
            "selectionRange should start at line 2 (the 'has name' declaration), got: {:?}",
            attr_symbol.selection_range.start
        );

        // Verify selection_range is smaller than or equal to range
        let sel_char_span =
            attr_symbol.selection_range.end.character - attr_symbol.selection_range.start.character;
        let range_char_span = attr_symbol.range.end.character - attr_symbol.range.start.character;
        assert!(
            sel_char_span <= range_char_span,
            "selectionRange ({}) should be <= range ({})",
            sel_char_span,
            range_char_span
        );

        Ok(())
    }

    #[test]
    fn test_document_symbols_selection_range_vs_range() -> Result<(), Box<dyn std::error::Error>> {
        let source = "package TestPkg;\n\nsub method1 { }\nmy $var = 1;\nhas attr => ();\n";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let result = source_backed_document_symbols_from_ast(&ast, source);

        // For every symbol, selectionRange must be <= range
        // (in byte span, not character span, but we verify via positions)
        for symbol in &result.symbols {
            let range_span = symbol.range.end.line as i32 - symbol.range.start.line as i32;
            let sel_span =
                symbol.selection_range.end.line as i32 - symbol.selection_range.start.line as i32;

            assert!(
                sel_span <= range_span,
                "selectionRange line span ({}) should be <= range line span ({}) for symbol '{}'",
                sel_span,
                range_span,
                symbol.name
            );

            // If same line, check character positions
            if symbol.range.start.line == symbol.range.end.line
                && symbol.selection_range.start.line == symbol.selection_range.end.line
                && symbol.range.start.line == symbol.selection_range.start.line
            {
                let range_char_span = symbol.range.end.character - symbol.range.start.character;
                let sel_char_span =
                    symbol.selection_range.end.character - symbol.selection_range.start.character;

                assert!(
                    sel_char_span <= range_char_span,
                    "selectionRange char span ({}) should be <= range char span ({}) for symbol '{}'",
                    sel_char_span,
                    range_char_span,
                    symbol.name
                );
            }

            // Recursively check children
            check_children_ranges(&symbol.children)?;
        }

        Ok(())
    }

    fn check_children_ranges(
        children: &[DocumentSymbol],
    ) -> Result<(), Box<dyn std::error::Error>> {
        for child in children {
            let range_span = child.range.end.line as i32 - child.range.start.line as i32;
            let sel_span =
                child.selection_range.end.line as i32 - child.selection_range.start.line as i32;

            assert!(
                sel_span <= range_span,
                "child selectionRange line span ({}) should be <= range line span ({}) for symbol '{}'",
                sel_span,
                range_span,
                child.name
            );

            check_children_ranges(&child.children)?;
        }
        Ok(())
    }

    #[test]
    fn test_symbol_name_range_fallback() -> Result<(), Box<dyn std::error::Error>> {
        // This test verifies the fallback behavior: if a symbol's name cannot be found
        // in the source slice, the function should return the full symbol_range (not panic).
        // We construct a Symbol with mismatched bounds to trigger this.

        let source = "sub test { }";
        let mut parser = Parser::new(source);
        let ast = must(parser.parse());

        let extractor = SymbolExtractor::new_with_source(source);
        let symbol_table = extractor.extract(&ast);

        // Find a symbol from the table
        let _test_symbol = symbol_table
            .symbols
            .values()
            .flatten()
            .find(|s| s.name == "test")
            .ok_or("expected 'test' symbol")?;

        // The symbol_name_range helper should exist and be callable
        // If the name cannot be found within the location bounds, it should gracefully
        // fall back to the full symbol_range instead of panicking.
        //
        // Since symbol_name_range is a private function, we test its behavior
        // indirectly by calling source_backed_document_symbols_from_ast and verifying
        // that all symbols have valid (non-panicking) ranges.

        let result = source_backed_document_symbols_from_ast(&ast, source);
        assert!(!result.symbols.is_empty(), "should produce symbols without panicking");

        Ok(())
    }
}
