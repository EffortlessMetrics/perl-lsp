//! Integration tests verifying that formerly-unwired LSP provider crates
//! are reachable from perl-lsp after the wiring refactor (#2756).
//!
//! Each test below imports a type or function from one of the 17 crates that
//! were confirmed to have zero call sites before this change. A compile-time
//! failure (crate not in Cargo.toml) or a runtime failure counts as
//! "not wired".
//!
//! Tests are intentionally minimal — their job is to confirm that the
//! dependency exists and the public API is reachable, not to re-test the
//! provider's own logic (which is covered by each crate's unit tests).

// ---------------------------------------------------------------------------
// perl-lsp-inline-completion
// ---------------------------------------------------------------------------

/// The crate must be a direct dependency of perl-lsp and its
/// InlineCompletionProvider must be accessible.
#[test]
fn test_wired_inline_completion_provider_accessible() {
    use perl_lsp_inline_completion::InlineCompletionProvider;
    let provider = InlineCompletionProvider::new();
    // Basic smoke: after `->` we should get a `new()` suggestion.
    let completions = provider.get_inline_completions("$obj->", 0, 6);
    assert!(!completions.items.is_empty(), "expected at least one inline completion");
    assert_eq!(completions.items[0].insert_text, "new()");
}

/// The crate must use the canonical utf16 position mapping.
/// `character` is a UTF-16 code-unit offset (LSP spec §3.17).
/// With the local duplicate the position was used as a raw byte offset,
/// which silently gave wrong results for non-BMP characters (emoji, CJK…).
/// The crate delegates to `utf16_line_col_to_offset`, so it handles these
/// correctly.
///
/// String: `"my $prefix = \"😀\"; $obj->"`
///   UTF-8 bytes  : 27  (emoji 😀 = 4 bytes; other 23 chars = 1 byte each)
///   UTF-16 units : 25  (emoji 😀 = 2 code units; other 23 chars = 1 unit each)
///
/// Pass `character = 25` (the UTF-16 end-of-string position).
/// The old local impl used `character.min(byte_len)` as a byte-slice index.
/// For this string that gives `min(25, 27) = 25`, which slices off the last
/// two bytes `->`, so `prefix` ends with `$obj` and no completion is returned.
///
/// We verify the crate API is reachable and returns the correct result.
#[test]
fn test_wired_inline_completion_utf16_position() {
    use perl_lsp_inline_completion::InlineCompletionProvider;
    let source = "my $prefix = \"😀\"; $obj->";
    // UTF-16 code-unit count for the full string (end-of-line position)
    let utf16_len: u32 = source.encode_utf16().count() as u32;
    let provider = InlineCompletionProvider::new();
    let completions = provider.get_inline_completions(source, 0, utf16_len);
    assert!(
        !completions.items.is_empty(),
        "expected inline completions at UTF-16 end-of-line position; got none"
    );
    assert_eq!(completions.items[0].insert_text, "new()", "expected 'new()' suggestion after '->'");
}

/// Real LSP requests provide a line number within a multi-line document.
/// The provider must find the correct line and apply the UTF-16 column offset
/// within that line — not across the entire document.
#[test]
fn test_wired_inline_completion_multiline_document() {
    use perl_lsp_inline_completion::InlineCompletionProvider;
    // Line 0: preamble; Line 1: the trigger line
    let source = "use strict;\nuse warnings;\n$obj->";
    let provider = InlineCompletionProvider::new();
    // line=2 is "$obj->", character=6 (UTF-16 units of "$obj->")
    let line2 = "$obj->";
    let col: u32 = line2.encode_utf16().count() as u32;
    let completions = provider.get_inline_completions(source, 2, col);
    assert!(
        !completions.items.is_empty(),
        "multi-line document: expected completion on line 2 at col {col}"
    );
    assert_eq!(completions.items[0].insert_text, "new()", "expected 'new()' after '->' on line 2");
}

// ---------------------------------------------------------------------------
// perl-lsp-workspace-symbols
// ---------------------------------------------------------------------------

/// WorkspaceSymbolsProvider must be reachable as a direct dependency.
#[test]
fn test_wired_workspace_symbols_provider_accessible() {
    use perl_lsp_rs_core::providers::workspace_symbols::WorkspaceSymbolsProvider;
    let provider = WorkspaceSymbolsProvider::new();
    // Empty provider returns no symbols — just confirm it compiles and runs.
    let results = provider.search("anything", &std::collections::HashMap::new());
    assert!(results.is_empty(), "fresh provider should return no symbols");
}

// ---------------------------------------------------------------------------
// perl-lsp-symbol-query
// ---------------------------------------------------------------------------

/// Symbol query helpers must be reachable.
#[test]
fn test_wired_symbol_query_matches() {
    use perl_lsp_rs_core::providers::symbol_query::matches_query;
    assert!(matches_query("process_data", "proc"));
    assert!(!matches_query("unrelated", "proc"));
}

// ---------------------------------------------------------------------------
// perl-lsp-completion-item
// ---------------------------------------------------------------------------

/// Completion item domain types must be reachable.
#[test]
fn test_wired_completion_item_dedup() {
    use perl_lsp_rs_core::providers::completion_item::{
        CompletionItem, CompletionItemKind, deduplicate_and_sort,
    };
    let make = |label: &str| CompletionItem {
        label: label.to_string(),
        kind: CompletionItemKind::Function,
        detail: None,
        documentation: None,
        insert_text: None,
        sort_text: None,
        filter_text: None,
        additional_edits: Vec::new(),
        text_edit_range: None,
        commit_characters: None,
    };
    let items = vec![make("say"), make("say")];
    let deduped = deduplicate_and_sort(items);
    assert_eq!(deduped.len(), 1, "duplicates should be removed");
}

// ---------------------------------------------------------------------------
// perl-ast-utils
// ---------------------------------------------------------------------------

/// AST utility helpers must be reachable.
#[test]
fn test_wired_ast_utils_find_function_insert_position() {
    use perl_parser::ast_utils::find_function_insert_position;
    let source = "package Foo;\n\nsub bar { 1 }\n";
    let pos = find_function_insert_position(source);
    // Current policy: insert at end-of-file.
    assert_eq!(pos, source.len(), "find_function_insert_position should return end-of-file offset");
}

// ---------------------------------------------------------------------------
// perl-lsp-formatting-types
// ---------------------------------------------------------------------------

/// Formatting types must be reachable.
#[test]
fn test_wired_formatting_types_accessible() {
    use perl_lsp_rs_core::providers::formatting_types::FormatRange;
    // FormatRange::whole_document parses the content to find the last line.
    // For a 3-line file the end line must be 2, not 0.
    let content = "line1\nline2\nline3";
    let range = FormatRange::whole_document(content);
    assert_eq!(range.start.line, 0, "whole_document range must start at line 0");
    assert_eq!(range.end.line, 2, "whole_document range must end at line 2 for a 3-line file");
}

// ---------------------------------------------------------------------------
// perl-lsp-critic-parser
// ---------------------------------------------------------------------------

/// Critic output parser must be reachable and parse valid output.
#[test]
fn test_wired_critic_parser_parses_output() {
    use perl_lsp_critic_parser::parse_perlcritic_output;
    // Canonical Perl::Critic colon-delimited format: file:line:col:severity:policy:message
    let output = "test.pl:1:1:5:TestingAndDebugging::RequireUseStrict:no strict\n";
    let lines = parse_perlcritic_output(output);
    assert_eq!(lines.len(), 1, "should parse one critic violation");
    assert_eq!(lines[0].line, 1);
    assert_eq!(lines[0].severity, 5);
}

// ---------------------------------------------------------------------------
// perl-lsp-import-management
// ---------------------------------------------------------------------------

/// Import management helpers must be reachable.
#[test]
fn test_wired_import_management_collect_imports() {
    use perl_lsp_rs_core::providers::import_management::collect_imports;
    let lines: Vec<String> = vec![
        "use strict;".to_string(),
        "use warnings;".to_string(),
        "use Scalar::Util qw(looks_like_number);".to_string(),
        "".to_string(),
        "sub foo { 1 }".to_string(),
    ];
    let imports = collect_imports(&lines);
    // 3 `use` lines + 1 blank + 1 sub — only the 3 use lines should be collected.
    assert_eq!(imports.len(), 3, "should collect exactly the 3 use statements");
    assert!(imports[0].contains("strict"), "first import should be 'use strict'");
    assert!(imports[2].contains("Scalar::Util"), "third import should be Scalar::Util");
}

// ---------------------------------------------------------------------------
// perl-lsp-rs-core::capability_map (absorbed from perl-lsp-capability-map)
// ---------------------------------------------------------------------------

/// Capability map helpers must be reachable from perl-lsp-rs-core.
#[test]
fn test_wired_capability_map_roundtrip() {
    use lsp_types::ServerCapabilities;
    use perl_lsp_rs_core::capability_map::{caps_from_feature_ids, feature_ids_from_caps};
    // Default (empty) capabilities → empty feature list
    let empty_caps = ServerCapabilities::default();
    let ids = feature_ids_from_caps(&empty_caps);
    assert!(ids.is_empty(), "empty capabilities should yield no feature ids");
    // Rebuild capabilities from a known feature id
    let caps = caps_from_feature_ids(&["lsp.hover"]);
    assert!(
        caps.hover_provider.is_some(),
        "caps_from_feature_ids should set hover_provider for lsp.hover"
    );
}

// ---------------------------------------------------------------------------
// perl-lsp-performance
// ---------------------------------------------------------------------------

/// Performance types must be reachable and AstCache must behave correctly.
#[test]
fn test_wired_performance_ast_cache_accessible() {
    use perl_lsp_performance::AstCache;
    let cache = AstCache::new(100, 60);
    // A freshly constructed cache must return None for any lookup —
    // this verifies the get() API is callable and the initial state is empty.
    let result = cache.get("file:///test.pl", "my $x = 1;");
    assert!(result.is_none(), "fresh AstCache should return None before any put()");
}

// ---------------------------------------------------------------------------
// perl-lsp-rs-core::features::flags (absorbed from perl-lsp-feature-flags)
// ---------------------------------------------------------------------------

/// Feature-flags types must be reachable from perl-lsp-rs-core and BuildFlags must be constructible.
#[test]
fn test_wired_feature_flags_accessible() {
    use perl_lsp_rs_core::features::flags::BuildFlags;
    // Default BuildFlags has all capabilities disabled
    let flags = BuildFlags::default();
    assert!(!flags.completion, "default BuildFlags should have completion disabled");
}

// ---------------------------------------------------------------------------
// perl-lsp-rs-core::features::policy (absorbed from perl-lsp-feature-policy)
// ---------------------------------------------------------------------------

/// Feature-policy helpers must be reachable from perl-lsp-rs-core.
#[test]
fn test_wired_feature_policy_accessible() {
    use perl_lsp_rs_core::features::policy::{FeatureProfile, flags_for_profile};
    let flags = flags_for_profile(FeatureProfile::Production);
    // Production profile must have core capabilities enabled.
    assert!(flags.completion, "production profile must enable completion");
    assert!(flags.hover, "production profile must enable hover");
}

// ---------------------------------------------------------------------------
// perl-lsp-rs-core::features::contracts (absorbed from perl-lsp-feature-contracts)
// ---------------------------------------------------------------------------

/// Feature-contracts types must be reachable from perl-lsp-rs-core.
#[test]
fn test_wired_feature_contracts_accessible() {
    use perl_lsp_rs_core::features::contracts::FEATURE_PROFILE_SPECS;
    // The canonical profile names are load-bearing — check specific known values.
    let canonicals: Vec<&str> = FEATURE_PROFILE_SPECS.iter().map(|s| s.canonical).collect();
    assert!(canonicals.contains(&"ga-lock"), "FEATURE_PROFILE_SPECS must contain ga-lock profile");
    assert!(
        canonicals.contains(&"production"),
        "FEATURE_PROFILE_SPECS must contain production profile"
    );
}

// ---------------------------------------------------------------------------
// perl-lsp-rs-core::features::grid (absorbed from perl-lsp-feature-grid)
// ---------------------------------------------------------------------------

/// Feature-grid re-exports must be reachable from perl-lsp-rs-core.
#[test]
fn test_wired_feature_grid_accessible() {
    use perl_lsp_rs_core::features::grid::feature_profile_specs;
    let specs = feature_profile_specs();
    // Verify the re-export returns the same data as the underlying contracts crate.
    let canonicals: Vec<&str> = specs.iter().map(|s| s.canonical).collect();
    assert!(
        canonicals.contains(&"production"),
        "feature_profile_specs must include production profile"
    );
}

// ---------------------------------------------------------------------------
// perl-lsp-rs-core::features::profile (absorbed from perl-lsp-feature-profile)
// ---------------------------------------------------------------------------

/// Feature-profile helpers must be reachable from perl-lsp-rs-core.
#[test]
fn test_wired_feature_profile_accessible() {
    use perl_lsp_rs_core::features::profile::supported_cli_profiles;
    let profiles = supported_cli_profiles();
    // Check that canonical token values are present, not just that the list is non-empty.
    assert!(profiles.contains(&"production"), "supported profiles must include 'production'");
    assert!(profiles.contains(&"ga-lock"), "supported profiles must include 'ga-lock'");
}

// ---------------------------------------------------------------------------
// perl-lsp-rs-core::features::profile_cli (absorbed from perl-lsp-feature-profile-cli)
// ---------------------------------------------------------------------------

/// Feature-profile-cli helpers must be reachable from perl-lsp-rs-core.
#[test]
fn test_wired_feature_profile_cli_accessible() {
    use perl_lsp_rs_core::features::profile_cli::{
        feature_profile_supported_tokens, parse_feature_profile_arg,
    };
    let tokens = feature_profile_supported_tokens();
    // Check specific tokens are present — this would fail if the list shrank unexpectedly.
    assert!(tokens.contains(&"production"), "supported tokens must include 'production'");
    // parse_feature_profile_arg must successfully parse known tokens.
    assert!(
        parse_feature_profile_arg("production").is_ok(),
        "should parse 'production' as a valid profile"
    );
    assert!(
        parse_feature_profile_arg("__invalid__").is_err(),
        "should reject unknown profile tokens"
    );
}

// ---------------------------------------------------------------------------
// perl-lsp-perltidy
// ---------------------------------------------------------------------------

/// Perltidy integration types must be reachable and defaults must be correct.
#[test]
fn test_wired_perltidy_config_accessible() {
    use perl_lsp_perltidy::PerlTidyConfig;
    let config = PerlTidyConfig::default();
    // Verify load-bearing defaults that downstream callers depend on.
    assert_eq!(config.indent_columns, Some(4), "default indent_columns must be 4");
    assert_eq!(config.maximum_line_length, Some(80), "default maximum_line_length must be 80");
    assert_eq!(config.tabs, Some(false), "default tabs must be false (spaces)");
}

// ---------------------------------------------------------------------------
// perl-lsp-document-links
// ---------------------------------------------------------------------------

/// Document links function must be reachable.
#[test]
fn test_wired_document_links_compute_links() {
    use perl_lsp_rs_core::providers::document_links::compute_links;
    use url::Url;
    let uri = "file:///test.pl";
    let text = "use Scalar::Util qw(looks_like_number);\n";
    let roots: Vec<Url> = vec![];
    let links = compute_links(uri, text, &roots);
    assert!(!links.is_empty(), "should produce a document link for Scalar::Util");
}
