#[cfg(test)]
mod integration_corpus {
    use std::fs;
    use std::path::Path;
    use crate::test_harness::{parse_corpus_file, test_corpus_file_parses};

    #[test]
    fn test_parse_all_corpus_files() {
        let corpus_dir = Path::new("test/corpus");
        let mut test_count = 0;
        let mut failed_files = Vec::new();

        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                test_count += 1;
                match test_corpus_file_parses(&path) {
                    Ok(()) => println!("✓ {:?}", path.file_name().unwrap()),
                    Err(e) => {
                        println!("✗ {:?}: {}", path.file_name().unwrap(), e);
                        failed_files.push((path, e));
                    }
                }
            }
        }

        println!("\nCorpus test summary:");
        println!("Total files: {}", test_count);
        println!("Passed: {}", test_count - failed_files.len());
        println!("Failed: {}", failed_files.len());

        if !failed_files.is_empty() {
            println!("\nFailed files:");
            for (path, error) in failed_files {
                println!("  {:?}: {}", path.file_name().unwrap(), error);
            }
            panic!("Some corpus files failed to parse");
        }
    }

    #[test]
    fn test_parse_specific_corpus_files() {
        // Test specific corpus files that are known to be complex
        let test_files = [
            "test/corpus/simple",
            "test/corpus/expressions", 
            "test/corpus/heredocs",
            "test/corpus/interpolation",
        ];

        for file_path in &test_files {
            let path = Path::new(file_path);
            assert!(
                test_corpus_file_parses(path).is_ok(),
                "Failed to parse {:?}",
                path
            );
        }
    }

    #[test]
    fn test_corpus_file_contents() {
        // Test that we can extract and validate the content of corpus files
        let corpus_dir = Path::new("test/corpus");
        
        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                // Read the file content
                let content = fs::read_to_string(&path).unwrap();
                
                // Ensure it's not empty (corpus files should have content)
                assert!(!content.trim().is_empty(), "Corpus file {:?} is empty", path);
                
                // Ensure it contains test cases (should have ==== separators)
                assert!(
                    content.contains("===="),
                    "Corpus file {:?} doesn't contain test case separators",
                    path
                );
            }
        }
    }
} 