//! Publishing functionality for crates and VSCode extension

use color_eyre::eyre::{Result, bail};
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn publish_crates(yes: bool, dry_run: bool) -> Result<()> {
    println!("📦 Publishing crates to crates.io");

    if !yes {
        println!("This will publish:");
        println!("  - perl-lexer");
        println!("  - perl-parser");
        println!();
        print!("Continue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Publishing cancelled.");
            return Ok(());
        }
    }

    // Publish perl-lexer first
    println!("Publishing perl-lexer...");
    let mut args = vec!["publish", "--no-verify"];
    if dry_run {
        args.push("--dry-run");
    }

    let output = Command::new("cargo").current_dir("crates/perl-lexer").args(&args).output()?;

    if !output.status.success() {
        bail!("Failed to publish perl-lexer: {}", String::from_utf8_lossy(&output.stderr));
    }
    println!("✅ perl-lexer published");

    if !dry_run {
        // Wait for crates.io to process
        println!("Waiting 30 seconds for crates.io to process...");
        thread::sleep(Duration::from_secs(30));
    }

    // Publish perl-parser
    println!("Publishing perl-parser...");
    let output = Command::new("cargo").current_dir("crates/perl-parser").args(&args).output()?;

    if !output.status.success() {
        bail!("Failed to publish perl-parser: {}", String::from_utf8_lossy(&output.stderr));
    }
    println!("✅ perl-parser published");

    println!();
    println!("✅ All crates published successfully!");

    Ok(())
}

pub fn publish_vscode(yes: bool, token: Option<String>) -> Result<()> {
    println!("🚀 Publishing VSCode extension to marketplace");

    // Check for token - try argument first, then environment variable
    let token = token.or_else(|| std::env::var("VSCE_PAT").ok());
    if token.is_none() {
        bail!("VSCE_PAT token required. Set via --token or VSCE_PAT environment variable.");
    }

    if !yes {
        println!("This will publish the VSCode extension to the marketplace.");
        println!();
        print!("Continue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Publishing cancelled.");
            return Ok(());
        }
    }

    // First compile the extension
    println!("Compiling extension...");
    let output =
        Command::new("npm").current_dir("vscode-extension").args(["run", "compile"]).output()?;

    if !output.status.success() {
        bail!("Failed to compile extension: {}", String::from_utf8_lossy(&output.stderr));
    }

    // Publish to marketplace
    println!("Publishing to marketplace...");
    let token = token.ok_or_else(|| {
        color_eyre::eyre::eyre!("VSCE_PAT environment variable is required for publishing")
    })?;
    let output = Command::new("npx")
        .current_dir("vscode-extension")
        .env("VSCE_PAT", token)
        .args(["vsce", "publish"])
        .output()?;

    if !output.status.success() {
        bail!("Failed to publish extension: {}", String::from_utf8_lossy(&output.stderr));
    }

    println!("✅ VSCode extension published successfully!");
    println!();
    println!(
        "View in marketplace: https://marketplace.visualstudio.com/items?itemName=perl.language-server"
    );

    Ok(())
}
