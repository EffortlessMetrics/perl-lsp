//! Type unification tests for #4429 Wave E consolidation.
//!
//! Verifies that DiagnosticSeverity and DiagnosticTag are a single unified type
//! after Wave E. The `types::` module re-exports from `codes::` — assigning
//! between them must compile and the types must be identical (same TypeId).
//!
//! This is an orchestrator-locked requirement (see plan-reviewer comment on #4429).

use perl_diagnostics::codes::DiagnosticSeverity as CodesSeverity;
use perl_diagnostics::codes::DiagnosticTag as CodesTag;
use perl_diagnostics::types::DiagnosticSeverity as TypesSeverity;
use perl_diagnostics::types::DiagnosticTag as TypesTag;

// Test 1: Severity — cross-path assignment compiles
#[test]
fn severity_is_unified_single_type() {
    let from_codes = CodesSeverity::Error;
    // This assignment must compile — proves they are the same type
    let _as_types: TypesSeverity = from_codes;

    // LSP spec verification
    assert_eq!(from_codes as u8, 1);
}

// Test 2: Severity — TypeId must be identical
#[test]
fn severity_has_identical_type_id_across_module_paths() {
    let codes_id = std::any::TypeId::of::<CodesSeverity>();
    let types_id = std::any::TypeId::of::<TypesSeverity>();

    assert_eq!(
        codes_id, types_id,
        "DiagnosticSeverity must be unified — same TypeId from both module paths"
    );
}

// Test 3: Severity — all variants are accessible from both paths
#[test]
fn severity_all_variants_accessible_from_both_paths() {
    let _error_codes = CodesSeverity::Error;
    let _error_types = TypesSeverity::Error;

    let _warning_codes = CodesSeverity::Warning;
    let _warning_types = TypesSeverity::Warning;

    let _info_codes = CodesSeverity::Information;
    let _info_types = TypesSeverity::Information;

    let _hint_codes = CodesSeverity::Hint;
    let _hint_types = TypesSeverity::Hint;
}

// Test 4: Severity — cross-path equality works
#[test]
fn severity_equality_across_module_paths() {
    let from_codes = CodesSeverity::Warning;
    let from_types = TypesSeverity::Warning;

    // Must be equal (same type, same variant)
    assert_eq!(from_codes, from_types);
}

// Test 5: Severity — ordering works across module paths
#[test]
fn severity_ordering_across_module_paths() {
    let error_codes = CodesSeverity::Error;
    let warning_types = TypesSeverity::Warning;

    // LSP ordering: Error < Warning < Information < Hint
    assert!(error_codes < warning_types);
}

// Test 6: Tag — cross-path assignment compiles
#[test]
fn tag_is_unified_single_type() {
    let from_codes = CodesTag::Unnecessary;
    // This assignment must compile — proves they are the same type
    let _as_types: TypesTag = from_codes;
}

// Test 7: Tag — TypeId must be identical
#[test]
fn tag_has_identical_type_id_across_module_paths() {
    let codes_id = std::any::TypeId::of::<CodesTag>();
    let types_id = std::any::TypeId::of::<TypesTag>();

    assert_eq!(
        codes_id, types_id,
        "DiagnosticTag must be unified — same TypeId from both module paths"
    );
}

// Test 8: Tag — all variants are accessible from both paths
#[test]
fn tag_all_variants_accessible_from_both_paths() {
    let _unnecessary_codes = CodesTag::Unnecessary;
    let _unnecessary_types = TypesTag::Unnecessary;

    let _deprecated_codes = CodesTag::Deprecated;
    let _deprecated_types = TypesTag::Deprecated;
}

// Test 9: Tag — cross-path equality works
#[test]
fn tag_equality_across_module_paths() {
    let from_codes = CodesTag::Deprecated;
    let from_types = TypesTag::Deprecated;

    assert_eq!(from_codes, from_types);
}

// Test 10: Diagnostic struct binds unified severity
#[test]
fn diagnostic_struct_binds_unified_severity_type() {
    use perl_diagnostics::types::Diagnostic;

    let diag = Diagnostic { severity: CodesSeverity::Error, ..Default::default() };

    // The field must accept CodesSeverity (same as TypesSeverity)
    let _severity: TypesSeverity = diag.severity;
}

// Test 11: Mixed severity assignments in same function
#[test]
fn mixed_severity_assignments_from_both_paths() {
    let mut severities: Vec<TypesSeverity> = vec![];

    // Add from codes path
    severities.push(CodesSeverity::Error);
    severities.push(CodesSeverity::Warning);

    // Add from types path
    severities.push(TypesSeverity::Information);
    severities.push(TypesSeverity::Hint);

    assert_eq!(severities.len(), 4);
}

// Test 12: Mixed tag assignments in same function
#[test]
fn mixed_tag_assignments_from_both_paths() {
    let mut tags: Vec<TypesTag> = vec![];

    // Add from codes path
    tags.push(CodesTag::Unnecessary);
    tags.push(CodesTag::Deprecated);

    // Add from types path
    tags.push(TypesTag::Unnecessary);
    tags.push(TypesTag::Deprecated);

    assert_eq!(tags.len(), 4);
}

// Test 13: Severity — to_lsp_value() available from both paths
#[test]
fn severity_to_lsp_value_works_from_both_paths() {
    use perl_diagnostics::codes::DiagnosticSeverity;

    // Both paths should have to_lsp_value() method
    let codes_lsp = CodesSeverity::Error.to_lsp_value();
    let types_lsp = TypesSeverity::Error.to_lsp_value();

    // LSP values must match
    assert_eq!(codes_lsp, types_lsp);
    assert_eq!(codes_lsp, 1);
}

// Test 14: Tag — to_lsp_value() available from both paths
#[test]
fn tag_to_lsp_value_works_from_both_paths() {
    // Both paths should have to_lsp_value() method
    let codes_lsp = CodesTag::Unnecessary.to_lsp_value();
    let types_lsp = TypesTag::Unnecessary.to_lsp_value();

    // LSP values must match
    assert_eq!(codes_lsp, types_lsp);
    assert_eq!(codes_lsp, 1);
}

// Test 15: Severity — Display trait available
#[test]
fn severity_display_works() {
    use std::fmt::Display;

    let error = CodesSeverity::Error;
    let display_str = format!("{}", error);

    assert!(!display_str.is_empty());
}

// Test 16: Tag — Display trait available
#[test]
fn tag_display_works() {
    use std::fmt::Display;

    let tag = CodesTag::Unnecessary;
    let display_str = format!("{}", tag);

    assert!(!display_str.is_empty());
}

// Test 17: Severity — Debug trait available
#[test]
fn severity_debug_works() {
    use std::fmt::Debug;

    let error = CodesSeverity::Error;
    let debug_str = format!("{:?}", error);

    assert!(!debug_str.is_empty());
}

// Test 18: Tag — Debug trait available
#[test]
fn tag_debug_works() {
    use std::fmt::Debug;

    let tag = CodesTag::Unnecessary;
    let debug_str = format!("{:?}", tag);

    assert!(!debug_str.is_empty());
}

// Test 19: Severity — Hash trait available (Copy types are always Hashable)
#[test]
fn severity_hash_works() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(CodesSeverity::Error);
    set.insert(TypesSeverity::Error); // Should be same as codes version
    set.insert(CodesSeverity::Warning);

    // Only 2 unique values (Error inserted twice, Warning once)
    assert_eq!(set.len(), 2);
}

// Test 20: Tag — Hash trait available
#[test]
fn tag_hash_works() {
    use std::collections::HashSet;

    let mut set = HashSet::new();
    set.insert(CodesTag::Unnecessary);
    set.insert(TypesTag::Unnecessary); // Should be same as codes version
    set.insert(CodesTag::Deprecated);

    // Only 2 unique values
    assert_eq!(set.len(), 2);
}
