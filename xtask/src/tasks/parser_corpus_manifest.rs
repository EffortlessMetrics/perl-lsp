use color_eyre::eyre::{Context, Result};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;
use walkdir::WalkDir;

const SCHEMA_VERSION: &str = "1.0.0";
const PROFILE_PR: &str = "pr";

#[derive(Debug, Clone)]
pub struct ParserCorpusManifestConfig {
    pub profile: String,
    pub out: PathBuf,
    pub receipt: Option<PathBuf>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParserCorpusManifest {
    pub schema_version: String,
    pub profile: String,
    pub sources: Vec<ManifestSource>,
    pub runner: RunnerInfo,
    pub files: Vec<ManifestFile>,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunnerInfo {
    pub os: String,
    pub perl_version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestSource {
    pub name: String,
    pub roots: Vec<String>,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ManifestFile {
    pub path: String,
    pub source: String,
    pub bytes: u64,
    pub sha256: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub concepts: Option<Vec<String>>,
}

pub fn run(config: ParserCorpusManifestConfig) -> Result<()> {
    let profile = config.profile.trim().to_lowercase();
    let requires_system = requires_system_corpus(&profile);

    let runner =
        RunnerInfo { os: std::env::consts::OS.to_string(), perl_version: detect_perl_version() };

    let mut sources = Vec::new();
    let mut files = Vec::new();

    let repo_patterns = [
        ("tests/perl-corpus", "repo:perl-corpus", "repo-perl-corpus"),
        ("tests/parser", "repo:parser-tests", "repo-parser-tests"),
    ];

    for (root, source_name, concept) in repo_patterns {
        let root_path = PathBuf::from(root);
        if !root_path.exists() {
            continue;
        }

        let mut root_files =
            collect_perl_files_under(&root_path, source_name, Some(concept.to_string()))?;
        files.append(&mut root_files);
        sources.push(ManifestSource {
            name: source_name.to_string(),
            roots: vec![root.to_string()],
            status: "ok".to_string(),
            note: None,
        });
    }

    match discover_system_perl_roots() {
        Ok(roots) => {
            if roots.is_empty() {
                let note = "perl Config returned no ambient system roots".to_string();
                if requires_system {
                    return Err(color_eyre::eyre::eyre!(note));
                }
                sources.push(ManifestSource {
                    name: "system:perl-config".to_string(),
                    roots: Vec::new(),
                    status: "advisory".to_string(),
                    note: Some(note),
                });
            } else {
                let root_display =
                    roots.iter().map(|path| path.display().to_string()).collect::<Vec<_>>();
                let mut discovered = Vec::new();
                for root in &roots {
                    let mut root_files = collect_perl_files_under(root, "system:ambient", None)?;
                    files.append(&mut root_files);
                    discovered.push(root.display().to_string());
                }
                sources.push(ManifestSource {
                    name: "system:perl-config".to_string(),
                    roots: root_display,
                    status: "ok".to_string(),
                    note: Some(format!("discovered {} roots", discovered.len())),
                });
            }
        }
        Err(err) => {
            if requires_system {
                return Err(err);
            }
            sources.push(ManifestSource {
                name: "system:perl-config".to_string(),
                roots: Vec::new(),
                status: "advisory".to_string(),
                note: Some(format!("system perl discovery failed: {err}")),
            });
        }
    }

    files.sort_by(|left, right| {
        left.path
            .cmp(&right.path)
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.sha256.cmp(&right.sha256))
    });

    let fingerprint = compute_fingerprint(&profile, &runner, &files);
    let manifest = ParserCorpusManifest {
        schema_version: SCHEMA_VERSION.to_string(),
        profile,
        sources,
        runner,
        files,
        fingerprint,
    };

    write_json(&config.out, &manifest)?;
    if let Some(receipt) = &config.receipt {
        write_json(receipt, &manifest)?;
    }

    println!(
        "Parser corpus manifest: {} files, fingerprint {}",
        manifest.files.len(),
        manifest.fingerprint
    );

    Ok(())
}

fn requires_system_corpus(profile: &str) -> bool {
    profile != PROFILE_PR
}

fn write_json(path: &Path, manifest: &ParserCorpusManifest) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create output directory {}", parent.display()))?;
    }

    let payload = serde_json::to_string_pretty(manifest).context("Failed to serialize manifest")?;
    fs::write(path, format!("{payload}\n"))
        .with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

fn collect_perl_files_under(
    root: &Path,
    source: &str,
    concept: Option<String>,
) -> Result<Vec<ManifestFile>> {
    let mut files = Vec::new();
    for entry in WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_map(std::result::Result::ok)
        .filter(|entry| entry.file_type().is_file())
    {
        let path = entry.path();
        if !is_perl_file(path) {
            continue;
        }

        if let Ok(record) = hash_file(path, source, concept.clone()) {
            files.push(record);
        }
    }

    Ok(files)
}

fn is_perl_file(path: &Path) -> bool {
    path.extension()
        .and_then(|ext| ext.to_str())
        .is_some_and(|ext| ext.eq_ignore_ascii_case("pm") || ext.eq_ignore_ascii_case("pl"))
}

fn hash_file(path: &Path, source: &str, concept: Option<String>) -> Result<ManifestFile> {
    let bytes = fs::read(path).with_context(|| format!("Failed to read {}", path.display()))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    let sha = hex_lower(hasher.finalize().as_slice());
    let canonical = fs::canonicalize(path)
        .with_context(|| format!("Failed to canonicalize {}", path.display()))?;

    Ok(ManifestFile {
        path: canonical.display().to_string(),
        source: source.to_string(),
        bytes: bytes.len() as u64,
        sha256: sha,
        concepts: concept.map(|item| vec![item]),
    })
}

fn compute_fingerprint(profile: &str, runner: &RunnerInfo, files: &[ManifestFile]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(SCHEMA_VERSION.as_bytes());
    hasher.update([0]);
    hasher.update(profile.as_bytes());
    hasher.update([0]);
    hasher.update(runner.os.as_bytes());
    hasher.update([0]);
    hasher.update(runner.perl_version.as_bytes());
    hasher.update([0]);

    for file in files {
        hasher.update(file.path.as_bytes());
        hasher.update([0]);
        hasher.update(file.source.as_bytes());
        hasher.update([0]);
        hasher.update(file.bytes.to_string().as_bytes());
        hasher.update([0]);
        hasher.update(file.sha256.as_bytes());
        hasher.update([0]);
    }

    hex_lower(hasher.finalize().as_slice())
}

fn discover_system_perl_roots() -> Result<Vec<PathBuf>> {
    let output = Command::new("perl")
        .args([
            "-MConfig",
            "-e",
            "print join(qq(\\n), map { $Config{$_}//q() } qw(privlib archlib vendorlib vendorarch));",
        ])
        .output()
        .context("Failed to run perl for Config path discovery")?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(color_eyre::eyre::eyre!(
            "perl Config discovery exited with {}: {}",
            output.status,
            stderr.trim()
        ));
    }

    let stdout =
        String::from_utf8(output.stdout).context("Invalid UTF-8 from perl Config output")?;
    let mut roots = BTreeSet::new();
    for line in stdout.lines().map(str::trim).filter(|line| !line.is_empty()) {
        let path = PathBuf::from(line);
        if path.exists() && path.is_dir() {
            roots.insert(path);
        }
    }

    Ok(roots.into_iter().collect())
}

fn detect_perl_version() -> String {
    let output = Command::new("perl").args(["-e", "print $^V"]).output();
    match output {
        Ok(output) if output.status.success() => String::from_utf8(output.stdout)
            .map(|value| {
                let trimmed = value.trim().to_string();
                if trimmed.is_empty() { "unknown".to_string() } else { trimmed }
            })
            .unwrap_or_else(|_| "unknown".to_string()),
        _ => "unknown".to_string(),
    }
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(nibble_to_hex(byte >> 4));
        out.push(nibble_to_hex(byte & 0x0f));
    }
    out
}

fn nibble_to_hex(value: u8) -> char {
    match value {
        0..=9 => (b'0' + value) as char,
        10..=15 => (b'a' + (value - 10)) as char,
        _ => '0',
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fingerprint_is_stable_for_same_file_set() -> Result<()> {
        let tmp = tempfile::tempdir().context("create tempdir")?;
        let file_path = tmp.path().join("sample.pm");
        fs::write(&file_path, "package Sample; 1;\n").context("write sample")?;

        let mut files = collect_perl_files_under(tmp.path(), "repo:perl-corpus", None)?;
        files.sort_by(|left, right| left.path.cmp(&right.path));

        let runner = RunnerInfo { os: "linux".to_string(), perl_version: "v5.36.0".to_string() };

        let first = compute_fingerprint("pr", &runner, &files);
        let second = compute_fingerprint("pr", &runner, &files);
        assert_eq!(first, second);
        Ok(())
    }

    #[test]
    fn source_discovery_deduplicates_roots() -> Result<()> {
        let roots = discover_system_perl_roots().unwrap_or_default();
        let unique_count = roots.iter().collect::<BTreeSet<_>>().len();
        assert_eq!(roots.len(), unique_count);
        Ok(())
    }
}
