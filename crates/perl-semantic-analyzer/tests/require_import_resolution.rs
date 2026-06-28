//! Unit tests for `require Module; Module->import('sym')` import resolution.
//! Issue #3476: literal require + explicit named import tracking.

use perl_semantic_analyzer::Parser;
use perl_semantic_analyzer::analysis::declaration::{
    symbol_at_cursor, symbol_at_cursor_with_source,
};

fn parse_and_symbol_at(code: &str, needle: &str) -> Option<String> {
    // Find the byte offset of needle in the code
    let offset = code.find(needle)?;
    let mut parser = Parser::new(code);
    let ast = parser.parse().ok()?;
    let key = symbol_at_cursor(&ast, offset, "main")?;
    Some(key.pkg.to_string())
}

/// Helper: parse `code`, find `needle`, call symbol_at_cursor_with_source.
/// Returns the SymbolKey's (pkg, name) as Strings, or None.
fn parse_and_symbol_with_source_at(code: &str, needle: &str) -> Option<(String, String)> {
    let offset = code.find(needle)?;
    let mut parser = Parser::new(code);
    let ast = parser.parse().ok()?;
    let key = symbol_at_cursor_with_source(&ast, offset, "main", code)?;
    Some((key.pkg.to_string(), key.name.to_string()))
}

#[test]
fn require_import_string_list_resolves_pkg() {
    let code = r#"require My::Loader;
My::Loader->import('load_data', 'process');
my $x = load_data();
"#;
    let pkg = parse_and_symbol_at(code, "load_data()");
    assert_eq!(
        pkg.as_deref(),
        Some("My::Loader"),
        "load_data() should resolve to My::Loader via require+import, got: {pkg:?}"
    );
}

#[test]
fn require_import_qw_list_resolves_pkg() {
    let code = r#"require My::Tools;
My::Tools->import(qw(helper_func));
my $v = helper_func();
"#;
    let pkg = parse_and_symbol_at(code, "helper_func()");
    assert_eq!(
        pkg.as_deref(),
        Some("My::Tools"),
        "helper_func() should resolve to My::Tools via require+qw-import, got: {pkg:?}"
    );
}

#[test]
fn use_import_still_resolves_correctly() {
    let code = r#"use Carp qw(croak);
croak("error");
"#;
    let pkg = parse_and_symbol_at(code, "croak(");
    assert_eq!(
        pkg.as_deref(),
        Some("Carp"),
        "croak() should still resolve to Carp via use+qw import, got: {pkg:?}"
    );
}

#[test]
fn require_import_multiple_symbols_both_resolve() {
    let code = r#"require My::Utils;
My::Utils->import('alpha', 'beta');
alpha();
beta();
"#;
    let pkg_alpha = parse_and_symbol_at(code, "alpha()");
    let pkg_beta = parse_and_symbol_at(code, "beta()");
    assert_eq!(
        pkg_alpha.as_deref(),
        Some("My::Utils"),
        "alpha() should resolve to My::Utils, got: {pkg_alpha:?}"
    );
    assert_eq!(
        pkg_beta.as_deref(),
        Some("My::Utils"),
        "beta() should resolve to My::Utils, got: {pkg_beta:?}"
    );
}

#[test]
fn require_import_known_tag_resolves_members() {
    let code = r#"require POSIX;
POSIX->import(':sys_wait_h');
my $ok = WIFEXITED($status);
"#;
    let pkg = parse_and_symbol_at(code, "WIFEXITED(");
    assert_eq!(
        pkg.as_deref(),
        Some("POSIX"),
        "WIFEXITED() should resolve to POSIX via require+tag import, got: {pkg:?}"
    );
}

#[test]
fn require_without_import_does_not_leak_symbol() {
    // require alone does NOT make symbols available — only with explicit import call
    let code = r#"require My::Loader;
load_data();
"#;
    // Without an explicit ->import() call, the symbol should NOT resolve to My::Loader
    let pkg = parse_and_symbol_at(code, "load_data()");
    assert_ne!(
        pkg.as_deref(),
        Some("My::Loader"),
        "load_data() should NOT resolve to My::Loader without explicit import call"
    );
}

#[test]
fn require_import_default_no_args_is_conservative() {
    // `Module->import()` with no args requests the module's default export
    // set (@EXPORT), but the semantic-analyzer's declaration lookup does not
    // have a workspace export table, so it conservatively does NOT claim
    // symbol ownership here.  The completion crate handles this separately.
    let code = r#"require My::Loader;
My::Loader->import();
load_data();
"#;
    let pkg = parse_and_symbol_at(code, "load_data()");
    assert_ne!(
        pkg.as_deref(),
        Some("My::Loader"),
        "default import() should NOT resolve without workspace export table, got: {pkg:?}"
    );
}

#[test]
fn require_file_path_then_import_resolves_pkg() {
    let code = r#"require 'My/Loader.pm';
My::Loader->import('load_data');
load_data();
"#;
    let pkg = parse_and_symbol_at(code, "load_data()");
    assert_eq!(
        pkg.as_deref(),
        Some("My::Loader"),
        "file path require should normalize to My::Loader, got: {pkg:?}"
    );
}

#[test]
fn module_runtime_alias_then_import_resolves_pkg() {
    let code = r#"my $loader = use_module('My::Loader');
$loader->import('load_data');
load_data();
"#;
    let pkg = parse_and_symbol_at(code, "load_data()");
    assert_eq!(
        pkg.as_deref(),
        Some("My::Loader"),
        "$loader->import() should resolve back to static use_module target, got: {pkg:?}"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Regression tests for NON_IMPORT_PRAGMAS false-positive fix (PR #5022)
// ──────────────────────────────────────────────────────────────────────────────

/// `use parent qw(Base::Class)` — cursor on `Base` should NOT resolve to a
/// bogus SymbolKey like `{ pkg: "parent", name: "Base::Class", kind: Sub }`.
/// `parent` is an inheritance pragma; its args are class names, not imports.
#[test]
fn use_parent_qw_does_not_resolve_as_import() {
    let code = "use parent qw(Base::Class);\n";
    // Position cursor on "Base" inside the qw list
    let result = parse_and_symbol_with_source_at(code, "Base");
    // Must NOT return pkg="parent", name="Base::Class"
    if let Some((pkg, name)) = &result {
        assert!(
            !(pkg == "parent" && name == "Base::Class"),
            "use parent qw(Base::Class) with cursor on 'Base' produced bogus SymbolKey \
             {{ pkg: {pkg:?}, name: {name:?} }} — parent args are not imported symbols"
        );
    }
    // Either None or resolved to the package itself (Pack kind) is acceptable.
}

/// `use Exporter 'import'` — cursor on `import` should NOT resolve to
/// `SymbolKey { pkg: "Exporter", name: "import", kind: Sub }`.
/// `'import'` here is a proxy-import method name, not an imported symbol.
#[test]
fn use_exporter_import_does_not_resolve_as_import() {
    let code = "use Exporter 'import';\n";
    // Position cursor on 'import' (the string literal)
    let result = parse_and_symbol_with_source_at(code, "import");
    if let Some((pkg, name)) = &result {
        assert!(
            !(pkg == "Exporter" && name == "import"),
            "use Exporter 'import' with cursor on 'import' produced bogus SymbolKey \
             {{ pkg: {pkg:?}, name: {name:?} }} — Exporter args are not imported symbols"
        );
    }
}

// ──────────────────────────────────────────────────────────────────────────────
// Import/export visibility regression bank (Box 5)
// ──────────────────────────────────────────────────────────────────────────────

#[test]
fn exporter_default_import_visibility_is_currently_conservative() {
    let code = r#"use MyLib;
foo();
"#;
    let pkg = parse_and_symbol_at(code, "foo()");
    assert_ne!(
        pkg.as_deref(),
        Some("MyLib"),
        "currently conservative: bare use MyLib should not claim @EXPORT ownership without export table"
    );
}

#[test]
fn exporter_explicit_export_ok_visibility_resolves_symbols() {
    let code = r#"use MyLib qw(bar baz);
bar();
baz();
"#;
    let bar_pkg = parse_and_symbol_at(code, "bar()");
    let baz_pkg = parse_and_symbol_at(code, "baz()");
    assert_eq!(bar_pkg.as_deref(), Some("MyLib"));
    assert_eq!(baz_pkg.as_deref(), Some("MyLib"));
}

#[test]
fn exporter_tag_import_visibility_is_currently_conservative() {
    let code = r#"use MyLib qw(:all);
foo();
bar();
baz();
"#;
    for symbol in ["foo()", "bar()", "baz()"] {
        let pkg = parse_and_symbol_at(code, symbol);
        assert_ne!(
            pkg.as_deref(),
            Some("MyLib"),
            "currently conservative: {symbol} should not be attributed via :all without export table"
        );
    }
}

#[test]
fn exporter_export_ok_not_visible_without_explicit_import() {
    let code = r#"use MyLib;
bar();
"#;
    let pkg = parse_and_symbol_at(code, "bar()");
    assert_ne!(
        pkg.as_deref(),
        Some("MyLib"),
        "@EXPORT_OK symbol should not be visible under bare use MyLib"
    );
}

// ──────────────────────────────────────────────────────────────────────────────
// Quote-operator whitespace conformance matrix — perl-semantic-analyzer consumer
// ──────────────────────────────────────────────────────────────────────────────
//
// Shared case matrix (defined once, applied across three consumers):
//
//   qw(foo bar)    → Explicit(["foo", "bar"])   compact paren
//   qw (foo bar)   → Explicit(["foo", "bar"])   space before paren
//   qw[foo bar]    → Explicit(["foo", "bar"])   compact bracket
//   qw [foo bar]   → Explicit(["foo", "bar"])   space before bracket
//   qw/foo bar/    → Explicit(["foo", "bar"])   compact slash
//   qw /foo bar/   → Explicit(["foo", "bar"])   space before slash
//   qw{foo bar}    → Explicit(["foo", "bar"])   compact brace
//   qw {foo bar}   → Explicit(["foo", "bar"])   space before brace
//   qwfoo          → NOT parsed as qw list (bareword arg, no qw delimiter)
//
// The AST-level parser normalizes ALL qw delimiter forms to `qw(...)` before
// ImportExtractor sees the args, so all qw variants should produce identical
// ImportSymbols.  This matrix verifies that invariant end-to-end.
//
// `q {text}` and `qq [text]` are not valid `use Module ...` import-symbol args
// in this consumer context; they are omitted here.

use perl_semantic_analyzer::analysis::import_extractor::ImportExtractor;
use perl_semantic_facts::{FileId, ImportSymbols};

fn parse_and_extract_import_specs(source: &str) -> Vec<perl_semantic_facts::ImportSpec> {
    let mut parser = perl_semantic_analyzer::Parser::new(source);
    match parser.parse() {
        Ok(ast) => ImportExtractor::extract(&ast, FileId(1)),
        Err(_) => Vec::new(),
    }
}

/// Extract the symbol list from a single-spec result; returns Err if not found
/// or if the symbols are not in Explicit form.
fn extract_explicit_symbols(source: &str) -> Result<Vec<String>, String> {
    let specs = parse_and_extract_import_specs(source);
    let spec = specs
        .into_iter()
        .find(|s| s.module == "Foo")
        .ok_or_else(|| format!("expected an ImportSpec for module 'Foo' in {source:?}"))?;
    match spec.symbols {
        ImportSymbols::Explicit(names) => Ok(names),
        other => Err(format!("expected ImportSymbols::Explicit, got {other:?}")),
    }
}

#[test]
fn conformance_matrix_qw_compact_paren_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw(foo bar);\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw(foo bar) must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_qw_space_before_paren_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw (foo bar);\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw (foo bar) must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_qw_compact_bracket_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw[foo bar];\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw[foo bar] must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_qw_space_before_bracket_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw [foo bar];\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw [foo bar] must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_qw_compact_slash_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw/foo bar/;\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw/foo bar/ must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_qw_space_before_slash_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw /foo bar/;\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw /foo bar/ must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_qw_compact_brace_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw{foo bar};\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw{{foo bar}} must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_qw_space_before_brace_produces_explicit_symbols() -> Result<(), String> {
    let syms = extract_explicit_symbols("use Foo qw {foo bar};\n")?;
    assert_eq!(syms, vec!["foo", "bar"], "qw {{foo bar}} must yield [foo, bar]");
    Ok(())
}

#[test]
fn conformance_matrix_space_and_compact_forms_are_identical() -> Result<(), String> {
    // Parity: compact and space-before-delimiter must produce the same symbol list.
    let pairs: &[(&str, &str)] = &[
        ("use Foo qw(foo bar);\n", "use Foo qw (foo bar);\n"),
        ("use Foo qw[foo bar];\n", "use Foo qw [foo bar];\n"),
        ("use Foo qw/foo bar/;\n", "use Foo qw /foo bar/;\n"),
        ("use Foo qw{foo bar};\n", "use Foo qw {foo bar};\n"),
    ];

    for (compact_src, spaced_src) in pairs {
        let compact_syms = extract_explicit_symbols(compact_src)?;
        let spaced_syms = extract_explicit_symbols(spaced_src)?;
        assert_eq!(
            compact_syms, spaced_syms,
            "parity: compact {compact_src:?} vs spaced {spaced_src:?}: \
             compact={compact_syms:?}, spaced={spaced_syms:?}"
        );
    }
    Ok(())
}

#[test]
fn conformance_matrix_qwfoo_bareword_is_not_parsed_as_qw_list() -> Result<(), String> {
    // `qwfoo` is NOT a valid qw operator — the lexer treats it as the bare word
    // `qwfoo`.  The ImportExtractor must NOT split it into qw-content words.
    // It may appear as a single symbol name "qwfoo" (bareword import arg), but
    // must never produce ["oo"] or similar qw-parsed fragments.
    let specs = parse_and_extract_import_specs("use Foo qwfoo;\n");
    let foo_spec = specs.into_iter().find(|s| s.module == "Foo");
    if let Some(spec) = foo_spec {
        // If we got a spec, the symbols must not be Explicit(["oo"]) or any
        // form that looks like qw-parsed content from "foo" after stripping "qw".
        match &spec.symbols {
            ImportSymbols::Explicit(names) => {
                for name in names {
                    assert_ne!(
                        name, "oo",
                        "qwfoo must not produce 'oo' — 'f' must not be treated as qw delimiter"
                    );
                    if name.is_empty() {
                        return Err("qwfoo must not produce empty symbol names".to_string());
                    }
                }
            }
            ImportSymbols::Default | ImportSymbols::None => {
                // Also acceptable — qwfoo treated as no-import or default
            }
            other => {
                return Err(format!("unexpected ImportSymbols variant for qwfoo: {other:?}"));
            }
        }
    }
    // If no spec found for Foo, that's also acceptable.
    Ok(())
}
