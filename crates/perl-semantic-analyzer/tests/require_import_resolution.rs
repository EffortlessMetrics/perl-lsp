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
