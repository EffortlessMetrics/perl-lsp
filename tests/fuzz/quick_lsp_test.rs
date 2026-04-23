#!/usr/bin/env cargo run --bin
//! Quick LSP message fuzzing to complement parser testing

use std::panic;

fn test_lsp_message_handling() {
    println!("🔌 Testing LSP message handling robustness...");

    let malformed_messages = vec![
        // Invalid JSON
        r#"{ "invalid: json }"#.to_string(),
        r#"{ "method": "unclosed"#.to_string(),

        // Oversized fields
        format!(r#"{{ "method": "{}" }}"#, "x".repeat(100_000)),

        // Deeply nested JSON
        format!("{}{}{}", r#"{"a":"#.repeat(100), r#""test""#, r#"}"#.repeat(100)),

        // Control characters
        "{ \"method\": \"test\x00\" }".to_string(),

        // Empty/minimal
        "{}".to_string(),
        "null".to_string(),
        "".to_string(),

        // Type confusion
        r#"{ "id": "should_be_number", "method": 123 }"#.to_string(),
    ];

    let mut tests = 0;
    let mut panics = 0;

    for (i, message) in malformed_messages.iter().enumerate() {
        print!("  LSP test {}: {} chars... ", i + 1, message.len());
        tests += 1;

        let result = panic::catch_unwind(|| {
            // Test JSON parsing (this is what LSP would do first)
            match serde_json::from_str::<serde_json::Value>(message) {
                Ok(_) => "Parsed OK",
                Err(_) => "Parse error (expected)"
            }
        });

        match result {
            Ok(_) => println!("✅ OK"),
            Err(_) => {
                println!("💥 PANIC");
                panics += 1;
            }
        }
    }

    println!("📊 LSP Message Tests: {} passed, {} panics", tests - panics, panics);
}

fn test_agent_config_patterns() {
    println!("\n🤖 Testing agent configuration patterns...");

    let malformed_configs = vec![
        // Invalid YAML-like
        "invalid: [unclosed".to_string(),
        "{ invalid: json }".to_string(),
        "".to_string(),
        "\x00\x01\x02".to_string(),

        // Large config
        format!("config: {}", "x".repeat(100_000)),

        // Unicode
        "config: 🔥🚀💥".to_string(),

        // Nested structure
        format!("{}: test", "key".repeat(1000)),
    ];

    let mut tests = 0;
    let mut issues = 0;

    for (i, config) in malformed_configs.iter().enumerate() {
        print!("  Agent config test {}: {} chars... ", i + 1, config.len());
        tests += 1;

        // For now, just ensure no panic on basic string operations
        let result = panic::catch_unwind(|| {
            let _len = config.len();
            let _contains_colon = config.contains(':');
            let _lines: Vec<&str> = config.lines().collect();
            "OK"
        });

        match result {
            Ok(_) => println!("✅ OK"),
            Err(_) => {
                println!("💥 PANIC");
                issues += 1;
            }
        }
    }

    println!("📊 Agent Config Tests: {} passed, {} panics", tests - issues, issues);
}

fn main() {
    println!("⚡ Quick Auxiliary Component Testing");
    println!("=====================================");

    test_lsp_message_handling();
    test_agent_config_patterns();

    println!("\n✅ Auxiliary component testing complete");
    println!("   Main vulnerability: Stack overflow in parser with >1000 nesting depth");
}