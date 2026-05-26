//! End-to-end content tests for `textDocument/moniker`.
//!
//! The existing 3.17 test only asserts the response is `null | array`. These
//! tests drive the JSON-RPC handler end-to-end and verify the *content* of
//! returned monikers: scheme, identifier shape, `kind` classification
//! (export / import / local), and `unique` scoping.

mod support;

use perl_tdd_support::must_some;
use serde_json::{Value, json};
use support::lsp_harness::LspHarness;

type TestResult = Result<(), Box<dyn std::error::Error>>;

/// Send a `textDocument/moniker` request and return the response as an array.
/// Returns an empty vec if the server returned null.
fn request_monikers(
    harness: &mut LspHarness,
    uri: &str,
    line: u32,
    character: u32,
) -> Result<Vec<Value>, String> {
    let response = harness.request(
        "textDocument/moniker",
        json!({
            "textDocument": { "uri": uri },
            "position": { "line": line, "character": character }
        }),
    )?;

    if response.is_null() {
        return Ok(Vec::new());
    }
    let arr = response.as_array().ok_or_else(|| format!("expected array, got {response}"))?.clone();
    Ok(arr)
}

fn moniker_kinds(monikers: &[Value]) -> Vec<&str> {
    monikers.iter().filter_map(|m| m.get("kind").and_then(Value::as_str)).collect()
}

fn moniker_identifiers(monikers: &[Value]) -> Vec<&str> {
    monikers.iter().filter_map(|m| m.get("identifier").and_then(Value::as_str)).collect()
}

fn find_moniker_of_kind<'a>(monikers: &'a [Value], kind: &str) -> Option<&'a Value> {
    monikers.iter().find(|m| m.get("kind").and_then(Value::as_str) == Some(kind))
}

fn assert_moniker_shape(moniker: &Value) {
    assert_eq!(
        moniker.get("scheme").and_then(Value::as_str),
        Some("perl"),
        "every moniker must use the perl scheme: {moniker}"
    );
    assert!(
        moniker.get("identifier").and_then(Value::as_str).is_some(),
        "moniker missing string identifier: {moniker}"
    );
    let kind = must_some(moniker.get("kind").and_then(Value::as_str));
    assert!(
        matches!(kind, "import" | "export" | "local"),
        "kind must be import/export/local, got {kind:?}"
    );
    let unique = must_some(moniker.get("unique").and_then(Value::as_str));
    assert!(
        matches!(unique, "document" | "project" | "global" | "scheme"),
        "unique must be a recognized LSP value, got {unique:?}"
    );
}

#[test]
fn moniker_exported_sub_is_classified_as_export() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///moniker_export.pm";
    harness.open(
        uri,
        "package Foo::Bar;\n\
         use Exporter 'import';\n\
         our @EXPORT_OK = qw(do_thing);\n\
         sub do_thing { return 1 }\n\
         1;\n",
    )?;

    // Cursor on `do_thing` inside its definition (line 3, after "sub ").
    let monikers = request_monikers(&mut harness, uri, 3, 6)?;
    assert!(!monikers.is_empty(), "expected at least one moniker for exported sub");
    for m in &monikers {
        assert_moniker_shape(m);
    }

    let kinds = moniker_kinds(&monikers);
    assert!(
        kinds.contains(&"export"),
        "expected an export moniker for symbol listed in @EXPORT_OK, got kinds={kinds:?}"
    );

    // Primary moniker should be qualified as Foo.Bar.do_thing (per moniker.rs:43).
    let identifiers = moniker_identifiers(&monikers);
    assert!(
        identifiers.iter().any(|id| id.contains("do_thing")),
        "expected an identifier containing 'do_thing', got {identifiers:?}"
    );
    assert!(
        identifiers.iter().any(|id| id.contains("Foo.Bar")),
        "expected an identifier using dot-qualified package, got {identifiers:?}"
    );

    // Exported subs should be globally unique.
    let export = must_some(find_moniker_of_kind(&monikers, "export"));
    assert_eq!(
        export.get("unique").and_then(Value::as_str),
        Some("global"),
        "exported sub should be globally unique"
    );

    Ok(())
}

#[test]
fn moniker_imported_sub_yields_import_kind_and_source_moniker() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///moniker_import.pl";
    harness.open(
        uri,
        "use List::Util qw(sum);\n\
         my $total = sum(1, 2, 3);\n",
    )?;

    // Cursor on the `sum` call on line 1, char 13 (start of `sum`).
    let monikers = request_monikers(&mut harness, uri, 1, 13)?;
    assert!(!monikers.is_empty(), "expected monikers for imported `sum` symbol");
    for m in &monikers {
        assert_moniker_shape(m);
    }

    let kinds = moniker_kinds(&monikers);
    assert!(
        kinds.contains(&"import"),
        "expected an import moniker for use List::Util qw(sum), got {kinds:?}"
    );

    // moniker.rs:53-65: imported symbols also emit a secondary `kind=export`
    // moniker pointing at the source module. Pin down both monikers
    // independently - a regression that drops the source-pointing one would
    // otherwise still pass the looser `identifiers.iter().any(...)` check
    // (e.g. a single {kind:import, identifier:"List.Util.sum"} response).
    assert!(
        monikers.len() >= 2,
        "imported symbol must yield at least two monikers (import + source export), got {monikers:?}"
    );
    assert!(
        kinds.contains(&"export"),
        "expected a secondary export moniker pointing at the source module, got kinds={kinds:?}"
    );

    let source_export = must_some(find_moniker_of_kind(&monikers, "export"));
    let source_id = must_some(source_export.get("identifier").and_then(Value::as_str));
    assert!(
        source_id.contains("List.Util") && source_id.contains("sum"),
        "export moniker identifier must point at the source (List.Util.sum), got {source_id:?}"
    );
    assert_eq!(
        source_export.get("unique").and_then(Value::as_str),
        Some("global"),
        "source-pointing export moniker should be globally unique (moniker.rs:61), got {source_export}"
    );

    Ok(())
}

#[test]
fn moniker_local_sub_in_main_is_document_scoped() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///moniker_local.pl";
    harness.open(
        uri,
        "sub helper { return 42 }\n\
         helper();\n",
    )?;

    // Cursor on the `helper` definition (line 0, character 4).
    let monikers = request_monikers(&mut harness, uri, 0, 4)?;
    assert!(!monikers.is_empty(), "expected at least one moniker for local sub");
    for m in &monikers {
        assert_moniker_shape(m);
    }

    let kinds = moniker_kinds(&monikers);
    assert!(
        kinds.contains(&"local"),
        "expected a local moniker for sub defined in main, got {kinds:?}"
    );

    // moniker.rs:130-140: subs in main:: that aren't exported are document-scoped.
    let local = must_some(find_moniker_of_kind(&monikers, "local"));
    assert_eq!(
        local.get("unique").and_then(Value::as_str),
        Some("document"),
        "local sub in main:: should be document-scoped"
    );

    Ok(())
}

#[test]
fn moniker_sub_in_package_uses_project_uniqueness() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///moniker_package.pm";
    harness.open(
        uri,
        "package My::Util;\n\
         sub compute { return 7 }\n\
         1;\n",
    )?;

    // Cursor on `compute` (line 1, character 4).
    let monikers = request_monikers(&mut harness, uri, 1, 4)?;
    assert!(!monikers.is_empty(), "expected monikers for sub in named package");
    for m in &monikers {
        assert_moniker_shape(m);
    }

    // Non-exported sub in a named package should be project-scoped per moniker.rs:136.
    let local = must_some(find_moniker_of_kind(&monikers, "local"));
    assert_eq!(
        local.get("unique").and_then(Value::as_str),
        Some("project"),
        "non-exported sub in named package should be project-scoped"
    );

    let identifiers = moniker_identifiers(&monikers);
    assert!(
        identifiers.iter().any(|id| id.contains("My.Util") && id.contains("compute")),
        "expected My.Util.compute style identifier, got {identifiers:?}"
    );

    Ok(())
}

#[test]
fn moniker_subs_in_package_with_use_base_get_parent_monikers() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///moniker_inheritance.pm";
    harness.open(
        uri,
        "package Child::Class;\n\
         use base 'Parent::Class';\n\
         sub greet { return 'hi' }\n\
         1;\n",
    )?;

    // Cursor on `greet` definition (line 2, character 4).
    let monikers = request_monikers(&mut harness, uri, 2, 4)?;
    assert!(!monikers.is_empty(), "expected monikers for sub in child class");
    for m in &monikers {
        assert_moniker_shape(m);
    }

    // moniker.rs:80-92 - subs in packages with use base emit additional
    // monikers pointing at potential parent definitions.
    let identifiers = moniker_identifiers(&monikers);
    assert!(
        identifiers.iter().any(|id| id.contains("Parent.Class") && id.contains("greet")),
        "expected parent-class moniker like Parent.Class.greet, got {identifiers:?}"
    );

    Ok(())
}

#[test]
fn moniker_on_empty_position_returns_empty_array() -> TestResult {
    let mut harness = LspHarness::new();
    harness.initialize(None)?;

    let uri = "file:///moniker_empty.pl";
    harness.open(uri, "# just a comment\n")?;

    // Cursor inside a comment - no symbol, so return an empty array per moniker.rs:100.
    let monikers = request_monikers(&mut harness, uri, 0, 5)?;
    assert!(
        monikers.is_empty(),
        "expected empty array for cursor inside a comment, got {monikers:?}"
    );

    Ok(())
}
