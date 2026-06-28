//! Validate that produced release archives ship the binaries downstream DAP
//! consumers depend on (notably `perl-dap` alongside `perllsp`).
//!
//! The release workflow (`.github/workflows/release.yml`) builds and packages
//! both binaries, but nothing proves that contract holds for a given `dist/`
//! before the release is published. This check closes that gap: it reads the
//! contract from `docs/reference/downstream-dap-integrations.json` and verifies
//! every archive against it.
//!
//! Archive layout produced by `release.yml` (mirrored by the fixtures):
//! each `perllsp-<version>-<triple>{.tar.gz,.zip}` unpacks to a
//! `perllsp-<version>-<triple>/` directory containing the LSP and DAP binaries,
//! a per-archive `SHA256SUMS.txt`, and the license/readme files. A consolidated
//! top-level `SHA256SUMS` lists every archive by basename.

use color_eyre::eyre::{Context, Result, bail};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use crate::utils::project_root;

const DEFAULT_CONTRACT_REL: &str = "docs/reference/downstream-dap-integrations.json";

/// CLI configuration for `cargo xtask release artifact-check`.
pub struct Config {
    /// Directory holding the release archives and consolidated `SHA256SUMS`.
    pub dist: PathBuf,
    /// Optional override for the contract JSON (defaults to the in-repo file).
    pub contract: Option<PathBuf>,
    /// If set, every archive name must contain this version string.
    pub version: Option<String>,
    /// Permit a `dist/` that does not cover every contract target triple.
    pub allow_partial: bool,
}

#[derive(Debug, Deserialize)]
struct Contract {
    archive_name_pattern: String,
    consolidated_checksums_file: String,
    platforms: BTreeMap<String, PlatformSpec>,
    targets: Vec<TargetSpec>,
}

#[derive(Debug, Deserialize)]
struct PlatformSpec {
    required_binaries: Vec<String>,
    ext: String,
    require_executable_bit: bool,
}

#[derive(Debug, Deserialize)]
struct TargetSpec {
    triple: String,
    platform: String,
}

/// A single file entry inside an archive (directories are filtered out).
#[derive(Debug, Clone)]
struct ArchiveEntry {
    /// Final path component, e.g. `perl-dap` from `perllsp-1.2.3-.../perl-dap`.
    base_name: String,
    /// Unix permission bits (0 when the archive does not record them, e.g. zip
    /// produced on Windows).
    mode: u32,
}

#[derive(Debug, PartialEq, Eq)]
struct Violation {
    location: String,
    message: String,
}

pub fn run(cfg: Config) -> Result<()> {
    let root = project_root()?;
    let contract_path = cfg.contract.clone().unwrap_or_else(|| root.join(DEFAULT_CONTRACT_REL));
    let contract = load_contract(&contract_path)?;

    let violations =
        validate_dist(&cfg.dist, &contract, cfg.version.as_deref(), cfg.allow_partial)?;

    if violations.is_empty() {
        println!(
            "Release artifact check passed: every archive in {} ships the required binaries.",
            cfg.dist.display()
        );
        return Ok(());
    }

    eprintln!("RELEASE ARTIFACT VIOLATIONS:");
    eprintln!("{}", "=".repeat(60));
    for v in &violations {
        eprintln!("  {}: {}", v.location, v.message);
    }
    eprintln!("{}", "=".repeat(60));
    bail!("release artifact check failed with {} violation(s)", violations.len())
}

fn load_contract(path: &Path) -> Result<Contract> {
    let text = fs::read_to_string(path)
        .with_context(|| format!("reading DAP integration contract {}", path.display()))?;
    let contract: Contract = serde_json::from_str(&text)
        .with_context(|| format!("parsing DAP integration contract {}", path.display()))?;
    Ok(contract)
}

/// Validate every archive in `dist` against `contract`. Returns the list of
/// violations (empty == pass). Returns `Err` only when validation cannot run
/// at all (missing `dist`, unreadable archive).
fn validate_dist(
    dist: &Path,
    contract: &Contract,
    expected_version: Option<&str>,
    allow_partial: bool,
) -> Result<Vec<Violation>> {
    if !dist.is_dir() {
        bail!("dist directory does not exist: {}", dist.display());
    }

    let mut violations = Vec::new();

    // Known extensions, longest first, so `.tar.gz` wins over a hypothetical `.gz`.
    let mut exts: Vec<&str> = contract.platforms.values().map(|p| p.ext.as_str()).collect();
    exts.sort_by_key(|e| std::cmp::Reverse(e.len()));
    exts.dedup();

    let triples: Vec<&str> = contract.targets.iter().map(|t| t.triple.as_str()).collect();
    // Single source of truth: the archive prefix comes from the contract's
    // declared `archive_name_pattern`, not a hardcoded literal.
    let prefix = archive_name_prefix(&contract.archive_name_pattern);

    // Collect the archives present in dist, tracked by triple.
    let mut seen_triples: BTreeSet<String> = BTreeSet::new();
    let mut archive_basenames: Vec<String> = Vec::new();

    let entries =
        fs::read_dir(dist).with_context(|| format!("reading dist directory {}", dist.display()))?;
    for entry in entries {
        let entry = entry.with_context(|| format!("reading entry in {}", dist.display()))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(file_name) = path.file_name().and_then(|n| n.to_str()) else {
            continue;
        };
        let Some((triple, ext, version)) = parse_archive_name(file_name, prefix, &triples, &exts)
        else {
            // A file that looks like one of our release archives (the contract's
            // declared prefix + a known archive extension) but does not map to a
            // declared target triple is a contract gap: a new release target was
            // added to release.yml without updating the contract. Fail closed
            // rather than letting an unvalidated, un-checksummed archive ship.
            if file_name.starts_with(prefix) && exts.iter().any(|e| file_name.ends_with(e)) {
                violations.push(Violation {
                    location: file_name.to_string(),
                    message:
                        "archive name does not match any target triple declared in the contract"
                            .to_string(),
                });
            }
            continue;
        };
        archive_basenames.push(file_name.to_string());

        if let Some(expected) = expected_version
            && version != expected
        {
            violations.push(Violation {
                location: file_name.to_string(),
                message: format!(
                    "archive version `{version}` does not match expected release version `{expected}`"
                ),
            });
        }

        let Some(platform) = platform_for_triple(contract, &triple) else {
            violations.push(Violation {
                location: file_name.to_string(),
                message: format!("triple `{triple}` is not declared in the contract"),
            });
            continue;
        };
        let Some(spec) = contract.platforms.get(platform) else {
            violations.push(Violation {
                location: file_name.to_string(),
                message: format!("platform `{platform}` is not declared in the contract"),
            });
            continue;
        };

        if ext != spec.ext {
            violations.push(Violation {
                location: file_name.to_string(),
                message: format!(
                    "archive extension `{ext}` does not match expected `{}` for {platform}",
                    spec.ext
                ),
            });
        }

        let archive_entries = read_archive_entries(&path, &ext)
            .with_context(|| format!("reading archive {}", path.display()))?;
        check_required_binaries(file_name, &archive_entries, spec, &mut violations);

        seen_triples.insert(triple);
    }

    // Completeness: every contract target should have an archive.
    if !allow_partial {
        for target in &contract.targets {
            if !seen_triples.contains(&target.triple) {
                let pattern = contract
                    .archive_name_pattern
                    .replace("{version}", expected_version.unwrap_or("<version>"))
                    .replace("{triple}", &target.triple)
                    .replace(
                        "{ext}",
                        contract
                            .platforms
                            .get(&target.platform)
                            .map(|p| p.ext.as_str())
                            .unwrap_or(""),
                    );
                violations.push(Violation {
                    location: dist.display().to_string(),
                    message: format!(
                        "no archive found for target `{}` (expected `{pattern}`)",
                        target.triple
                    ),
                });
            }
        }
    }

    if archive_basenames.is_empty() {
        violations.push(Violation {
            location: dist.display().to_string(),
            message: "no release archives found (expected perllsp-<version>-<triple>.{tar.gz,zip})"
                .to_string(),
        });
        return Ok(violations);
    }

    // Consolidated checksums: every archive must be listed (and, when the
    // digest is present, it must match the file on disk).
    check_consolidated_checksums(dist, contract, &archive_basenames, &mut violations)?;

    Ok(violations)
}

/// Parse `perllsp-<version>-<triple><ext>` into `(triple, ext, version)`.
/// Returns `None` for files that are not release archives.
/// The literal prefix of `archive_name_pattern` — everything before the first
/// `{...}` placeholder (e.g. `perllsp-` from `perllsp-{version}-{triple}{ext}`).
/// Derived from the contract so the validator stays single-source-of-truth.
fn archive_name_prefix(pattern: &str) -> &str {
    match pattern.find('{') {
        Some(idx) => &pattern[..idx],
        None => pattern,
    }
}

fn parse_archive_name(
    file_name: &str,
    prefix: &str,
    triples: &[&str],
    exts: &[&str],
) -> Option<(String, String, String)> {
    let ext = exts.iter().copied().find(|e| file_name.ends_with(*e))?;
    let core = file_name.strip_suffix(ext)?;
    let rest = core.strip_prefix(prefix)?; // "<version>-<triple>"

    // Triples contain hyphens, so match the longest known triple suffix that is
    // preceded by a `-`.
    let mut best: Option<&str> = None;
    for triple in triples {
        if let Some(version_part) = rest.strip_suffix(triple)
            && version_part.ends_with('-')
            && best.is_none_or(|b| triple.len() > b.len())
        {
            best = Some(triple);
        }
    }
    let triple = best?;
    let version = rest.strip_suffix(triple)?.strip_suffix('-')?;
    if version.is_empty() {
        return None;
    }
    Some((triple.to_string(), ext.to_string(), version.to_string()))
}

fn platform_for_triple<'a>(contract: &'a Contract, triple: &str) -> Option<&'a str> {
    contract.targets.iter().find(|t| t.triple == triple).map(|t| t.platform.as_str())
}

fn check_required_binaries(
    location: &str,
    entries: &[ArchiveEntry],
    spec: &PlatformSpec,
    violations: &mut Vec<Violation>,
) {
    for required in &spec.required_binaries {
        let matches: Vec<&ArchiveEntry> =
            entries.iter().filter(|e| &e.base_name == required).collect();
        if matches.is_empty() {
            violations.push(Violation {
                location: location.to_string(),
                message: format!("missing required binary `{required}`"),
            });
            continue;
        }
        if spec.require_executable_bit && !matches.iter().any(|e| e.mode & 0o111 != 0) {
            violations.push(Violation {
                location: location.to_string(),
                message: format!("binary `{required}` is present but not marked executable"),
            });
        }
    }
}

fn check_consolidated_checksums(
    dist: &Path,
    contract: &Contract,
    archive_basenames: &[String],
    violations: &mut Vec<Violation>,
) -> Result<()> {
    let checksums_path = dist.join(&contract.consolidated_checksums_file);
    if !checksums_path.is_file() {
        violations.push(Violation {
            location: dist.display().to_string(),
            message: format!("consolidated `{}` is missing", contract.consolidated_checksums_file),
        });
        return Ok(());
    }

    let text = fs::read_to_string(&checksums_path)
        .with_context(|| format!("reading {}", checksums_path.display()))?;
    let listed = parse_checksums(&text);

    for name in archive_basenames {
        match listed.get(name) {
            None => violations.push(Violation {
                location: contract.consolidated_checksums_file.clone(),
                message: format!("archive `{name}` is not listed in the consolidated checksums"),
            }),
            Some(expected_digest) => {
                let actual =
                    sha256_hex(&dist.join(name)).with_context(|| format!("hashing {name}"))?;
                if !expected_digest.eq_ignore_ascii_case(&actual) {
                    violations.push(Violation {
                        location: contract.consolidated_checksums_file.clone(),
                        message: format!(
                            "checksum mismatch for `{name}` (listed {expected_digest}, actual {actual})"
                        ),
                    });
                }
            }
        }
    }
    Ok(())
}

/// Parse `sha256sum`-style lines (`<hex>  <name>`) into a name -> digest map.
fn parse_checksums(text: &str) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    for line in text.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let mut parts = line.splitn(2, char::is_whitespace);
        let Some(digest) = parts.next() else { continue };
        let Some(name) = parts.next() else { continue };
        // Drop the binary-mode `*` marker and any leading `./`.
        let name = name.trim().trim_start_matches('*').trim_start_matches("./");
        map.insert(name.to_string(), digest.to_string());
    }
    map
}

fn sha256_hex(path: &Path) -> Result<String> {
    let mut file =
        File::open(path).with_context(|| format!("opening {} for hashing", path.display()))?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 64 * 1024];
    loop {
        let n = file.read(&mut buf).with_context(|| format!("reading {}", path.display()))?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(hex_lower(&hasher.finalize()))
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        s.push(char::from_digit(u32::from(b >> 4), 16).unwrap_or('0'));
        s.push(char::from_digit(u32::from(b & 0x0f), 16).unwrap_or('0'));
    }
    s
}

fn read_archive_entries(path: &Path, ext: &str) -> Result<Vec<ArchiveEntry>> {
    if ext.ends_with(".zip") { read_zip_entries(path) } else { read_tar_gz_entries(path) }
}

fn read_tar_gz_entries(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let file = File::open(path).with_context(|| format!("opening {}", path.display()))?;
    let decoder = flate2::read::GzDecoder::new(file);
    let mut archive = tar::Archive::new(decoder);
    let mut out = Vec::new();
    for entry in archive.entries().context("iterating tar entries")? {
        let entry = entry.context("reading tar entry")?;
        if !entry.header().entry_type().is_file() {
            continue;
        }
        let mode = entry.header().mode().unwrap_or(0);
        let path_in_tar = entry.path().context("decoding tar entry path")?;
        if let Some(base) = path_in_tar.file_name().and_then(|n| n.to_str()) {
            out.push(ArchiveEntry { base_name: base.to_string(), mode });
        }
    }
    Ok(out)
}

/// Read a ZIP central directory and return its file entries. Only the central
/// directory is parsed (entry names + external attributes); file data is never
/// decompressed. ZIP64 archives are not supported (release archives are small).
fn read_zip_entries(path: &Path) -> Result<Vec<ArchiveEntry>> {
    let data = fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    const EOCD_SIG: [u8; 4] = [0x50, 0x4b, 0x05, 0x06];
    const CD_SIG: [u8; 4] = [0x50, 0x4b, 0x01, 0x02];

    // Locate the End Of Central Directory record by scanning backwards.
    if data.len() < 22 {
        bail!("file too small to be a zip archive: {}", path.display());
    }
    let max_back = data.len().saturating_sub(22);
    let mut eocd: Option<usize> = None;
    for i in (0..=max_back).rev() {
        if data.get(i..i + 4) == Some(&EOCD_SIG[..]) {
            eocd = Some(i);
            break;
        }
    }
    let Some(eocd) = eocd else {
        bail!("zip End Of Central Directory record not found: {}", path.display());
    };

    let total_entries = read_u16(&data, eocd + 10)?;
    let cd_offset = read_u32(&data, eocd + 16)? as usize;

    let mut out = Vec::new();
    let mut pos = cd_offset;
    for _ in 0..total_entries {
        // All offsets use checked arithmetic and `.get()` slicing so a malformed
        // or hostile central directory can never overflow `usize` (even on 32-bit
        // targets) into an out-of-bounds slice — it breaks the loop instead of
        // panicking. read_u16/read_u32 are likewise `.get()`-based.
        let Some(header_end) = pos.checked_add(46) else {
            break;
        };
        if data.get(pos..pos + 4) != Some(&CD_SIG[..]) {
            break;
        }
        let external_attrs = read_u32(&data, pos + 38)?;
        let name_len = read_u16(&data, pos + 28)? as usize;
        let extra_len = read_u16(&data, pos + 30)? as usize;
        let comment_len = read_u16(&data, pos + 32)? as usize;
        let name_start = header_end;
        let Some(name_end) = name_start.checked_add(name_len) else {
            break;
        };
        let Some(raw_name) = data.get(name_start..name_end) else {
            break;
        };
        let normalized = String::from_utf8_lossy(raw_name).replace('\\', "/");
        // Directory entries end in `/`.
        if !normalized.ends_with('/') {
            let base = normalized.rsplit('/').next().unwrap_or(&normalized).to_string();
            let mode = (external_attrs >> 16) & 0xffff;
            out.push(ArchiveEntry { base_name: base, mode });
        }
        let Some(next_pos) =
            name_end.checked_add(extra_len).and_then(|n| n.checked_add(comment_len))
        else {
            break;
        };
        pos = next_pos;
    }
    Ok(out)
}

fn read_u16(data: &[u8], at: usize) -> Result<u16> {
    match at.checked_add(2).and_then(|end| data.get(at..end)) {
        Some([a, b]) => Ok(u16::from_le_bytes([*a, *b])),
        _ => bail!("zip: truncated u16 field at offset {at}"),
    }
}

fn read_u32(data: &[u8], at: usize) -> Result<u32> {
    match at.checked_add(4).and_then(|end| data.get(at..end)) {
        Some([a, b, c, d]) => Ok(u32::from_le_bytes([*a, *b, *c, *d])),
        _ => bail!("zip: truncated u32 field at offset {at}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_contract() -> Result<Contract> {
        let json = r#"{
            "archive_name_pattern": "perllsp-{version}-{triple}{ext}",
            "consolidated_checksums_file": "SHA256SUMS",
            "platforms": {
                "unix": { "required_binaries": ["perllsp", "perl-dap"], "ext": ".tar.gz", "require_executable_bit": true },
                "windows": { "required_binaries": ["perllsp.exe", "perl-dap.exe"], "ext": ".zip", "require_executable_bit": false }
            },
            "targets": [
                { "triple": "x86_64-unknown-linux-gnu", "platform": "unix" },
                { "triple": "x86_64-pc-windows-msvc", "platform": "windows" }
            ]
        }"#;
        Ok(serde_json::from_str(json)?)
    }

    fn fixtures_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/release-artifacts")
    }

    // The integration-only code paths (validate_dist, the archive readers,
    // checksum verification, run, load_contract) are also exercised here as
    // in-process unit tests so they are captured by the `--bin xtask` coverage
    // run (the CLI integration tests run as a separate binary that the coverage
    // command does not instrument).

    #[test]
    fn load_contract_reads_repo_contract() -> Result<()> {
        let root = project_root()?;
        let contract = load_contract(&root.join(DEFAULT_CONTRACT_REL))?;
        assert!(!contract.targets.is_empty());
        assert!(contract.platforms.contains_key("unix"));
        Ok(())
    }

    #[test]
    fn read_tar_gz_entries_lists_binaries_with_exec_bit() -> Result<()> {
        let tar = fixtures_root().join("good/perllsp-9.9.9-x86_64-unknown-linux-gnu.tar.gz");
        let entries = read_archive_entries(&tar, ".tar.gz")?;
        assert!(entries.iter().any(|e| e.base_name == "perllsp"));
        let dap = entries.iter().find(|e| e.base_name == "perl-dap");
        assert!(dap.is_some_and(|e| e.mode & 0o111 != 0), "perl-dap should carry the exec bit");
        Ok(())
    }

    #[test]
    fn read_zip_entries_lists_windows_binaries() -> Result<()> {
        let zip = fixtures_root().join("good/perllsp-9.9.9-x86_64-pc-windows-msvc.zip");
        let entries = read_archive_entries(&zip, ".zip")?;
        assert!(entries.iter().any(|e| e.base_name == "perllsp.exe"));
        assert!(entries.iter().any(|e| e.base_name == "perl-dap.exe"));
        Ok(())
    }

    #[test]
    fn validate_dist_passes_on_good_fixture() -> Result<()> {
        let contract = test_contract()?;
        let violations =
            validate_dist(&fixtures_root().join("good"), &contract, Some("9.9.9"), true)?;
        assert!(violations.is_empty(), "good fixture should be clean: {violations:?}");
        Ok(())
    }

    #[test]
    fn validate_dist_flags_missing_required_binary() -> Result<()> {
        let contract = test_contract()?;
        let violations =
            validate_dist(&fixtures_root().join("bad-missing-dap"), &contract, None, true)?;
        assert!(violations.iter().any(|v| v.message.contains("perl-dap")));
        Ok(())
    }

    #[test]
    fn validate_dist_flags_checksum_mismatch() -> Result<()> {
        let contract = test_contract()?;
        let violations =
            validate_dist(&fixtures_root().join("bad-checksum"), &contract, None, true)?;
        assert!(violations.iter().any(|v| v.message.contains("checksum mismatch")));
        Ok(())
    }

    #[test]
    fn validate_dist_flags_version_mismatch() -> Result<()> {
        let contract = test_contract()?;
        let violations =
            validate_dist(&fixtures_root().join("good"), &contract, Some("0.0.0"), true)?;
        assert!(violations.iter().any(|v| v.message.contains("does not match expected")));
        Ok(())
    }

    #[test]
    fn validate_dist_requires_all_targets_without_allow_partial() -> Result<()> {
        // The good fixture covers only 2 of the contract's targets; with
        // completeness enforced (allow_partial = false) the missing ones fail.
        let contract = test_contract()?;
        let violations =
            validate_dist(&fixtures_root().join("good"), &contract, Some("9.9.9"), true)?;
        assert!(violations.is_empty());
        // bad-checksum has only the linux archive, so windows-msvc is missing.
        let strict = validate_dist(&fixtures_root().join("bad-checksum"), &contract, None, false)?;
        assert!(strict.iter().any(|v| v.message.contains("no archive found for target")));
        Ok(())
    }

    #[test]
    fn validate_dist_flags_undeclared_archive() -> Result<()> {
        // A contract that knows the .zip extension (so the file "looks like" a
        // release archive) but does not declare the windows triple must flag the
        // good fixture's windows zip as undeclared.
        let json = r#"{
            "archive_name_pattern": "perllsp-{version}-{triple}{ext}",
            "consolidated_checksums_file": "SHA256SUMS",
            "platforms": {
                "unix": { "required_binaries": ["perllsp", "perl-dap"], "ext": ".tar.gz", "require_executable_bit": true },
                "windows": { "required_binaries": ["perllsp.exe", "perl-dap.exe"], "ext": ".zip", "require_executable_bit": false }
            },
            "targets": [
                { "triple": "x86_64-unknown-linux-gnu", "platform": "unix" }
            ]
        }"#;
        let contract: Contract = serde_json::from_str(json)?;
        let violations = validate_dist(&fixtures_root().join("good"), &contract, None, true)?;
        assert!(
            violations.iter().any(|v| v.message.contains("does not match any target triple")),
            "undeclared windows zip should be flagged: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn run_succeeds_on_good_fixture() -> Result<()> {
        // `contract: None` resolves the in-repo contract via project_root().
        run(Config {
            dist: fixtures_root().join("good"),
            contract: None,
            version: None,
            allow_partial: true,
        })
    }

    #[test]
    fn run_fails_on_missing_dap() {
        let result = run(Config {
            dist: fixtures_root().join("bad-missing-dap"),
            contract: None,
            version: None,
            allow_partial: true,
        });
        assert!(result.is_err(), "missing perl-dap must make `run` bail");
    }

    #[test]
    fn run_errors_on_missing_dist() {
        let result = run(Config {
            dist: fixtures_root().join("does-not-exist"),
            contract: None,
            version: None,
            allow_partial: true,
        });
        assert!(result.is_err(), "a nonexistent dist must error");
    }

    // --- Defensive / error-branch coverage (tempdir-constructed dists) ---

    fn good_linux_tar() -> PathBuf {
        fixtures_root().join("good/perllsp-9.9.9-x86_64-unknown-linux-gnu.tar.gz")
    }

    fn good_windows_zip() -> PathBuf {
        fixtures_root().join("good/perllsp-9.9.9-x86_64-pc-windows-msvc.zip")
    }

    #[test]
    fn read_zip_entries_rejects_non_zip() {
        // A gzip stream has no ZIP End-Of-Central-Directory record.
        assert!(read_archive_entries(&good_linux_tar(), ".zip").is_err());
    }

    #[test]
    fn read_zip_entries_rejects_tiny_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let p = dir.path().join("tiny.zip");
        fs::write(&p, b"PK")?;
        assert!(read_archive_entries(&p, ".zip").is_err());
        Ok(())
    }

    #[test]
    fn read_tar_gz_entries_rejects_non_gzip() -> Result<()> {
        let dir = tempfile::tempdir()?;
        let p = dir.path().join("bad.tar.gz");
        fs::write(&p, b"this is not a gzip stream")?;
        assert!(read_archive_entries(&p, ".tar.gz").is_err());
        Ok(())
    }

    #[test]
    fn validate_dist_flags_missing_checksums_file() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::copy(
            good_linux_tar(),
            dir.path().join("perllsp-9.9.9-x86_64-unknown-linux-gnu.tar.gz"),
        )?;
        let contract = test_contract()?;
        let violations = validate_dist(dir.path(), &contract, None, true)?;
        assert!(violations.iter().any(|v| v.message.contains("is missing")));
        Ok(())
    }

    #[test]
    fn validate_dist_flags_archive_not_in_checksums() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::copy(
            good_linux_tar(),
            dir.path().join("perllsp-9.9.9-x86_64-unknown-linux-gnu.tar.gz"),
        )?;
        fs::write(dir.path().join("SHA256SUMS"), "deadbeef  some-other-file.tar.gz\n")?;
        let contract = test_contract()?;
        let violations = validate_dist(dir.path(), &contract, None, true)?;
        assert!(violations.iter().any(|v| v.message.contains("not listed")));
        Ok(())
    }

    #[test]
    fn validate_dist_flags_no_archives() -> Result<()> {
        let dir = tempfile::tempdir()?;
        fs::write(dir.path().join("README.txt"), b"nothing here")?;
        let contract = test_contract()?;
        let violations = validate_dist(dir.path(), &contract, None, true)?;
        assert!(violations.iter().any(|v| v.message.contains("no release archives found")));
        Ok(())
    }

    #[test]
    fn validate_dist_flags_extension_mismatch() -> Result<()> {
        // A .zip named for the unix (linux-gnu) triple: the name parses (known
        // triple + known .zip ext) but the unix platform declares .tar.gz.
        let dir = tempfile::tempdir()?;
        fs::copy(
            good_windows_zip(),
            dir.path().join("perllsp-9.9.9-x86_64-unknown-linux-gnu.zip"),
        )?;
        let contract = test_contract()?;
        let violations = validate_dist(dir.path(), &contract, None, true)?;
        assert!(violations.iter().any(|v| v.message.contains("extension")));
        Ok(())
    }

    #[test]
    fn validate_dist_flags_target_platform_without_spec() -> Result<()> {
        // A target referencing a platform that is not in `platforms` exercises
        // the defensive "platform is not declared" branch.
        let json = r#"{
            "archive_name_pattern": "perllsp-{version}-{triple}{ext}",
            "consolidated_checksums_file": "SHA256SUMS",
            "platforms": {
                "unix": { "required_binaries": ["perllsp"], "ext": ".tar.gz", "require_executable_bit": false }
            },
            "targets": [
                { "triple": "x86_64-unknown-linux-gnu", "platform": "ghost" }
            ]
        }"#;
        let contract: Contract = serde_json::from_str(json)?;
        let violations = validate_dist(&fixtures_root().join("good"), &contract, None, true)?;
        assert!(
            violations.iter().any(|v| v.message.contains("platform `ghost` is not declared")),
            "expected platform-not-declared violation: {violations:?}"
        );
        Ok(())
    }

    #[test]
    fn archive_name_prefix_strips_at_first_placeholder() {
        assert_eq!(archive_name_prefix("perllsp-{version}-{triple}{ext}"), "perllsp-");
        assert_eq!(archive_name_prefix("foo-bar-{version}{ext}"), "foo-bar-");
        assert_eq!(archive_name_prefix("noplaceholders"), "noplaceholders");
    }

    #[test]
    fn parses_unix_archive_name() {
        let triples = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"];
        let exts = [".tar.gz", ".zip"];
        let parsed = parse_archive_name(
            "perllsp-0.16.0-x86_64-unknown-linux-gnu.tar.gz",
            "perllsp-",
            &triples,
            &exts,
        );
        assert_eq!(
            parsed,
            Some((
                "x86_64-unknown-linux-gnu".to_string(),
                ".tar.gz".to_string(),
                "0.16.0".to_string()
            ))
        );
    }

    #[test]
    fn parses_windows_zip_name() {
        let triples = ["x86_64-unknown-linux-gnu", "x86_64-pc-windows-msvc"];
        let exts = [".tar.gz", ".zip"];
        let parsed = parse_archive_name(
            "perllsp-1.2.3-x86_64-pc-windows-msvc.zip",
            "perllsp-",
            &triples,
            &exts,
        );
        assert_eq!(
            parsed,
            Some(("x86_64-pc-windows-msvc".to_string(), ".zip".to_string(), "1.2.3".to_string()))
        );
    }

    #[test]
    fn ignores_non_archive_files() {
        let triples = ["x86_64-unknown-linux-gnu"];
        let exts = [".tar.gz", ".zip"];
        assert_eq!(parse_archive_name("SHA256SUMS", "perllsp-", &triples, &exts), None);
        assert_eq!(parse_archive_name("README.md", "perllsp-", &triples, &exts), None);
        assert_eq!(
            parse_archive_name("perllsp-0.1.0-unknown-triple.tar.gz", "perllsp-", &triples, &exts),
            None
        );
    }

    #[test]
    fn missing_dap_binary_is_a_violation() {
        let spec = PlatformSpec {
            required_binaries: vec!["perllsp".to_string(), "perl-dap".to_string()],
            ext: ".tar.gz".to_string(),
            require_executable_bit: true,
        };
        let entries = vec![ArchiveEntry { base_name: "perllsp".to_string(), mode: 0o755 }];
        let mut violations = Vec::new();
        check_required_binaries("ok.tar.gz", &entries, &spec, &mut violations);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("perl-dap"));
    }

    #[test]
    fn non_executable_unix_binary_is_a_violation() {
        let spec = PlatformSpec {
            required_binaries: vec!["perl-dap".to_string()],
            ext: ".tar.gz".to_string(),
            require_executable_bit: true,
        };
        let entries = vec![ArchiveEntry { base_name: "perl-dap".to_string(), mode: 0o644 }];
        let mut violations = Vec::new();
        check_required_binaries("ok.tar.gz", &entries, &spec, &mut violations);
        assert_eq!(violations.len(), 1);
        assert!(violations[0].message.contains("not marked executable"));
    }

    #[test]
    fn windows_binary_does_not_require_exec_bit() {
        let spec = PlatformSpec {
            required_binaries: vec!["perl-dap.exe".to_string()],
            ext: ".zip".to_string(),
            require_executable_bit: false,
        };
        let entries = vec![ArchiveEntry { base_name: "perl-dap.exe".to_string(), mode: 0 }];
        let mut violations = Vec::new();
        check_required_binaries("ok.zip", &entries, &spec, &mut violations);
        assert!(violations.is_empty());
    }

    #[test]
    fn parse_checksums_handles_modes_and_prefixes() {
        let text = "abc123  perllsp-1.0.0-x86_64-unknown-linux-gnu.tar.gz\n\
                    def456 *perllsp-1.0.0-x86_64-pc-windows-msvc.zip\n";
        let map = parse_checksums(text);
        assert_eq!(
            map.get("perllsp-1.0.0-x86_64-unknown-linux-gnu.tar.gz"),
            Some(&"abc123".to_string())
        );
        assert_eq!(
            map.get("perllsp-1.0.0-x86_64-pc-windows-msvc.zip"),
            Some(&"def456".to_string())
        );
    }

    #[test]
    fn contract_platforms_present() -> Result<()> {
        let contract = test_contract()?;
        assert!(contract.platforms.contains_key("unix"));
        assert!(contract.platforms.contains_key("windows"));
        assert_eq!(platform_for_triple(&contract, "x86_64-pc-windows-msvc"), Some("windows"));
        Ok(())
    }
}
