use criterion::{Criterion, criterion_group, criterion_main};
use perl_parser_core::tokens::token_stream::{Token, TokenKind, TokenStream};
use std::hint::black_box;

fn bench_buffered_eof_synthesis(c: &mut Criterion) {
    c.bench_function("token_stream_buffered_eof_synthesis", |b| {
        b.iter(|| {
            let mut stream = TokenStream::from_vec(vec![Token::new(TokenKind::My, "my", 0, 2)]);
            let _ = black_box(stream.next());
            let _ = black_box(stream.next());
        });
    });
}

fn bench_parser_token_construction(c: &mut Criterion) {
    c.bench_function("token_new_eof_constructor", |b| {
        b.iter(|| {
            let eof = Token::eof(black_box(0), black_box(0));
            black_box(eof);
        });
    });
}

criterion_group!(benches, bench_buffered_eof_synthesis, bench_parser_token_construction);
criterion_main!(benches);
