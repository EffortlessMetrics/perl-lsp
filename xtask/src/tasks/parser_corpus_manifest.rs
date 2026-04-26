use std::collections::BTreeSet;
use std::fs;
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::Command;

use clap::ValueEnum;
use color_eyre::eyre::{Context, Result, eyre};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::utils;

const SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "snake_case")]
pub enum CorpusProfile {
    Pr,
    SystemRequired,
}

#[derive(Clone, Debug)]
pub struct ManifestConfig {
    pub profile: CorpusProfile,
    pub out_path: PathBuf,
    pub receipt_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusManifest {
    pub schema_version: u32,
    pub profile: CorpusProfile,
    pub sources: Vec<CorpusSource>,
    pub runner: RunnerInfo,
    pub files: Vec<CorpusFile>,
    pub fingerprint: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct RunnerInfo {
    pub os: String,
    pub perl_version: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusSource {
    pub id: String,
    pub source_type: String,
    pub root: String,
    pub status: String,
    pub note: Option<String>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CorpusFile {
    pub path: String,
    pub source: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concepts: Option<Vec<String>>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ManifestReceipt {
    pub schema_version: u32,
    pub profile: CorpusProfile,
    pub outcome: String,
    pub message: String,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub advisories: Vec<String>,
    pub manifest_path: String,
    pub fingerprint: Option<String>,
    pub file_count: usize,
}

pub fn run(config: ManifestConfig) -> Result<()> {
    let mut advisories = Vec::new();
    let mut sources = Vec::new();
    let mut files = Vec::new();

    collect_repo_sources(&mut sources, &mut files)?;

    let runner_os = std::env::consts::OS.to_string();
    let perl_version_result = query_perl_version();
    let perl_version = match perl_version_result {
        Ok(version) => version,
        Err(err) => {
            let advisory = format!("system perl version probe failed: {err}");
            match config.profile {
                CorpusProfile::Pr => {
                    advisories.push(advisory);
                    "unknown".to_string()
                }
                CorpusProfile::SystemRequired => {
                    return Err(err);
                }
            }
        }
    };

    match collect_system_perl_sources() {
        Ok((mut discovered_sources, mut discovered_files)) => {
            sources.append(&mut discovered_sources);
            files.append(&mut discovered_files);
        }
        Err(err) => match config.profile {
            CorpusProfile::Pr => {
                advisories.push(format!("system perl discovery failed: {err}"));
                sources.push(CorpusSource {
                    id: "system-perl".to_string(),
                    source_type: "system".to_string(),
                    root: "perl-config".to_string(),
                    status: "advisory".to_string(),
                    note: Some("system perl discovery failed in PR profile".to_string()),
                });
            }
            CorpusProfile::SystemRequired => {
                return Err(err);
            }
        },
    }

    files.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.source.cmp(&b.source)));
    sources.sort_by(|a, b| a.id.cmp(&b.id));

    let fingerprint = compute_fingerprint(&config.profile, &runner_os, &perl_version, &files);

    let manifest = CorpusManifest {
        schema_version: SCHEMA_VERSION,
        profile: config.profile.clone(),
        sources,
        runner: RunnerInfo { os: runner_os, perl_version },
        files,
        fingerprint: fingerprint.clone(),
    };

    if let Some(parent) = config.out_path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory: {}", parent.display()))?;
    }
    let manifest_json = serde_json::to_vec_pretty(&manifest)?;
    fs::write(&config.out_path, manifest_json)
        .with_context(|| format!("Failed to write manifest: {}", config.out_path.display()))?;

    if let Some(receipt_path) = config.receipt_path {
        if let Some(parent) = receipt_path.parent() {
            fs::create_dir_all(parent).with_context(|| {
                format!("Failed to create receipt directory: {}", parent.display())
            })?;
        }
        let (outcome, message) = if advisories.is_empty() {
            ("ok".to_string(), "manifest generated".to_string())
        } else {
            (
                "advisory".to_string(),
                "manifest generated with infrastructure advisories".to_string(),
            )
        };
        let receipt = ManifestReceipt {
            schema_version: SCHEMA_VERSION,
            profile: config.profile,
            outcome,
            message,
            advisories,
            manifest_path: portable_path(&config.out_path),
            fingerprint: Some(fingerprint),
            file_count: manifest.files.len(),
        };
        let receipt_json = serde_json::to_vec_pretty(&receipt)?;
        fs::write(&receipt_path, receipt_json)
            .with_context(|| format!("Failed to write receipt: {}", receipt_path.display()))?;
    }

    Ok(())
}

fn collect_repo_sources(
    sources: &mut Vec<CorpusSource>,
    files: &mut Vec<CorpusFile>,
) -> Result<()> {
    let root = utils::project_root()?;
    let repo_roots = ["tests/perl-corpus", "tests/parser"];

    for rel in repo_roots {
        let abs = root.join(rel);
        if !abs.exists() {
            continue;
        }
        sources.push(CorpusSource {
            id: format!("repo:{rel}"),
            source_type: "repo".to_string(),
            root: rel.to_string(),
            status: "ok".to_string(),
            note: None,
        });

        for entry in WalkDir::new(&abs).into_iter().filter_map(std::result::Result::ok) {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_perl_file(path) {
                continue;
            }
            let rel_path =
                path.strip_prefix(&root).map(portable_path).unwrap_or_else(|_| portable_path(path));
            files.push(file_record(path, rel_path, format!("repo:{rel}"))?);
        }
    }
    Ok(())
}

fn collect_system_perl_sources() -> Result<(Vec<CorpusSource>, Vec<CorpusFile>)> {
    let mut sources = Vec::new();
    let mut files = Vec::new();
    let mut seen = BTreeSet::new();

    for (key, root) in perl_config_roots()? {
        let root_path = PathBuf::from(&root);
        let source_id = format!("system:{key}");

        if !root_path.exists() {
            sources.push(CorpusSource {
                id: source_id,
                source_type: "system".to_string(),
                root,
                status: "missing".to_string(),
                note: Some("path does not exist".to_string()),
            });
            continue;
        }
        if !root_path.is_dir() {
            sources.push(CorpusSource {
                id: source_id,
                source_type: "system".to_string(),
                root,
                status: "skipped".to_string(),
                note: Some("path is not a directory".to_string()),
            });
            continue;
        }

        sources.push(CorpusSource {
            id: format!("system:{key}"),
            source_type: "system".to_string(),
            root: portable_path(&root_path),
            status: "ok".to_string(),
            note: None,
        });

        for entry in WalkDir::new(&root_path).into_iter().filter_map(std::result::Result::ok) {
            let path = entry.path();
            if !entry.file_type().is_file() || !is_perl_file(path) {
                continue;
            }
            let portable = portable_path(path);
            if seen.insert(portable.clone()) {
                files.push(file_record(path, portable, format!("system:{key}"))?);
            }
        }
    }

    Ok((sources, files))
}

fn perl_config_roots() -> Result<Vec<(String, String)>> {
    let script = r#"use Config; for my $k (qw(privlib archlib vendorlib vendorarch)) { my $v = $Config{$k}; next if !defined($v) || $v eq ''; print "$k=$v\n"; }"#;
    let output = Command::new("perl")
        .args(["-e", script])
        .output()
        .context("Failed to run perl for Config path discovery")?;

    if !output.status.success() {
        return Err(eyre!(
            "Perl Config path discovery failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8(output.stdout).context("Perl Config output was not UTF-8")?;
    let mut pairs = Vec::new();
    for line in stdout.lines() {
        if line.trim().is_empty() {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        pairs.push((key.to_string(), value.to_string()));
    }
    pairs.sort();
    pairs.dedup();
    Ok(pairs)
}

fn query_perl_version() -> Result<String> {
    let output = Command::new("perl")
        .args(["-e", r#"print $^V"#])
        .output()
        .context("Failed to run perl for version discovery")?;
    if !output.status.success() {
        return Err(eyre!(
            "Perl version discovery failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    let version = String::from_utf8(output.stdout).context("Perl version output was not UTF-8")?;
    Ok(version.trim().to_string())
}

fn is_perl_file(path: &Path) -> bool {
    matches!(path.extension().and_then(|ext| ext.to_str()), Some("pm" | "pl"))
}

fn file_record(path: &Path, portable: String, source: String) -> Result<CorpusFile> {
    let mut file =
        fs::File::open(path).with_context(|| format!("Failed to open file: {}", path.display()))?;
    let mut hasher = Sha256::new();
    let mut bytes: u64 = 0;
    let mut buf = [0_u8; 16 * 1024];
    loop {
        let read = file
            .read(&mut buf)
            .with_context(|| format!("Failed to read file: {}", path.display()))?;
        if read == 0 {
            break;
        }
        bytes += read as u64;
        hasher.update(&buf[..read]);
    }

    Ok(CorpusFile {
        path: portable,
        source,
        bytes,
        sha256: format!("{:x}", hasher.finalize()),
        concepts: None,
    })
}

fn portable_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn compute_fingerprint(
    profile: &CorpusProfile,
    runner_os: &str,
    perl_version: &str,
    files: &[CorpusFile],
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(format!("schema={SCHEMA_VERSION}\n"));
    hasher.update(format!("profile={profile:?}\n"));
    hasher.update(format!("os={runner_os}\n"));
    hasher.update(format!("perl={perl_version}\n"));
    for file in files {
        hasher.update(file.path.as_bytes());
        hasher.update(b"\0");
        hasher.update(file.source.as_bytes());
        hasher.update(b"\0");
        hasher.update(file.bytes.to_string().as_bytes());
        hasher.update(b"\0");
        hasher.update(file.sha256.as_bytes());
        hasher.update(b"\n");
    }
    format!("{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_same_ordered_files() {
        let files = vec![
            CorpusFile {
                path: "a.pm".to_string(),
                source: "repo:tests/parser".to_string(),
                bytes: 12,
                sha256: "abc".to_string(),
                concepts: None,
            },
            CorpusFile {
                path: "b.pm".to_string(),
                source: "system:privlib".to_string(),
                bytes: 33,
                sha256: "def".to_string(),
                concepts: None,
            },
        ];

        let one = compute_fingerprint(&CorpusProfile::Pr, "linux", "v5.38.2", &files);
        let two = compute_fingerprint(&CorpusProfile::Pr, "linux", "v5.38.2", &files);
        assert_eq!(one, two);
    }

    #[test]
    fn fingerprint_changes_when_file_set_changes() {
        let files_a = vec![CorpusFile {
            path: "a.pm".to_string(),
            source: "repo:tests/parser".to_string(),
            bytes: 12,
            sha256: "abc".to_string(),
            concepts: None,
        }];
        let files_b = vec![CorpusFile {
            path: "a.pm".to_string(),
            source: "repo:tests/parser".to_string(),
            bytes: 13,
            sha256: "abc".to_string(),
            concepts: None,
        }];

        let one = compute_fingerprint(&CorpusProfile::Pr, "linux", "v5.38.2", &files_a);
        let two = compute_fingerprint(&CorpusProfile::Pr, "linux", "v5.38.2", &files_b);
        assert_ne!(one, two);
    }
}
