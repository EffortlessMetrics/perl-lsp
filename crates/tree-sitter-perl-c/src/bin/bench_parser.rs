use std::path::Path;
use tree_sitter_perl_c::{create_parser, parse_perl_code, parse_perl_file};
use walkdir::WalkDir;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let corpus_dir = args.get(1).unwrap_or(&"test/corpus".to_string());
    
    println!("Benchmarking C implementation...");
    println!("Corpus directory: {}", corpus_dir);
    
    let mut total_files = 0;
    let mut total_time = std::time::Duration::ZERO;
    let mut successful_parses = 0;
    let mut failed_parses = 0;
    
    // Walk through corpus directory
    for entry in WalkDir::new(corpus_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let path = entry.path();
        if let Some(extension) = path.extension() {
            if extension == "txt" || extension == "pm" || extension.to_str().is_none() {
                total_files += 1;
                
                let start = std::time::Instant::now();
                match parse_perl_file(path) {
                    Ok(tree) => {
                        let duration = start.elapsed();
                        total_time += duration;
                        successful_parses += 1;
                        
                        if tree.root_node().has_error() {
                            println!("⚠️  Parse errors in: {}", path.display());
                        }
                    }
                    Err(e) => {
                        failed_parses += 1;
                        println!("❌ Failed to parse: {} - {}", path.display(), e);
                    }
                }
            }
        }
    }
    
    println!("\n=== C Implementation Benchmark Results ===");
    println!("Total files processed: {}", total_files);
    println!("Successful parses: {}", successful_parses);
    println!("Failed parses: {}", failed_parses);
    println!("Total time: {:?}", total_time);
    
    if successful_parses > 0 {
        let avg_time = total_time / successful_parses;
        println!("Average parse time: {:?}", avg_time);
        println!("Parse success rate: {:.2}%", 
                 (successful_parses as f64 / total_files as f64) * 100.0);
    }
    
    // Test with some sample code
    println!("\n=== Sample Code Benchmarks ===");
    let sample_codes = vec![
        ("Simple variable", "my $var = 'hello';"),
        ("Function call", "print \"Hello, World!\";"),
        ("Control structure", "if ($condition) { return 1; } else { return 0; }"),
        ("Subroutine", "sub hello { my $name = shift; return \"Hello, $name!\"; }"),
        ("Complex expression", "my $result = $a + $b * ($c / $d) % $e;"),
    ];
    
    for (name, code) in sample_codes {
        let start = std::time::Instant::now();
        let iterations = 1000;
        
        for _ in 0..iterations {
            let _ = parse_perl_code(code);
        }
        
        let duration = start.elapsed();
        let avg_time = duration / iterations;
        println!("{}: {:?} per parse", name, avg_time);
    }
} 