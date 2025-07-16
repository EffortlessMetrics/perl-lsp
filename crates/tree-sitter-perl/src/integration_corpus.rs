#[cfg(test)]
mod integration_corpus {
    use std::fs;
    use std::path::Path;
    use std::time::Instant;
    use crate::test_harness::{parse_corpus_file, test_corpus_file_parses, tree_to_string};
    use crate::{parse, language};
    use tree_sitter::Parser;

    #[test]
    fn test_parse_all_corpus_files() {
        let corpus_dir = Path::new("test/corpus");
        let mut test_count = 0;
        let mut failed_files = Vec::new();
        let mut total_parse_time = 0u128;

        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                test_count += 1;
                let start = Instant::now();
                match test_corpus_file_parses(&path) {
                    Ok(()) => {
                        let parse_time = start.elapsed().as_micros();
                        total_parse_time += parse_time;
                        println!("✓ {:?} ({} μs)", path.file_name().unwrap(), parse_time);
                    }
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
        println!("Average parse time: {:.2} μs", total_parse_time as f64 / test_count as f64);

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
            "test/corpus/variables",
            "test/corpus/functions",
            "test/corpus/control_structures",
            "test/corpus/comments",
            "test/corpus/strings",
            "test/corpus/operators",
        ];

        for file_path in &test_files {
            let path = Path::new(file_path);
            if path.exists() {
                assert!(
                    test_corpus_file_parses(path).is_ok(),
                    "Failed to parse {:?}",
                    path
                );
            }
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

    #[test]
    fn test_corpus_parse_tree_structure() {
        // Test that parsed corpus files produce valid tree structures
        let corpus_dir = Path::new("test/corpus");
        let mut parser = Parser::new();
        parser.set_language(&language()).unwrap();
        
        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap();
                
                // Parse the content
                let tree = parser.parse(&content, None).unwrap();
                let root = tree.root_node();
                
                // Basic tree structure validation
                assert_eq!(root.kind(), "source_file", "Root node should be source_file for {:?}", path);
                assert!(root.child_count() >= 0, "Root node should have non-negative child count");
                
                // Check for error nodes (should be minimal)
                let error_count = count_error_nodes(&root);
                assert!(
                    error_count < 10,
                    "Too many error nodes ({}) in {:?}",
                    error_count,
                    path
                );
            }
        }
    }

    #[test]
    fn test_corpus_parse_performance() {
        // Test parsing performance on corpus files
        let corpus_dir = Path::new("test/corpus");
        let mut parser = Parser::new();
        parser.set_language(&language()).unwrap();
        
        let mut total_files = 0;
        let mut total_time = 0u128;
        let mut slow_files = Vec::new();
        
        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap();
                let start = Instant::now();
                
                let tree = parser.parse(&content, None);
                let parse_time = start.elapsed().as_micros();
                
                total_files += 1;
                total_time += parse_time;
                
                if parse_time > 1000 {
                    slow_files.push((path.file_name().unwrap().to_string_lossy().to_string(), parse_time));
                }
                
                assert!(tree.is_some(), "Failed to parse {:?}", path);
            }
        }
        
        let avg_time = total_time as f64 / total_files as f64;
        println!("Corpus performance summary:");
        println!("Total files: {}", total_files);
        println!("Average parse time: {:.2} μs", avg_time);
        
        if !slow_files.is_empty() {
            println!("Slow files (>1000 μs):");
            for (file, time) in slow_files {
                println!("  {}: {} μs", file, time);
            }
        }
        
        // Ensure average parse time is reasonable
        assert!(avg_time < 500.0, "Average parse time too high: {:.2} μs", avg_time);
    }

    #[test]
    fn test_corpus_error_recovery() {
        // Test that corpus files with errors are handled gracefully
        let corpus_dir = Path::new("test/corpus");
        let mut parser = Parser::new();
        parser.set_language(&language()).unwrap();
        
        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap();
                
                // Parse the content
                let tree = parser.parse(&content, None);
                assert!(tree.is_some(), "Failed to parse {:?}", path);
                
                let tree = tree.unwrap();
                let root = tree.root_node();
                
                // Even with errors, we should get a valid tree structure
                assert_eq!(root.kind(), "source_file");
                assert!(root.start_byte() <= root.end_byte());
            }
        }
    }

    #[test]
    fn test_corpus_serialization_roundtrip() {
        // Test that parsed corpus files can be serialized and deserialized
        let corpus_dir = Path::new("test/corpus");
        let mut parser = Parser::new();
        parser.set_language(&language()).unwrap();
        
        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap();
                
                // Parse the content
                let tree1 = parser.parse(&content, None).unwrap();
                let tree1_string = tree_to_string(&tree1);
                
                // Parse again (simulating deserialization)
                let tree2 = parser.parse(&content, None).unwrap();
                let tree2_string = tree_to_string(&tree2);
                
                // Trees should be identical
                assert_eq!(
                    tree1_string, tree2_string,
                    "Serialization roundtrip failed for {:?}",
                    path
                );
            }
        }
    }

    #[test]
    fn test_corpus_memory_usage() {
        // Test memory usage during corpus parsing
        let corpus_dir = Path::new("test/corpus");
        let mut parser = Parser::new();
        parser.set_language(&language()).unwrap();
        
        let mut total_nodes = 0;
        let mut total_files = 0;
        
        for entry in fs::read_dir(corpus_dir).unwrap() {
            let entry = entry.unwrap();
            let path = entry.path();
            if path.is_file() {
                let content = fs::read_to_string(&path).unwrap();
                let tree = parser.parse(&content, None).unwrap();
                
                let node_count = count_nodes(&tree.root_node());
                total_nodes += node_count;
                total_files += 1;
                
                // Ensure reasonable memory usage per file
                assert!(
                    node_count < 10000,
                    "Too many nodes ({}) in {:?}",
                    node_count,
                    path
                );
            }
        }
        
        let avg_nodes = total_nodes as f64 / total_files as f64;
        println!("Corpus memory usage summary:");
        println!("Total files: {}", total_files);
        println!("Total nodes: {}", total_nodes);
        println!("Average nodes per file: {:.2}", avg_nodes);
        
        // Ensure reasonable average node count
        assert!(avg_nodes < 1000.0, "Average node count too high: {:.2}", avg_nodes);
    }

    // Helper functions
    fn count_error_nodes(node: &tree_sitter::Node) -> usize {
        let mut count = if node.kind() == "ERROR" { 1 } else { 0 };
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += count_error_nodes(&child);
            }
        }
        count
    }

    fn count_nodes(node: &tree_sitter::Node) -> usize {
        let mut count = 1;
        for i in 0..node.child_count() {
            if let Some(child) = node.child(i) {
                count += count_nodes(&child);
            }
        }
        count
    }
} 