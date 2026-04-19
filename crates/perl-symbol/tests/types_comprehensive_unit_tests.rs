//! Comprehensive unit tests for the `perl-symbol-types` crate.
//!
//! Covers: VarKind, SymbolKind, LSP mappings, derived traits, serde round-trips,
//! category predicates, convenience constructors, and edge cases.

use std::collections::HashSet;

use perl_symbol::{SymbolKind, VarKind};

// ---------------------------------------------------------------------------
// Helper: every SymbolKind variant for exhaustive iteration
// ---------------------------------------------------------------------------

fn all_symbol_kinds() -> Vec<SymbolKind> {
    vec![
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Variable(VarKind::Scalar),
        SymbolKind::Variable(VarKind::Array),
        SymbolKind::Variable(VarKind::Hash),
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ]
}

fn all_var_kinds() -> Vec<VarKind> {
    vec![VarKind::Scalar, VarKind::Array, VarKind::Hash]
}

// ===========================================================================
// VarKind tests
// ===========================================================================

#[test]
fn varkind_sigil_scalar() -> Result<(), String> {
    let s = VarKind::Scalar.sigil();
    if s != "$" {
        return Err(format!("expected '$', got '{s}'"));
    }
    Ok(())
}

#[test]
fn varkind_sigil_array() -> Result<(), String> {
    let s = VarKind::Array.sigil();
    if s != "@" {
        return Err(format!("expected '@', got '{s}'"));
    }
    Ok(())
}

#[test]
fn varkind_sigil_hash() -> Result<(), String> {
    let s = VarKind::Hash.sigil();
    if s != "%" {
        return Err(format!("expected '%', got '{s}'"));
    }
    Ok(())
}

#[test]
fn varkind_all_sigils_are_single_char() -> Result<(), String> {
    for vk in all_var_kinds() {
        let s = vk.sigil();
        if s.len() != 1 {
            return Err(format!("sigil for {vk:?} is not a single char: '{s}'"));
        }
    }
    Ok(())
}

#[test]
fn varkind_sigils_are_unique() -> Result<(), String> {
    let sigils: Vec<&str> = all_var_kinds().iter().map(|vk| vk.sigil()).collect();
    let unique: HashSet<&&str> = sigils.iter().collect();
    if unique.len() != sigils.len() {
        return Err(format!("duplicate sigils found: {sigils:?}"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// VarKind: derived traits
// ---------------------------------------------------------------------------

#[test]
fn varkind_clone_copy() -> Result<(), String> {
    let a = VarKind::Scalar;
    let b = a; // Copy
    let c = a; // Clone
    if a != b || a != c {
        return Err("Clone/Copy mismatch".into());
    }
    Ok(())
}

#[test]
fn varkind_eq_and_ne() -> Result<(), String> {
    if VarKind::Scalar != VarKind::Scalar {
        return Err("Scalar != Scalar".into());
    }
    if VarKind::Scalar == VarKind::Array {
        return Err("Scalar == Array".into());
    }
    if VarKind::Array == VarKind::Hash {
        return Err("Array == Hash".into());
    }
    Ok(())
}

#[test]
fn varkind_hash_trait() -> Result<(), String> {
    let mut set = HashSet::new();
    set.insert(VarKind::Scalar);
    set.insert(VarKind::Array);
    set.insert(VarKind::Hash);
    set.insert(VarKind::Scalar); // duplicate
    if set.len() != 3 {
        return Err(format!("expected 3 unique VarKinds, got {}", set.len()));
    }
    Ok(())
}

#[test]
fn varkind_debug_format() -> Result<(), String> {
    let dbg = format!("{:?}", VarKind::Scalar);
    if !dbg.contains("Scalar") {
        return Err(format!("Debug format missing 'Scalar': '{dbg}'"));
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// VarKind: serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn varkind_serde_roundtrip() -> Result<(), Box<dyn std::error::Error>> {
    for vk in all_var_kinds() {
        let json = serde_json::to_string(&vk)?;
        let back: VarKind = serde_json::from_str(&json)?;
        if back != vk {
            return Err(format!("round-trip failed for {vk:?}: json={json}").into());
        }
    }
    Ok(())
}

#[test]
fn varkind_serde_json_values() -> Result<(), Box<dyn std::error::Error>> {
    let json_scalar = serde_json::to_string(&VarKind::Scalar)?;
    let json_array = serde_json::to_string(&VarKind::Array)?;
    let json_hash = serde_json::to_string(&VarKind::Hash)?;

    // Verify they produce distinct JSON representations
    let unique: HashSet<&str> = [
        json_scalar.as_str(),
        json_array.as_str(),
        json_hash.as_str(),
    ]
    .into_iter()
    .collect();
    if unique.len() != 3 {
        return Err("VarKind variants serialize to non-unique JSON".into());
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: LSP workspace mapping (to_lsp_kind)
// ===========================================================================

#[test]
fn lsp_kind_package() -> Result<(), String> {
    if SymbolKind::Package.to_lsp_kind() != 2 {
        return Err("Package should map to 2 (Module)".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_class() -> Result<(), String> {
    if SymbolKind::Class.to_lsp_kind() != 5 {
        return Err("Class should map to 5".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_role() -> Result<(), String> {
    if SymbolKind::Role.to_lsp_kind() != 8 {
        return Err("Role should map to 8 (Interface)".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_subroutine() -> Result<(), String> {
    if SymbolKind::Subroutine.to_lsp_kind() != 12 {
        return Err("Subroutine should map to 12 (Function)".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_method() -> Result<(), String> {
    if SymbolKind::Method.to_lsp_kind() != 6 {
        return Err("Method should map to 6".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_all_variables_are_13() -> Result<(), String> {
    for vk in all_var_kinds() {
        let kind = SymbolKind::Variable(vk).to_lsp_kind();
        if kind != 13 {
            return Err(format!(
                "Variable({vk:?}) workspace LSP kind should be 13, got {kind}"
            ));
        }
    }
    Ok(())
}

#[test]
fn lsp_kind_constant() -> Result<(), String> {
    if SymbolKind::Constant.to_lsp_kind() != 14 {
        return Err("Constant should map to 14".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_import() -> Result<(), String> {
    if SymbolKind::Import.to_lsp_kind() != 2 {
        return Err("Import should map to 2 (Module)".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_export() -> Result<(), String> {
    if SymbolKind::Export.to_lsp_kind() != 12 {
        return Err("Export should map to 12 (Function)".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_label() -> Result<(), String> {
    if SymbolKind::Label.to_lsp_kind() != 20 {
        return Err("Label should map to 20 (Key)".into());
    }
    Ok(())
}

#[test]
fn lsp_kind_format() -> Result<(), String> {
    if SymbolKind::Format.to_lsp_kind() != 23 {
        return Err("Format should map to 23 (Struct)".into());
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: LSP document-symbol mapping (to_lsp_kind_document_symbol)
// ===========================================================================

#[test]
fn doc_sym_non_variable_matches_workspace() -> Result<(), String> {
    let non_vars = [
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for sk in non_vars {
        let ws = sk.to_lsp_kind();
        let ds = sk.to_lsp_kind_document_symbol();
        if ws != ds {
            return Err(format!(
                "{sk:?}: workspace={ws} but document_symbol={ds}, expected equal"
            ));
        }
    }
    Ok(())
}

#[test]
fn doc_sym_scalar_is_variable_13() -> Result<(), String> {
    let v = SymbolKind::Variable(VarKind::Scalar).to_lsp_kind_document_symbol();
    if v != 13 {
        return Err(format!("Scalar doc sym expected 13, got {v}"));
    }
    Ok(())
}

#[test]
fn doc_sym_array_is_18() -> Result<(), String> {
    let v = SymbolKind::Variable(VarKind::Array).to_lsp_kind_document_symbol();
    if v != 18 {
        return Err(format!("Array doc sym expected 18, got {v}"));
    }
    Ok(())
}

#[test]
fn doc_sym_hash_is_19() -> Result<(), String> {
    let v = SymbolKind::Variable(VarKind::Hash).to_lsp_kind_document_symbol();
    if v != 19 {
        return Err(format!("Hash doc sym expected 19, got {v}"));
    }
    Ok(())
}

#[test]
fn doc_sym_variable_types_are_distinct() -> Result<(), String> {
    let vals: Vec<u32> = all_var_kinds()
        .iter()
        .map(|vk| SymbolKind::Variable(*vk).to_lsp_kind_document_symbol())
        .collect();
    let unique: HashSet<&u32> = vals.iter().collect();
    if unique.len() != vals.len() {
        return Err(format!(
            "document symbol variable types not distinct: {vals:?}"
        ));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: sigil()
// ===========================================================================

#[test]
fn sigil_returns_some_for_variables() -> Result<(), String> {
    for vk in all_var_kinds() {
        if SymbolKind::Variable(vk).sigil().is_none() {
            return Err(format!("Variable({vk:?}).sigil() returned None"));
        }
    }
    Ok(())
}

#[test]
fn sigil_returns_none_for_non_variables() -> Result<(), String> {
    let non_vars = [
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for sk in non_vars {
        if sk.sigil().is_some() {
            return Err(format!("{sk:?}.sigil() should be None"));
        }
    }
    Ok(())
}

#[test]
fn sigil_matches_varkind_sigil() -> Result<(), String> {
    for vk in all_var_kinds() {
        let sym_sigil = SymbolKind::Variable(vk).sigil();
        let vk_sigil = vk.sigil();
        if sym_sigil != Some(vk_sigil) {
            return Err(format!(
                "Variable({vk:?}).sigil()={sym_sigil:?} but VarKind.sigil()='{vk_sigil}'"
            ));
        }
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: is_variable()
// ===========================================================================

#[test]
fn is_variable_true_for_all_var_kinds() -> Result<(), String> {
    for vk in all_var_kinds() {
        if !SymbolKind::Variable(vk).is_variable() {
            return Err(format!("Variable({vk:?}).is_variable() returned false"));
        }
    }
    Ok(())
}

#[test]
fn is_variable_false_for_non_variables() -> Result<(), String> {
    let non_vars = [
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for sk in non_vars {
        if sk.is_variable() {
            return Err(format!("{sk:?}.is_variable() should be false"));
        }
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: is_callable()
// ===========================================================================

#[test]
fn is_callable_true_for_subroutine_and_method() -> Result<(), String> {
    if !SymbolKind::Subroutine.is_callable() {
        return Err("Subroutine.is_callable() returned false".into());
    }
    if !SymbolKind::Method.is_callable() {
        return Err("Method.is_callable() returned false".into());
    }
    Ok(())
}

#[test]
fn is_callable_false_for_non_callables() -> Result<(), String> {
    let non_callables = [
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Variable(VarKind::Scalar),
        SymbolKind::Variable(VarKind::Array),
        SymbolKind::Variable(VarKind::Hash),
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for sk in non_callables {
        if sk.is_callable() {
            return Err(format!("{sk:?}.is_callable() should be false"));
        }
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: is_namespace()
// ===========================================================================

#[test]
fn is_namespace_true_for_package_class_role() -> Result<(), String> {
    for sk in [SymbolKind::Package, SymbolKind::Class, SymbolKind::Role] {
        if !sk.is_namespace() {
            return Err(format!("{sk:?}.is_namespace() returned false"));
        }
    }
    Ok(())
}

#[test]
fn is_namespace_false_for_non_namespaces() -> Result<(), String> {
    let non_ns = [
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Variable(VarKind::Scalar),
        SymbolKind::Variable(VarKind::Array),
        SymbolKind::Variable(VarKind::Hash),
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for sk in non_ns {
        if sk.is_namespace() {
            return Err(format!("{sk:?}.is_namespace() should be false"));
        }
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: convenience constructors
// ===========================================================================

#[test]
fn scalar_constructor() -> Result<(), String> {
    if SymbolKind::scalar() != SymbolKind::Variable(VarKind::Scalar) {
        return Err("scalar() mismatch".into());
    }
    Ok(())
}

#[test]
fn array_constructor() -> Result<(), String> {
    if SymbolKind::array() != SymbolKind::Variable(VarKind::Array) {
        return Err("array() mismatch".into());
    }
    Ok(())
}

#[test]
fn hash_constructor() -> Result<(), String> {
    if SymbolKind::hash() != SymbolKind::Variable(VarKind::Hash) {
        return Err("hash() mismatch".into());
    }
    Ok(())
}

#[test]
fn constructors_are_variables() -> Result<(), String> {
    for sk in [
        SymbolKind::scalar(),
        SymbolKind::array(),
        SymbolKind::hash(),
    ] {
        if !sk.is_variable() {
            return Err(format!("{sk:?} from constructor is not a variable"));
        }
    }
    Ok(())
}

#[test]
fn constructors_have_sigils() -> Result<(), String> {
    let expected = [
        ("$", SymbolKind::scalar()),
        ("@", SymbolKind::array()),
        ("%", SymbolKind::hash()),
    ];
    for (exp, sk) in expected {
        match sk.sigil() {
            Some(s) if s == exp => {}
            other => {
                return Err(format!(
                    "{sk:?}.sigil() = {other:?}, expected Some(\"{exp}\")"
                ));
            }
        }
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: derived traits
// ===========================================================================

#[test]
fn symbolkind_clone_copy() -> Result<(), String> {
    let a = SymbolKind::Subroutine;
    let b = a; // Copy
    let c = a; // Clone
    if a != b || a != c {
        return Err("Clone/Copy mismatch".into());
    }
    Ok(())
}

#[test]
fn symbolkind_eq_ne() -> Result<(), String> {
    if SymbolKind::Package != SymbolKind::Package {
        return Err("Package != Package".into());
    }
    if SymbolKind::Package == SymbolKind::Class {
        return Err("Package == Class".into());
    }
    // Variable equality considers inner VarKind
    if SymbolKind::Variable(VarKind::Scalar) == SymbolKind::Variable(VarKind::Array) {
        return Err("Scalar variable == Array variable".into());
    }
    Ok(())
}

#[test]
fn symbolkind_hash_trait() -> Result<(), String> {
    let kinds = all_symbol_kinds();
    let mut set = HashSet::new();
    for sk in &kinds {
        set.insert(*sk);
    }
    // All 13 variants should be unique
    if set.len() != kinds.len() {
        return Err(format!(
            "expected {} unique SymbolKinds, got {}",
            kinds.len(),
            set.len()
        ));
    }
    Ok(())
}

#[test]
fn symbolkind_debug_contains_variant_name() -> Result<(), String> {
    let cases = [
        (SymbolKind::Package, "Package"),
        (SymbolKind::Class, "Class"),
        (SymbolKind::Role, "Role"),
        (SymbolKind::Subroutine, "Subroutine"),
        (SymbolKind::Method, "Method"),
        (SymbolKind::Constant, "Constant"),
        (SymbolKind::Import, "Import"),
        (SymbolKind::Export, "Export"),
        (SymbolKind::Label, "Label"),
        (SymbolKind::Format, "Format"),
        (SymbolKind::Variable(VarKind::Scalar), "Scalar"),
    ];
    for (sk, expected_substr) in cases {
        let dbg = format!("{sk:?}");
        if !dbg.contains(expected_substr) {
            return Err(format!(
                "Debug of {sk:?} missing '{expected_substr}': '{dbg}'"
            ));
        }
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// SymbolKind: serde round-trip
// ---------------------------------------------------------------------------

#[test]
fn symbolkind_serde_roundtrip_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    for sk in all_symbol_kinds() {
        let json = serde_json::to_string(&sk)?;
        let back: SymbolKind = serde_json::from_str(&json)?;
        if back != sk {
            return Err(format!("round-trip failed for {sk:?}: json={json}").into());
        }
    }
    Ok(())
}

#[test]
fn symbolkind_serde_all_variants_distinct() -> Result<(), Box<dyn std::error::Error>> {
    let jsons: Vec<String> = all_symbol_kinds()
        .iter()
        .map(serde_json::to_string)
        .collect::<Result<Vec<_>, _>>()?;
    let unique: HashSet<&String> = jsons.iter().collect();
    if unique.len() != jsons.len() {
        return Err("some SymbolKind variants serialize to the same JSON".into());
    }
    Ok(())
}

#[test]
fn symbolkind_deserialize_rejects_invalid() -> Result<(), String> {
    let bad = "\"NotARealVariant\"";
    let result: Result<SymbolKind, _> = serde_json::from_str(bad);
    if result.is_ok() {
        return Err("expected deserialization error for invalid variant".into());
    }
    Ok(())
}

// ===========================================================================
// Cross-cutting / edge cases
// ===========================================================================

#[test]
fn lsp_kinds_are_in_valid_range() -> Result<(), String> {
    // LSP SymbolKind values are 1..=26 per the LSP spec
    for sk in all_symbol_kinds() {
        let ws = sk.to_lsp_kind();
        let ds = sk.to_lsp_kind_document_symbol();
        if !(1..=26).contains(&ws) {
            return Err(format!("{sk:?}.to_lsp_kind()={ws} out of LSP range 1..=26"));
        }
        if !(1..=26).contains(&ds) {
            return Err(format!(
                "{sk:?}.to_lsp_kind_document_symbol()={ds} out of LSP range 1..=26"
            ));
        }
    }
    Ok(())
}

#[test]
fn import_and_package_share_lsp_kind() -> Result<(), String> {
    // Both map to Module (2) — intentional per spec
    if SymbolKind::Import.to_lsp_kind() != SymbolKind::Package.to_lsp_kind() {
        return Err("Import and Package should share LSP kind 2 (Module)".into());
    }
    Ok(())
}

#[test]
fn export_and_subroutine_share_lsp_kind() -> Result<(), String> {
    // Both map to Function (12) — intentional per spec
    if SymbolKind::Export.to_lsp_kind() != SymbolKind::Subroutine.to_lsp_kind() {
        return Err("Export and Subroutine should share LSP kind 12 (Function)".into());
    }
    Ok(())
}

#[test]
fn predicates_are_mutually_exclusive_for_leaf_types() -> Result<(), String> {
    // For non-variable leaf types, at most one category predicate should be true
    let leaf_kinds = [
        SymbolKind::Package,
        SymbolKind::Class,
        SymbolKind::Role,
        SymbolKind::Subroutine,
        SymbolKind::Method,
        SymbolKind::Constant,
        SymbolKind::Import,
        SymbolKind::Export,
        SymbolKind::Label,
        SymbolKind::Format,
    ];
    for sk in leaf_kinds {
        let count = [sk.is_variable(), sk.is_callable(), sk.is_namespace()]
            .iter()
            .filter(|&&b| b)
            .count();
        if count > 1 {
            return Err(format!(
                "{sk:?} matches {count} category predicates (should be ≤1)"
            ));
        }
    }
    Ok(())
}

#[test]
fn variable_is_not_callable_or_namespace() -> Result<(), String> {
    for vk in all_var_kinds() {
        let sk = SymbolKind::Variable(vk);
        if sk.is_callable() || sk.is_namespace() {
            return Err(format!(
                "Variable({vk:?}) should not be callable or namespace"
            ));
        }
    }
    Ok(())
}

#[test]
fn every_variant_has_nonzero_lsp_kind() -> Result<(), String> {
    for sk in all_symbol_kinds() {
        if sk.to_lsp_kind() == 0 {
            return Err(format!("{sk:?}.to_lsp_kind() == 0"));
        }
        if sk.to_lsp_kind_document_symbol() == 0 {
            return Err(format!("{sk:?}.to_lsp_kind_document_symbol() == 0"));
        }
    }
    Ok(())
}
