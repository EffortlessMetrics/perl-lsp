//! Rust toolchain check implementation.

use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use std::cmp::Ordering;
use std::process::Command;

use crate::utils::project_root;

#[derive(Deserialize)]
struct RustToolchainFile {
    toolchain: RustToolchain,
}

#[derive(Deserialize)]
struct RustToolchain {
    channel: String,
}

pub fn run(doctor: bool) -> Result<()> {
    let root = project_root()?;
    let toolchain_file = root.join("rust-toolchain.toml");

    if !toolchain_file.exists() {
        println!("⚠️  rust-toolchain.toml not found; skipping pinned toolchain check");
        return Ok(());
    }

    let raw = std::fs::read_to_string(&toolchain_file)
        .with_context(|| format!("Failed to read {}", toolchain_file.display()))?;
    let toolchain: RustToolchainFile =
        toml::from_str(&raw).context("Failed to parse rust-toolchain.toml")?;
    let required = toolchain
        .toolchain
        .channel
        .trim()
        .trim_matches('\"')
        .trim_matches('\'')
        .to_string();
    let required_parts = parse_version_parts(&required);

    if required_parts.is_empty() {
        println!("⚠️  Could not parse pinned toolchain from rust-toolchain.toml");
        return Ok(());
    }

    let rustc_output = Command::new("rustc")
        .arg("--version")
        .output()
        .context("Failed to run rustc --version")?;
    let rustc_text = String::from_utf8(rustc_output.stdout)
        .context("rustc --version output is not valid UTF-8")?;
    let current = rustc_text
        .split_whitespace()
        .nth(1)
        .ok_or_else(|| color_eyre::eyre::eyre!("Unexpected rustc --version output"))?
        .to_string();
    let current_parts = parse_version_parts(&current);

    if current_parts.is_empty() {
        bail!("Could not parse rustc version from {:?}", rustc_text);
    }

    match compare_versions(&current_parts, &required_parts) {
        Ordering::Less => {
            bail!(
                "Rust {current} is older than pinned MSRV {required}; install {} and set override",
                required
            );
        }
        Ordering::Equal => {
            println!("✅ Rust toolchain matches pinned version: {current}");
        }
        Ordering::Greater => {
            if doctor {
                println!(
                    "⚠️  Using Rust {current} while rust-toolchain.toml pins {required}; use 'rustup override set {required}' for exact parity"
                );
            } else {
                println!("✅ Rust {current} satisfies pinned MSRV {required}");
            }
        }
    }

    Ok(())
}

fn parse_version_parts(version: &str) -> Vec<u32> {
    version
        .split(['.', '-', '+'])
        .filter_map(|part| {
            part.chars()
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u32>()
                .ok()
        })
        .collect()
}

fn compare_versions(actual: &[u32], required: &[u32]) -> Ordering {
    let max_len = std::cmp::max(actual.len(), required.len());
    for index in 0..max_len {
        let a = actual.get(index).copied().unwrap_or(0);
        let b = required.get(index).copied().unwrap_or(0);
        match a.cmp(&b) {
            Ordering::Equal => {}
            other => return other,
        }
    }
    Ordering::Equal
}
