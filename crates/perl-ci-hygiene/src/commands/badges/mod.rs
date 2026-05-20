mod facts;
mod readme;

use color_eyre::eyre::{Context, Result};
use std::path::Path;

use crate::{GREEN, NC, RED};

pub(crate) fn generate(repo_root: &Path, check_mode: bool) -> Result<i32> {
    let installs = facts::read_vscode_marketplace_installs(repo_root)?;
    let badge_url = format!(
        "https://img.shields.io/badge/VS%20Marketplace-{}%20installs-0078D4",
        installs.value
    );

    let root_readme = repo_root.join("README.md");
    let ext_readme = repo_root.join("vscode-extension/README.md");

    if check_mode {
        return check_badges([root_readme.as_path(), ext_readme.as_path()], &installs, &badge_url);
    }

    update_badges([root_readme.as_path(), ext_readme.as_path()], installs.value, &badge_url)
}

fn check_badges<const N: usize>(
    readme_paths: [&Path; N],
    installs: &facts::VscodeMarketplaceInstalls,
    badge_url: &str,
) -> Result<i32> {
    let mut has_drift = false;
    for readme_path in readme_paths {
        if !readme_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(readme_path)?;
        if content.contains(badge_url) {
            continue;
        }

        eprintln!("{}VS Marketplace badge drift in {}{}", RED, readme_path.display(), NC);
        eprintln!("  expected installs: {} from {}", installs.value, installs.facts_file.display());

        if let Some(found) = readme::stale_installs_value(&content) {
            eprintln!(
                "  stale badge found: {} but expected {} in {}",
                found,
                installs.value,
                readme_path.display()
            );
        }
        has_drift = true;
    }

    if has_drift {
        eprintln!("Run: cargo xtask ci-hygiene generate-badges");
        return Ok(1);
    }

    println!("{}✓ VS Marketplace badge check passed{}", GREEN, NC);
    Ok(0)
}

fn update_badges<const N: usize>(
    readme_paths: [&Path; N],
    vscode_installs: u32,
    badge_url: &str,
) -> Result<i32> {
    for readme_path in readme_paths {
        if !readme_path.exists() {
            continue;
        }

        let content = std::fs::read_to_string(readme_path)?;
        let updated = readme::update_badge_in_content(&content, badge_url)?;

        if updated != content {
            std::fs::write(readme_path, &updated)
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
