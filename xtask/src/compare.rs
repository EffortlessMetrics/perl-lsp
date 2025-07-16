use clap::Args;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::Instant;
use serde_json::json;

#[derive(Args)]
pub struct CompareArgs {
    /// Test cases to run (comma-separated file paths or "corpus" for all corpus files)
    #[arg(short, long, default_value = "corpus")]
    test_cases: String,

    /// Number of iterations per test case
    #[arg(short, long, default_value = "100")]
    iterations: usize,

    /// Output directory for results
    #[arg(short, long, default_value = "benchmark_results")]
    output_dir: PathBuf,

    /// Generate detailed report
    #[arg(long)]
    report: bool,
}

pub fn run_compare(args: CompareArgs) -> Result<(), Box<dyn std::error::Error>> {
    println!("🔍 Comparing C and Rust implementations of tree-sitter-perl");
    println!("==================================================");

    // Ensure output directory exists
    std::fs::create_dir_all(&args.output_dir)?;

    // Get test cases
    let test_cases = if args.test_cases == "corpus" {
        get_corpus_files()?
    } else {
        args.test_cases
            .split(',')
            .map(|s| s.trim().to_string())
            .collect()
    };

    println!("📁 Found {} test cases", test_cases.len());

    // Test C implementation
    println!("\n🔧 Testing C implementation...");
    let c_results = test_implementation("c", &test_cases, args.iterations)?;

    // Test Rust implementation
    println!("\n🦀 Testing Rust implementation...");
    let rust_results = test_implementation("rust", &test_cases, args.iterations)?;

    // Generate comparison report
    println!("\n📊 Generating comparison report...");
    let report = generate_comparison_report(&c_results, &rust_results, &test_cases)?;

    // Save results
    let results_file = args.output_dir.join("comparison_results.json");
    std::fs::write(&results_file, serde_json::to_string_pretty(&report)?)?;
    println!("💾 Results saved to: {}", results_file.display());

    if args.report {
        let report_file = args.output_dir.join("comparison_report.md");
        std::fs::write(&report_file, generate_markdown_report(&report)?)?;
        println!("📄 Report saved to: {}", report_file.display());
    }

    // Print summary
    print_summary(&report);

    Ok(())
}

fn get_corpus_files() -> Result<Vec<String>, Box<dyn std::error::Error>> {
    let corpus_dir = PathBuf::from("tree-sitter-perl/test/corpus");
    if !corpus_dir.exists() {
        return Err("Corpus directory not found".into());
    }

    let mut files = Vec::new();
    for entry in std::fs::read_dir(corpus_dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_file() && path.extension().map_or(false, |ext| ext == "txt") {
            files.push(path.to_string_lossy().to_string());
        }
    }

    Ok(files)
}

fn test_implementation(
    impl_type: &str,
    test_cases: &[String],
    iterations: usize,
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut results = json!({
        "implementation": impl_type,
        "test_cases": {},
        "summary": {}
    });

    let mut total_time = 0.0;
    let mut total_memory = 0.0;
    let mut success_count = 0;

    for (i, test_case) in test_cases.iter().enumerate() {
        println!("  [{}/{}] Testing: {}", i + 1, test_cases.len(), test_case);

        let test_result = run_single_test(impl_type, test_case, iterations)?;
        
        if let Some(result) = test_result {
            total_time += result["avg_time"].as_f64().unwrap_or(0.0);
            total_memory += result["avg_memory"].as_f64().unwrap_or(0.0);
            success_count += 1;
        }

        results["test_cases"][test_case] = test_result.unwrap_or(json!(null));
    }

    // Calculate summary statistics
    results["summary"] = json!({
        "total_test_cases": test_cases.len(),
        "successful_tests": success_count,
        "total_time": total_time,
        "total_memory": total_memory,
        "avg_time_per_test": if success_count > 0 { total_time / success_count as f64 } else { 0.0 },
        "avg_memory_per_test": if success_count > 0 { total_memory / success_count as f64 } else { 0.0 }
    });

    Ok(results)
}

fn run_single_test(
    impl_type: &str,
    test_case: &str,
    iterations: usize,
) -> Result<Option<serde_json::Value>, Box<dyn std::error::Error>> {
    let test_content = std::fs::read_to_string(test_case)?;
    
    let mut times = Vec::new();
    let mut memories = Vec::new();
    let mut success = false;

    for _ in 0..iterations {
        let start = Instant::now();
        
        let result = match impl_type {
            "c" => test_c_implementation(&test_content),
            "rust" => test_rust_implementation(&test_content),
            _ => return Err("Unknown implementation type".into()),
        };

        let elapsed = start.elapsed().as_micros() as f64;
        
        match result {
            Ok(_) => {
                times.push(elapsed);
                memories.push(0.0); // TODO: Add memory measurement
                success = true;
            }
            Err(e) => {
                eprintln!("    ⚠️  Test failed: {}", e);
                break;
            }
        }
    }

    if !success {
        return Ok(None);
    }

    // Calculate statistics
    times.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let avg_time = times.iter().sum::<f64>() / times.len() as f64;
    let min_time = times[0];
    let max_time = times[times.len() - 1];
    let median_time = if times.len() % 2 == 0 {
        (times[times.len() / 2 - 1] + times[times.len() / 2]) / 2.0
    } else {
        times[times.len() / 2]
    };

    Ok(Some(json!({
        "iterations": iterations,
        "successful_iterations": times.len(),
        "avg_time": avg_time,
        "min_time": min_time,
        "max_time": max_time,
        "median_time": median_time,
        "avg_memory": 0.0, // TODO: Add memory measurement
        "file_size": test_content.len()
    })))
}

fn test_c_implementation(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Change to C implementation directory
    let c_dir = PathBuf::from("tree-sitter-perl");
    
    // Create a temporary test file
    let temp_file = std::env::temp_dir().join("test_perl_c.tmp");
    std::fs::write(&temp_file, content)?;

    // Run the C implementation test
    let output = Command::new("cargo")
        .current_dir(&c_dir)
        .args(["test", "--lib", "--", "--nocapture"])
        .stdin(Stdio::from_file(std::fs::File::open(&temp_file)?))
        .output()?;

    // Clean up
    let _ = std::fs::remove_file(temp_file);

    if !output.status.success() {
        return Err(format!("C implementation test failed: {}", 
            String::from_utf8_lossy(&output.stderr)).into());
    }

    Ok(())
}

fn test_rust_implementation(content: &str) -> Result<(), Box<dyn std::error::Error>> {
    // Change to Rust implementation directory
    let rust_dir = PathBuf::from("crates/tree-sitter-perl-rs");
    
    // Create a temporary test file
    let temp_file = std::env::temp_dir().join("test_perl_rust.tmp");
    std::fs::write(&temp_file, content)?;

    // Run the Rust implementation test
    let output = Command::new("cargo")
        .current_dir(&rust_dir)
        .args(["test", "--lib", "--", "--nocapture"])
        .stdin(Stdio::from_file(std::fs::File::open(&temp_file)?))
        .output()?;

    // Clean up
    let _ = std::fs::remove_file(temp_file);

    if !output.status.success() {
        return Err(format!("Rust implementation test failed: {}", 
            String::from_utf8_lossy(&output.stderr)).into());
    }

    Ok(())
}

fn generate_comparison_report(
    c_results: &serde_json::Value,
    rust_results: &serde_json::Value,
    test_cases: &[String],
) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
    let mut report = json!({
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "test_cases": test_cases,
        "implementations": {
            "c": c_results,
            "rust": rust_results
        },
        "comparison": {}
    });

    // Calculate performance differences
    let c_summary = &c_results["summary"];
    let rust_summary = &rust_results["summary"];

    let c_avg_time = c_summary["avg_time"].as_f64().unwrap_or(0.0);
    let rust_avg_time = rust_summary["avg_time"].as_f64().unwrap_or(0.0);

    let time_diff = if c_avg_time > 0.0 {
        ((rust_avg_time - c_avg_time) / c_avg_time) * 100.0
    } else {
        0.0
    };

    report["comparison"] = json!({
        "time_difference_percent": time_diff,
        "rust_faster": time_diff < 0.0,
        "performance_ratio": if c_avg_time > 0.0 { rust_avg_time / c_avg_time } else { 1.0 },
        "success_rate": {
            "c": c_summary["successful_tests"].as_u64().unwrap_or(0),
            "rust": rust_summary["successful_tests"].as_u64().unwrap_or(0),
            "total": test_cases.len()
        }
    });

    Ok(report)
}

fn generate_markdown_report(report: &serde_json::Value) -> Result<String, Box<dyn std::error::Error>> {
    let timestamp = report["timestamp"].as_str().unwrap_or("Unknown");
    let comparison = &report["comparison"];
    let c_results = &report["implementations"]["c"];
    let rust_results = &report["implementations"]["rust"];

    let mut markdown = format!("# Tree-sitter Perl Implementation Comparison\n\n");
    markdown.push_str(&format!("**Generated:** {}\n\n", timestamp));

    // Summary
    markdown.push_str("## Summary\n\n");
    
    let time_diff = comparison["time_difference_percent"].as_f64().unwrap_or(0.0);
    let rust_faster = comparison["rust_faster"].as_bool().unwrap_or(false);
    
    markdown.push_str(&format!("- **Performance:** Rust implementation is {:.1}% {} than C implementation\n", 
        time_diff.abs(), if rust_faster { "faster" } else { "slower" }));
    
    let c_success = comparison["success_rate"]["c"].as_u64().unwrap_or(0);
    let rust_success = comparison["success_rate"]["rust"].as_u64().unwrap_or(0);
    let total = comparison["success_rate"]["total"].as_u64().unwrap_or(0);
    
    markdown.push_str(&format!("- **Success Rate:** C: {}/{} ({}%), Rust: {}/{} ({}%)\n",
        c_success, total, (c_success as f64 / total as f64 * 100.0) as i32,
        rust_success, total, (rust_success as f64 / total as f64 * 100.0) as i32));

    // Detailed Results
    markdown.push_str("\n## Detailed Results\n\n");
    
    let c_avg_time = c_results["summary"]["avg_time"].as_f64().unwrap_or(0.0);
    let rust_avg_time = rust_results["summary"]["avg_time"].as_f64().unwrap_or(0.0);
    
    markdown.push_str("| Metric | C Implementation | Rust Implementation | Difference |\n");
    markdown.push_str("|--------|------------------|---------------------|------------|\n");
    markdown.push_str(&format!("| Avg Time (μs) | {:.2} | {:.2} | {:.1}% |\n",
        c_avg_time, rust_avg_time, time_diff));
    
    let c_total_time = c_results["summary"]["total_time"].as_f64().unwrap_or(0.0);
    let rust_total_time = rust_results["summary"]["total_time"].as_f64().unwrap_or(0.0);
    markdown.push_str(&format!("| Total Time (μs) | {:.2} | {:.2} | {:.1}% |\n",
        c_total_time, rust_total_time, 
        if c_total_time > 0.0 { ((rust_total_time - c_total_time) / c_total_time) * 100.0 } else { 0.0 }));

    markdown.push_str("\n## Test Case Results\n\n");
    markdown.push_str("| Test Case | C Time (μs) | Rust Time (μs) | Difference |\n");
    markdown.push_str("|-----------|-------------|----------------|------------|\n");

    for test_case in report["test_cases"].as_array().unwrap_or(&Vec::new()) {
        let test_case_str = test_case.as_str().unwrap_or("Unknown");
        let c_result = &c_results["test_cases"][test_case_str];
        let rust_result = &rust_results["test_cases"][test_case_str];
        
        if let (Some(c_time), Some(rust_time)) = (
            c_result["avg_time"].as_f64(),
            rust_result["avg_time"].as_f64()
        ) {
            let diff = if c_time > 0.0 { ((rust_time - c_time) / c_time) * 100.0 } else { 0.0 };
            markdown.push_str(&format!("| {} | {:.2} | {:.2} | {:.1}% |\n",
                test_case_str, c_time, rust_time, diff));
        }
    }

    Ok(markdown)
}

fn print_summary(report: &serde_json::Value) {
    let comparison = &report["comparison"];
    let time_diff = comparison["time_difference_percent"].as_f64().unwrap_or(0.0);
    let rust_faster = comparison["rust_faster"].as_bool().unwrap_or(false);
    
    println!("\n📈 Comparison Summary");
    println!("===================");
    println!("🦀 Rust is {:.1}% {} than C", 
        time_diff.abs(), if rust_faster { "faster" } else { "slower" });
    
    let c_success = comparison["success_rate"]["c"].as_u64().unwrap_or(0);
    let rust_success = comparison["success_rate"]["rust"].as_u64().unwrap_or(0);
    let total = comparison["success_rate"]["total"].as_u64().unwrap_or(0);
    
    println!("✅ Success Rate - C: {}/{} ({}%), Rust: {}/{} ({}%)",
        c_success, total, (c_success as f64 / total as f64 * 100.0) as i32,
        rust_success, total, (rust_success as f64 / total as f64 * 100.0) as i32);
} 