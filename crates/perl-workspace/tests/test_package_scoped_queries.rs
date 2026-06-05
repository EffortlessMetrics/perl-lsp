//! Tests for file-scoped and package-scoped symbol queries.
//!
//! Exercises `WorkspaceIndex::file_packages` and
//! `WorkspaceIndex::file_package_symbols`, the two read-only accessors added
//! by issue #900.

use perl_symbol::SymbolKind;
use perl_workspace::workspace::workspace_index::WorkspaceIndex;
use url::Url;

// ---------------------------------------------------------------------------
// Helper
// ---------------------------------------------------------------------------

fn index_with_code(uri: &str, code: &str) -> Result<WorkspaceIndex, Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    let url = Url::parse(uri)?;
    index.index_file(url, code.to_string())?;
    Ok(index)
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

/// Statement form (`package Foo;`), block form (`package Bar { }`), and nested
/// package (`package Foo::Nested { }`) all appear in `file_packages()`.
#[test]
fn test_file_packages_statement_and_block_forms() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Foo;\nsub hello { 1 }\npackage Bar { sub greet { 2 } }\npackage Foo::Nested { sub nested { 3 } }\n";
    let index = index_with_code("file:///lib/Multi.pm", code)?;

    let all_symbols = index.file_symbols("file:///lib/Multi.pm");
    let mut package_kind_names: Vec<String> = all_symbols
        .iter()
        .filter(|symbol| symbol.kind == SymbolKind::Package)
        .map(|symbol| symbol.name.clone())
        .collect();
    package_kind_names.sort();
    let non_package_names: Vec<&str> = all_symbols
        .iter()
        .filter(|symbol| symbol.kind != SymbolKind::Package)
        .map(|symbol| symbol.name.as_str())
        .collect();
    assert_eq!(
        package_kind_names,
        vec!["Bar", "Foo", "Foo::Nested"],
        "the fixture must include package-kind symbols"
    );
    assert!(non_package_names.contains(&"hello"), "the fixture must include a non-package symbol");
    assert!(
        all_symbols
            .iter()
            .any(|symbol| symbol.name == "hello" && symbol.kind == SymbolKind::Subroutine),
        "the fixture must include a non-package symbol"
    );

    let mut pkgs = index.file_packages("file:///lib/Multi.pm");
    pkgs.sort();
    assert_eq!(pkgs, vec!["Bar", "Foo", "Foo::Nested"], "all three package forms must appear");
    Ok(())
}

/// Symbols (subs, `our` vars) are attributed to the correct package.
#[test]
fn test_file_package_symbols_correct_attribution() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Foo;\nsub hello { 1 }\npackage Bar { sub greet { 2 } }\nour $shared = 3;\n";
    let index = index_with_code("file:///lib/PkgSym.pm", code)?;

    let all_symbols = index.file_symbols("file:///lib/PkgSym.pm");
    let mut expected_bar_names: Vec<String> = all_symbols
        .iter()
        .filter(|symbol| symbol.container_name.as_deref() == Some("Bar"))
        .map(|symbol| symbol.name.clone())
        .collect();
    expected_bar_names.sort();
    let mut expected_foo_names: Vec<String> = all_symbols
        .iter()
        .filter(|symbol| symbol.container_name.as_deref() == Some("Foo"))
        .map(|symbol| symbol.name.clone())
        .collect();
    expected_foo_names.sort();

    let bar_syms = index.file_package_symbols("file:///lib/PkgSym.pm", "Bar");
    let mut bar_names: Vec<String> = bar_syms.iter().map(|s| s.name.clone()).collect();
    bar_names.sort();
    assert!(
        bar_syms.iter().all(|symbol| symbol.container_name.as_deref() == Some("Bar")),
        "every returned Bar symbol must have Bar as its container"
    );
    assert!(
        bar_syms.iter().all(|symbol| symbol.kind != SymbolKind::Package),
        "package declaration symbols must not be returned as package members"
    );
    assert_eq!(bar_names, expected_bar_names, "Bar package symbols must match the Bar filter");
    assert!(bar_names.iter().any(|name| name == "greet"), "Bar must contain greet");
    assert!(!bar_names.iter().any(|name| name == "hello"), "hello must not be in Bar");

    let foo_syms = index.file_package_symbols("file:///lib/PkgSym.pm", "Foo");
    let mut foo_names: Vec<String> = foo_syms.iter().map(|s| s.name.clone()).collect();
    foo_names.sort();
    assert!(
        foo_syms.iter().all(|symbol| symbol.container_name.as_deref() == Some("Foo")),
        "every returned Foo symbol must have Foo as its container"
    );
    assert!(
        foo_syms.iter().all(|symbol| symbol.kind != SymbolKind::Package),
        "package declaration symbols must not be returned as package members"
    );
    assert_eq!(foo_names, expected_foo_names, "Foo package symbols must match the Foo filter");
    assert!(foo_names.iter().any(|name| name == "hello"), "Foo must contain hello");
    assert!(!foo_names.iter().any(|name| name == "greet"), "greet must not be in Foo");

    Ok(())
}

/// A file with no explicit `package` declaration returns an empty vec.
/// There is no implicit `main` symbol; the extractor does not emit one.
#[test]
fn test_file_packages_empty_for_no_explicit_package() -> Result<(), Box<dyn std::error::Error>> {
    let code = "sub helper { 42 }\n";
    let index = index_with_code("file:///lib/NoPackage.pm", code)?;
    assert!(
        index.file_packages("file:///lib/NoPackage.pm").is_empty(),
        "file with no explicit package decl must return empty vec"
    );
    Ok(())
}

/// An explicit `package main;` is a real package declaration and is surfaced.
#[test]
fn test_file_packages_includes_explicit_main_package() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package main;\nsub entrypoint { 1 }\n";
    let index = index_with_code("file:///lib/Main.pm", code)?;

    assert_eq!(index.file_packages("file:///lib/Main.pm"), vec!["main"]);

    let symbols = index.file_package_symbols("file:///lib/Main.pm", "main");
    let names: Vec<&str> = symbols.iter().map(|symbol| symbol.name.as_str()).collect();
    assert!(names.contains(&"entrypoint"), "main package symbols should include entrypoint");

    Ok(())
}

/// Asking for symbols in a package that does not exist in the file returns
/// empty, not an error.
#[test]
fn test_file_package_symbols_nonexistent_package() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Foo;\nsub bar { }\n";
    let index = index_with_code("file:///lib/Foo.pm", code)?;
    assert!(
        index.file_package_symbols("file:///lib/Foo.pm", "DoesNotExist").is_empty(),
        "nonexistent package must return empty vec"
    );
    Ok(())
}

/// Query URIs are normalized through the same path as `file_symbols()`.
#[test]
fn test_file_package_queries_accept_normalized_path_input() -> Result<(), Box<dyn std::error::Error>>
{
    let path = "/lib/Normalized.pm";
    let normalized = Url::from_file_path(path).map_err(|()| "path cannot become URI")?.to_string();
    let code = "package Normalized;\nsub seen { 1 }\n";
    let index = index_with_code(&normalized, code)?;

    assert_eq!(index.file_packages(path), vec!["Normalized"]);

    let symbols = index.file_package_symbols(path, "Normalized");
    let names: Vec<&str> = symbols.iter().map(|symbol| symbol.name.as_str()).collect();
    assert!(names.contains(&"seen"), "normalized path lookup should find package symbols");

    Ok(())
}

/// Both methods return empty for a URI that was never indexed.
/// This exercises the `unwrap_or_default()` path.
#[test]
fn test_missing_uri_returns_empty() -> Result<(), Box<dyn std::error::Error>> {
    let index = WorkspaceIndex::new();
    assert!(
        index.file_packages("file:///does/not/exist.pm").is_empty(),
        "file_packages on unknown URI must return empty"
    );
    assert!(
        index.file_package_symbols("file:///does/not/exist.pm", "Foo").is_empty(),
        "file_package_symbols on unknown URI must return empty"
    );
    Ok(())
}

/// Block-scoped package restores outer context after the block closes.
/// A sub declared after `package Inner { }` belongs to the outer package,
/// not to `Inner`.
#[test]
fn test_block_package_restores_outer_context() -> Result<(), Box<dyn std::error::Error>> {
    let code = "package Outer;\npackage Inner { sub inside { } }\nsub outside { }\n";
    let index = index_with_code("file:///lib/Restore.pm", code)?;

    let outer_syms = index.file_package_symbols("file:///lib/Restore.pm", "Outer");
    let outer_names: Vec<&str> = outer_syms.iter().map(|s| s.name.as_str()).collect();
    assert!(
        outer_names.contains(&"outside"),
        "sub after block belongs to Outer, got: {:?}",
        outer_names
    );

    let inner_syms = index.file_package_symbols("file:///lib/Restore.pm", "Inner");
    let inner_names: Vec<&str> = inner_syms.iter().map(|s| s.name.as_str()).collect();
    assert!(
        inner_names.contains(&"inside"),
        "sub inside block belongs to Inner, got: {:?}",
        inner_names
    );

    Ok(())
}
