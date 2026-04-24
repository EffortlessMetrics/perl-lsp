use perl_token::{Token, TokenKind};
use std::hint::black_box;
use std::time::{Duration, Instant};

const ITERATIONS: usize = 2_000_000;
const ROUNDS: usize = 5;

fn time_it<F>(label: &str, mut f: F) -> Duration
where
    F: FnMut() -> Token,
{
    let mut best = Duration::MAX;
    for _ in 0..ROUNDS {
        let start = Instant::now();
        for i in 0..ITERATIONS {
            let token = f();
            black_box((token.start, token.end, token.kind));
            black_box(i);
        }
        let elapsed = start.elapsed();
        if elapsed < best {
            best = elapsed;
        }
    }

    println!("{label}: best={best:?} for {ITERATIONS} iters");
    best
}

fn main() {
    let baseline = time_it("baseline_token_new_eof", || {
        Token::new(TokenKind::Eof, "", black_box(0), black_box(0))
    });

    let optimized = time_it("optimized_token_eof_at", || Token::eof_at(black_box(0)));

    let speedup = baseline.as_secs_f64() / optimized.as_secs_f64();
    let delta_pct =
        ((baseline.as_secs_f64() - optimized.as_secs_f64()) / baseline.as_secs_f64()) * 100.0;

    println!("speedup: {speedup:.3}x, improvement: {delta_pct:.2}%");
}
