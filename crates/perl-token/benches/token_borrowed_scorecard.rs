use criterion::{Criterion, criterion_group, criterion_main};
use perl_token::{Token, TokenKind, TokenRef};
use std::hint::black_box;

fn bench_token_construction(c: &mut Criterion) {
    let inputs = [
        (TokenKind::My, "my", 0usize, 2usize),
        (TokenKind::ScalarSigil, "$", 3usize, 4usize),
        (TokenKind::Identifier, "value", 4usize, 9usize),
        (TokenKind::Assign, "=", 10usize, 11usize),
        (TokenKind::Number, "42", 12usize, 14usize),
    ];

    c.bench_function("token_scorecard/construct_owned", |b| {
        b.iter(|| {
            inputs
                .iter()
                .map(|(kind, text, start, end)| {
                    Token::new(*kind, black_box(*text), black_box(*start), black_box(*end))
                })
                .collect::<Vec<_>>()
        });
    });

    c.bench_function("token_scorecard/construct_borrowed", |b| {
        b.iter(|| {
            inputs
                .iter()
                .map(|(kind, text, start, end)| {
                    TokenRef::new(*kind, black_box(*text), black_box(*start), black_box(*end))
                })
                .collect::<Vec<_>>()
        });
    });

    c.bench_function("token_scorecard/borrowed_to_owned", |b| {
        b.iter(|| {
            inputs
                .iter()
                .map(|(kind, text, start, end)| {
                    TokenRef::new(*kind, text, *start, *end).to_owned_token()
                })
                .collect::<Vec<_>>()
        });
    });
}

criterion_group!(benches, bench_token_construction);
criterion_main!(benches);
