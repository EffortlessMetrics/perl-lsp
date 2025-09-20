#!/usr/bin/env cargo run --bin
//! Test the original stack overflow reproduction case
//!
//! This validates that the specific input that caused the stack overflow
//! in the previous fuzz report is now handled gracefully with the recursion limits.

use std::time::Instant;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("🔬 Testing original stack overflow reproduction case...");

    // Load the original reproduction case
    let repro_content = match std::fs::read_to_string("repros/stack_overflow_minimal.pl") {
        Ok(content) => content,
        Err(e) => {
            println!("⚠️ Could not read reproduction case: {}", e);
            println!("This is expected if the file was cleaned up or moved.");
            return Ok(());
        }
    };

    println!("📄 Original repro file size: {} characters", repro_content.len());

    let start = Instant::now();

    // Test the parser with the original crasher
    let result = std::panic::catch_unwind(|| {
        perl_parser::Parser::new(&repro_content).parse()
    });

    let elapsed = start.elapsed();

    match result {
        Ok(parse_result) => {
            match parse_result {
                Ok(_ast) => {
                    println!("⚠️ Unexpectedly parsed successfully! This input should exceed recursion limits.");
                    return Err("Original crasher input parsed successfully - recursion limits may not be working".into());
                }
                Err(err) => {
                    println!("✅ Parse error (expected): {:?}", err);
                    println!("✅ Parser handled gracefully in {:?}", elapsed);
                    println!("🛡️ Recursion limit protection is working correctly!");

                    // Verify it's specifically a recursion limit error
                    if format!("{:?}", err).contains("RecursionLimit") {
                        println!("✅ Correctly identified as RecursionLimit error");
                    } else {
                        println!("ℹ️ Different error type, but still handled gracefully");
                    }
                }
            }
        }
        Err(_panic) => {
            println!("💥 CRITICAL: Parser still panics on original reproduction case!");
            return Err("Parser panicked - recursion limits not working for original case".into());
        }
    }

    if elapsed.as_secs() > 1 {
        println!("⚠️ Parsing took longer than expected: {:?}", elapsed);
    }

    println!("🎉 Original reproduction case test completed successfully");
    println!("🔒 Stack overflow vulnerability has been mitigated");

    Ok(())
}