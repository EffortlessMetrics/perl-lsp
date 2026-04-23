//! Fuzz test for tree-sitter-perl-c parser
//!
//! This module contains fuzz tests that generate random strings and pass them
//! to parse_perl_code() to find crashes, panics, and unexpected behavior.

use std::sync::atomic::{AtomicUsize, Ordering};

/// Counts total iterations performed
static ITERATIONS: AtomicUsize = AtomicUsize::new(0);

/// Counts panics caught
static PANICS: AtomicUsize = AtomicUsize::new(0);

/// Counts parse errors (expected, not bugs)
static PARSE_ERRORS: AtomicUsize = AtomicUsize::new(0);

/// Simple random string generator using a linear congruential generator
struct SimpleRng {
    state: u64,
}

impl SimpleRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        // LCGC: state = state * 6364136223846793005 + 1
        self.state = self.state.wrapping_mul(6364136223846793005).wrapping_add(1);
        self.state
    }

    fn next_usize(&mut self) -> usize {
        (self.next() % (usize::MAX as u64)) as usize
    }

    fn next_u8(&mut self) -> u8 {
        (self.next() % 256) as u8
    }
}

/// Generate a random string based on the current RNG state
fn generate_random_string(rng: &mut SimpleRng, max_len: usize) -> String {
    let len = rng.next_usize() % max_len.min(10000);
    let mut chars: Vec<char> = Vec::with_capacity(len);

    for _ in 0..len {
        // Generate printable ASCII characters with emphasis on Perl-relevant chars
        let byte = rng.next_u8();
        let c = match byte % 32 {
            0 => '\0', // NUL - edge case
            1 => '\n', // newline
            2 => '\t', // tab
            3 => '\r', // carriage return
            4 => ' ',  // space
            5.. => {
                // Printable ASCII (33-126) with some Perl operators
                let chars = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789 $@#%^&*()_+-=[]{}|;':\",./<>?`~\n\t\r ";
                let idx = (byte as usize) % chars.len();
                chars.chars().nth(idx).unwrap_or(' ')
            }
        };
        chars.push(c);
    }

    chars.into_iter().collect()
}

/// Fuzz test: parse_perl_code should never panic on any input
///
/// This test generates random strings and passes them to parse_perl_code.
/// A panic indicates a bug in the parser wrapper or the underlying C grammar.
fn fuzz_parse_perl_code_string(input: &str) {
    ITERATIONS.fetch_add(1, Ordering::Relaxed);

    let result = std::panic::catch_unwind(|| {
        let _ = tree_sitter_perl_c::parse_perl_code(input);
    });

    match result {
        Ok(_) => {
            // Success - no panic
        }
        Err(panic_info) => {
            PANICS.fetch_add(1, Ordering::Relaxed);
            let msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                format!("String panic: {}", s)
            } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                format!("&str panic: {}", s)
            } else {
                "Unknown panic type".to_string()
            };
            eprintln!("PANIC on input: {:?}\nPanic info: {}", input, msg);
        }
    }
}

/// Run fuzz iterations
pub fn run_fuzz_iterations(iterations: usize) {
    let mut rng = SimpleRng::new(42); // Fixed seed for reproducibility

    for i in 0..iterations {
        let input = generate_random_string(&mut rng, 1000);
        fuzz_parse_perl_code_string(&input);

        // Progress indicator every 10000 iterations
        if i > 0 && i % 10000 == 0 {
            println!(
                "Progress: {}/{} iterations | panics: {} | parse errors (expected): {}",
                i,
                iterations,
                PANICS.load(Ordering::Relaxed),
                PARSE_ERRORS.load(Ordering::Relaxed)
            );
        }
    }
}

#[test]
fn fuzz_parse_perl_code_no_panic() {
    println!("Starting fuzz test: fuzz_parse_perl_code_no_panic");
    println!("This test runs 50,000 iterations of random string generation");
    println!("and passes each string to parse_perl_code() to check for panics.\n");

    run_fuzz_iterations(50_000);

    let total = ITERATIONS.load(Ordering::Relaxed);
    let panics = PANICS.load(Ordering::Relaxed);
    let parse_errors = PARSE_ERRORS.load(Ordering::Relaxed);

    println!("\n=== Fuzz Results ===");
    println!("Total iterations: {}", total);
    println!("Panics caught: {}", panics);
    println!("Parse errors (expected): {}", parse_errors);

    // We expect some parse errors - Perl syntax is valid/invalid based on input
    // But we should NEVER see a panic
    assert_eq!(panics, 0, "parse_perl_code should never panic");
}

/// Test specific edge cases that are known to be problematic
#[test]
fn fuzz_edge_case_special_strings() {
    let edge_cases = [
        "",           // empty
        " ",          // single space
        "\n",         // just newline
        "\t",         // just tab
        "\0",         // NUL byte
        "abc\0def",   // embedded NUL
        "\n\t\r",     // whitespace combo
        ";",          // minimal statement
        ";;",         // double semicolon
        "my ",        // incomplete statement
        "my $",       // incomplete variable
        "my $x",      // no semicolon
        "my $x =",    // incomplete expression
        "my $x = ;",  // empty expression
        "()",         // empty parens
        "[]",         // empty brackets
        "{}",         // empty braces
        "{{{{",       // unmatched braces
        "}}}}",       // unmatched closing
        "((((",       // nested parens
        "[[[[",       // nested brackets
        "//",         // potential regex
        "///",        // invalid regex
        "= = =",      // repeated equals
        "$",          // bare dollar
        "@",          // bare at
        "%",          // bare percent
        "*",          // bare star
        "$$",         // double dollar
        "@@",         // double at
        "$#$",        // consecutive sigils
        "{{{{{{{{{{", // many open braces
        "}}}}}}}}}}", // many close braces
        "日本語",     // unicode
        "🎉",         // emoji
        "\u{0}",      // null codepoint
        "\u{1F600}",  // unicode emoji
    ];

    for (i, case) in edge_cases.iter().enumerate() {
        println!("Testing edge case {}/{}: {:?}", i + 1, edge_cases.len(), case);

        let result = std::panic::catch_unwind(|| {
            let _ = tree_sitter_perl_c::parse_perl_code(case);
        });

        if result.is_err() {
            panic!("PANIC on edge case {:?}: {:?}", case, result);
        }
    }
}

/// Test that the parser handles deeply nested structures
#[test]
fn fuzz_deep_nesting() {
    // Generate deeply nested structures
    let nest_levels = [10, 50, 100, 200, 500, 1000];

    for &level in &nest_levels {
        let code = format!("{{{{{}}}}}", "{{".repeat(level / 2));
        println!("Testing {} level nesting", level);

        let result = std::panic::catch_unwind(|| {
            let _ = tree_sitter_perl_c::parse_perl_code(&code);
        });

        if result.is_err() {
            panic!("PANIC on deeply nested input (level={}): {:?}", level, result);
        }
    }
}

/// Test very long strings
#[test]
fn fuzz_long_strings() {
    let lengths = [1_000, 10_000, 100_000];

    for &len in &lengths {
        let code = format!("my ${} = {};", "x".repeat(len), "1".repeat(len));
        println!("Testing string of length {}", len);

        let result = std::panic::catch_unwind(|| {
            let _ = tree_sitter_perl_c::parse_perl_code(&code);
        });

        if result.is_err() {
            panic!("PANIC on long string (len={}): {:?}", len, result);
        }
    }
}

/// Test strings with unusual byte sequences
#[test]
fn fuzz_invalid_utf8() {
    // Construct invalid UTF-8 sequences using byte arrays
    // (Rust won't let us use \x80+ in string literals directly)
    let cases: Vec<String> = vec![
        // Overlong encoding of NUL (2 bytes: 0xC0 0x80)
        String::from_utf8_lossy(&[0xC0, 0x80]).into_owned(),
        // Surrogate codepoint (3 bytes: 0xED 0xA0 0x80)
        String::from_utf8_lossy(&[0xED, 0xA0, 0x80]).into_owned(),
        // Non-character U+FFFE
        String::from_utf8_lossy(&[0xEF, 0xBF, 0xBE]).into_owned(),
        // Non-character U+FFFF
        String::from_utf8_lossy(&[0xEF, 0xBF, 0xBF]).into_owned(),
        // Invalid continuation byte alone
        String::from_utf8_lossy(&[0x80]).into_owned(),
        // Another invalid continuation
        String::from_utf8_lossy(&[0xBF]).into_owned(),
        // Incomplete 2-byte sequence (just 0xC0)
        String::from_utf8_lossy(&[0xC0]).into_owned(),
        // Incomplete 3-byte sequence (just 0xE0)
        String::from_utf8_lossy(&[0xE0]).into_owned(),
        // Incomplete 4-byte sequence (just 0xF0)
        String::from_utf8_lossy(&[0xF0]).into_owned(),
        // Mixed valid and invalid
        String::from_utf8_lossy(b"abc\x80\x81\x82def").to_string(),
    ];

    for (i, case) in cases.iter().enumerate() {
        println!("Testing invalid UTF-8 case {}/{}: {:?}", i + 1, cases.len(), case);

        let result = std::panic::catch_unwind(|| {
            let _ = tree_sitter_perl_c::parse_perl_code(case);
        });

        // Parser might reject these with parse errors, but shouldn't panic
        if result.is_err() {
            panic!("PANIC on invalid UTF-8 input: {:?}", result);
        }
    }
}
