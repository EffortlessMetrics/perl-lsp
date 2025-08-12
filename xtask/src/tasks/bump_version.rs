//! Version bumping functionality

use color_eyre::eyre::{Result, bail};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::process::Command;

pub fn run(version: String, yes: bool) -> Result<()> {
    println!("🔄 Bumping version to {}", version);

    // Validate version format
    let version_regex = Regex::new(r"^\d+\.\d+\.\d+$")?;
    if !version_regex.is_match(&version) {
        bail!("Invalid version format. Expected: X.Y.Z");
    }

    // Files to update
    let cargo_files = vec![
        "crates/perl-lexer/Cargo.toml",
        "crates/perl-parser/Cargo.toml",
    ];

    let package_json = "vscode-extension/package.json";

    // Show what will be updated
    if !yes {
        println!("This will update version to {} in:", version);
        for file in &cargo_files {
            println!("  - {}", file);
        }
        println!("  - {}", package_json);
        println!();
        print!("Continue? [y/N] ");

        use std::io::{self, Write};
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Version bump cancelled.");
            return Ok(());
        }
    }

    // Update Cargo.toml files
    for file in cargo_files {
        update_cargo_version(&file, &version)?;
        println!("  ✅ Updated {}", file);
    }

    // Update package.json
    update_package_json_version(&package_json, &version)?;
    println!("  ✅ Updated {}", package_json);

    // Update version strings in source files
    update_source_versions(&version)?;
    println!("  ✅ Updated version strings in source files");

    // Update README
    update_readme_version(&version)?;
    println!("  ✅ Updated README.md");

    // Show git status
    println!();
    println!("📋 Changed files:");
    let output = Command::new("git").args(&["status", "--short"]).output()?;
    print!("{}", String::from_utf8_lossy(&output.stdout));

    println!();
    println!("✅ Version bumped to {}", version);
    println!();
    println!("💡 Next steps:");
    println!("1. Review the changes: git diff");
    println!(
        "2. Commit: git commit -am 'chore: bump version to {}'",
        version
    );
    println!("3. Run release: cargo xtask release {}", version);

    Ok(())
}

fn update_cargo_version(path: &str, version: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let version_regex = Regex::new(r#"^version = "[^"]+""#)?;

    let mut updated = false;
    let new_content = content
        .lines()
        .map(|line| {
            if version_regex.is_match(line) && !updated {
                updated = true;
                format!(r#"version = "{}""#, version)
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n");

    if !updated {
        bail!("Failed to find version in {}", path);
    }

    fs::write(path, new_content)?;
    Ok(())
}

fn update_package_json_version(path: &str, version: &str) -> Result<()> {
    let content = fs::read_to_string(path)?;
    let json: serde_json::Value = serde_json::from_str(&content)?;

    let mut json = json;
    json["version"] = serde_json::Value::String(version.to_string());

    let new_content = serde_json::to_string_pretty(&json)? + "\n";
    fs::write(path, new_content)?;

    Ok(())
}

fn update_source_versions(version: &str) -> Result<()> {
    // Update version strings in Rust source files
    let patterns = vec![
        (
            r"Perl Language Server v\d+\.\d+\.\d+",
            format!("Perl Language Server v{}", version),
        ),
        (
            r"Perl Debug Adapter v\d+\.\d+\.\d+",
            format!("Perl Debug Adapter v{}", version),
        ),
    ];

    let source_dirs = vec!["crates/perl-parser/src"];

    for dir in source_dirs {
        update_directory_versions(dir, &patterns)?;
    }

    Ok(())
}

fn update_directory_versions(dir: &str, patterns: &[(impl AsRef<str>, String)]) -> Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();

        if path.is_file() && path.extension().and_then(|s| s.to_str()) == Some("rs") {
            let content = fs::read_to_string(&path)?;
            let mut new_content = content.clone();

            for (pattern, replacement) in patterns {
                let regex = Regex::new(pattern.as_ref())?;
                new_content = regex
                    .replace_all(&new_content, replacement.as_str())
                    .to_string();
            }

            if new_content != content {
                fs::write(&path, new_content)?;
            }
        } else if path.is_dir() {
            // Recurse into subdirectories
            update_directory_versions(path.to_str().unwrap(), patterns)?;
        }
    }

    Ok(())
}

fn update_readme_version(version: &str) -> Result<()> {
    let path = "README.md";
    if !Path::new(path).exists() {
        return Ok(());
    }

    let content = fs::read_to_string(path)?;

    // Update perl-parser dependency version (X.Y format)
    let major_minor = version.split('.').take(2).collect::<Vec<_>>().join(".");
    let regex = Regex::new(r#"perl-parser = "\d+\.\d+""#)?;
    let new_content = regex
        .replace_all(&content, format!(r#"perl-parser = "{}""#, major_minor))
        .to_string();

    // Update VSIX filename references
    let regex = Regex::new(r"perl-language-server-\d+\.\d+\.\d+\.vsix")?;
    let new_content = regex
        .replace_all(
            &new_content,
            format!("perl-language-server-{}.vsix", version),
        )
        .to_string();

    fs::write(path, new_content)?;
    Ok(())
}
