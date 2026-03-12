/// Regression test against existing fuzzed corpus
///
/// This test ensures parser improvements don't reintroduce panics when
/// parsing known fuzz-generated and hand-curated stress inputs.
use perl_parser::Parser;
use std::fs;
use std::path::{Path, PathBuf};

const MAX_CORPUS_FILES: usize = 200;

fn collect_files_from_dir(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_file() {
                files.push(path);
            }
        }
    }

    files
}

fn candidate_corpus_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();

    if let Ok(custom_dir) = std::env::var("PERL_FUZZ_CORPUS_DIR") {
        let custom_path = PathBuf::from(custom_dir);
        if custom_path.exists() {
            dirs.push(custom_path);
        }
    }

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));

    // Canonical repository corpus location.
    let repo_corpus = manifest_dir.join("../perl-corpus/fuzz");
    if repo_corpus.exists() {
        dirs.push(repo_corpus);
    }

    // Legacy benchmark corpus location retained for compatibility.
    let benchmark_corpus = manifest_dir.join("../../benchmark_tests/fuzzed");
    if benchmark_corpus.exists() {
        dirs.push(benchmark_corpus);
    }

    dirs
}

fn get_fuzzed_files() -> Vec<PathBuf> {
    let mut files = Vec::new();

    for dir in candidate_corpus_dirs() {
        files.extend(collect_files_from_dir(&dir));
    }

    files.sort();
    files.dedup();

    if files.len() > MAX_CORPUS_FILES {
        files.truncate(MAX_CORPUS_FILES);
    }

    files
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
    let mut utf8_read_failures = 0;
    let mut total_files = 0;

    for file_path in fuzzed_files {
        total_files += 1;

        match fs::read_to_string(&file_path) {
            Ok(content) => {
                // Test that parser doesn't panic on existing fuzzed content.
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
            Err(_) => {
                // Some fuzz artifacts can be raw bytes and not valid UTF-8.
                utf8_read_failures += 1;
            }
        }
    }

    println!("Fuzz corpus regression test results:");
    println!("  Total files tested: {}", total_files);
    println!("  Parse failures: {}", parse_failures);
    println!("  Parse panics: {}", parse_panics);
    println!("  UTF-8 read failures (skipped): {}", utf8_read_failures);

    // The key invariant: parser should never panic, even on malformed input.
    assert_eq!(parse_panics, 0, "Parser should never panic on fuzzed corpus");

    // Parse failures are acceptable for malformed fuzzed input.
    // We're primarily checking for crashes/panics, not parse success.
}
