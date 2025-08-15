//! Pure Rust parser task

use color_eyre::eyre::{Context, Result};
use std::fs;
use std::path::PathBuf;
use std::process::Command;

pub fn run(source: PathBuf, sexp: bool, ast: bool, bench: bool) -> Result<()> {
    // Read the source file to verify it exists
    let _content = fs::read_to_string(&source)
        .with_context(|| format!("Failed to read file: {}", source.display()))?;

    // Build the command to run the compare_parsers binary with pure-rust feature
    let mut cmd = Command::new("cargo");
    cmd.args(&["run", "--features", "pure-rust test-utils", "--bin", "compare_parsers", "--"]);

    // Add the source file
    cmd.arg(&source);

    // If benchmarking, run multiple iterations
    if bench {
        cmd.arg("1000");
    } else {
        cmd.arg("1");
    }

    // Run the command
    let output = cmd.output().context("Failed to run compare_parsers binary")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(color_eyre::eyre::eyre!("Parser failed: {}", stderr));
    }

    // Print the output
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Filter output based on flags
    if sexp || ast {
        for line in stdout.lines() {
            if sexp && line.contains("S-expr") {
                println!("{}", line);
            }
            if ast && line.contains("AST") {
                println!("{}", line);
            }
        }
    } else {
        // Print all output
        print!("{}", stdout);
    }

    Ok(())
}
