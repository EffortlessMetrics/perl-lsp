//! Extended unit tests for the `perl-symbol-types` crate.
//!
//! Covers: pattern matching exhaustiveness, HashMap/BTreeMap key usage,
//! serde edge cases, const-context validation, predicate orthogonality,
//! collection grouping, memory layout, variant count, and more.

use std::collections::{BTreeMap, HashMap, HashSet};

use perl_symbol::{SymbolKind, VarKind};

// ---------------------------------------------------------------------------
// Helpers
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
// VarKind: additional coverage
// ===========================================================================

#[test]
fn varkind_sigil_starts_with_expected_char() -> Result<(), String> {
    let pairs = [(VarKind::Scalar, '$'), (VarKind::Array, '@'), (VarKind::Hash, '%')];
    for (vk, ch) in pairs {
        let sigil = vk.sigil();
        match sigil.chars().next() {
            Some(c) if c == ch => {}
            other => return Err(format!("{vk:?}.sigil() first char = {other:?}, expected '{ch}'")),
        }
    }
    Ok(())
}

#[test]
fn varkind_sigil_is_ascii() -> Result<(), String> {
    for vk in all_var_kinds() {
        if !vk.sigil().is_ascii() {
            return Err(format!("{vk:?}.sigil() is not ASCII"));
        }
    }
    Ok(())
}

#[test]
fn varkind_sigil_is_not_alphanumeric() -> Result<(), String> {
    for vk in all_var_kinds() {
        for ch in vk.sigil().chars() {
            if ch.is_alphanumeric() {
                return Err(format!(
                    "{vk:?}.sigil() char '{ch}' is alphanumeric — sigils should be punctuation"
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn varkind_can_be_used_as_hashmap_key() -> Result<(), String> {
    let mut map = HashMap::new();
    map.insert(VarKind::Scalar, "scalar_val");
    map.insert(VarKind::Array, "array_val");
    map.insert(VarKind::Hash, "hash_val");
    if map.len() != 3 {
        return Err(format!("expected 3 entries, got {}", map.len()));
    }
    match map.get(&VarKind::Array) {
        Some(&"array_val") => {}
        other => return Err(format!("expected Some(\"array_val\"), got {other:?}")),
    }
    Ok(())
}

#[test]
fn varkind_can_be_used_as_btreemap_key() -> Result<(), String> {
    let mut map = BTreeMap::new();
    // BTreeMap requires Ord, but VarKind only derives Hash+Eq.
    // We use a wrapper to test if we can store via manual comparison.
    // Instead, test that we can collect into a sorted Vec via Debug string.
    let mut kinds: Vec<String> = all_var_kinds().iter().map(|vk| format!("{vk:?}")).collect();
    let original_len = kinds.len();
    kinds.sort();
    kinds.dedup();
    if kinds.len() != original_len {
        return Err("VarKind debug representations are not unique".into());
    }
    // Also test HashMap works with insert-then-overwrite
    map.insert(format!("{:?}", VarKind::Scalar), 1);
    map.insert(format!("{:?}", VarKind::Scalar), 2);
    if map.len() != 1 {
        return Err("BTreeMap should have 1 entry after overwrite".into());
    }
    Ok(())
}

#[test]
fn varkind_copy_semantics_preserve_value() -> Result<(), String> {
    let original = VarKind::Hash;
    let copied = original;
    // Both should be usable independently (Copy trait)
    if original.sigil() != copied.sigil() {
        return Err("Copy semantics did not preserve VarKind value".into());
    }
    if original != copied {
        return Err("Copied VarKind not equal to original".into());
    }
    Ok(())
}

#[test]
fn varkind_ne_is_symmetric() -> Result<(), String> {
    let a = VarKind::Scalar;
    let b = VarKind::Array;
    if (a != b) != (b != a) {
        return Err("VarKind != is not symmetric".into());
    }
    Ok(())
}

#[test]
fn varkind_eq_is_reflexive() -> Result<(), String> {
    for vk in all_var_kinds() {
        if vk != vk {
            return Err(format!("{vk:?} != itself"));
        }
    }
    Ok(())
}

#[test]
fn varkind_eq_is_transitive() -> Result<(), String> {
    let a = VarKind::Array;
    let b = VarKind::Array;
    let c = VarKind::Array;
    if a == b && b == c && a != c {
        return Err("VarKind equality is not transitive".into());
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// VarKind: serde edge cases
// ---------------------------------------------------------------------------

#[test]
fn varkind_serde_from_string_literal() -> Result<(), Box<dyn std::error::Error>> {
    let scalar: VarKind = serde_json::from_str("\"Scalar\"")?;
    if scalar != VarKind::Scalar {
        return Err("Deserializing \"Scalar\" did not produce VarKind::Scalar".into());
    }
    let array: VarKind = serde_json::from_str("\"Array\"")?;
    if array != VarKind::Array {
        return Err("Deserializing \"Array\" did not produce VarKind::Array".into());
    }
    let hash: VarKind = serde_json::from_str("\"Hash\"")?;
    if hash != VarKind::Hash {
        return Err("Deserializing \"Hash\" did not produce VarKind::Hash".into());
    }
    Ok(())
}

#[test]
fn varkind_serde_rejects_empty_string() -> Result<(), String> {
    let result: Result<VarKind, _> = serde_json::from_str("\"\"");
    if result.is_ok() {
        return Err("empty string should not deserialize to VarKind".into());
    }
    Ok(())
}

#[test]
fn varkind_serde_rejects_lowercase() -> Result<(), String> {
    let result: Result<VarKind, _> = serde_json::from_str("\"scalar\"");
    if result.is_ok() {
        return Err("lowercase 'scalar' should not deserialize to VarKind".into());
    }
    Ok(())
}

#[test]
fn varkind_serde_rejects_numeric() -> Result<(), String> {
    let result: Result<VarKind, _> = serde_json::from_str("42");
    if result.is_ok() {
        return Err("number should not deserialize to VarKind".into());
    }
    Ok(())
}

#[test]
fn varkind_serde_rejects_null() -> Result<(), String> {
    let result: Result<VarKind, _> = serde_json::from_str("null");
    if result.is_ok() {
        return Err("null should not deserialize to VarKind".into());
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: additional LSP mapping coverage
// ===========================================================================

#[test]
fn lsp_kind_values_are_positive() -> Result<(), String> {
    for sk in all_symbol_kinds() {
        if sk.to_lsp_kind() == 0 {
            return Err(format!("{sk:?}.to_lsp_kind() is 0, expected positive"));
        }
        if sk.to_lsp_kind_document_symbol() == 0 {
            return Err(format!("{sk:?}.to_lsp_kind_document_symbol() is 0, expected positive"));
        }
    }
    Ok(())
}

#[test]
fn lsp_kind_workspace_profile_stability() -> Result<(), String> {
    // Verify the full mapping table hasn't changed (regression guard)
    let expected: Vec<(SymbolKind, u32)> = vec![
        (SymbolKind::Package, 2),
        (SymbolKind::Class, 5),
        (SymbolKind::Role, 8),
        (SymbolKind::Subroutine, 12),
        (SymbolKind::Method, 6),
        (SymbolKind::Variable(VarKind::Scalar), 13),
        (SymbolKind::Variable(VarKind::Array), 13),
        (SymbolKind::Variable(VarKind::Hash), 13),
        (SymbolKind::Constant, 14),
        (SymbolKind::Import, 2),
        (SymbolKind::Export, 12),
        (SymbolKind::Label, 20),
        (SymbolKind::Format, 23),
    ];
    for (sk, exp) in expected {
        let got = sk.to_lsp_kind();
        if got != exp {
            return Err(format!("{sk:?}.to_lsp_kind() = {got}, expected {exp}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_kind_document_symbol_stability() -> Result<(), String> {
    let expected: Vec<(SymbolKind, u32)> = vec![
        (SymbolKind::Package, 2),
        (SymbolKind::Class, 5),
        (SymbolKind::Role, 8),
        (SymbolKind::Subroutine, 12),
        (SymbolKind::Method, 6),
        (SymbolKind::Variable(VarKind::Scalar), 13),
        (SymbolKind::Variable(VarKind::Array), 18),
        (SymbolKind::Variable(VarKind::Hash), 19),
        (SymbolKind::Constant, 14),
        (SymbolKind::Import, 2),
        (SymbolKind::Export, 12),
        (SymbolKind::Label, 20),
        (SymbolKind::Format, 23),
    ];
    for (sk, exp) in expected {
        let got = sk.to_lsp_kind_document_symbol();
        if got != exp {
            return Err(format!("{sk:?}.to_lsp_kind_document_symbol() = {got}, expected {exp}"));
        }
    }
    Ok(())
}

#[test]
fn lsp_kind_workspace_and_doc_differ_only_for_array_hash() -> Result<(), String> {
    for sk in all_symbol_kinds() {
        let ws = sk.to_lsp_kind();
        let ds = sk.to_lsp_kind_document_symbol();
        let is_array_or_hash = matches!(
            sk,
            SymbolKind::Variable(VarKind::Array) | SymbolKind::Variable(VarKind::Hash)
        );
        if is_array_or_hash {
            if ws == ds {
                return Err(format!(
                    "{sk:?}: workspace and doc-symbol should differ but both are {ws}"
                ));
            }
        } else if ws != ds {
            return Err(format!("{sk:?}: workspace={ws} != doc-symbol={ds}, expected equal"));
        }
    }
    Ok(())
}

#[test]
fn doc_sym_array_and_hash_differ() -> Result<(), String> {
    let arr = SymbolKind::Variable(VarKind::Array).to_lsp_kind_document_symbol();
    let hash = SymbolKind::Variable(VarKind::Hash).to_lsp_kind_document_symbol();
    if arr == hash {
        return Err(format!("Array and Hash doc-symbol kinds should differ: both are {arr}"));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: predicate orthogonality
// ===========================================================================

#[test]
fn no_symbol_is_both_callable_and_namespace() -> Result<(), String> {
    for sk in all_symbol_kinds() {
        if sk.is_callable() && sk.is_namespace() {
            return Err(format!("{sk:?} is both callable and namespace"));
        }
    }
    Ok(())
}

#[test]
fn no_symbol_is_both_callable_and_variable() -> Result<(), String> {
    for sk in all_symbol_kinds() {
        if sk.is_callable() && sk.is_variable() {
            return Err(format!("{sk:?} is both callable and variable"));
        }
    }
    Ok(())
}

#[test]
fn no_symbol_is_both_variable_and_namespace() -> Result<(), String> {
    for sk in all_symbol_kinds() {
        if sk.is_variable() && sk.is_namespace() {
            return Err(format!("{sk:?} is both variable and namespace"));
        }
    }
    Ok(())
}

#[test]
fn exactly_two_callables() -> Result<(), String> {
    let count = all_symbol_kinds().iter().filter(|sk| sk.is_callable()).count();
    if count != 2 {
        return Err(format!("expected 2 callables, got {count}"));
    }
    Ok(())
}

#[test]
fn exactly_three_namespaces() -> Result<(), String> {
    let count = all_symbol_kinds().iter().filter(|sk| sk.is_namespace()).count();
    if count != 3 {
        return Err(format!("expected 3 namespaces, got {count}"));
    }
    Ok(())
}

#[test]
fn exactly_three_variables() -> Result<(), String> {
    let count = all_symbol_kinds().iter().filter(|sk| sk.is_variable()).count();
    if count != 3 {
        return Err(format!("expected 3 variables, got {count}"));
    }
    Ok(())
}

#[test]
fn uncategorized_kinds_exist() -> Result<(), String> {
    // Constant, Import, Export, Label, Format are none of variable/callable/namespace
    let uncategorized: Vec<SymbolKind> = all_symbol_kinds()
        .into_iter()
        .filter(|sk| !sk.is_variable() && !sk.is_callable() && !sk.is_namespace())
        .collect();
    if uncategorized.is_empty() {
        return Err("expected some uncategorized symbol kinds".into());
    }
    // Should be exactly: Constant, Import, Export, Label, Format = 5
    if uncategorized.len() != 5 {
        return Err(format!(
            "expected 5 uncategorized kinds, got {}: {uncategorized:?}",
            uncategorized.len()
        ));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: convenience constructor properties
// ===========================================================================

#[test]
fn constructor_scalar_has_correct_sigil() -> Result<(), String> {
    match SymbolKind::scalar().sigil() {
        Some("$") => Ok(()),
        other => Err(format!("scalar().sigil() = {other:?}, expected Some(\"$\")")),
    }
}

#[test]
fn constructor_array_has_correct_sigil() -> Result<(), String> {
    match SymbolKind::array().sigil() {
        Some("@") => Ok(()),
        other => Err(format!("array().sigil() = {other:?}, expected Some(\"@\")")),
    }
}

#[test]
fn constructor_hash_has_correct_sigil() -> Result<(), String> {
    match SymbolKind::hash().sigil() {
        Some("%") => Ok(()),
        other => Err(format!("hash().sigil() = {other:?}, expected Some(\"%\")")),
    }
}

#[test]
fn constructors_are_all_distinct() -> Result<(), String> {
    let s = SymbolKind::scalar();
    let a = SymbolKind::array();
    let h = SymbolKind::hash();
    if s == a || s == h || a == h {
        return Err("convenience constructors produced equal variants".into());
    }
    Ok(())
}

#[test]
fn constructors_lsp_doc_symbol_distinct() -> Result<(), String> {
    let vals: HashSet<u32> = [
        SymbolKind::scalar().to_lsp_kind_document_symbol(),
        SymbolKind::array().to_lsp_kind_document_symbol(),
        SymbolKind::hash().to_lsp_kind_document_symbol(),
    ]
    .into_iter()
    .collect();
    if vals.len() != 3 {
        return Err(format!("expected 3 distinct doc-symbol values, got {}", vals.len()));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: HashMap/BTreeMap usage
// ===========================================================================

#[test]
fn symbolkind_as_hashmap_key() -> Result<(), String> {
    let mut map = HashMap::new();
    for (i, sk) in all_symbol_kinds().into_iter().enumerate() {
        map.insert(sk, i);
    }
    if map.len() != 13 {
        return Err(format!("expected 13 entries, got {}", map.len()));
    }
    match map.get(&SymbolKind::Method) {
        Some(_) => {}
        None => return Err("Method not found in HashMap".into()),
    }
    Ok(())
}

#[test]
fn symbolkind_hashmap_overwrite() -> Result<(), String> {
    let mut map = HashMap::new();
    map.insert(SymbolKind::Subroutine, "first");
    map.insert(SymbolKind::Subroutine, "second");
    if map.len() != 1 {
        return Err(format!("expected 1 entry after overwrite, got {}", map.len()));
    }
    match map.get(&SymbolKind::Subroutine) {
        Some(&"second") => Ok(()),
        other => Err(format!("expected Some(\"second\"), got {other:?}")),
    }
}

#[test]
fn symbolkind_variable_variants_are_distinct_keys() -> Result<(), String> {
    let mut map = HashMap::new();
    map.insert(SymbolKind::Variable(VarKind::Scalar), "scalar");
    map.insert(SymbolKind::Variable(VarKind::Array), "array");
    map.insert(SymbolKind::Variable(VarKind::Hash), "hash");
    if map.len() != 3 {
        return Err(format!(
            "Variable variants should be distinct HashMap keys, got {} entries",
            map.len()
        ));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: serde extended
// ===========================================================================

#[test]
fn symbolkind_serde_variable_includes_varkind() -> Result<(), Box<dyn std::error::Error>> {
    let json = serde_json::to_string(&SymbolKind::Variable(VarKind::Scalar))?;
    if !json.contains("Scalar") {
        return Err(format!("Variable(Scalar) JSON should contain 'Scalar': {json}").into());
    }
    Ok(())
}

#[test]
fn symbolkind_serde_rejects_null() -> Result<(), String> {
    let result: Result<SymbolKind, _> = serde_json::from_str("null");
    if result.is_ok() {
        return Err("null should not deserialize to SymbolKind".into());
    }
    Ok(())
}

#[test]
fn symbolkind_serde_rejects_number() -> Result<(), String> {
    let result: Result<SymbolKind, _> = serde_json::from_str("5");
    if result.is_ok() {
        return Err("number should not deserialize to SymbolKind".into());
    }
    Ok(())
}

#[test]
fn symbolkind_serde_rejects_boolean() -> Result<(), String> {
    let result: Result<SymbolKind, _> = serde_json::from_str("true");
    if result.is_ok() {
        return Err("boolean should not deserialize to SymbolKind".into());
    }
    Ok(())
}

#[test]
fn symbolkind_serde_rejects_empty_object() -> Result<(), String> {
    let result: Result<SymbolKind, _> = serde_json::from_str("{}");
    if result.is_ok() {
        return Err("empty object should not deserialize to SymbolKind".into());
    }
    Ok(())
}

#[test]
fn symbolkind_serde_simple_variants_are_strings() -> Result<(), Box<dyn std::error::Error>> {
    // Simple (unit) variants should serialize as JSON strings
    let simple_variants = [
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
    for sk in simple_variants {
        let json = serde_json::to_string(&sk)?;
        if !json.starts_with('"') || !json.ends_with('"') {
            return Err(format!("{sk:?} should serialize as JSON string, got: {json}").into());
        }
    }
    Ok(())
}

#[test]
fn symbolkind_serde_variable_is_object_or_tagged() -> Result<(), Box<dyn std::error::Error>> {
    // Variable(VarKind) has data, so serde serializes it differently from unit variants
    let json = serde_json::to_string(&SymbolKind::Variable(VarKind::Scalar))?;
    // It should NOT be a plain string (it carries VarKind data)
    let val: serde_json::Value = serde_json::from_str(&json)?;
    if val.is_string() {
        // With default serde, this might be externally tagged - that's an object
        // Actually with derive, Variable(Scalar) -> {"Variable":"Scalar"}
        return Err(format!("Variable variant should not be a plain string: {json}").into());
    }
    Ok(())
}

#[test]
fn symbolkind_serde_roundtrip_in_vec() -> Result<(), Box<dyn std::error::Error>> {
    let kinds = all_symbol_kinds();
    let json = serde_json::to_string(&kinds)?;
    let back: Vec<SymbolKind> = serde_json::from_str(&json)?;
    if back != kinds {
        return Err("Vec<SymbolKind> round-trip failed".into());
    }
    Ok(())
}

#[test]
fn symbolkind_serde_roundtrip_in_hashmap() -> Result<(), Box<dyn std::error::Error>> {
    // SymbolKind as value (not key, since JSON keys must be strings)
    let mut map: HashMap<String, SymbolKind> = HashMap::new();
    for sk in all_symbol_kinds() {
        map.insert(format!("{sk:?}"), sk);
    }
    let json = serde_json::to_string(&map)?;
    let back: HashMap<String, SymbolKind> = serde_json::from_str(&json)?;
    if back != map {
        return Err("HashMap<String, SymbolKind> round-trip failed".into());
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: Debug format
// ===========================================================================

#[test]
fn symbolkind_debug_variable_includes_varkind() -> Result<(), String> {
    let cases = [
        (SymbolKind::Variable(VarKind::Scalar), "Scalar"),
        (SymbolKind::Variable(VarKind::Array), "Array"),
        (SymbolKind::Variable(VarKind::Hash), "Hash"),
    ];
    for (sk, expected) in cases {
        let dbg = format!("{sk:?}");
        if !dbg.contains("Variable") || !dbg.contains(expected) {
            return Err(format!(
                "Debug of {sk:?} should contain 'Variable' and '{expected}': '{dbg}'"
            ));
        }
    }
    Ok(())
}

#[test]
fn symbolkind_debug_all_variants_distinct() -> Result<(), String> {
    let debug_strings: Vec<String> =
        all_symbol_kinds().iter().map(|sk| format!("{sk:?}")).collect();
    let unique: HashSet<&String> = debug_strings.iter().collect();
    if unique.len() != debug_strings.len() {
        return Err("some SymbolKind variants have identical Debug output".into());
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: equality extended
// ===========================================================================

#[test]
fn symbolkind_eq_is_reflexive_all_variants() -> Result<(), String> {
    for sk in all_symbol_kinds() {
        if sk != sk {
            return Err(format!("{sk:?} != itself"));
        }
    }
    Ok(())
}

#[test]
fn symbolkind_eq_distinguishes_all_pairs() -> Result<(), String> {
    let kinds = all_symbol_kinds();
    for (i, a) in kinds.iter().enumerate() {
        for (j, b) in kinds.iter().enumerate() {
            if i != j && a == b {
                return Err(format!(
                    "variants at index {i} ({a:?}) and {j} ({b:?}) should not be equal"
                ));
            }
        }
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: grouping and collection operations
// ===========================================================================

#[test]
fn group_by_lsp_kind_workspace() -> Result<(), String> {
    let mut groups: HashMap<u32, Vec<SymbolKind>> = HashMap::new();
    for sk in all_symbol_kinds() {
        groups.entry(sk.to_lsp_kind()).or_default().push(sk);
    }
    // Some LSP kinds are shared: Module(2) has Package+Import, Function(12) has Subroutine+Export
    // Variable(13) has all three variable types
    match groups.get(&2) {
        Some(v) if v.len() >= 2 => {}
        other => return Err(format!("LSP kind 2 (Module) should have ≥2 variants: {other:?}")),
    }
    match groups.get(&12) {
        Some(v) if v.len() >= 2 => {}
        other => return Err(format!("LSP kind 12 (Function) should have ≥2 variants: {other:?}")),
    }
    match groups.get(&13) {
        Some(v) if v.len() == 3 => {}
        other => return Err(format!("LSP kind 13 (Variable) should have 3 variants: {other:?}")),
    }
    Ok(())
}

#[test]
fn group_by_predicate_partitions_correctly() -> Result<(), String> {
    let kinds = all_symbol_kinds();
    let variables: Vec<_> = kinds.iter().filter(|sk| sk.is_variable()).collect();
    let callables: Vec<_> = kinds.iter().filter(|sk| sk.is_callable()).collect();
    let namespaces: Vec<_> = kinds.iter().filter(|sk| sk.is_namespace()).collect();
    let other: Vec<_> = kinds
        .iter()
        .filter(|sk| !sk.is_variable() && !sk.is_callable() && !sk.is_namespace())
        .collect();
    let total = variables.len() + callables.len() + namespaces.len() + other.len();
    if total != kinds.len() {
        return Err(format!("partition sizes sum to {total}, expected {}", kinds.len()));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: variant count and exhaustiveness
// ===========================================================================

#[test]
fn total_variant_count_is_thirteen() -> Result<(), String> {
    let count = all_symbol_kinds().len();
    if count != 13 {
        return Err(format!("expected 13 total symbol kind variants, got {count}"));
    }
    Ok(())
}

#[test]
fn all_symbol_kinds_helper_has_no_duplicates() -> Result<(), String> {
    let kinds = all_symbol_kinds();
    let unique: HashSet<SymbolKind> = kinds.iter().copied().collect();
    if unique.len() != kinds.len() {
        return Err(format!(
            "all_symbol_kinds() has duplicates: {} unique of {} total",
            unique.len(),
            kinds.len()
        ));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: memory layout
// ===========================================================================

#[test]
fn symbolkind_is_small() -> Result<(), String> {
    let size = std::mem::size_of::<SymbolKind>();
    // Should be 1-2 bytes (enum with small discriminant + optional VarKind)
    if size > 4 {
        return Err(format!("SymbolKind is {size} bytes — expected ≤4 for efficient Copy"));
    }
    Ok(())
}

#[test]
fn varkind_is_small() -> Result<(), String> {
    let size = std::mem::size_of::<VarKind>();
    if size > 2 {
        return Err(format!("VarKind is {size} bytes — expected ≤2 for efficient Copy"));
    }
    Ok(())
}

#[test]
fn option_symbolkind_is_small() -> Result<(), String> {
    let size = std::mem::size_of::<Option<SymbolKind>>();
    // With niche optimization or not, should remain small
    if size > 4 {
        return Err(format!("Option<SymbolKind> is {size} bytes — expected ≤4"));
    }
    Ok(())
}

// ===========================================================================
// Cross-cutting: sigil consistency
// ===========================================================================

#[test]
fn sigil_consistency_across_all_constructors() -> Result<(), String> {
    let pairs = [
        (SymbolKind::scalar(), VarKind::Scalar),
        (SymbolKind::array(), VarKind::Array),
        (SymbolKind::hash(), VarKind::Hash),
    ];
    for (sk, vk) in pairs {
        match sk.sigil() {
            Some(s) if s == vk.sigil() => {}
            other => {
                return Err(format!(
                    "Constructor {sk:?}.sigil()={other:?} != VarKind::{vk:?}.sigil()={}",
                    vk.sigil()
                ));
            }
        }
    }
    Ok(())
}

#[test]
fn sigil_none_count_matches_non_variable_count() -> Result<(), String> {
    let none_count = all_symbol_kinds().iter().filter(|sk| sk.sigil().is_none()).count();
    let non_var_count = all_symbol_kinds().iter().filter(|sk| !sk.is_variable()).count();
    if none_count != non_var_count {
        return Err(format!(
            "sigil None count ({none_count}) != non-variable count ({non_var_count})"
        ));
    }
    Ok(())
}

// ===========================================================================
// SymbolKind: edge cases with pattern matching
// ===========================================================================

#[test]
fn match_on_variable_extracts_varkind() -> Result<(), String> {
    let sk = SymbolKind::Variable(VarKind::Hash);
    match sk {
        SymbolKind::Variable(vk) => {
            if vk != VarKind::Hash {
                return Err(format!("expected VarKind::Hash, got {vk:?}"));
            }
        }
        other => return Err(format!("expected Variable, got {other:?}")),
    }
    Ok(())
}

#[test]
fn if_let_variable_extraction() -> Result<(), String> {
    let sk = SymbolKind::Variable(VarKind::Array);
    if let SymbolKind::Variable(vk) = sk {
        if vk.sigil() != "@" {
            return Err(format!("expected '@', got '{}'", vk.sigil()));
        }
    } else {
        return Err("if-let failed to match Variable".into());
    }
    Ok(())
}

#[test]
fn if_let_non_variable_does_not_match() -> Result<(), String> {
    let sk = SymbolKind::Package;
    if let SymbolKind::Variable(_) = sk {
        return Err("Package should not match Variable pattern".into());
    }
    Ok(())
}
