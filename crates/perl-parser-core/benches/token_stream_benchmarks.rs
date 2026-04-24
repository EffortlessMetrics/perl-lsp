use criterion::{Criterion, criterion_group, criterion_main};
use perl_parser_core::tokens::token_stream::{Token, TokenKind, TokenStream};
use std::hint::black_box;

fn bench_buffered_empty_eof(c: &mut Criterion) {
    c.bench_function("token_stream/buffered_empty_eof", |b| {
        b.iter(|| {
            let mut stream = TokenStream::from_vec(Vec::new());
            let token = stream.next().expect("empty buffered stream should synthesize EOF");
            black_box(token.kind == TokenKind::Eof)
        });
    });
}

fn bench_buffered_eof_after_token(c: &mut Criterion) {
    c.bench_function("token_stream/buffered_eof_after_token", |b| {
        b.iter(|| {
            let mut stream = TokenStream::from_vec(vec![Token::new(TokenKind::My, "my", 0, 2)]);
            let _first = stream.next().expect("first token should exist");
            let eof = stream.next().expect("second token should be EOF");
            black_box(eof.kind == TokenKind::Eof)
        });
    });
}

criterion_group!(benches, bench_buffered_empty_eof, bench_buffered_eof_after_token);
criterion_main!(benches);
