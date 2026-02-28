//! Publishing functionality for crates and VSCode extension

use color_eyre::eyre::{Result, bail, eyre};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::process::Command;
use std::thread;
use std::time::Duration;

pub fn publish_crates(yes: bool, dry_run: bool) -> Result<()> {
    println!("📦 Publishing crates to crates.io");

    let publish_targets = load_publish_targets()?;

    if !yes {
        println!("This will publish:");
        for target in &publish_targets {
            println!("  - {}", target.name);
        }
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

    let mut args = vec!["publish", "--no-verify"];
    if dry_run {
        args.push("--dry-run");
    }

    for (index, target) in publish_targets.iter().enumerate() {
        println!("Publishing {}...", target.name);
        let crate_dir = target.manifest_path.parent().ok_or_else(|| {
            eyre!(
                "Invalid manifest path for publish target '{}': {:?}",
                target.name,
                target.manifest_path
            )
        })?;

        let output = Command::new("cargo").current_dir(crate_dir).args(&args).output()?;
        if !output.status.success() {
            bail!("Failed to publish {}: {}", target.name, String::from_utf8_lossy(&output.stderr));
        }
        println!("✅ {} published", target.name);

        if !dry_run && index + 1 != publish_targets.len() {
            // Wait for crates.io to process before publishing the next package.
            println!("Waiting 30 seconds for crates.io to process...");
            thread::sleep(Duration::from_secs(30));
        }
    }
    println!();
    println!("✅ All crates published successfully!");

    Ok(())
}

#[derive(Deserialize)]
struct CargoMetadata {
    metadata: Option<WorkspaceMetadata>,
    packages: Vec<MetadataPackage>,
}

#[derive(Deserialize)]
struct WorkspaceMetadata {
    publish: Option<PublishMetadata>,
}

#[derive(Deserialize)]
struct PublishMetadata {
    allow: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
    manifest_path: PathBuf,
}

struct PublishTarget {
    name: String,
    manifest_path: PathBuf,
}

fn load_publish_targets() -> Result<Vec<PublishTarget>> {
    let output =
        Command::new("cargo").args(["metadata", "--format-version", "1", "--no-deps"]).output()?;
    if !output.status.success() {
        bail!(
            "Failed to load workspace metadata for publish allowlist: {}",
            String::from_utf8_lossy(&output.stderr)
        );
    }

    let metadata: CargoMetadata = serde_json::from_slice(&output.stdout)?;

    let allowlist = metadata
        .metadata
        .and_then(|workspace| workspace.publish)
        .and_then(|publish| publish.allow)
        .ok_or_else(|| {
            eyre!(
                "Publish allowlist missing. Add [workspace.metadata.publish.allow] in the workspace Cargo.toml."
            )
        })?;

    if allowlist.is_empty() {
        bail!("Publish allowlist is empty. Add crates to [workspace.metadata.publish.allow].");
    }

    let mut package_map = HashMap::new();
    for package in metadata.packages {
        package_map.insert(package.name, package.manifest_path);
    }

    let mut seen = HashSet::new();
    let mut targets = Vec::new();

    for crate_name in allowlist {
        if !seen.insert(crate_name.clone()) {
            continue;
        }

        let manifest_path = package_map.get(&crate_name).ok_or_else(|| {
            eyre!(
                "Crate '{}' listed in [workspace.metadata.publish.allow] is not a workspace member.",
                crate_name
            )
        })?;

        targets.push(PublishTarget { name: crate_name, manifest_path: manifest_path.clone() });
    }

    Ok(targets)
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

pub fn compute_publish_order() -> Result<()> {
    let output =
        Command::new("cargo").args(["metadata", "--format-version=1", "--no-deps"]).output()?;

    if !output.status.success() {
        bail!("Failed to run cargo metadata: {}", String::from_utf8_lossy(&output.stderr));
    }

    let meta: serde_json::Value = serde_json::from_slice(&output.stdout)?;

    let workspace_members: HashSet<&str> =
        meta["workspace_members"].as_array().unwrap().iter().map(|v| v.as_str().unwrap()).collect();

    let mut packages = HashMap::new();
    if let Some(pkgs) = meta["packages"].as_array() {
        for pkg in pkgs {
            if let (Some(id), Some(name), Some(version)) =
                (pkg["id"].as_str(), pkg["name"].as_str(), pkg["version"].as_str())
                && workspace_members.contains(id) {
                    packages.insert(name.to_string(), (pkg, version.to_string()));
                }
        }
    }

    let mut deps: HashMap<String, HashSet<String>> = HashMap::new();
    for (name, (pkg, _)) in &packages {
        let mut package_deps = HashSet::new();
        if let Some(dependencies) = pkg["dependencies"].as_array() {
            for dep in dependencies {
                if let Some(dep_name) = dep["name"].as_str() {
                    let kind = dep.get("kind").and_then(|k| k.as_str());
                    if packages.contains_key(dep_name) && kind != Some("dev") {
                        package_deps.insert(dep_name.to_string());
                    }
                }
            }
        }
        deps.insert(name.clone(), package_deps);
    }

    let mut in_degree: HashMap<String, usize> =
        deps.keys().map(|k| (k.clone(), deps[k].len())).collect();
    let mut queue: Vec<String> =
        in_degree.iter().filter(|&(_, &deg)| deg == 0).map(|(name, _)| name.clone()).collect();
    queue.sort_by(|a, b| b.cmp(a));

    let mut order = Vec::new();
    while let Some(node) = queue.pop() {
        order.push(node.clone());
        for (name, dep_set) in deps.iter() {
            if dep_set.contains(&node)
                && let Some(deg) = in_degree.get_mut(name) {
                    *deg -= 1;
                    if *deg == 0 {
                        queue.push(name.clone());
                        queue.sort_by(|a, b| b.cmp(a)); // Reverse sort because we pop from end
                    }
                }
        }
    }

    if order.len() != packages.len() {
        eprintln!("ERROR: cycle detected in dependency graph");
        std::process::exit(1);
    }

    let allowlist_val = meta
        .pointer("/workspace_metadata/publish/allow")
        .or_else(|| meta.pointer("/metadata/publish/allow"));

    let mut allowed = Vec::new();
    if let Some(allowlist) = allowlist_val.and_then(|v| v.as_array()) {
        for val in allowlist {
            if let Some(crate_name) = val.as_str() {
                if !allowed.contains(&crate_name.to_string()) {
                    if !packages.contains_key(crate_name) {
                        eprintln!(
                            "ERROR: Crate in publish allowlist is not a workspace member: {}",
                            crate_name
                        );
                        std::process::exit(1);
                    }
                    allowed.push(crate_name.to_string());
                }
            } else {
                eprintln!("ERROR: Invalid publish allowlist entry (not a string): {:?}", val);
                std::process::exit(1);
            }
        }
    } else {
        eprintln!(
            "ERROR: Workspace publish allowlist must be a list at [workspace.metadata.publish.allow]."
        );
        std::process::exit(1);
    }

    if allowed.is_empty() {
        eprintln!(
            "ERROR: Publish allowlist is empty. Set [workspace.metadata.publish.allow] in workspace Cargo.toml."
        );
        std::process::exit(1);
    }

    let mut result = Vec::new();
    for name in &order {
        if allowed.contains(name) {
            let (_, version) = packages.get(name).unwrap();
            let mut item = serde_json::Map::new();
            item.insert("name".to_string(), serde_json::Value::String(name.clone()));
            item.insert("version".to_string(), serde_json::Value::String(version.clone()));
            result.push(serde_json::Value::Object(item));
        }
    }

    println!("{}", serde_json::to_string(&result)?);
    Ok(())
}
