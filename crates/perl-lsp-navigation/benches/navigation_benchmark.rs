#![allow(clippy::unwrap_used, clippy::expect_used)]

use criterion::{Criterion, criterion_group, criterion_main};
use perl_lsp_navigation::{WorkspaceSymbolsProvider, find_references_single_file};
use perl_parser_core::Parser;
use std::collections::HashMap;
use std::hint::black_box;

const SIMPLE_SCRIPT: &str = "use strict;\nmy $count = 42;\nmy $name = \"hello\";\nmy @items = (1, 2, 3);\nsub process { my ($input) = @_; return $input * 2; }\nmy $result = process($count);\n";

const MODULE_WITH_OO: &str = "package MyApp::Handler;\nuse strict;\nour $VERSION = '1.00';\nsub new { my ($class, %args) = @_; bless { name => $args{name} // 'default', count => $args{count} // 0 }, $class }\nsub process_data { my ($self, $data) = @_; return 1; }\nsub transform { my ($self, $value) = @_; return $value * 2; }\nsub get_summary { my ($self) = @_; return { name => 'test' }; }\n1;\n";

const LARGE_FILE: &str = "package MyApp::Large;\nuse strict;\nmy $shared = 'value';\nsub func_a { return $shared; }\nsub func_b { return $shared; }\nsub func_c { my $x = $shared; return $x; }\nsub func_d { print $shared; }\nsub func_e { my @arr = ($shared, $shared); }\nsub func_f { return $shared . $shared; }\nsub func_g { return length($shared); }\nsub func_h { return uc($shared); }\nsub func_i { return lc($shared); }\nsub func_j { return substr($shared, 0, 1); }\n1;\n";

fn bench_find_references_variable(c: &mut Criterion) {
    let source = "use strict;\nmy $count = 42;\nprint $count;\nmy $x = $count + 1;\n";
    let mut parser = Parser::new(source);
    let ast = parser.parse().expect("must parse");
    let offset = source.find("$count").expect("find $count");
    c.bench_function("nav_find_refs_variable", |b| {
        b.iter(|| {
            black_box(find_references_single_file(
                black_box(&ast),
                black_box(offset),
            ))
        })
    });
}

fn bench_find_references_subroutine(c: &mut Criterion) {
    let mut parser = Parser::new(SIMPLE_SCRIPT);
    let ast = parser.parse().expect("must parse");
    let offset = SIMPLE_SCRIPT.find("process").expect("find process");
    c.bench_function("nav_find_refs_subroutine", |b| {
        b.iter(|| {
            black_box(find_references_single_file(
                black_box(&ast),
                black_box(offset),
            ))
        })
    });
}

fn bench_find_references_large_file(c: &mut Criterion) {
    let mut parser = Parser::new(LARGE_FILE);
    let ast = parser.parse().expect("must parse");
    let offset = LARGE_FILE.find("$shared").expect("find $shared");
    c.bench_function("nav_find_refs_large_file", |b| {
        b.iter(|| {
            black_box(find_references_single_file(
                black_box(&ast),
                black_box(offset),
            ))
        })
    });
}

fn bench_workspace_symbol_search_exact(c: &mut Criterion) {
    let mut provider = WorkspaceSymbolsProvider::new();
    let sources: Vec<(&str, &str)> = vec![
        ("file:///lib/MyApp/Handler.pm", MODULE_WITH_OO),
        ("file:///lib/MyApp/Large.pm", LARGE_FILE),
        ("file:///script.pl", SIMPLE_SCRIPT),
    ];
    let mut source_map = HashMap::new();
    for (uri, content) in &sources {
        let mut parser = Parser::new(content);
        let ast = parser.parse().expect("must parse");
        provider.index_document(uri, &ast, content);
        source_map.insert(uri.to_string(), content.to_string());
    }
    c.bench_function("nav_workspace_symbol_exact", |b| {
        b.iter(|| black_box(provider.search(black_box("process"), black_box(&source_map))))
    });
}

fn bench_workspace_symbol_search_prefix(c: &mut Criterion) {
    let mut provider = WorkspaceSymbolsProvider::new();
    let sources: Vec<(&str, &str)> = vec![
        ("file:///lib/MyApp/Handler.pm", MODULE_WITH_OO),
        ("file:///lib/MyApp/Large.pm", LARGE_FILE),
        ("file:///script.pl", SIMPLE_SCRIPT),
    ];
    let mut source_map = HashMap::new();
    for (uri, content) in &sources {
        let mut parser = Parser::new(content);
        let ast = parser.parse().expect("must parse");
        provider.index_document(uri, &ast, content);
        source_map.insert(uri.to_string(), content.to_string());
    }
    c.bench_function("nav_workspace_symbol_prefix", |b| {
        b.iter(|| black_box(provider.search(black_box("func_"), black_box(&source_map))))
    });
}

fn bench_workspace_symbol_indexing(c: &mut Criterion) {
    let sources: Vec<(&str, &str)> = vec![
        ("file:///lib/MyApp/Handler.pm", MODULE_WITH_OO),
        ("file:///lib/MyApp/Large.pm", LARGE_FILE),
        ("file:///script.pl", SIMPLE_SCRIPT),
        (
            "file:///lib/MyApp/Extra.pm",
            "package MyApp::Extra;\nsub alpha { 1 }\nsub beta { 2 }\n1;\n",
        ),
    ];
    let parsed: Vec<_> = sources
        .iter()
        .map(|(uri, content)| {
            let mut parser = Parser::new(content);
            let ast = parser.parse().expect("must parse");
            (*uri, ast, *content)
        })
        .collect();
    c.bench_function("nav_workspace_symbol_indexing", |b| {
        b.iter(|| {
            let mut provider = WorkspaceSymbolsProvider::new();
            for (uri, ast, content) in &parsed {
                provider.index_document(black_box(uri), black_box(ast), black_box(content));
            }
            black_box(provider)
        })
    });
}

criterion_group!(
    benches,
    bench_find_references_variable,
    bench_find_references_subroutine,
    bench_find_references_large_file,
    bench_workspace_symbol_search_exact,
    bench_workspace_symbol_search_prefix,
    bench_workspace_symbol_indexing,
);
criterion_main!(benches);
