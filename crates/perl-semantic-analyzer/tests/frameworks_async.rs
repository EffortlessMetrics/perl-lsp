//! Framework semantic extraction tests for async frameworks.

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
    table
        .symbols
        .get(name)
        .is_some_and(|symbols| symbols.iter().any(|symbol| symbol.kind == kind))
}

fn symbol_attrs(table: &SymbolTable, name: &str, kind: SymbolKind) -> Vec<String> {
    table
        .symbols
        .get(name)
        .and_then(|symbols| symbols.iter().find(|symbol| symbol.kind == kind))
        .map(|symbol| symbol.attributes.clone())
        .unwrap_or_default()
}

#[test]
fn io_async_use_synthesizes_class_symbols_for_common_namespaces() {
    let code = r#"
use IO::Async;

my $loop = IO::Async::Loop->new;
my $stream = IO::Async::Stream->new;
my $handle = IO::Async::Handle->new;
"#;

    let table = extract_symbols(code);

    for name in ["IO::Async::Loop", "IO::Async::Stream", "IO::Async::Handle"] {
        assert!(
            has_symbol(&table, name, SymbolKind::Class),
            "expected synthetic IO::Async class symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Class);
        assert!(
            attrs.iter().any(|attr| attr == "framework=IO::Async"),
            "expected `framework=IO::Async` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn io_async_namespace_import_enables_symbol_synthesis() {
    let code = r#"
use IO::Async::Loop;

my $loop = IO::Async::Loop->new;
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "IO::Async::Loop", SymbolKind::Class),
        "expected namespace import to enable IO::Async class synthesis"
    );
}

#[test]
fn io_async_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $loop = IO::Async::Loop->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "IO::Async::Loop", SymbolKind::Class),
        "did not expect IO::Async class synthesis without `use IO::Async`"
    );
}

#[test]
fn anyevent_use_synthesizes_core_class_symbols_for_method_calls() {
    let code = r#"
use AnyEvent;

my $cv = AnyEvent->condvar;
my $timer = AnyEvent::Timer->new;
my $io = AnyEvent::IO->new;
my $other = AnyEvent::CondVar->new;
"#;

    let table = extract_symbols(code);

    for name in [
        "AnyEvent",
        "AnyEvent::CondVar",
        "AnyEvent::Timer",
        "AnyEvent::IO",
    ] {
        assert!(
            has_symbol(&table, name, SymbolKind::Class),
            "expected synthetic AnyEvent class symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Class);
        assert!(
            attrs.iter().any(|attr| attr == "framework=AnyEvent"),
            "expected `framework=AnyEvent` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn anyevent_core_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $cv = AnyEvent->condvar;
my $timer = AnyEvent::Timer->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "AnyEvent", SymbolKind::Class),
        "did not expect AnyEvent class synthesis without `use AnyEvent`"
    );
}

#[test]
fn anyevent_http_names_are_not_synthesized_as_core_support() {
    let code = r#"
use AnyEvent;

my $http = AnyEvent::HTTP->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "AnyEvent::HTTP", SymbolKind::Class),
        "did not expect AnyEvent::HTTP synthesis in the core-only MVP"
    );
}

#[test]
fn ev_use_synthesizes_root_and_common_api_symbols() {
    let code = r#"
use EV;

EV::timer();
EV::io();
EV::signal();
EV::idle();
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "EV", SymbolKind::Class),
        "expected EV namespace symbol when framework is in use"
    );
    let attrs = symbol_attrs(&table, "EV", SymbolKind::Class);
    assert!(
        attrs.iter().any(|attr| attr == "framework=EV"),
        "expected `framework=EV` on EV, got {attrs:?}"
    );

    for name in ["EV::timer", "EV::io", "EV::signal", "EV::idle"] {
        assert!(
            has_symbol(&table, name, SymbolKind::Subroutine),
            "expected synthetic EV API symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Subroutine);
        assert!(
            attrs.iter().any(|attr| attr == "framework=EV"),
            "expected `framework=EV` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn ev_names_are_not_synthesized_without_framework_use() {
    let code = r#"
EV::timer();
EV::io();
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "EV", SymbolKind::Class),
        "did not expect EV namespace synthesis without `use EV`"
    );
    assert!(
        !has_symbol(&table, "EV::timer", SymbolKind::Subroutine),
        "did not expect EV::timer synthesis without `use EV`"
    );
}

#[test]
fn mojo_redis_use_synthesizes_framework_class_symbol() {
    let code = r#"
use Mojo::Redis;

my $redis = Mojo::Redis->new;
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "Mojo::Redis", SymbolKind::Class),
        "expected Mojo::Redis class symbol when framework is in use"
    );
    let attrs = symbol_attrs(&table, "Mojo::Redis", SymbolKind::Class);
    assert!(
        attrs.iter().any(|attr| attr == "framework=Mojo::Redis"),
        "expected `framework=Mojo::Redis` on Mojo::Redis, got {attrs:?}"
    );
}

#[test]
fn mojo_redis_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $redis = Mojo::Redis->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "Mojo::Redis", SymbolKind::Class),
        "did not expect Mojo::Redis class synthesis without `use Mojo::Redis`"
    );
}

#[test]
fn mojo_pg_use_synthesizes_framework_class_symbol() {
    let code = r#"
use Mojo::Pg;

my $pg = Mojo::Pg->new;
my $mode = Mojo::Pg->strict_mode;
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "Mojo::Pg", SymbolKind::Class),
        "expected Mojo::Pg class symbol when framework is in use"
    );
    let attrs = symbol_attrs(&table, "Mojo::Pg", SymbolKind::Class);
    assert!(
        attrs.iter().any(|attr| attr == "framework=Mojo::Pg"),
        "expected `framework=Mojo::Pg` on Mojo::Pg, got {attrs:?}"
    );
}

#[test]
fn mojo_pg_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $pg = Mojo::Pg->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "Mojo::Pg", SymbolKind::Class),
        "did not expect Mojo::Pg class synthesis without `use Mojo::Pg`"
    );
}

#[test]
fn future_use_synthesizes_class_symbol_for_method_calls() {
    let code = r#"
use Future;

my $future = Future->new;
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "Future", SymbolKind::Class),
        "expected Future class symbol when framework is in use"
    );
    let attrs = symbol_attrs(&table, "Future", SymbolKind::Class);
    assert!(
        attrs.iter().any(|attr| attr == "framework=Future"),
        "expected `framework=Future` on Future, got {attrs:?}"
    );
}

#[test]
fn future_use_synthesizes_common_chain_methods() {
    let code = r#"
use Future;

my $future = Future->new;
my $next = $future->then(sub { return Future->done(1) });
$future->catch(sub { return Future->fail("boom") });
$future->finally(sub { });
$future->get;
$future->is_done;
$future->is_ready;
Future->wait_all($future);
Future->needs_all($future);
Future->needs_any($future);
"#;

    let table = extract_symbols(code);

    for name in [
        "new",
        "then",
        "catch",
        "finally",
        "get",
        "is_done",
        "is_ready",
        "wait_all",
        "needs_all",
        "needs_any",
    ] {
        assert!(
            has_symbol(&table, name, SymbolKind::Subroutine),
            "expected synthetic Future API symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Subroutine);
        assert!(
            attrs.iter().any(|attr| attr == "framework=Future"),
            "expected `framework=Future` on `{name}`, got {attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|attr| attr == &format!("future_api={name}")),
            "expected `future_api={name}` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn future_xs_use_synthesizes_class_symbol_for_method_calls() {
    let code = r#"
use Future::XS;

my $future = Future::XS->new;
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "Future::XS", SymbolKind::Class),
        "expected Future::XS class symbol when framework is in use"
    );
    let attrs = symbol_attrs(&table, "Future::XS", SymbolKind::Class);
    assert!(
        attrs.iter().any(|attr| attr == "framework=Future::XS"),
        "expected `framework=Future::XS` on Future::XS, got {attrs:?}"
    );
}

#[test]
fn future_xs_use_synthesizes_common_chain_methods() {
    let code = r#"
use Future::XS;

my $future = Future::XS->new;
my $next = $future->then(sub { return Future::XS->done(1) });
$future->catch(sub { return Future::XS->fail("boom") });
$future->finally(sub { });
$future->get;
$future->is_done;
$future->is_ready;
Future::XS->wait_all($future);
Future::XS->needs_all($future);
Future::XS->needs_any($future);
"#;

    let table = extract_symbols(code);

    for name in [
        "new",
        "then",
        "catch",
        "finally",
        "get",
        "is_done",
        "is_ready",
        "wait_all",
        "needs_all",
        "needs_any",
    ] {
        assert!(
            has_symbol(&table, name, SymbolKind::Subroutine),
            "expected synthetic Future::XS API symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Subroutine);
        assert!(
            attrs.iter().any(|attr| attr == "framework=Future::XS"),
            "expected `framework=Future::XS` on `{name}`, got {attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|attr| attr == &format!("future_api={name}")),
            "expected `future_api={name}` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn future_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $future = Future->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "Future", SymbolKind::Class),
        "did not expect Future class synthesis without `use Future`"
    );
}

#[test]
fn future_api_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $future = Future->new;
$future->then(sub { return Future->done(1) });
Future->wait_all($future);
"#;

    let table = extract_symbols(code);

    for name in [
        "new",
        "then",
        "catch",
        "finally",
        "get",
        "is_done",
        "is_ready",
        "wait_all",
        "needs_all",
        "needs_any",
    ] {
        assert!(
            !has_symbol(&table, name, SymbolKind::Subroutine),
            "did not expect synthetic Future API symbol `{name}` without `use Future`"
        );
    }
}

#[test]
fn future_xs_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $future = Future::XS->new;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "Future::XS", SymbolKind::Class),
        "did not expect Future::XS class synthesis without `use Future::XS`"
    );
}

#[test]
fn promise_use_synthesizes_class_symbol_for_method_calls() {
    let code = r#"
use Promise;

my $promise = Promise->new(sub { return 1 });
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "Promise", SymbolKind::Class),
        "expected Promise class symbol when framework is in use"
    );
    let attrs = symbol_attrs(&table, "Promise", SymbolKind::Class);
    assert!(
        attrs.iter().any(|attr| attr == "framework=Promise"),
        "expected `framework=Promise` on Promise, got {attrs:?}"
    );
}

#[test]
fn promise_use_synthesizes_common_chain_methods() {
    let code = r#"
use Promise;

my $promise = Promise->new(sub { return 1 });
my $next = $promise->then(sub { return Promise->resolve(1) });
$promise->catch(sub { return Promise->reject("boom") });
$promise->finally(sub { });
$promise->resolve(1);
$promise->reject("boom");
Promise->all($promise);
Promise->race($promise);
Promise->any($promise);
"#;

    let table = extract_symbols(code);

    for name in [
        "new", "then", "catch", "finally", "resolve", "reject", "all", "race", "any",
    ] {
        assert!(
            has_symbol(&table, name, SymbolKind::Subroutine),
            "expected synthetic Promise API symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Subroutine);
        assert!(
            attrs.iter().any(|attr| attr == "framework=Promise"),
            "expected `framework=Promise` on `{name}`, got {attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|attr| attr == &format!("future_api={name}")),
            "expected `future_api={name}` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn promise_xs_use_synthesizes_class_symbol_for_method_calls() {
    let code = r#"
use Promise::XS;

my $promise = Promise::XS->new(sub { return 1 });
"#;

    let table = extract_symbols(code);

    assert!(
        has_symbol(&table, "Promise::XS", SymbolKind::Class),
        "expected Promise::XS class symbol when framework is in use"
    );
    let attrs = symbol_attrs(&table, "Promise::XS", SymbolKind::Class);
    assert!(
        attrs.iter().any(|attr| attr == "framework=Promise::XS"),
        "expected `framework=Promise::XS` on Promise::XS, got {attrs:?}"
    );
}

#[test]
fn promise_xs_use_synthesizes_common_chain_methods() {
    let code = r#"
use Promise::XS;

my $promise = Promise::XS->new(sub { return 1 });
my $next = $promise->then(sub { return Promise::XS->resolve(1) });
$promise->catch(sub { return Promise::XS->reject("boom") });
$promise->finally(sub { });
$promise->resolve(1);
$promise->reject("boom");
Promise::XS->all($promise);
Promise::XS->race($promise);
Promise::XS->any($promise);
"#;

    let table = extract_symbols(code);

    for name in [
        "new", "then", "catch", "finally", "resolve", "reject", "all", "race", "any",
    ] {
        assert!(
            has_symbol(&table, name, SymbolKind::Subroutine),
            "expected synthetic Promise::XS API symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Subroutine);
        assert!(
            attrs.iter().any(|attr| attr == "framework=Promise::XS"),
            "expected `framework=Promise::XS` on `{name}`, got {attrs:?}"
        );
        assert!(
            attrs
                .iter()
                .any(|attr| attr == &format!("future_api={name}")),
            "expected `future_api={name}` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn promise_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $promise = Promise->new(sub { return 1 });
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "Promise", SymbolKind::Class),
        "did not expect Promise class synthesis without `use Promise`"
    );
}

#[test]
fn poe_use_synthesizes_class_symbols_for_method_calls() {
    let code = r#"
use POE qw(Session Wheel::ReadWrite Component::Client::TCP);

my $session = POE::Session->create;
my $wheel = POE::Wheel::ReadWrite->new;
my $component = POE::Component::Client::TCP->new;
"#;

    let table = extract_symbols(code);

    for name in [
        "POE::Session",
        "POE::Wheel::ReadWrite",
        "POE::Component::Client::TCP",
    ] {
        assert!(
            has_symbol(&table, name, SymbolKind::Class),
            "expected synthetic POE class symbol `{name}`"
        );
        let attrs = symbol_attrs(&table, name, SymbolKind::Class);
        assert!(
            attrs.iter().any(|attr| attr == "framework=POE"),
            "expected `framework=POE` on `{name}`, got {attrs:?}"
        );
    }
}

#[test]
fn poe_names_are_not_synthesized_without_framework_use() {
    let code = r#"
my $session = POE::Session->create;
"#;

    let table = extract_symbols(code);

    assert!(
        !has_symbol(&table, "POE::Session", SymbolKind::Class),
        "did not expect POE class synthesis without `use POE`"
    );
}
