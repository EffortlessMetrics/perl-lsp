//! crates.io launch-preparation helper.

use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use serde_json::Value as JsonValue;
use std::collections::HashSet;
use std::fs;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::process::Command;
use tempfile::TempDir;

use crate::utils::project_root;

const CORE_LAUNCH_CRATES: &[&str] = &[
    "perl-parser",
    "perl-lexer",
    "perl-lsp-rs",
    "perllsp",
    "perl-dap",
    "perl-corpus",
];

#[derive(Deserialize)]
struct RootCargoManifest {
    workspace: RootWorkspace,
}

#[derive(Deserialize)]
struct RootWorkspace {
    metadata: Option<RootWorkspaceMetadata>,
}

#[derive(Deserialize)]
struct RootWorkspaceMetadata {
    publish: Option<RootPublishMetadata>,
}

#[derive(Deserialize)]
struct RootPublishMetadata {
    allow: Option<Vec<String>>,
}

#[derive(Deserialize)]
struct CargoMetadata {
    packages: Vec<MetadataPackage>,
    target_directory: PathBuf,
}

#[derive(Deserialize)]
struct MetadataPackage {
    name: String,
    version: String,
    manifest_path: String,
    publish: Option<JsonValue>,
}

pub fn run(all: bool) -> Result<()> {
    let root = project_root()?;
    let launch_crates = if all {
        load_publish_allowlist(&root)?
    } else {
        CORE_LAUNCH_CRATES
            .iter()
            .map(|name| (*name).to_string())
            .collect()
    };

    let metadata = load_cargo_metadata(&root)?;
    let package_names: HashSet<_> = metadata
        .packages
        .iter()
        .map(|package| package.name.as_str())
        .collect();

    let unknown = launch_crates
        .iter()
        .filter(|name| !package_names.contains(name.as_str()))
        .collect::<Vec<_>>();

    if !unknown.is_empty() {
        let unknown_list = unknown
            .iter()
            .map(|name| name.as_str())
            .collect::<Vec<_>>()
            .join(", ");
        bail!("unknown crates for launch prep: {unknown_list}");
    }

    println!(
        "🚀 crates.io launch prep ({})",
        if all {
            "all publish-allowlist crates"
        } else {
            "core launch crates"
        }
    );
    println!(
        "📦 Running cargo check + cargo package + offline packaged-manifest check for {} crate(s)",
        launch_crates.len()
    );

    for crate_name in launch_crates {
        println!();
        println!("==> {crate_name}");
        run_cargo_check(&root, &crate_name)?;
        let patch_args = package_patch_args(&metadata, Some(&crate_name));
        let package = metadata
            .packages
            .iter()
            .find(|package| package.name == crate_name)
            .ok_or_else(|| color_eyre::eyre::eyre!("missing cargo metadata for {crate_name}"))?;
        run_cargo_package_check(&root, &metadata.target_directory, package, &patch_args)?;
    }

    println!();
    println!(
        "✅ crates.io launch prep completed ({})",
        if all { "all" } else { "core" }
    );

    Ok(())
}

fn package_patch_args(metadata: &CargoMetadata, skip_name: Option<&str>) -> Vec<String> {
    let mut args = Vec::new();

    for package in &metadata.packages {
        if !is_publish_candidate(&package.publish) {
            continue;
        }
        if skip_name == Some(package.name.as_str()) {
            continue;
        }

        let manifest_path = Path::new(&package.manifest_path);
        let crate_dir = match manifest_path.parent() {
            Some(dir) => dir,
            None => continue,
        };

        args.push(format!(
            "--config=patch.crates-io.{}.path=\"{}\"",
            package.name,
            toml_safe_path(crate_dir)
        ));
    }

    args
}

fn toml_safe_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn is_publish_candidate(publish: &Option<JsonValue>) -> bool {
    match publish {
        Some(value) => match value.as_array() {
            Some(entries) => !entries.is_empty(),
            None => true,
        },
        None => true,
    }
}

fn load_cargo_metadata(root: &Path) -> Result<CargoMetadata> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["metadata", "--format-version", "1", "--no-deps"])
        .output()
        .context("failed to run cargo metadata")?;

    if !output.status.success() {
        bail!(
            "cargo metadata failed: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    let metadata: CargoMetadata =
        serde_json::from_slice(&output.stdout).context("failed to parse cargo metadata output")?;
    Ok(metadata)
}

fn load_publish_allowlist(root: &Path) -> Result<Vec<String>> {
    let manifest_text = fs::read_to_string(root.join("Cargo.toml"))
        .context("failed to read workspace Cargo.toml")?;
    let manifest: RootCargoManifest =
        toml::from_str(&manifest_text).context("failed to parse workspace Cargo.toml")?;

    let allowlist = manifest
        .workspace
        .metadata
        .and_then(|metadata| metadata.publish)
        .and_then(|publish| publish.allow)
        .ok_or_else(|| {
            color_eyre::eyre::eyre!("[workspace.metadata.publish.allow] is missing from Cargo.toml")
        })?;

    if allowlist.is_empty() {
        bail!("publish allowlist is empty");
    }

    Ok(allowlist)
}

fn run_cargo_check(root: &Path, crate_name: &str) -> Result<()> {
    let output = Command::new("cargo")
        .current_dir(root)
        .args(["check", "--locked", "-p"])
        .arg(crate_name)
        .output()
        .context("failed to run cargo check")?;

    if !output.status.success() {
        bail!(
            "cargo check failed for {crate_name}: {}",
            String::from_utf8_lossy(&output.stderr).trim_end()
        );
    }

    Ok(())
}

fn package_output_dir(target_directory: &Path) -> PathBuf {
    target_directory.join("package")
}

fn crate_archive_path(target_directory: &Path, package: &MetadataPackage) -> PathBuf {
    package_output_dir(target_directory).join(format!("{}-{}.crate", package.name, package.version))
}

fn run_cargo_package_check(
    root: &Path,
    target_directory: &Path,
    package: &MetadataPackage,
    patch_args: &[String],
) -> Result<()> {
    let archive_path = crate_archive_path(target_directory, package);
    if archive_path.exists() {
        fs::remove_file(&archive_path).with_context(|| {
            format!(
                "failed to remove stale crate archive {}",
                archive_path.display()
            )
        })?;
    }

    let mut args = vec![
        "package".to_string(),
        "--locked".to_string(),
        "--no-verify".to_string(),
        "-p".to_string(),
        package.name.clone(),
    ];
    args.extend(patch_args.iter().cloned());

    let output = Command::new("cargo")
        .current_dir(root)
        .args(args)
        .output()
        .context("failed to run cargo package")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        let metadata_learning_failed = stderr.contains("could not learn metadata for:");
        if !(metadata_learning_failed && archive_path.is_file()) {
            bail!(
                "cargo package failed for {}: {}",
                package.name,
                stderr.trim_end()
            );
        }
    }

    let unpack_dir = unpack_crate_archive(target_directory, package)?;
    let packaged_manifest = unpack_dir
        .path()
        .join(format!("{}-{}", package.name, package.version))
        .join("Cargo.toml");

    let mut verify_args = vec![
        "check".to_string(),
        "--offline".to_string(),
        "--manifest-path".to_string(),
        packaged_manifest.to_string_lossy().into_owned(),
    ];
    verify_args.extend(patch_args.iter().cloned());

    let verify_output = Command::new("cargo")
        .current_dir(root)
        .args(verify_args)
        .output()
        .context("failed to run cargo check on packaged manifest")?;

    if !verify_output.status.success() {
        bail!(
            "offline packaged-manifest check failed for {}: {}",
            package.name,
            String::from_utf8_lossy(&verify_output.stderr).trim_end()
        );
    }

    Ok(())
}

fn unpack_crate_archive(target_directory: &Path, package: &MetadataPackage) -> Result<TempDir> {
    let archive_path = crate_archive_path(target_directory, package);
    if !archive_path.is_file() {
        bail!(
            "crate archive missing for {}: {}",
            package.name,
            archive_path.display()
        );
    }

    let unpack_dir = tempfile::Builder::new()
        .prefix(&format!("{}-{}-", package.name, package.version))
        .tempdir_in(package_output_dir(target_directory))
        .context("failed to create temp directory for crate verification")?;

    let archive_file = File::open(&archive_path)
        .with_context(|| format!("failed to open crate archive {}", archive_path.display()))?;
    let decoder = flate2::read::GzDecoder::new(archive_file);
    let mut archive = tar::Archive::new(decoder);
    archive
        .unpack(unpack_dir.path())
        .with_context(|| format!("failed to unpack crate archive {}", archive_path.display()))?;

    let manifest_path = unpack_dir
        .path()
        .join(format!("{}-{}", package.name, package.version))
        .join("Cargo.toml");
    if !manifest_path.is_file() {
        bail!(
            "unpacked manifest missing for {}: {}",
            package.name,
            manifest_path.display()
        );
    }

    Ok(unpack_dir)
}

#[cfg(test)]
mod tests {
    use super::{
        CargoMetadata, MetadataPackage, crate_archive_path, package_output_dir, package_patch_args,
        toml_safe_path,
    };
    use serde_json::Value as JsonValue;
    use std::path::{Path, PathBuf};

    #[test]
    fn toml_safe_path_normalizes_backslashes() {
        assert_eq!(
            toml_safe_path(Path::new(r"crates\perl-ast")),
            "crates/perl-ast"
        );
    }

    #[test]
    fn package_patch_args_skips_current_crate() {
        let metadata = CargoMetadata {
            target_directory: PathBuf::from("/workspace/target"),
            packages: vec![
                MetadataPackage {
                    name: "perl-parser".to_string(),
                    version: "0.12.1".to_string(),
                    manifest_path: "/workspace/crates/perl-parser/Cargo.toml".to_string(),
                    publish: Some(JsonValue::Bool(true)),
                },
                MetadataPackage {
                    name: "perl-lexer".to_string(),
                    version: "0.12.1".to_string(),
                    manifest_path: "/workspace/crates/perl-lexer/Cargo.toml".to_string(),
                    publish: Some(JsonValue::Bool(true)),
                },
            ],
        };

        let args = package_patch_args(&metadata, Some("perl-parser"));

        assert_eq!(
            args,
            vec![
                "--config=patch.crates-io.perl-lexer.path=\"/workspace/crates/perl-lexer\""
                    .to_string()
            ]
        );
    }

    #[test]
    fn package_output_dir_uses_cargo_target_directory() {
        let path = package_output_dir(Path::new("/workspace/custom-target"));

        assert_eq!(path, Path::new("/workspace/custom-target/package"));
    }

    #[test]
    fn crate_archive_path_points_to_packaged_archive_in_target_directory() {
        let package = MetadataPackage {
            name: "perl-parser".to_string(),
            version: "0.12.1".to_string(),
            manifest_path: "/workspace/crates/perl-parser/Cargo.toml".to_string(),
            publish: Some(JsonValue::Bool(true)),
        };

        let path = crate_archive_path(Path::new("/workspace/custom-target"), &package);

        assert_eq!(
            path,
            Path::new("/workspace/custom-target/package/perl-parser-0.12.1.crate")
        );
    }
}
