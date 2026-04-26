use color_eyre::eyre::{Context, Result};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const SCHEMA_VERSION: &str = "1.0.0";

#[derive(Debug, Clone)]
pub struct ManifestConfig {
    pub profile: String,
    pub out: PathBuf,
    pub receipt: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CorpusManifest {
    pub schema_version: String,
    pub profile: String,
    pub sources: Vec<ManifestSource>,
    pub runner: RunnerInfo,
    pub files: Vec<ManifestFile>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestSource {
    pub id: String,
    pub kind: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RunnerInfo {
    pub os: String,
    pub perl_version: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestFile {
    pub path: String,
    pub source: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concepts: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ManifestReceipt {
    pub schema_version: String,
    pub profile: String,
    pub out: String,
    pub fingerprint: String,
    pub total_files: usize,
    pub advisory: bool,
    pub advisories: Vec<String>,
}

pub fn run(config: ManifestConfig) -> Result<()> {
    let mut sources = Vec::new();
    let mut files = Vec::new();
    let mut advisories = Vec::new();

    collect_repo_sources(&mut sources, &mut files)?;
    collect_system_perl_sources(&mut sources, &mut files, &mut advisories)?;

    files.sort_by(|a, b| a.path.cmp(&b.path).then(a.source.cmp(&b.source)));

    let runner =
        RunnerInfo { os: std::env::consts::OS.to_string(), perl_version: detect_perl_version() };

    let fingerprint = compute_fingerprint(&config.profile, &runner, &files);
    let manifest = CorpusManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        profile: config.profile.clone(),
        sources,
        runner,
        files,
        fingerprint: fingerprint.clone(),
    };

    if let Some(parent) = config.out.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create output directory {}", parent.display()))?;
    }
    fs::write(&config.out, serde_json::to_string_pretty(&manifest)?)
        .with_context(|| format!("failed to write manifest to {}", config.out.display()))?;

    if let Some(receipt_path) = config.receipt {
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("failed to create receipt directory {}", parent.display())
            })?;
        }
        let advisory = !advisories.is_empty() && !profile_requires_system_perl(&config.profile);
        let receipt = ManifestReceipt {
            schema_version: SCHEMA_VERSION.to_string(),
            profile: config.profile,
            out: config.out.display().to_string(),
            fingerprint,
            total_files: manifest.files.len(),
            advisory,
            advisories,
        };
        fs::write(receipt_path, serde_json::to_string_pretty(&receipt)?)
            .context("failed to write parser corpus manifest receipt")?;
    }

    Ok(())
}

fn collect_repo_sources(
    sources: &mut Vec<ManifestSource>,
    files: &mut Vec<ManifestFile>,
) -> Result<()> {
    let repo_patterns = ["tests/perl-corpus", "tests/parser"];
    for root in repo_patterns {
        let path = PathBuf::from(root);
        if !path.exists() {
            sources.push(ManifestSource {
                id: format!("repo:{}", root),
                kind: "repo".to_string(),
                path: Some(root.to_string()),
                status: "missing".to_string(),
                note: None,
            });
            continue;
        }
        let mut added = 0usize;
        for entry in WalkDir::new(&path)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let Some(ext) = entry.path().extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            if ext != "pl" && ext != "pm" {
                continue;
            }
            if let Some(file) = make_file_entry(entry.path(), "repo")? {
                files.push(file);
                added += 1;
            }
        }
        sources.push(ManifestSource {
            id: format!("repo:{}", root),
            kind: "repo".to_string(),
            path: Some(root.to_string()),
            status: "ok".to_string(),
            note: Some(format!("{added} files")),
        });
    }
    Ok(())
}

fn collect_system_perl_sources(
    sources: &mut Vec<ManifestSource>,
    files: &mut Vec<ManifestFile>,
    advisories: &mut Vec<String>,
) -> Result<()> {
    let output = Command::new("perl")
        .args([
            "-MConfig",
            "-e",
            "print join(qq{\\n}, map { \"$_=$Config{$_}\" } qw(privlib archlib vendorlib vendorarch));",
        ])
        .output();

    let output = match output {
        Ok(value) if value.status.success() => value,
        Ok(value) => {
            advisories
                .push(format!("system perl config discovery failed with status {}", value.status));
            sources.push(ManifestSource {
                id: "system:perl-config".to_string(),
                kind: "system".to_string(),
                path: None,
                status: "error".to_string(),
                note: Some("unable to query perl config".to_string()),
            });
            return Ok(());
        }
        Err(error) => {
            advisories.push(format!("system perl config discovery failed: {error}"));
            sources.push(ManifestSource {
                id: "system:perl-config".to_string(),
                kind: "system".to_string(),
                path: None,
                status: "error".to_string(),
                note: Some("perl executable unavailable".to_string()),
            });
            return Ok(());
        }
    };

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut unique_paths = BTreeSet::new();
    for line in stdout.lines().filter(|line| !line.trim().is_empty()) {
        let Some((key, raw_path)) = line.split_once('=') else {
            continue;
        };
        let path = PathBuf::from(raw_path);
        if !unique_paths.insert(path.clone()) {
            continue;
        }

        if !path.exists() {
            sources.push(ManifestSource {
                id: format!("system:{key}"),
                kind: "system".to_string(),
                path: Some(path.display().to_string()),
                status: "missing".to_string(),
                note: None,
            });
            continue;
        }
        if !path.is_dir() {
            sources.push(ManifestSource {
                id: format!("system:{key}"),
                kind: "system".to_string(),
                path: Some(path.display().to_string()),
                status: "unreadable".to_string(),
                note: Some("not a directory".to_string()),
            });
            advisories.push(format!("system perl path is not a directory: {}", path.display()));
            continue;
        }

        let mut added = 0usize;
        for entry in WalkDir::new(&path)
            .follow_links(false)
            .into_iter()
            .filter_map(std::result::Result::ok)
            .filter(|entry| entry.file_type().is_file())
        {
            let Some(ext) = entry.path().extension().and_then(|ext| ext.to_str()) else {
                continue;
            };
            if ext != "pm" && ext != "pl" {
                continue;
            }

            match make_file_entry(entry.path(), &format!("system:{key}")) {
                Ok(Some(file)) => {
                    files.push(file);
                    added += 1;
                }
                Ok(None) => {}
                Err(error) => {
                    advisories.push(format!(
                        "skipped unreadable file {}: {error}",
                        entry.path().display()
                    ));
                }
            }
        }

        sources.push(ManifestSource {
            id: format!("system:{key}"),
            kind: "system".to_string(),
            path: Some(path.display().to_string()),
            status: "ok".to_string(),
            note: Some(format!("{added} files")),
        });
    }

    Ok(())
}

fn make_file_entry(path: &Path, source: &str) -> Result<Option<ManifestFile>> {
    let Ok(bytes) = fs::read(path) else {
        return Ok(None);
    };
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha = format!("{:x}", hasher.finalize());

    Ok(Some(ManifestFile {
        path: portable_path(path),
        source: source.to_string(),
        bytes: bytes.len() as u64,
        sha256: sha,
        concepts: None,
    }))
}

fn portable_path(path: &Path) -> String {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..");
    if let Ok(rel) = path.strip_prefix(&root) {
        return rel.display().to_string();
    }
    path.display().to_string()
}

fn detect_perl_version() -> String {
    let output = Command::new("perl").args(["-e", "print $^V"]).output();
    match output {
        Ok(value) if value.status.success() => {
            String::from_utf8_lossy(&value.stdout).trim().to_string()
        }
        _ => "unknown".to_string(),
    }
}

fn compute_fingerprint(profile: &str, runner: &RunnerInfo, files: &[ManifestFile]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!(
        "schema={SCHEMA_VERSION}\nprofile={profile}\nos={}\nperl={}\n",
        runner.os, runner.perl_version
    ));
    for file in files {
        hasher.update(format!("{}\t{}\t{}\t{}\n", file.path, file.source, file.bytes, file.sha256));
    }
    format!("{:x}", hasher.finalize())
}

fn profile_requires_system_perl(profile: &str) -> bool {
    profile == "strict"
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_same_sorted_files() {
        let runner = RunnerInfo { os: "linux".to_string(), perl_version: "v5.38.2".to_string() };
        let files = vec![
            ManifestFile {
                path: "a.pm".to_string(),
                source: "repo".to_string(),
                bytes: 1,
                sha256: "aa".to_string(),
                concepts: None,
            },
            ManifestFile {
                path: "b.pm".to_string(),
                source: "system:privlib".to_string(),
                bytes: 2,
                sha256: "bb".to_string(),
                concepts: None,
            },
        ];
        let one = compute_fingerprint("pr", &runner, &files);
        let two = compute_fingerprint("pr", &runner, &files);
        assert_eq!(one, two);
    }
}
