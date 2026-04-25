use criterion::{Criterion, criterion_group, criterion_main};
use perl_token::{Token, TokenKind, TokenRef};
use std::hint::black_box;

const START: usize = 12;
const END: usize = 18;

fn bench_borrowed_token_construction(c: &mut Criterion) {
    c.bench_function("token/borrowed_construction", |b| {
        b.iter(|| {
            black_box(TokenRef::new(
                TokenKind::Identifier,
                black_box("foobar"),
                black_box(START),
                black_box(END),
            ))
        });
    });
}

fn bench_owned_token_construction(c: &mut Criterion) {
    c.bench_function("token/owned_construction", |b| {
        b.iter(|| {
            black_box(Token::new(
                TokenKind::Identifier,
                black_box("foobar"),
                black_box(START),
                black_box(END),
            ))
        });
    });
}

fn bench_borrowed_to_owned_conversion(c: &mut Criterion) {
    let borrowed = TokenRef::new(TokenKind::Identifier, "foobar", START, END);
    c.bench_function("token/borrowed_to_owned_conversion", |b| {
        b.iter(|| black_box(borrowed).to_owned_token());
    });
}

criterion_group!(
    token_scorecard,
    bench_borrowed_token_construction,
    bench_owned_token_construction,
    bench_borrowed_to_owned_conversion
);
criterion_main!(token_scorecard);
