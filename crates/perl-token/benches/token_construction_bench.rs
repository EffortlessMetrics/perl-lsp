use criterion::{criterion_group, criterion_main, Criterion};
use perl_token::{Token, TokenKind};

fn bench_eof_token_construction(c: &mut Criterion) {
    c.bench_function("token_new_eof", |b| {
        b.iter(|| Token::new(TokenKind::Eof, "", 1024, 1024));
    });

    c.bench_function("token_eof_shared_empty_arc", |b| {
        b.iter(|| Token::eof(1024, 1024));
    });
}

criterion_group!(benches, bench_eof_token_construction);
criterion_main!(benches);
