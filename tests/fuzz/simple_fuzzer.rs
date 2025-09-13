#!/usr/bin/env cargo run --bin
//! Simple focused fuzzer to identify stack overflow and other issues

use std::panic;
use std::time::{Duration, Instant};

/// Simple fuzz result
#[derive(Debug)]
pub enum FuzzResult {
    Success,
    Panic(String),
    Timeout,
}

/// Simple fuzzing harness with timeout and panic catching
pub struct SimpleFuzzer {
    pub timeout: Duration,
}

impl Default for SimpleFuzzer {
    fn default() -> Self {
        Self {
            timeout: Duration::from_secs(2),
        }
    }
}

impl SimpleFuzzer {
    pub fn new() -> Self {
        Self::default()
    }

    /// Test parser with panic catching and timeout
    pub fn test_parser(&self, input: &str) -> FuzzResult {
        let result = panic::catch_unwind(|| {
            let start = Instant::now();

            // Use perl-parser to parse the input
            use perl_parser::Parser;
            let mut parser = Parser::new(input);

            match parser.parse() {
                Ok(_) => {
                    if start.elapsed() > self.timeout {
                        FuzzResult::Timeout
                    } else {
                        FuzzResult::Success
                    }
                }
                Err(_) => {
                    // Parse errors are expected and fine
                    FuzzResult::Success
                }
            }
        });

        match result {
            Ok(res) => res,
            Err(panic_info) => {
                let panic_msg = if let Some(s) = panic_info.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = panic_info.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "Unknown panic".to_string()
                };
                FuzzResult::Panic(panic_msg)
            }
        }
    }

    /// Generate problematic inputs that might cause stack overflow
    pub fn generate_stack_overflow_inputs(&self) -> Vec<String> {
        vec![
            // Deep nesting that caused the original stack overflow
            format!("{}{}{}", "{ ".repeat(1000), "42", " }".repeat(1000)),
            format!("{}{}{}", "( ".repeat(500), "42", " )".repeat(500)),
            format!("{}{}{}", "[ ".repeat(500), "42", " ]".repeat(500)),

            // Deep function call nesting
            format!("{}42{}", "f(".repeat(200), ")".repeat(200)),

            // Deep if-else nesting
            format!("{}1{}{}", "if(1){".repeat(200), "}else{".repeat(199), "}"),

            // Deep array/hash access
            format!("$x{}", "->{a}".repeat(500)),
            format!("$x{}", "[0]".repeat(500)),

            // Deep regex nesting
            format!("/{}pattern{}/", "(".repeat(100), ")".repeat(100)),

            // More moderate nesting levels
            format!("{}{}{}", "{ ".repeat(50), "42", " }".repeat(50)),
            format!("{}{}{}", "( ".repeat(25), "42", " )".repeat(25)),
        ]
    }

    /// Generate other problematic patterns
    pub fn generate_problematic_inputs(&self) -> Vec<String> {
        vec![
            // Unclosed constructs
            "sub foo {".to_string(),
            "if ($x) {".to_string(),
            "while (1) {".to_string(),

            // Empty/minimal
            "".to_string(),
            "{".to_string(),
            "}".to_string(),

            // Recursive patterns
            "my $x = $x;".to_string(),
            "sub f { f() }".to_string(),

            // Large single tokens
            format!("my ${} = 42;", "a".repeat(10000)),
            format!("\"{}\"", "x".repeat(10000)),

            // Unicode/special chars
            "my $\u{1F4A9} = 42;".to_string(),
            format!("my $x = \"{}\";", "\x00\x01\x02"),
        ]
    }

    /// Run focused fuzz test
    pub fn run_focused_fuzz(&self) {
        println!("🎯 Running focused parser fuzzing...");

        let mut total_tests = 0;
        let mut panics = 0;
        let mut timeouts = 0;
        let mut crash_cases = Vec::new();

        // Test stack overflow inputs first
        println!("Testing stack overflow patterns...");
        let stack_inputs = self.generate_stack_overflow_inputs();

        for (i, input) in stack_inputs.iter().enumerate() {
            print!("  Stack test {}/{}: {} chars... ", i + 1, stack_inputs.len(), input.len());
            total_tests += 1;

            match self.test_parser(input) {
                FuzzResult::Success => println!("✅ OK"),
                FuzzResult::Panic(msg) => {
                    println!("💥 PANIC: {}", msg);
                    panics += 1;
                    crash_cases.push((format!("stack_{}", i), input.clone(), msg));
                }
                FuzzResult::Timeout => {
                    println!("⏰ TIMEOUT");
                    timeouts += 1;
                }
            }
        }

        // Test other problematic patterns
        println!("\nTesting other problematic patterns...");
        let problem_inputs = self.generate_problematic_inputs();

        for (i, input) in problem_inputs.iter().enumerate() {
            print!("  Problem test {}/{}: {} chars... ", i + 1, problem_inputs.len(), input.len());
            total_tests += 1;

            match self.test_parser(input) {
                FuzzResult::Success => println!("✅ OK"),
                FuzzResult::Panic(msg) => {
                    println!("💥 PANIC: {}", msg);
                    panics += 1;
                    crash_cases.push((format!("problem_{}", i), input.clone(), msg));
                }
                FuzzResult::Timeout => {
                    println!("⏰ TIMEOUT");
                    timeouts += 1;
                }
            }
        }

        // Test a few existing fuzz corpus files (if available)
        println!("\nTesting existing corpus samples...");
        if let Ok(mut entries) = std::fs::read_dir("../../benchmark_tests/fuzzed") {
            let mut corpus_tests = 0;
            while corpus_tests < 10 {
                if let Some(Ok(entry)) = entries.next() {
                    if let Ok(content) = std::fs::read_to_string(entry.path()) {
                        print!("  Corpus test: {}... ", entry.file_name().to_string_lossy());
                        total_tests += 1;
                        corpus_tests += 1;

                        match self.test_parser(&content) {
                            FuzzResult::Success => println!("✅ OK"),
                            FuzzResult::Panic(msg) => {
                                println!("💥 PANIC: {}", msg);
                                panics += 1;
                                crash_cases.push((format!("corpus_{}", entry.file_name().to_string_lossy()), content, msg));
                            }
                            FuzzResult::Timeout => {
                                println!("⏰ TIMEOUT");
                                timeouts += 1;
                            }
                        }
                    }
                } else {
                    break;
                }
            }
        }

        // Report results
        println!("\n📊 Fuzz Test Results:");
        println!("  Total tests: {}", total_tests);
        println!("  Successes: {}", total_tests - panics - timeouts);
        println!("  Panics: {}", panics);
        println!("  Timeouts: {}", timeouts);
        println!("  Success rate: {:.1}%", (total_tests - panics - timeouts) as f64 / total_tests as f64 * 100.0);

        // Save crash cases
        if !crash_cases.is_empty() {
            println!("\n💾 Saving {} crash reproduction cases...", crash_cases.len());
            let _ = std::fs::create_dir_all("repros");

            for (i, (name, input, crash_info)) in crash_cases.iter().enumerate() {
                let filename = format!("repros/crash_{:03}_{}.pl", i, name);
                if let Ok(mut file) = std::fs::File::create(&filename) {
                    use std::io::Write;
                    let _ = writeln!(file, "#!/usr/bin/perl");
                    let _ = writeln!(file, "# Crash reproduction case: {}", name);
                    let _ = writeln!(file, "# Crash info: {}", crash_info);
                    let _ = writeln!(file, "# Size: {} chars", input.len());
                    let _ = writeln!(file);
                    let _ = write!(file, "{}", input);
                    println!("  Saved: {}", filename);
                }
            }
        }
    }
}

fn main() {
    let fuzzer = SimpleFuzzer::new();
    fuzzer.run_focused_fuzz();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_inputs() {
        let fuzzer = SimpleFuzzer::new();

        // These should not panic
        let result = fuzzer.test_parser("my $x = 42;");
        matches!(result, FuzzResult::Success);

        let result = fuzzer.test_parser("");
        matches!(result, FuzzResult::Success);
    }

    #[test]
    fn test_moderate_nesting() {
        let fuzzer = SimpleFuzzer::new();

        // This should be safe
        let nested = format!("{}{}{}", "{ ".repeat(10), "42", " }".repeat(10));
        let result = fuzzer.test_parser(&nested);
        matches!(result, FuzzResult::Success);
    }
}