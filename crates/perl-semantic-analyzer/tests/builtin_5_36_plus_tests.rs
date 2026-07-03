//! Tests for Perl 5.36+ `use builtin` function documentation (issue #1765).
//!
//! Verifies that hover documentation is available for all functions introduced
//! by Perl 5.36–5.40 via `use builtin`, both as bare names (after import) and
//! via the `builtin::` qualified form.

use perl_semantic_analyzer::analysis::semantic::{get_builtin_documentation, is_builtin_function};
use perl_tdd_support::must_some;

// ── Perl 5.36: boolean values ────────────────────────────────────────────────

#[test]
fn builtin_true_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("true"));
    assert!(
        doc.signature.contains("true"),
        "true signature must contain 'true': {}",
        doc.signature
    );
    assert!(
        doc.description.contains("boolean"),
        "true description must mention boolean: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.36"),
        "true description must mention Perl 5.36: {}",
        doc.description
    );
}

#[test]
fn builtin_false_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("false"));
    assert!(
        doc.description.contains("boolean"),
        "false description must mention boolean: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.36"),
        "false description must mention Perl 5.36: {}",
        doc.description
    );
}

#[test]
fn builtin_is_bool_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("is_bool"));
    assert!(
        doc.signature.contains("VALUE"),
        "is_bool signature must show VALUE parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("boolean"),
        "is_bool description must mention boolean: {}",
        doc.description
    );
}

// ── Perl 5.36: reference utilities ───────────────────────────────────────────

#[test]
fn builtin_weaken_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("weaken"));
    assert!(
        doc.signature.contains("REF"),
        "weaken signature must show REF parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("weak"),
        "weaken description must mention weak reference: {}",
        doc.description
    );
}

#[test]
fn builtin_unweaken_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("unweaken"));
    assert!(
        doc.signature.contains("REF"),
        "unweaken signature must show REF parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("strong") || doc.description.contains("strengthen"),
        "unweaken description must mention restoring strong reference: {}",
        doc.description
    );
}

#[test]
fn builtin_is_weak_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("is_weak"));
    assert!(
        doc.signature.contains("REF"),
        "is_weak signature must show REF parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("weak"),
        "is_weak description must mention weak reference: {}",
        doc.description
    );
}

#[test]
fn builtin_refaddr_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("refaddr"));
    assert!(
        doc.signature.contains("REF"),
        "refaddr signature must show REF parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("address") || doc.description.contains("memory"),
        "refaddr description must mention memory address: {}",
        doc.description
    );
}

#[test]
fn builtin_reftype_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("reftype"));
    assert!(
        doc.signature.contains("REF"),
        "reftype signature must show REF parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("HASH") || doc.description.contains("type"),
        "reftype description must mention reference types: {}",
        doc.description
    );
}

// ── Perl 5.36: object utilities ──────────────────────────────────────────────

#[test]
fn builtin_blessed_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("blessed"));
    assert!(
        doc.signature.contains("EXPR"),
        "blessed signature must show EXPR parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("blessed") || doc.description.contains("package"),
        "blessed description must mention package or blessed reference: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.36"),
        "blessed description must mention Perl 5.36: {}",
        doc.description
    );
}

// ── Perl 5.38: math + string utilities ───────────────────────────────────────

#[test]
fn builtin_ceil_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("ceil"));
    assert!(
        doc.signature.contains("EXPR"),
        "ceil signature must show EXPR parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("ceil") || doc.description.contains("ceiling"),
        "ceil description must mention ceiling rounding: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.36"),
        "ceil description must mention Perl 5.36: {}",
        doc.description
    );
}

#[test]
fn builtin_floor_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("floor"));
    assert!(
        doc.signature.contains("EXPR"),
        "floor signature must show EXPR parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("floor"),
        "floor description must mention floor function: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.36"),
        "floor description must mention Perl 5.36: {}",
        doc.description
    );
}

#[test]
fn builtin_inf_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("inf"));
    assert!(
        doc.description.contains("infinity") || doc.description.contains("infinite"),
        "inf description must mention infinity: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.40"),
        "inf description must mention Perl 5.40: {}",
        doc.description
    );
}

#[test]
fn builtin_nan_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("nan"));
    assert!(
        doc.description.contains("NaN") || doc.description.contains("Not-a-Number"),
        "nan description must mention NaN: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.40"),
        "nan description must mention Perl 5.40: {}",
        doc.description
    );
}

#[test]
fn builtin_trim_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("trim"));
    assert!(
        doc.signature.contains("STRING"),
        "trim signature must show STRING parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("whitespace"),
        "trim description must mention whitespace: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.36"),
        "trim description must mention Perl 5.36: {}",
        doc.description
    );
}

#[test]
fn builtin_indexed_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("indexed"));
    assert!(
        doc.signature.contains("LIST"),
        "indexed signature must show LIST parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("index") || doc.description.contains("pair"),
        "indexed description must mention index or pair: {}",
        doc.description
    );
}

// ── Perl 5.38: taint inspection ──────────────────────────────────────────────

#[test]
fn builtin_is_tainted_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("is_tainted"));
    assert!(
        doc.signature.contains("EXPR"),
        "is_tainted signature must show EXPR parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("taint") || doc.description.contains("tainted"),
        "is_tainted description must mention taint: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.38"),
        "is_tainted description must mention Perl 5.38: {}",
        doc.description
    );
}

// ── Perl 5.40: module loading ─────────────────────────────────────────────────

#[test]
fn builtin_load_module_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("load_module"));
    assert!(
        doc.signature.contains("MODULE"),
        "load_module signature must show MODULE parameter: {}",
        doc.signature
    );
    assert!(
        doc.description.contains("module") || doc.description.contains("load"),
        "load_module description must mention loading a module: {}",
        doc.description
    );
    assert!(
        doc.description.contains("5.40"),
        "load_module description must mention Perl 5.40: {}",
        doc.description
    );
}

#[test]
fn builtin_export_lexically_has_hover_doc() {
    let doc = must_some(get_builtin_documentation("export_lexically"));
    assert!(
        doc.description.contains("lexical"),
        "export_lexically description must mention lexical scope: {}",
        doc.description
    );
}

// ── builtin:: qualified prefix support ───────────────────────────────────────

#[test]
fn builtin_prefix_true_resolves_to_same_doc() {
    let bare = must_some(get_builtin_documentation("true"));
    let qualified = must_some(get_builtin_documentation("builtin::true"));
    assert_eq!(
        bare.signature, qualified.signature,
        "builtin::true and true must resolve to the same documentation"
    );
    assert_eq!(
        bare.description, qualified.description,
        "builtin::true and true must have identical descriptions"
    );
}

#[test]
fn builtin_prefix_trim_resolves_to_same_doc() {
    let bare = must_some(get_builtin_documentation("trim"));
    let qualified = must_some(get_builtin_documentation("builtin::trim"));
    assert_eq!(
        bare.signature, qualified.signature,
        "builtin::trim and trim must resolve to the same documentation"
    );
}

#[test]
fn builtin_prefix_weaken_resolves_to_same_doc() {
    let bare = must_some(get_builtin_documentation("weaken"));
    let qualified = must_some(get_builtin_documentation("builtin::weaken"));
    assert_eq!(
        bare.signature, qualified.signature,
        "builtin::weaken and weaken must resolve to the same documentation"
    );
}

#[test]
fn builtin_prefix_ceil_resolves_to_same_doc() {
    let bare = must_some(get_builtin_documentation("ceil"));
    let qualified = must_some(get_builtin_documentation("builtin::ceil"));
    assert_eq!(
        bare.description, qualified.description,
        "builtin::ceil and ceil must resolve to the same documentation"
    );
}

// ── builtin:: qualified prefix (continued) ───────────────────────────────────

#[test]
fn builtin_prefix_blessed_resolves_to_same_doc() {
    let bare = must_some(get_builtin_documentation("blessed"));
    let qualified = must_some(get_builtin_documentation("builtin::blessed"));
    assert_eq!(
        bare.signature, qualified.signature,
        "builtin::blessed and blessed must resolve to the same documentation"
    );
    assert_eq!(
        bare.description, qualified.description,
        "builtin::blessed and blessed must have identical descriptions"
    );
}

#[test]
fn builtin_prefix_is_tainted_resolves_to_same_doc() {
    let bare = must_some(get_builtin_documentation("is_tainted"));
    let qualified = must_some(get_builtin_documentation("builtin::is_tainted"));
    assert_eq!(
        bare.signature, qualified.signature,
        "builtin::is_tainted and is_tainted must resolve to the same documentation"
    );
}

// ── is_builtin_function coverage ─────────────────────────────────────────────

#[test]
fn perl_5_36_functions_are_recognized_as_builtins() {
    for name in [
        "true", "false", "is_bool", "weaken", "unweaken", "is_weak", "refaddr", "reftype",
        "blessed",
    ] {
        assert!(
            is_builtin_function(name),
            "is_builtin_function must return true for Perl 5.36 builtin '{name}'"
        );
    }
}

#[test]
fn perl_5_38_functions_are_recognized_as_builtins() {
    for name in ["ceil", "floor", "inf", "nan", "trim", "indexed", "is_tainted"] {
        assert!(
            is_builtin_function(name),
            "is_builtin_function must return true for Perl 5.38 builtin '{name}'"
        );
    }
}

#[test]
fn perl_5_40_functions_are_recognized_as_builtins() {
    for name in ["load_module", "export_lexically"] {
        assert!(
            is_builtin_function(name),
            "is_builtin_function must return true for Perl 5.40 builtin '{name}'"
        );
    }
}

#[test]
fn builtin_prefix_functions_are_recognized_as_builtins() {
    for name in ["builtin::true", "builtin::false", "builtin::ceil", "builtin::trim"] {
        assert!(
            is_builtin_function(name),
            "is_builtin_function must return true for '{name}' (builtin:: prefix)"
        );
    }
}

// ── Completions: catalog coverage ────────────────────────────────────────────
// Verify via an integration path: the docs catalog must have entries for all
// `use builtin` names the completion catalog advertises.

#[test]
fn all_use_builtin_functions_have_hover_docs() {
    let all_builtins = [
        // 5.36
        "true",
        "false",
        "is_bool",
        "weaken",
        "unweaken",
        "is_weak",
        "refaddr",
        "reftype",
        "blessed",
        // 5.38
        "ceil",
        "floor",
        "inf",
        "nan",
        "trim",
        "indexed",
        "is_tainted",
        // 5.40
        "load_module",
        "export_lexically",
    ];
    for name in all_builtins {
        assert!(
            get_builtin_documentation(name).is_some(),
            "get_builtin_documentation must return Some for `use builtin` function '{name}'"
        );
    }
}
