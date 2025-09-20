/// Regression test against existing fuzzed corpus
///
/// This test ensures that the substitution operator parsing improvements
/// don't break parsing of the existing fuzzed corpus files.
use perl_parser::Parser;
use std::fs;
use std::path::PathBuf;

fn get_fuzzed_files() -> Vec<PathBuf> {
    let fuzz_dir = "/home/steven/code/Rust/perl-lsp/review/benchmark_tests/fuzzed";

    if let Ok(entries) = fs::read_dir(fuzz_dir) {
        entries
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|s| s.to_str()) == Some("pl"))
            .take(20) // Limit to first 20 files for bounded testing
            .collect()
    } else {
        Vec::new()
    }
}

#[test]
fn test_fuzz_corpus_regression() {
    let fuzzed_files = get_fuzzed_files();

    if fuzzed_files.is_empty() {
        println!("No fuzzed files found, skipping corpus regression test");
        return;
    }

    let mut parse_failures = 0;
    let mut parse_panics = 0;
    let mut total_files = 0;

    for file_path in fuzzed_files {
        total_files += 1;

        if let Ok(content) = fs::read_to_string(&file_path) {
            // Test that parser doesn't panic on existing fuzzed content
            let result = std::panic::catch_unwind(|| {
                let mut parser = Parser::new(&content);
                parser.parse()
            });

            match result {
                Ok(parse_result) => {
                    if parse_result.is_err() {
                        parse_failures += 1;
                    }
                }
                Err(_) => {
                    parse_panics += 1;
                    eprintln!("Parser panicked on file: {:?}", file_path);
                }
            }
        }
    }

    println!("Fuzz corpus regression test results:");
    println!("  Total files tested: {}", total_files);
    println!("  Parse failures: {}", parse_failures);
    println!("  Parse panics: {}", parse_panics);

    // The key invariant: parser should never panic, even on malformed input
    assert_eq!(parse_panics, 0, "Parser should never panic on fuzzed corpus");

    // Parse failures are acceptable for malformed fuzzed input
    // We're primarily checking for crashes/panics, not parse success
}
