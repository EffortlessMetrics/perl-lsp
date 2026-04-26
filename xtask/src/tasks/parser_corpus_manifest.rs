use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use color_eyre::eyre::{Context, Result, bail};
use serde::Serialize;
use sha2::{Digest, Sha256};
use walkdir::WalkDir;

use crate::utils;

const SCHEMA_VERSION: &str = "1.0.0";

#[derive(Clone, Copy, Debug)]
pub enum Profile {
    Pr,
}

impl Profile {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pr => "pr",
        }
    }

    fn requires_system_perl(self) -> bool {
        match self {
            Self::Pr => false,
        }
    }
}

#[derive(Debug)]
pub struct ManifestConfig {
    pub profile: Profile,
    pub out: PathBuf,
    pub receipt: Option<PathBuf>,
}

#[derive(Serialize)]
struct Manifest {
    schema_version: String,
    profile: String,
    sources: Vec<ManifestSource>,
    runner: RunnerInfo,
    files: Vec<ManifestFile>,
    fingerprint: String,
}

#[derive(Serialize)]
struct ManifestSource {
    source: String,
    root: String,
    files: usize,
}

#[derive(Serialize)]
struct RunnerInfo {
    os: String,
    perl_version: String,
}

#[derive(Serialize, Clone)]
struct ManifestFile {
    path: String,
    source: String,
    bytes: u64,
    sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    concepts: Option<Vec<String>>,
}

#[derive(Serialize)]
struct ManifestReceipt {
    schema_version: String,
    profile: String,
    status: String,
    manifest_path: String,
    fingerprint: String,
    file_count: usize,
    advisory: Vec<String>,
}

pub fn run(config: ManifestConfig) -> Result<()> {
    let root = utils::project_root()?;
    let mut advisory = Vec::new();

    let repo_files = discover_repo_files(&root)?;

    let (perl_version, system_paths) = match discover_system_perl_paths() {
        Ok(found) => found,
        Err(error) => {
            if config.profile.requires_system_perl() {
                bail!(
                    "system Perl discovery required for profile {}: {error}",
                    config.profile.as_str()
                );
            }
            advisory.push(format!("system Perl discovery unavailable: {error}"));
            ("unknown".to_string(), BTreeMap::new())
        }
    };

    let system_files = discover_system_files(&system_paths, &mut advisory)?;

    let mut files = Vec::new();
    files.extend(repo_files);
    files.extend(system_files);
    files.sort_by(|a, b| a.path.cmp(&b.path).then_with(|| a.source.cmp(&b.source)));

    let sources = summarize_sources(&files);
    let fingerprint = compute_fingerprint(config.profile, &files);

    let manifest = Manifest {
        schema_version: SCHEMA_VERSION.to_string(),
        profile: config.profile.as_str().to_string(),
        sources,
        runner: RunnerInfo { os: std::env::consts::OS.to_string(), perl_version },
        files,
        fingerprint: fingerprint.clone(),
    };

    write_json(&config.out, &manifest)?;

    if let Some(receipt_path) = config.receipt {
        let receipt = ManifestReceipt {
            schema_version: SCHEMA_VERSION.to_string(),
            profile: config.profile.as_str().to_string(),
            status: if advisory.is_empty() { "ok".to_string() } else { "advisory".to_string() },
            manifest_path: normalize_path(&config.out),
            fingerprint,
            file_count: manifest.files.len(),
            advisory,
        };
        write_json(&receipt_path, &receipt)?;
    }

    Ok(())
}

fn write_json<T: Serialize>(path: &Path, value: &T) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("creating parent dir for {}", path.display()))?;
    }
    let payload = serde_json::to_string_pretty(value).context("serializing json")?;
    fs::write(path, format!("{payload}\n"))
        .with_context(|| format!("writing {}", path.display()))?;
    Ok(())
}

fn discover_repo_files(root: &Path) -> Result<Vec<ManifestFile>> {
    let mut files = Vec::new();
    let patterns =
        [("repo:tests/perl-corpus", "tests/perl-corpus"), ("repo:tests/parser", "tests/parser")];

    for (source, rel) in patterns {
        let dir = root.join(rel);
        if !dir.exists() {
            continue;
        }
        files.extend(discover_files_under(&dir, source, &mut Vec::new())?);
    }

    Ok(files)
}

fn discover_system_perl_paths() -> Result<(String, BTreeMap<String, PathBuf>)> {
    let output = Command::new("perl")
        .args([
            "-MConfig",
            "-e",
            "print join(qq{\\n}, map { defined $Config{$_} ? qq{$_=$Config{$_}} : () } qw(privlib archlib vendorlib vendorarch));",
        ])
        .output()
        .context("running perl Config probe")?;

    if !output.status.success() {
        bail!("perl Config probe failed with status {}", output.status);
    }

    let stdout = String::from_utf8(output.stdout).context("perl Config probe emitted non-utf8")?;

    let version_output = Command::new("perl")
        .args(["-e", "print $^V"])
        .output()
        .context("running perl version probe")?;

    let perl_version = if version_output.status.success() {
        String::from_utf8(version_output.stdout)
            .map(|s| s.trim().to_string())
            .unwrap_or_else(|_| "unknown".to_string())
    } else {
        "unknown".to_string()
    };

    let mut paths = BTreeMap::new();
    for line in stdout.lines() {
        if let Some((key, value)) = line.split_once('=')
            && !value.trim().is_empty()
        {
            paths.insert(key.to_string(), PathBuf::from(value.trim()));
        }
    }

    Ok((perl_version, paths))
}

fn discover_system_files(
    perl_paths: &BTreeMap<String, PathBuf>,
    advisory: &mut Vec<String>,
) -> Result<Vec<ManifestFile>> {
    let mut files = Vec::new();
    let mut dedup = BTreeSet::new();

    for (bucket, path) in perl_paths {
        if !path.exists() {
            advisory.push(format!("system path missing: {}={}", bucket, path.display()));
            continue;
        }

        let source = format!("system:{bucket}");
        let bucket_files = discover_files_under(path, &source, advisory)?;
        for item in bucket_files {
            if dedup.insert(item.path.clone()) {
                files.push(item);
            }
        }
    }

    Ok(files)
}

fn discover_files_under(
    path: &Path,
    source: &str,
    advisory: &mut Vec<String>,
) -> Result<Vec<ManifestFile>> {
    let mut files = Vec::new();

    for entry in WalkDir::new(path).follow_links(false).into_iter().filter_map(Result::ok) {
        let file_type = entry.file_type();
        if !file_type.is_file() {
            continue;
        }

        let ext = entry.path().extension().and_then(|e| e.to_str());
        if !matches!(ext, Some("pm" | "pl")) {
            continue;
        }

        let record = match build_file_record(entry.path(), source) {
            Ok(value) => value,
            Err(error) => {
                advisory
                    .push(format!("skipping unreadable file {}: {error}", entry.path().display()));
                continue;
            }
        };

        files.push(record);
    }

    Ok(files)
}

fn build_file_record(path: &Path, source: &str) -> Result<ManifestFile> {
    let bytes = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha256 = format!("{:x}", hasher.finalize());

    Ok(ManifestFile {
        path: normalize_path(path),
        source: source.to_string(),
        bytes: u64::try_from(bytes.len()).context("file length does not fit u64")?,
        sha256,
        concepts: None,
    })
}

fn summarize_sources(files: &[ManifestFile]) -> Vec<ManifestSource> {
    let mut counts: BTreeMap<(String, String), usize> = BTreeMap::new();
    for file in files {
        let root = source_root(&file.path, &file.source);
        *counts.entry((file.source.clone(), root)).or_default() += 1;
    }

    counts
        .into_iter()
        .map(|((source, root), files)| ManifestSource { source, root, files })
        .collect()
}

fn source_root(path: &str, source: &str) -> String {
    if source.starts_with("repo:") {
        let maybe = path.split("/tests/").next();
        if let Some(prefix) = maybe
            && !prefix.is_empty()
        {
            return prefix.to_string();
        }
    }

    let parent = Path::new(path).parent().map(normalize_path);
    parent.unwrap_or_else(|| ".".to_string())
}

fn compute_fingerprint(profile: Profile, files: &[ManifestFile]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(profile.as_str().as_bytes());
    hasher.update([0u8]);

    for file in files {
        hasher.update(file.path.as_bytes());
        hasher.update([0u8]);
        hasher.update(file.source.as_bytes());
        hasher.update([0u8]);
        hasher.update(file.bytes.to_string().as_bytes());
        hasher.update([0u8]);
        hasher.update(file.sha256.as_bytes());
        hasher.update([0u8]);
    }

    format!("{:x}", hasher.finalize())
}

fn normalize_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

#[cfg(test)]
mod tests {
    use super::*;
    use color_eyre::eyre::Result;

    #[test]
    fn fingerprint_is_stable_for_identical_inputs() {
        let files = vec![ManifestFile {
            path: "a.pm".to_string(),
            source: "repo:tests/perl-corpus".to_string(),
            bytes: 1,
            sha256: "abc".to_string(),
            concepts: None,
        }];

        let a = compute_fingerprint(Profile::Pr, &files);
        let b = compute_fingerprint(Profile::Pr, &files);
        assert_eq!(a, b);
    }

    #[test]
    fn fingerprint_changes_when_file_changes() {
        let a = vec![ManifestFile {
            path: "a.pm".to_string(),
            source: "repo:tests/perl-corpus".to_string(),
            bytes: 1,
            sha256: "abc".to_string(),
            concepts: None,
        }];
        let b = vec![ManifestFile {
            path: "a.pm".to_string(),
            source: "repo:tests/perl-corpus".to_string(),
            bytes: 2,
            sha256: "abc".to_string(),
            concepts: None,
        }];

        assert_ne!(compute_fingerprint(Profile::Pr, &a), compute_fingerprint(Profile::Pr, &b));
    }

    #[test]
    fn missing_system_path_is_advisory_not_error() -> Result<()> {
        let mut advisory = Vec::new();
        let mut paths = BTreeMap::new();
        paths.insert("vendorlib".to_string(), PathBuf::from("/definitely/missing/path"));

        let files = discover_system_files(&paths, &mut advisory)?;
        assert!(files.is_empty());
        assert!(!advisory.is_empty());
        Ok(())
    }
}
