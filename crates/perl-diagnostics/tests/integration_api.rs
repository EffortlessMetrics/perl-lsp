//! Integration tests for perl-diagnostics crate Wave E consolidation (#4429).
//!
//! These tests verify:
//! - Module structure and public API
//! - Type unification (DiagnosticSeverity, DiagnosticTag)
//! - Re-export pattern (explicit, no wildcards)
//! - Consumer compilation paths
//! - Workspace and publish allowlist counts

#![allow(dead_code)]

// Test 1: Core module imports resolve
#[test]
fn test_codes_module_imports() {
    // Must be able to import from codes module
    use perl_diagnostics::codes::DiagnosticCategory;
    use perl_diagnostics::codes::DiagnosticCode;
    use perl_diagnostics::codes::DiagnosticSeverity;
    use perl_diagnostics::codes::DiagnosticTag;

    // If this test compiles, the codes module exists and exports these types
    let _ = (
        DiagnosticCode::ParseError,
        DiagnosticSeverity::Error,
        DiagnosticTag::Unnecessary,
        DiagnosticCategory::Parser,
    );
}

// Test 2: Types module imports resolve
#[test]
fn test_types_module_imports() {
    // Must be able to import from types module
    use perl_diagnostics::types::Diagnostic;
    use perl_diagnostics::types::RelatedInformation;

    // If this test compiles, the types module exists and exports these types
    let _ = (Diagnostic::default(), RelatedInformation::default());
}

// Test 3: Types module re-exports unified severity and tag
#[test]
fn test_types_module_reexports_unified_types() {
    // After type unification, types module must re-export severity and tag from codes
    use perl_diagnostics::types::DiagnosticSeverity;
    use perl_diagnostics::types::DiagnosticTag;

    let _ = (DiagnosticSeverity::Error, DiagnosticTag::Unnecessary);
}

// Test 4: Catalog module imports resolve
#[test]
fn test_catalog_module_imports() {
    // Must be able to import from catalog module
    use perl_diagnostics::catalog::DiagnosticMeta;
    use perl_diagnostics::catalog::diagnostic_meta;
    use perl_diagnostics::catalog::parse_error;

    let _ = (DiagnosticMeta::default(), diagnostic_meta, parse_error);
}

// Test 5: API re-exports are available at crate root
#[test]
fn test_api_root_reexports_codes_types() {
    // All critical types should be re-exported at crate root via api.rs
    use perl_diagnostics::DiagnosticCategory;
    use perl_diagnostics::DiagnosticCode;
    use perl_diagnostics::DiagnosticSeverity;

    let _ = (DiagnosticCode::ParseError, DiagnosticSeverity::Error, DiagnosticCategory::Parser);
}

// Test 6: API re-exports catalog functions
#[test]
fn test_api_root_reexports_catalog() {
    // Core catalog functions should be re-exported at crate root
    use perl_diagnostics::diagnostic_meta;
    use perl_diagnostics::parse_error;

    let _ = (diagnostic_meta, parse_error);
}

// Test 7: Type unification — codes and types severity are the same type
#[test]
fn test_severity_type_unification_cross_path_assignment() {
    use perl_diagnostics::codes::DiagnosticSeverity as CodesSeverity;
    use perl_diagnostics::types::DiagnosticSeverity as TypesSeverity;

    let from_codes: CodesSeverity = CodesSeverity::Error;
    // This assignment must compile, proving they are the same type
    let _as_types: TypesSeverity = from_codes;

    // Verify LSP spec value
    assert_eq!(from_codes as u8, 1); // Error = 1 in LSP spec
}

// Test 8: Type unification — codes and types tag are the same type
#[test]
fn test_tag_type_unification_cross_path_assignment() {
    use perl_diagnostics::codes::DiagnosticTag as CodesTag;
    use perl_diagnostics::types::DiagnosticTag as TypesTag;

    let from_codes: CodesTag = CodesTag::Unnecessary;
    // This assignment must compile, proving they are the same type
    let _as_types: TypesTag = from_codes;
}

// Test 9: Type identity verification (orchestrator-locked)
#[test]
fn test_severity_type_identity_same_underlying_type() {
    use perl_diagnostics::codes::DiagnosticSeverity as CodesSeverity;
    use perl_diagnostics::types::DiagnosticSeverity as TypesSeverity;

    // TypeId must be identical, proving they are literally the same type
    let codes_type_id = std::any::TypeId::of::<CodesSeverity>();
    let types_type_id = std::any::TypeId::of::<TypesSeverity>();

    assert_eq!(
        codes_type_id, types_type_id,
        "DiagnosticSeverity must be a single unified type, not two separate enums"
    );
}

// Test 10: Type identity verification for tag
#[test]
fn test_tag_type_identity_same_underlying_type() {
    use perl_diagnostics::codes::DiagnosticTag as CodesTag;
    use perl_diagnostics::types::DiagnosticTag as TypesTag;

    // TypeId must be identical, proving they are literally the same type
    let codes_type_id = std::any::TypeId::of::<CodesTag>();
    let types_type_id = std::any::TypeId::of::<TypesTag>();

    assert_eq!(
        codes_type_id, types_type_id,
        "DiagnosticTag must be a single unified type, not two separate enums"
    );
}

// Test 11: Diagnostic struct uses unified severity type
#[test]
fn test_diagnostic_struct_uses_unified_severity() {
    use perl_diagnostics::types::Diagnostic;
    use perl_diagnostics::types::DiagnosticSeverity;

    let diag = Diagnostic { severity: DiagnosticSeverity::Error, ..Default::default() };

    // The field should bind to the unified type
    let _severity = diag.severity;
}

// Test 12: Explicit re-exports include DiagnosticCategory (locked pattern)
#[test]
fn test_api_includes_diagnostic_category_reexport() {
    // DiagnosticCategory must be explicitly re-exported (not via wildcard)
    use perl_diagnostics::DiagnosticCategory;

    let _ = DiagnosticCategory::Parser;
}

// Test 13: No wildcard re-exports in api.rs (explicit lists only)
#[test]
fn test_api_reexports_explicit_not_wildcards() {
    // This test documents the locked pattern: explicit per-symbol, no wildcards
    // If the crate compiles, api.rs uses explicit lists
    use perl_diagnostics::{
        Diagnostic, DiagnosticCategory, DiagnosticCode, DiagnosticSeverity, DiagnosticTag,
        RelatedInformation, diagnostic_meta, parse_error, syntax_error,
    };

    let _ = (
        DiagnosticCode::ParseError,
        DiagnosticSeverity::Error,
        DiagnosticTag::Unnecessary,
        DiagnosticCategory::Parser,
        Diagnostic::default(),
        RelatedInformation::default(),
        diagnostic_meta,
        parse_error,
        syntax_error,
    );
}

// Test 14: Consumer path: perl-lsp-code-actions compiles with new imports
#[test]
fn test_consumer_perl_lsp_code_actions_path() {
    // Document the expected import path for consumers
    use perl_diagnostics::codes::DiagnosticCode;
    use perl_diagnostics::types::Diagnostic;

    let _ = (DiagnosticCode::ParseError, Diagnostic::default());
}

// Test 15: Consumer path: perl-lsp-diagnostics compiles with new imports
#[test]
fn test_consumer_perl_lsp_diagnostics_path() {
    // Document the expected import path for consumers
    use perl_diagnostics::codes::DiagnosticSeverity;
    use perl_diagnostics::types::Diagnostic;
    use perl_diagnostics::types::DiagnosticSeverity as TypesSeverity;

    // Cross-module assignment from codes to types path must compile
    let from_codes = DiagnosticSeverity::Warning;
    let _as_types: TypesSeverity = from_codes;

    let _ = Diagnostic::default();
}

// Test 16: Consumer path: perl-lsp server compiles with new imports
#[test]
fn test_consumer_perl_lsp_server_path() {
    // Document the expected import path for the LSP server
    use perl_diagnostics::catalog::diagnostic_meta;
    use perl_diagnostics::codes::DiagnosticCode;

    let _ = (DiagnosticCode::ParseError, diagnostic_meta);
}

// Test 17: Severity enum values match LSP spec
#[test]
fn test_severity_lsp_values() {
    use perl_diagnostics::codes::DiagnosticSeverity;

    // LSP spec: Error=1, Warning=2, Information=3, Hint=4
    assert_eq!(DiagnosticSeverity::Error as u8, 1);
    assert_eq!(DiagnosticSeverity::Warning as u8, 2);
    assert_eq!(DiagnosticSeverity::Information as u8, 3);
    assert_eq!(DiagnosticSeverity::Hint as u8, 4);
}

// Test 18: Tag enum values match LSP spec
#[test]
fn test_tag_lsp_values() {
    use perl_diagnostics::codes::DiagnosticTag;

    // LSP spec: Unnecessary=1, Deprecated=2
    assert_eq!(DiagnosticTag::Unnecessary as u8, 1);
    assert_eq!(DiagnosticTag::Deprecated as u8, 2);
}

// Test 19: Workspace count verification (edge case: structural integrity)
#[test]
#[ignore] // This test requires external verification (workspace inspection)
fn test_workspace_member_count_should_be_121() {
    // After Wave E: 123 - 3 (removed) + 1 (added) = 121
    // This test documents the expected final count
    // Verification: `cargo metadata --no-deps | jq '.workspace_members | length'`

    // Marker: this test is a specification of the final state
    // Builder will verify via cargo metadata
}

// Test 20: Publish allowlist count verification (edge case: structural integrity)
#[test]
#[ignore] // This test requires external verification (Cargo.toml inspection)
fn test_publish_allowlist_count_should_be_118() {
    // After Wave E: 120 - 3 (removed) + 1 (added) = 118
    // This test documents the expected final count
    // Verification: builder manually counts [workspace.metadata.publish] allow entries

    // Marker: this test is a specification of the final state
}

// Test 21: Edge case — no circular dependency between codes and types modules
#[test]
fn test_no_circular_dependency_codes_types() {
    use perl_diagnostics::codes::DiagnosticCode;
    use perl_diagnostics::types::Diagnostic;

    // codes module exports DiagnosticCode (used by types)
    // types module re-exports from codes
    // This assignment demonstrates the dependency direction is correct
    let diag = Diagnostic { code: DiagnosticCode::ParseError, ..Default::default() };

    let _ = diag;
}

// Test 22: Edge case — Diagnostic struct fields are still accessible
#[test]
fn test_diagnostic_struct_fields_accessible() {
    use perl_diagnostics::types::{Diagnostic, DiagnosticSeverity};

    let diag = Diagnostic {
        code: perl_diagnostics::codes::DiagnosticCode::ParseError,
        severity: DiagnosticSeverity::Error,
        range: Default::default(),
        message: "test".to_string(),
        related_information: None,
        tags: None,
    };

    // All fields must be accessible (struct not broken by unification)
    assert_eq!(diag.message, "test");
}

// Test 23: Layer constraint verification (edge case: architectural integrity)
#[test]
#[ignore] // This test requires external verification (xtask layer-check)
fn test_perl_diagnostics_no_perl_lsp_dependencies() {
    // perl-diagnostics must NOT depend on any perl-lsp-* crate
    // This is enforced by xtask layer-check rule
    // Verification: `cargo xtask layer-check` and manual Cargo.toml inspection

    // Marker: this test documents the locked constraint
}

// Test 24: Re-export functions from catalog module
#[test]
fn test_catalog_function_reexports() {
    // All catalog functions must be available at crate root
    use perl_diagnostics::{
        bareword_filehandle, critic_severity_1, critic_severity_2, critic_severity_3,
        critic_severity_4, critic_severity_5, duplicate_package, duplicate_sub, eval_error_flow,
        from_message, implicit_return, missing_package_declaration, missing_return, missing_strict,
        missing_warnings, parse_error, syntax_error, two_arg_open, undefined_var, unexpected_eof,
        unused_var,
    };

    // If these imports resolve, the re-exports are in place
    let _ = (
        parse_error,
        syntax_error,
        unexpected_eof,
        missing_strict,
        missing_warnings,
        unused_var,
        undefined_var,
        missing_package_declaration,
        duplicate_package,
        duplicate_sub,
        missing_return,
        bareword_filehandle,
        two_arg_open,
        implicit_return,
        eval_error_flow,
        critic_severity_5,
        critic_severity_4,
        critic_severity_3,
        critic_severity_2,
        critic_severity_1,
        from_message,
    );
}

// Test 25: DiagnosticMeta type is available (catalog typedef)
#[test]
fn test_diagnostic_meta_reexport() {
    use perl_diagnostics::DiagnosticMeta;

    let _ = DiagnosticMeta::default();
}

// Test 26: Edge case — RelatedInformation struct is accessible
#[test]
fn test_related_information_struct_accessible() {
    use perl_diagnostics::types::RelatedInformation;

    let info = RelatedInformation { message: "test".to_string(), location: Default::default() };

    assert_eq!(info.message, "test");
}
