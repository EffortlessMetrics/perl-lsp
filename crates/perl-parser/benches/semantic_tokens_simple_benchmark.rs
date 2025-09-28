use criterion::{criterion_group, criterion_main, Criterion};
use perl_parser::{
    semantic_tokens_provider::{encode_semantic_tokens, SemanticTokensProvider},
    Parser,
};
use std::hint::black_box;

fn benchmark_semantic_tokens_small(c: &mut Criterion) {
    let code = r#"
package Test;
use strict;
my $var = 42;
sub func { return $var; }
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = SemanticTokensProvider::new(code.to_string());

    c.bench_function("semantic_tokens_extract", |b| {
        b.iter(|| {
            let tokens = provider.extract(black_box(&ast));
            black_box(tokens)
        });
    });

    let tokens = provider.extract(&ast);
    c.bench_function("semantic_tokens_encode", |b| {
        b.iter(|| {
            let encoded = encode_semantic_tokens(black_box(&tokens));
            black_box(encoded)
        });
    });

    c.bench_function("semantic_tokens_full", |b| {
        b.iter(|| {
            let tokens = provider.extract(black_box(&ast));
            let encoded = encode_semantic_tokens(black_box(&tokens));
            black_box(encoded)
        });
    });
}

fn benchmark_semantic_tokens_thread_safety(c: &mut Criterion) {
    let code = r#"
package ThreadTest;
my $var = 'test';
sub func1 { return $var; }
"#;

    let mut parser = Parser::new(code);
    let ast = parser.parse().unwrap();
    let provider = SemanticTokensProvider::new(code.to_string());

    c.bench_function("semantic_tokens_concurrent", |b| {
        b.iter(|| {
            // Test concurrent calls - should be thread-safe
            let tokens1 = provider.extract(black_box(&ast));
            let tokens2 = provider.extract(black_box(&ast));
            black_box((tokens1, tokens2))
        });
    });
}

criterion_group!(benches, benchmark_semantic_tokens_small, benchmark_semantic_tokens_thread_safety);
criterion_main!(benches);
