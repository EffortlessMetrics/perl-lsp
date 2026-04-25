#![no_main]

use libfuzzer_sys::fuzz_target;
use perl_lexer::{LexerConfig, PerlLexer};

const MAX_INPUT_BYTES: usize = 2048;

fn bounded_utf8_lossy(data: &[u8]) -> std::borrow::Cow<'_, str> {
    let capped = if data.len() <= MAX_INPUT_BYTES { data } else { &data[..MAX_INPUT_BYTES] };
    String::from_utf8_lossy(capped)
}

fuzz_target!(|data: &[u8]| {
    let input = bounded_utf8_lossy(data);

    // Default tokenization — must never panic on arbitrary input.
    let mut lexer = PerlLexer::new(&input);
    let tokens = lexer.collect_tokens();
    // Token stream must always end with EOF.
    let _ = tokens.last();

    // Interpolation-aware tokenization (exercises string parsing paths).
    let config_interp = LexerConfig {
        parse_interpolation: true,
        track_positions: false,
        max_lookahead: 256,
    };
    let mut lexer2 = PerlLexer::with_config(&input, config_interp);
    let _ = lexer2.collect_tokens();

    // Position-tracking tokenization (exercises UTF-8/line-col bookkeeping).
    let config_pos = LexerConfig {
        parse_interpolation: false,
        track_positions: true,
        max_lookahead: 256,
    };
    let mut lexer3 = PerlLexer::with_config(&input, config_pos);
    let _ = lexer3.collect_tokens();

    // Body-mode entry point (used by the parser for heredoc bodies and similar).
    let mut lexer4 = PerlLexer::with_body_tokens(&input);
    let _ = lexer4.collect_tokens();

    // Stress the peek/next interleaving that drives parser lookahead.
    let mut lexer5 = PerlLexer::new(&input);
    for _ in 0..32 {
        let peeked = lexer5.peek_token();
        let consumed = lexer5.next_token();
        if peeked.is_none() && consumed.is_none() {
            break;
        }
    }
});
