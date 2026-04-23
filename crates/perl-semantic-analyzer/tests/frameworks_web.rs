//! Framework semantic extraction tests for Dancer2/Mojolicious route definitions.
//!
//! These tests verify that route handler symbols are synthesized when a web
//! framework `use` statement is detected, enabling goto-definition and hover
//! on route method names.

use perl_semantic_analyzer::{
    Parser,
    declaration::{current_package_at, symbol_at_cursor},
    symbol::{SymbolExtractor, SymbolKind, SymbolTable},
};
use perl_tdd_support::{must, must_some};

fn extract_symbols(code: &str) -> SymbolTable {
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());
    SymbolExtractor::new_with_source(code).extract(&ast)
}

fn has_symbol(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table.symbols.get(name).is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == kind))
}

fn has_reference(table: &SymbolTable, name: &str, kind: SymbolKind) -> bool {
    table
        .references
        .get(name)
        .is_some_and(|references| references.iter().any(|reference| reference.kind == kind))
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

#[test]
fn dancer_route_target_string_adds_subroutine_reference() {
    let code = r#"
use Dancer;

get '/about' => 'show_about';

sub show_about {
    return 'About';
}
"#;
    let table = extract_symbols(code);
    assert!(
        has_reference(&table, "show_about", SymbolKind::Subroutine),
        "expected route target string `show_about` to be recorded as a Subroutine reference"
    );
}

#[test]
fn dancer2_route_target_string_adds_subroutine_reference() {
    let code = r#"
use Dancer2;

get '/status' => 'show_status';

sub show_status {
    return 'ok';
}
"#;
    let table = extract_symbols(code);
    assert!(
        has_reference(&table, "show_status", SymbolKind::Subroutine),
        "expected route target string `show_status` to be recorded as a Subroutine reference"
    );
}

// === Plack::Builder middleware chain detection ===

#[test]
fn plack_builder_enable_emits_middleware_symbol() {
    let code = r#"
use Plack::Builder;

builder {
    enable 'Static';
    enable 'Plack::Middleware::Session';
};
"#;
    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "Plack::Middleware::Static", SymbolKind::Package),
        "expected quoted middleware `Static` to normalize to `Plack::Middleware::Static`"
    );
    assert!(
        has_symbol(&table, "Plack::Middleware::Session", SymbolKind::Package),
        "expected quoted middleware `Plack::Middleware::Session` to be preserved"
    );

    let attrs = symbol_attrs(&table, "Plack::Middleware::Static", SymbolKind::Package);
    assert!(
        attrs.iter().any(|a| a == "framework=Plack::Builder"),
        "expected Plack framework attribute on middleware symbol, got: {attrs:?}"
    );

    let doc = symbol_doc(&table, "Plack::Middleware::Static", SymbolKind::Package);
    assert!(
        doc.is_some_and(|d| d.contains("middleware") && d.contains("Static")),
        "expected middleware documentation to mention the normalized module name"
    );
}

#[test]
fn plack_builder_enable_symbol_at_cursor_resolves_middleware_package() {
    let code = r#"
use Plack::Builder;

builder {
    enable 'Static';
    enable 'Plack::Middleware::Session';
};
"#;
    let mut parser = Parser::new(code);
    let ast = must(parser.parse());

    let static_offset = must_some(code.find("Static"));
    let current_pkg = current_package_at(&ast, static_offset);
    let symbol = must_some(symbol_at_cursor(&ast, static_offset, current_pkg));

    assert_eq!(
        symbol.pkg.as_ref(),
        "Plack::Middleware::Static",
        "short-name Plack middleware should normalize to the full package name"
    );
    assert_eq!(
        symbol.name.as_ref(),
        "Plack::Middleware::Static",
        "short-name Plack middleware should normalize to the full package name"
    );
}

#[test]
fn plack_builder_mount_emits_mount_symbol() {
    let code = r#"
use Plack::Builder;

builder {
    mount '/api' => $api_app;
    mount '/' => $app;
};
"#;
    let table = extract_symbols(code);

    assert!(has_symbol(&table, "/api", SymbolKind::Subroutine), "expected `/api` mount symbol");
    assert!(has_symbol(&table, "/", SymbolKind::Subroutine), "expected `/` mount symbol");

    let attrs = symbol_attrs(&table, "/api", SymbolKind::Subroutine);
    assert!(
        attrs.iter().any(|a| a == "mount_path=/api"),
        "expected mount path attribute on `/api`, got: {attrs:?}"
    );
    assert!(
        attrs.iter().any(|a| a == "mount_target=$api_app"),
        "expected mount target attribute on `/api`, got: {attrs:?}"
    );
}

#[test]
fn plack_builder_without_use_is_not_synthesized() {
    let code = r#"
builder {
    enable Static;
    mount '/api' => $api_app;
};
"#;
    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "Plack::Middleware::Static", SymbolKind::Package),
        "bare `builder` should not synthesize Plack middleware symbols"
    );
    assert!(
        !has_symbol(&table, "/api", SymbolKind::Subroutine),
        "bare `builder` should not synthesize mount symbols"
    );
}

#[test]
fn builder_io_fusion_builder_enable_emits_middleware_symbol() {
    let code = r#"
use Builder::IO::Fusion;

builder {
    enable 'Static';
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "Plack::Middleware::Static", SymbolKind::Package),
        "expected Fusion builder middleware to normalize to Plack::Middleware::* symbol"
    );
}

#[test]
fn builder_io_fusion_builder_mount_emits_mount_symbol() {
    let code = r#"
use Builder::IO::Fusion;

builder {
    mount '/api' => $api_app;
};
"#;
    let table = extract_symbols(code);
    assert!(
        has_symbol(&table, "/api", SymbolKind::Subroutine),
        "expected Fusion builder mount to synthesize mount symbol"
    );
}
