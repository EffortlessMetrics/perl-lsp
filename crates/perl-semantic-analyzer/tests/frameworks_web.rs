//! Framework semantic extraction tests for Dancer2/Mojolicious route definitions.
//!
//! These tests verify that route handler symbols are synthesized when a web
//! framework `use` statement is detected, enabling goto-definition and hover
//! on route method names.

use perl_semantic_analyzer::{
    Parser,
    symbol::{SymbolExtractor, SymbolKind, SymbolTable},
};
use perl_tdd_support::must;

fn extract_symbols(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == kind))
}

fn symbol_doc(table: &SymbolTable, name: &str, kind: SymbolKind) -> Option<String> {
    table
        .symbols
        .get(name)
        .and_then(|symbols| symbols.iter().find(|s| s.kind == kind))
        .and_then(|s| s.documentation.clone())
}

fn symbol_attrs(table: &SymbolTable, name: &str, kind: SymbolKind) -> Vec<String> {
    table
        .symbols
        .get(name)
        .and_then(|symbols| symbols.iter().find(|s| s.kind == kind))
        .map(|s| s.attributes.clone())
        .unwrap_or_default()
}

// === Dancer2 route detection ===

#[test]
fn dancer2_get_route_emits_subroutine_symbol() {
    let code = r#"
use Dancer2;

get '/hello' => sub {
    return 'Hello World';
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/hello", SymbolKind::Subroutine),
        "expected route symbol `/hello` as Subroutine for `get '/hello' => sub`"
    );
}

#[test]
fn dancer2_post_route_emits_subroutine_symbol() {
    let code = r#"
use Dancer2;

post '/api/users' => sub {
    my $body = request->body;
    return $body;
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/api/users", SymbolKind::Subroutine),
        "expected route symbol `/api/users` as Subroutine for `post '/api/users' => sub`"
    );
}

#[test]
fn dancer2_put_route_emits_subroutine_symbol() {
    let code = r#"
use Dancer2;

put '/api/users/:id' => sub {
    return 'updated';
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/api/users/:id", SymbolKind::Subroutine),
        "expected route symbol `/api/users/:id` from `put` route"
    );
}

#[test]
fn dancer2_del_route_emits_subroutine_symbol() {
    let code = r#"
use Dancer2;

del '/api/users/:id' => sub {
    return 'deleted';
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/api/users/:id", SymbolKind::Subroutine),
        "expected route symbol `/api/users/:id` from `del` route"
    );
}

#[test]
fn dancer2_patch_route_emits_subroutine_symbol() {
    let code = r#"
use Dancer2;

patch '/api/users/:id' => sub {
    return 'patched';
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/api/users/:id", SymbolKind::Subroutine),
        "expected route symbol `/api/users/:id` from `patch` route"
    );
}

#[test]
fn dancer2_route_symbol_has_http_method_attribute() {
    let code = r#"
use Dancer2;

get '/status' => sub { return 'ok' };
"#;
    let table = extract_symbols(code);
    let attrs = symbol_attrs(&table, "/status", SymbolKind::Subroutine);
    assert!(
        attrs.iter().any(|a| a == "http_method=GET"),
        "expected `http_method=GET` attribute on route symbol, got: {attrs:?}"
    );
}

#[test]
fn dancer2_route_symbol_has_documentation() {
    let code = r#"
use Dancer2;

get '/status' => sub { return 'ok' };
"#;
    let table = extract_symbols(code);
    let doc = symbol_doc(&table, "/status", SymbolKind::Subroutine);
    assert!(
        doc.is_some_and(|d| d.contains("GET") && d.contains("/status")),
        "expected documentation mentioning GET and /status"
    );
}

#[test]
fn dancer2_multiple_routes_emit_distinct_symbols() {
    let code = r#"
use Dancer2;

get '/foo' => sub { 'foo' };
post '/bar' => sub { 'bar' };
get '/baz' => sub { 'baz' };
"#;
    let table = extract_symbols(code);
    assert!(has_symbol(&table, "/foo", SymbolKind::Subroutine), "expected /foo route symbol");
    assert!(has_symbol(&table, "/bar", SymbolKind::Subroutine), "expected /bar route symbol");
    assert!(has_symbol(&table, "/baz", SymbolKind::Subroutine), "expected /baz route symbol");
}

#[test]
fn dancer2_route_without_use_is_not_synthesized() {
    // Without `use Dancer2`, a bare `get` call should NOT produce a route symbol
    let code = r#"
get '/hello' => sub {
    return 'Hello World';
};
"#;
    let table = extract_symbols(code);
    assert!(
        !has_symbol(&table, "/hello", SymbolKind::Subroutine),
        "bare `get` without `use Dancer2` should NOT produce a route symbol"
    );
}

// === Mojolicious::Lite route detection ===

#[test]
fn mojolicious_lite_get_route_emits_subroutine_symbol() {
    let code = r#"
use Mojolicious::Lite;

get '/hello' => sub {
    my $c = shift;
    $c->render(text => 'Hello World');
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/hello", SymbolKind::Subroutine),
        "expected route symbol `/hello` for Mojolicious::Lite `get '/hello' => sub`"
    );
}

#[test]
fn mojolicious_lite_post_route_emits_subroutine_symbol() {
    let code = r#"
use Mojolicious::Lite;

post '/api/submit' => sub {
    my $c = shift;
    $c->render(json => { ok => 1 });
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/api/submit", SymbolKind::Subroutine),
        "expected route symbol `/api/submit` for Mojolicious::Lite POST route"
    );
}

#[test]
fn mojolicious_lite_route_symbol_has_http_method_attribute() {
    let code = r#"
use Mojolicious::Lite;

post '/submit' => sub { my $c = shift };
"#;
    let table = extract_symbols(code);
    let attrs = symbol_attrs(&table, "/submit", SymbolKind::Subroutine);
    assert!(
        attrs.iter().any(|a| a == "http_method=POST"),
        "expected `http_method=POST` attribute on Mojo route symbol, got: {attrs:?}"
    );
}

// === any route (Dancer2) ===

#[test]
fn dancer2_any_route_emits_subroutine_symbol() {
    let code = r#"
use Dancer2;

any '/multi' => sub { return 'multi' };
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/multi", SymbolKind::Subroutine),
        "expected route symbol `/multi` from `any` route"
    );
}
