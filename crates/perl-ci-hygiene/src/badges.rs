use color_eyre::eyre::{Context, Result};
use regex::Regex;
use std::fs;
use std::path::Path;
use std::sync::LazyLock;
use toml::Value as TomlValue;

const RED: &str = "\x1b[0;31m";
const GREEN: &str = "\x1b[0;32m";
const NC: &str = "\x1b[0m";

static VS_MARKETPLACE_INSTALLS_BADGE_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"VS%20Marketplace-(\d+)%20installs").unwrap_or_else(|error| {
        unreachable!("VS_MARKETPLACE_INSTALLS_BADGE_RE is a known-good static pattern: {error}")
    })
});

static VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?s)<!-- perl-lsp:vs-marketplace-installs-badge:start -->.*?<!-- perl-lsp:vs-marketplace-installs-badge:end -->",
    )
    .unwrap_or_else(|error| {
        unreachable!("VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE is a known-good static pattern: {error}")
    })
});

pub(crate) fn cmd_generate_badges(repo_root: &Path, check_mode: bool) -> Result<i32> {
    let facts_file = repo_root.join("docs/project/publication-facts.toml");
    let facts_content = fs::read_to_string(&facts_file)
        .with_context(|| format!("reading facts file {:?}", facts_file))?;
    let facts: TomlValue = toml::from_str(&facts_content)
        .with_context(|| format!("parsing facts file {:?}", facts_file))?;

    let vscode_installs_i64 = facts
        .get("external")
        .and_then(|e| e.get("vscode_marketplace_installs"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_integer())
        .ok_or_else(|| {
            color_eyre::eyre::eyre!(
                "Could not read vscode_marketplace_installs.value from {}",
                facts_file.display()
            )
        })?;

    let vscode_installs = u32::try_from(vscode_installs_i64).with_context(|| {
        format!("vscode_marketplace_installs value {} is out of range for u32", vscode_installs_i64)
    })?;

    let badge_url = format!(
        "https://img.shields.io/badge/VS%20Marketplace-{}%20installs-0078D4",
        vscode_installs
    );

    let root_readme = repo_root.join("README.md");
    let ext_readme = repo_root.join("vscode-extension/README.md");

    if check_mode {
        let mut has_drift = false;
        for readme_path in [&root_readme, &ext_readme] {
            if readme_path.exists() {
                let content = fs::read_to_string(readme_path)?;
                if !content.contains(&badge_url) {
                    eprintln!(
                        "{}VS Marketplace badge drift in {}{}",
                        RED,
                        readme_path.display(),
                        NC
                    );
                    eprintln!(
                        "  expected installs: {} from {}",
                        vscode_installs,
                        facts_file.display()
                    );

                    if let Some(caps) = VS_MARKETPLACE_INSTALLS_BADGE_RE.captures(&content)
                        && let Some(found) = caps.get(1)
                    {
                        eprintln!(
                            "  stale badge found: {} but expected {} in {}",
                            found.as_str(),
                            vscode_installs,
                            readme_path.display()
                        );
                    }
                    has_drift = true;
                }
            }
        }
        if has_drift {
            eprintln!("Run: cargo xtask ci-hygiene generate-badges");
            return Ok(1);
        }
        println!("{}✓ VS Marketplace badge check passed{}", GREEN, NC);
        return Ok(0);
    }

    // Generate mode: update badges
    for readme_path in [&root_readme, &ext_readme] {
        if !readme_path.exists() {
            continue;
        }

        let content = fs::read_to_string(readme_path)?;
        let updated = update_badge_in_content(&content, &badge_url)?;

        if updated != content {
            fs::write(readme_path, &updated)
                .with_context(|| format!("writing updated badge to {:?}", readme_path))?;
            println!("{}✓ Updated VS Marketplace badge in {}{}", GREEN, readme_path.display(), NC);
        }
    }

    println!(
        "{}✓ Badges updated from value {} in publication-facts.toml{}",
        GREEN, vscode_installs, NC
    );
    Ok(0)
}

fn update_badge_in_content(content: &str, badge_url: &str) -> Result<String> {
    if !VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE.is_match(content) {
        return Ok(content.to_string());
    }

    // Determine if we're dealing with HTML or Markdown format
    if content.contains("href=\"https://marketplace.visualstudio.com") {
        // HTML format
        let replacement = format!(
            "<!-- perl-lsp:vs-marketplace-installs-badge:start -->\n  <a href=\"https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs\"><img src=\"{}\" alt=\"VS Marketplace installs\" /></a>\n  <!-- perl-lsp:vs-marketplace-installs-badge:end -->",
            badge_url
        );
        return Ok(VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE
            .replace_all(content, replacement)
            .into_owned());
    }

    // Markdown format
    let replacement = format!(
        "<!-- perl-lsp:vs-marketplace-installs-badge:start -->\n[![VS Marketplace Installs (manual)]({})](https://marketplace.visualstudio.com/items?itemName=EffortlessMetrics.perl-lsp-rs)\n<!-- perl-lsp:vs-marketplace-installs-badge:end -->",
        badge_url
    );
    Ok(VS_MARKETPLACE_INSTALLS_BADGE_BLOCK_RE.replace_all(content, replacement).into_owned())
}
