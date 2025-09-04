// Test harness for corpus gap coverage
// These tests ensure the parser handles real-world Perl features missing from original corpus

use std::fs;
use std::path::Path;

#[cfg(test)]
mod corpus_gap_tests {
    use super::*;
    
    // Helper to test a corpus file doesn't crash the parser
    fn test_corpus_file(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        let path = Path::new("test_corpus").join(filename);
        let content = fs::read_to_string(&path)?;
        
        // Test with v3 parser (recommended)
        #[cfg(feature = "v3-parser")]
        {
            use perl_parser::{parse, parse_file};
            let result = parse(&content);
            assert!(result.is_ok(), "v3 parser failed on {}: {:?}", filename, result);
            
            // Ensure we get an AST
            let ast = result.unwrap();
            assert!(ast.root.is_some(), "v3 parser produced no AST for {}", filename);
        }
        
        // Test with v2 parser (Pest-based)
        #[cfg(feature = "pure-rust")]
        {
            use tree_sitter_perl_rs::parse_perl;
            let result = parse_perl(&content);
            assert!(result.is_ok(), "v2 parser failed on {}: {:?}", filename, result);
        }
        
        Ok(())
    }
    
    // Helper to test LSP doesn't hang or crash
    #[cfg(feature = "lsp")]
    fn test_lsp_stability(filename: &str) -> Result<(), Box<dyn std::error::Error>> {
        use perl_parser::lsp::{LspServer, InitializeParams};
        use lsp_types::{Url, TextDocumentItem, DidOpenTextDocumentParams};
        
        let path = Path::new("test_corpus").join(filename);
        let content = fs::read_to_string(&path)?;
        let uri = Url::from_file_path(&path).unwrap();
        
        // Initialize LSP
        let mut server = LspServer::new();
        server.initialize(InitializeParams::default())?;
        
        // Open document
        let params = DidOpenTextDocumentParams {
            text_document: TextDocumentItem {
                uri: uri.clone(),
                language_id: "perl".to_string(),
                version: 1,
                text: content,
            },
        };
        
        // Ensure basic operations don't hang
        let result = server.did_open(params);
        assert!(result.is_ok(), "LSP did_open failed for {}", filename);
        
        // Request diagnostics (shouldn't hang even on complex syntax)
        let diagnostics = server.get_diagnostics(&uri);
        assert!(diagnostics.is_ok(), "LSP diagnostics failed for {}", filename);
        
        // Request document symbols (should handle all packages/subs)
        let symbols = server.document_symbol(&uri);
        assert!(symbols.is_ok(), "LSP symbols failed for {}", filename);
        
        Ok(())
    }
    
    #[test]
    fn test_source_filters() {
        test_corpus_file("source_filters.pl")
            .expect("Source filter test failed");
    }
    
    #[test]
    fn test_xs_inline_ffi() {
        test_corpus_file("xs_inline_ffi.pl")
            .expect("XS/Inline/FFI test failed");
    }
    
    #[test]
    fn test_modern_perl_features() {
        test_corpus_file("modern_perl_features.pl")
            .expect("Modern Perl features test failed");
    }
    
    #[test]
    fn test_advanced_regex() {
        test_corpus_file("advanced_regex.pl")
            .expect("Advanced regex test failed");
    }
    
    #[test]
    fn test_data_sections() {
        test_corpus_file("data_end_sections.pl")
            .expect("DATA section test failed");
            
        test_corpus_file("end_section.pl")
            .expect("END section test failed");
    }
    
    #[test]
    fn test_packages_versions() {
        test_corpus_file("packages_versions.pl")
            .expect("Package/version test failed");
    }
    
    #[test]
    fn test_legacy_syntax() {
        test_corpus_file("legacy_syntax.pl")
            .expect("Legacy syntax test failed");
    }

    #[test]
    fn test_given_when_default() {
        test_corpus_file("given_when_default.pl")
            .expect("given/when/default test failed");
    }
    
    // LSP-specific tests
    #[test]
    #[cfg(feature = "lsp")]
    fn test_lsp_source_filters() {
        test_lsp_stability("source_filters.pl")
            .expect("LSP source filter test failed");
    }
    
    #[test]
    #[cfg(feature = "lsp")]
    fn test_lsp_modern_features() {
        test_lsp_stability("modern_perl_features.pl")
            .expect("LSP modern features test failed");
    }
    
    #[test]
    #[cfg(feature = "lsp")]
    fn test_lsp_data_sections() {
        use perl_parser::lsp::get_diagnostics;
        
        let path = Path::new("test_corpus/data_end_sections.pl");
        let content = fs::read_to_string(&path).expect("Failed to read file");
        
        // Parse and get line where __DATA__ appears
        let data_line = content.lines()
            .position(|line| line.trim() == "__DATA__")
            .expect("No __DATA__ found");
        
        // Get diagnostics
        let diagnostics = get_diagnostics(&content);
        
        // Assert no diagnostics after __DATA__ line
        for diag in &diagnostics {
            assert!(
                diag.range.start.line < data_line as u32,
                "Found diagnostic after __DATA__ at line {}: {}",
                diag.range.start.line,
                diag.message
            );
        }
    }
    
    #[test]
    #[cfg(feature = "lsp")]
    fn test_lsp_multi_package_symbols() {
        use perl_parser::lsp::{get_document_symbols};
        
        let path = Path::new("test_corpus/packages_versions.pl");
        let content = fs::read_to_string(&path).expect("Failed to read file");
        
        let symbols = get_document_symbols(&content);
        
        // Should find multiple packages
        let package_names: Vec<String> = symbols.iter()
            .filter(|s| s.kind == SymbolKind::Module)
            .map(|s| s.name.clone())
            .collect();
        
        assert!(package_names.len() >= 5, "Should find at least 5 packages");
        assert!(package_names.contains(&"Simple::Package".to_string()));
        assert!(package_names.contains(&"Versioned::Package".to_string()));
        assert!(package_names.contains(&"First::Module".to_string()));
    }
    
    #[test]
    fn test_regex_folding_ranges() {
        use perl_parser::lsp::get_folding_ranges;
        
        let path = Path::new("test_corpus/advanced_regex.pl");
        let content = fs::read_to_string(&path).expect("Failed to read file");
        
        let ranges = get_folding_ranges(&content);
        
        // Should have folding ranges for multi-line regex
        assert!(!ranges.is_empty(), "Should have folding ranges for complex regex");
    }
    
    // Property-based test for delimiters
    #[test]
    fn test_arbitrary_delimiters() {
        let delimiters = vec![
            ('!', '!'), ('{', '}'), ('[', ']'), 
            ('(', ')'), ('<', '>'), ('|', '|'),
            ('#', '#'), ('/', '/'), ('@', '@'),
        ];
        
        for (open, close) in delimiters {
            let code = format!("m{open}pattern{close}", open=open, close=close);
            
            #[cfg(feature = "v3-parser")]
            {
                use perl_parser::parse;
                let result = parse(&code);
                assert!(result.is_ok(), "Failed to parse m{}{}", open, close);
            }
        }
    }
    
    // Benchmark corpus files (optional, run with --release)
    #[test]
    #[ignore] // Run with: cargo test --ignored --release
    fn bench_corpus_files() {
        use std::time::Instant;
        
        let files = vec![
            "source_filters.pl",
            "xs_inline_ffi.pl", 
            "modern_perl_features.pl",
            "advanced_regex.pl",
            "data_end_sections.pl",
            "packages_versions.pl",
            "legacy_syntax.pl",
        ];
        
        for file in files {
            let path = Path::new("test_corpus").join(file);
            let content = fs::read_to_string(&path).expect("Failed to read file");
            
            let start = Instant::now();
            for _ in 0..100 {
                #[cfg(feature = "v3-parser")]
                {
                    use perl_parser::parse;
                    let _ = parse(&content);
                }
            }
            let duration = start.elapsed();
            
            println!("{}: {:?} per parse", file, duration / 100);
        }
    }
}