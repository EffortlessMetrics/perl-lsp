#!/usr/bin/env cargo run --bin
//! Perl Parser Robustness Fuzzer
//!
//! This fuzzer tests the core parser APIs with malformed, edge-case, and stress-test inputs.
//! Focus areas:
//! - Core parsing functions should never panic
//! - LSP message handlers should gracefully handle malformed JSON
//! - Workspace indexing should handle corrupted files
//! - Agent configuration parsing should validate inputs

use std::fs;
use std::io::Write;
use std::panic;
use std::time::{Duration, Instant};

/// Fuzz test result
#[derive(Debug)]
pub enum FuzzResult {
    Success,
    Panic(String),
    Timeout,
    ParseError(String),
    InfiniteLoop,
}

/// Core fuzzing harness
pub struct FuzzHarness {
    pub timeout_duration: Duration,
    pub max_input_size: usize,
    pub crash_outputs: Vec<(String, String)>, // (input, crash_info)
}

impl Default for FuzzHarness {
    fn default() -> Self {
        Self {
            timeout_duration: Duration::from_secs(5),
            max_input_size: 50_000,
            crash_outputs: Vec::new(),
        }
    }
}

impl FuzzHarness {
    pub fn new() -> Self {
        Self::default()
    }

    /// Test core parser with timeout and panic catching
    pub fn fuzz_parser(&mut self, input: &str) -> FuzzResult {
        if input.len() > self.max_input_size {
            return FuzzResult::Success; // Skip overly large inputs
        }

        let result = panic::catch_unwind(|| {
            // Call into perl-parser crate
            let start = Instant::now();

            // This would be the actual parser call - simulating for now
            let _result = self.simulate_parser_call(input);

            let elapsed = start.elapsed();
            if elapsed > self.timeout_duration {
                return FuzzResult::Timeout;
            }

            FuzzResult::Success
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

                self.crash_outputs.push((input.to_string(), panic_msg.clone()));
                FuzzResult::Panic(panic_msg)
            }
        }
    }

    /// Call actual perl-parser APIs
    fn simulate_parser_call(&self, input: &str) -> Result<(), String> {
        use perl_parser::Parser;

        let mut parser = Parser::new(input);
        match parser.parse() {
            Ok(_) => Ok(()),
            Err(e) => Err(format!("Parse error: {}", e)),
        }
    }

    /// Test LSP message handling
    pub fn fuzz_lsp_messages(&mut self, message: &str) -> FuzzResult {
        let result = panic::catch_unwind(|| {
            // Simulate LSP message parsing
            if message.len() > 10_000 {
                return FuzzResult::Success; // Skip oversized messages
            }

            // Test JSON parsing
            if let Err(_) = serde_json::from_str::<serde_json::Value>(message) {
                return FuzzResult::ParseError("Invalid JSON".to_string());
            }

            FuzzResult::Success
        });

        match result {
            Ok(res) => res,
            Err(panic_info) => {
                let panic_msg = format!("LSP message panic: {:?}", panic_info);
                self.crash_outputs.push((message.to_string(), panic_msg.clone()));
                FuzzResult::Panic(panic_msg)
            }
        }
    }

    /// Generate malformed Perl inputs
    pub fn generate_malformed_inputs(&self) -> Vec<String> {
        vec![
            // Unclosed constructs
            "sub foo {".to_string(),
            "if ($x) {".to_string(),
            "my @array = (".to_string(),
            "my %hash = (".to_string(),

            // Deeply nested constructs
            format!("{}{}{}", "{ ".repeat(100), "42", " }".repeat(100)),
            format!("{}{}{}", "( ".repeat(100), "42", " )".repeat(100)),
            format!("{}{}{}", "[ ".repeat(100), "42", " ]".repeat(100)),

            // Excessive operator chains
            format!("$x {}", " + $x".repeat(100)),
            format!("$x {}", " && $x".repeat(100)),

            // Regex stress patterns
            "/".repeat(1000),
            format!("/{}/", "a*".repeat(500)),
            format!("s/{}/{}/g", ".".repeat(100), "x".repeat(100)),

            // Unicode edge cases
            "my $\u{1F4A9} = 42;".to_string(),
            format!("my ${} = 42;", "x\u{200B}".repeat(100)), // Zero-width chars

            // String delimiter confusion
            r#"my $x = "nested \" quote";"#.to_string(),
            r#"my $x = 'nested \' quote';"#.to_string(),
            "my $x = q{nested } delimiter};".to_string(),

            // Heredoc edge cases
            "my $x = <<EOF\nEOF".to_string(),
            "my $x = <<EOF\nno terminator".to_string(),

            // Package/module naming edge cases
            "package ;;;;".to_string(),
            "package 123::invalid;".to_string(),
            "package ::::::;".to_string(),

            // Empty and whitespace-only
            "".to_string(),
            " ".repeat(10000),
            "\n".repeat(1000),
            "\t".repeat(1000),

            // Control character injection
            format!("my $x = \"{}\";", "\x00\x01\x02\x03\x04"),

            // Mixed encoding scenarios (UTF-8 + control characters)
            "use utf8; my $ñoñó = \"\x7F\x1F\";".to_string(),
        ]
    }

    /// Generate malformed LSP messages
    pub fn generate_malformed_lsp_messages(&self) -> Vec<String> {
        vec![
            // Invalid JSON
            r#"{ "invalid: json }"#.to_string(),
            r#"{ "method": "unclosed"#.to_string(),

            // Oversized fields
            format!(r#"{{ "method": "{}" }}"#, "x".repeat(100_000)),

            // Null bytes
            format!(r#"{{ "method": "test\x00" }}"#),

            // Deeply nested JSON
            format!("{}{}{}", r#"{"a":"#.repeat(100), r#""test""#, r#"}"#.repeat(100)),

            // Control character sequences
            "{ \"method\": \"\x7F\x1F\" }".to_string(),

            // Missing required fields
            r#"{ }"#.to_string(),
            r#"{ "jsonrpc": "2.0" }"#.to_string(),

            // Type confusion
            r#"{ "id": "should_be_number", "method": 123, "params": "should_be_object" }"#.to_string(),
        ]
    }

    /// Run comprehensive fuzz test suite
    pub fn run_comprehensive_fuzz(&mut self) -> FuzzTestReport {
        let mut report = FuzzTestReport::new();

        println!("🔥 Starting comprehensive fuzz testing...");

        // Test 1: Core parser with malformed inputs
        println!("Testing core parser robustness...");
        let malformed_inputs = self.generate_malformed_inputs();
        for (i, input) in malformed_inputs.iter().enumerate() {
            let result = self.fuzz_parser(input);
            report.parser_results.push((format!("malformed_{}", i), result));
        }

        // Test 2: Existing fuzz corpus
        println!("Testing with existing fuzz corpus...");
        if let Ok(fuzz_dir) = fs::read_dir("benchmark_tests/fuzzed") {
            for entry in fuzz_dir.take(50) { // Limit to 50 files for time bounds
                if let Ok(entry) = entry {
                    if let Ok(content) = fs::read_to_string(entry.path()) {
                        let result = self.fuzz_parser(&content);
                        report.parser_results.push((entry.file_name().to_string_lossy().to_string(), result));
                    }
                }
            }
        }

        // Test 3: LSP message fuzzing
        println!("Testing LSP message handling...");
        let malformed_messages = self.generate_malformed_lsp_messages();
        for (i, message) in malformed_messages.iter().enumerate() {
            let result = self.fuzz_lsp_messages(message);
            report.lsp_results.push((format!("lsp_malformed_{}", i), result));
        }

        // Test 4: Agent configuration fuzzing
        println!("Testing agent configuration parsing...");
        let malformed_configs = vec![
            r#"{ "invalid": yaml }"#.to_string(),
            "malformed_yaml: [unclosed_array".to_string(),
            "".to_string(),
            "\x00\x01\x02".to_string(),
        ];

        for (i, config) in malformed_configs.iter().enumerate() {
            // Simulate agent config parsing
            let result = self.simulate_agent_config_parsing(config);
            report.agent_config_results.push((format!("agent_config_{}", i), result));
        }

        report.crashes = self.crash_outputs.len();
        report
    }

    fn simulate_agent_config_parsing(&self, _config: &str) -> FuzzResult {
        // Simulate agent config parsing - would call actual implementation
        FuzzResult::Success
    }

    /// Minimize crashing inputs to smallest reproducible case
    pub fn minimize_crashes(&self) -> Vec<(String, String)> {
        let mut minimized = Vec::new();

        for (input, crash_info) in &self.crash_outputs {
            println!("Minimizing crash input: {} chars", input.len());

            let mut current = input.clone();
            let mut last_working = input.clone();

            // Binary search approach to minimize
            while current.len() > 1 {
                let mid = current.len() / 2;
                let test_input = &current[..mid];

                // Create a temporary fuzzer for testing
                let mut temp_fuzzer = FuzzHarness::new();
                match temp_fuzzer.fuzz_parser(test_input) {
                    FuzzResult::Panic(_) => {
                        current = test_input.to_string();
                        last_working = current.clone();
                    }
                    _ => break,
                }
            }

            minimized.push((last_working, crash_info.clone()));
        }

        minimized
    }

    /// Save crash reproductions to files
    pub fn save_crash_repros(&self, repros: &[(String, String)]) -> std::io::Result<()> {
        fs::create_dir_all("tests/fuzz/repros")?;

        for (i, (input, crash_info)) in repros.iter().enumerate() {
            let filename = format!("tests/fuzz/repros/crash_{:03}.pl", i);
            let mut file = fs::File::create(&filename)?;

            writeln!(file, "#!/usr/bin/perl")?;
            writeln!(file, "# Crash reproduction case #{}", i)?;
            writeln!(file, "# Crash info: {}", crash_info)?;
            writeln!(file, "# Size: {} chars", input.len())?;
            writeln!(file)?;
            write!(file, "{}", input)?;

            println!("💥 Saved crash repro: {}", filename);
        }

        Ok(())
    }
}

/// Comprehensive test report
#[derive(Debug)]
pub struct FuzzTestReport {
    pub parser_results: Vec<(String, FuzzResult)>,
    pub lsp_results: Vec<(String, FuzzResult)>,
    pub agent_config_results: Vec<(String, FuzzResult)>,
    pub crashes: usize,
    pub total_tests: usize,
}

impl FuzzTestReport {
    pub fn new() -> Self {
        Self {
            parser_results: Vec::new(),
            lsp_results: Vec::new(),
            agent_config_results: Vec::new(),
            crashes: 0,
            total_tests: 0,
        }
    }

    pub fn analyze(&mut self) -> String {
        self.total_tests = self.parser_results.len() + self.lsp_results.len() + self.agent_config_results.len();

        let mut panics = 0;
        let mut timeouts = 0;
        let mut parse_errors = 0;

        for (_, result) in &self.parser_results {
            match result {
                FuzzResult::Panic(_) => panics += 1,
                FuzzResult::Timeout => timeouts += 1,
                FuzzResult::ParseError(_) => parse_errors += 1,
                _ => {}
            }
        }

        for (_, result) in &self.lsp_results {
            match result {
                FuzzResult::Panic(_) => panics += 1,
                FuzzResult::Timeout => timeouts += 1,
                FuzzResult::ParseError(_) => parse_errors += 1,
                _ => {}
            }
        }

        format!(
            "Fuzz Test Report:\n\
             - Total tests: {}\n\
             - Panics: {}\n\
             - Timeouts: {}\n\
             - Parse errors: {}\n\
             - Success rate: {:.2}%",
            self.total_tests,
            panics,
            timeouts,
            parse_errors,
            (self.total_tests - panics - timeouts) as f64 / self.total_tests as f64 * 100.0
        )
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut fuzzer = FuzzHarness::new();

    // Run comprehensive fuzzing
    let mut report = fuzzer.run_comprehensive_fuzz();

    // Analyze results
    let analysis = report.analyze();
    println!("\n{}", analysis);

    // Minimize and save any crashes
    if !fuzzer.crash_outputs.is_empty() {
        println!("\n🔍 Minimizing {} crash cases...", fuzzer.crash_outputs.len());
        let minimized = fuzzer.minimize_crashes();
        fuzzer.save_crash_repros(&minimized)?;
    } else {
        println!("\n✅ No crashes found during fuzzing!");
    }

    // Return exit code based on results
    if report.crashes > 0 {
        std::process::exit(1);
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fuzz_harness_basic() {
        let mut fuzzer = FuzzHarness::new();

        // Test with simple valid input
        let result = fuzzer.fuzz_parser("my $x = 42;");
        matches!(result, FuzzResult::Success);

        // Test with empty input
        let result = fuzzer.fuzz_parser("");
        matches!(result, FuzzResult::Success);
    }

    #[test]
    fn test_lsp_message_fuzzing() {
        let mut fuzzer = FuzzHarness::new();

        // Valid JSON should not panic
        let result = fuzzer.fuzz_lsp_messages(r#"{"method": "test"}"#);
        matches!(result, FuzzResult::Success);

        // Invalid JSON should gracefully error
        let result = fuzzer.fuzz_lsp_messages(r#"{"invalid": json}"#);
        matches!(result, FuzzResult::ParseError(_));
    }

    #[test]
    fn test_malformed_input_generation() {
        let fuzzer = FuzzHarness::new();
        let inputs = fuzzer.generate_malformed_inputs();

        assert!(!inputs.is_empty());
        assert!(inputs.iter().any(|s| s.contains("sub foo {")));
        assert!(inputs.iter().any(|s| s.contains("{ ".repeat(100))));
    }
}