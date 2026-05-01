//! Generate Homebrew formula for release artifacts using SHA256SUMS.

use color_eyre::eyre::{Context, Result, eyre};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::utils::project_root;

pub struct UpdateHomebrewConfig {
    pub version: String,
    pub owner: String,
    pub repo: String,
    pub prefix: String,
    pub output: PathBuf,
}

const MAC_ARM: &str = "aarch64-apple-darwin.tar.gz";
const MAC_X64: &str = "x86_64-apple-darwin.tar.gz";
const LIN_ARM: &str = "aarch64-unknown-linux-gnu.tar.gz";
const LIN_X64: &str = "x86_64-unknown-linux-gnu.tar.gz";

pub fn run(config: UpdateHomebrewConfig) -> Result<()> {
    let release_version = strip_version_prefix(config.version.trim());
    let release_tag = config.version.trim().to_string();
    let sums_url = format!(
        "https://github.com/{}/{}/releases/download/{}/SHA256SUMS",
        config.owner, config.repo, release_tag
    );

    let raw_sums = download_sha256sums(&sums_url)?;
    let checksums = parse_sha256sums(&raw_sums)?;

    let mac_sha_arm = checksum_for(&config, MAC_ARM, &checksums, &release_version)?;
    let mac_sha_x64 = checksum_for(&config, MAC_X64, &checksums, &release_version)?;
    let linux_sha_arm = checksum_for(&config, LIN_ARM, &checksums, &release_version)?;
    let linux_sha_x64 = checksum_for(&config, LIN_X64, &checksums, &release_version)?;

    let formula = build_brew_formula(
        &config,
        &release_tag,
        &release_version,
        &Checksums {
            mac_arm: &mac_sha_arm,
            mac_x64: &mac_sha_x64,
            linux_arm: &linux_sha_arm,
            linux_x64: &linux_sha_x64,
        },
    );
    let output = resolve_output_path(config.output)?;
    write_formula(&output, &formula)?;

    println!("✅ Homebrew formula updated for version {release_version}");
    println!("Users can install with: brew install EffortlessMetrics/tap/perllsp");
    Ok(())
}

struct Checksums<'a> {
    mac_arm: &'a str,
    mac_x64: &'a str,
    linux_arm: &'a str,
    linux_x64: &'a str,
}

fn strip_version_prefix(version: &str) -> String {
    version.trim_start_matches('v').to_string()
}

fn resolve_output_path(path: PathBuf) -> Result<PathBuf> {
    if path.is_absolute() {
        return Ok(path);
    }
    let root = project_root()?;
    Ok(root.join(path))
}

fn download_sha256sums(url: &str) -> Result<String> {
    let output = Command::new("curl")
        .args(["-sSfL", url])
        .output()
        .with_context(|| format!("failed to run curl for {url}"))?;
    if !output.status.success() {
        return Err(eyre!(
            "failed to fetch SHA256SUMS from {url}: {}",
            String::from_utf8_lossy(&output.stderr)
        ));
    }
    String::from_utf8(output.stdout)
        .with_context(|| format!("response from {url} was not valid UTF-8"))
}

fn parse_sha256sums(raw: &str) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        if let (Some(hash), Some(file)) = (parts.next(), parts.next()) {
            map.insert(file.to_string(), hash.to_string());
        }
    }
    if map.is_empty() {
        return Err(eyre!("SHA256SUMS did not contain any valid checksums"));
    }
    Ok(map)
}

fn checksum_for(
    config: &UpdateHomebrewConfig,
    artifact: &str,
    checksums: &BTreeMap<String, String>,
    version: &str,
) -> Result<String> {
    let filename = format!("{}-{}-{artifact}", config.prefix, version);
    checksums.get(&filename).cloned().ok_or_else(|| eyre!("missing checksum for {filename}"))
}

fn build_brew_formula(
    config: &UpdateHomebrewConfig,
    release_tag: &str,
    version: &str,
    checksums: &Checksums<'_>,
) -> String {
    let base = format!(
        "https://github.com/{}/{}/releases/download/{release_tag}",
        config.owner, config.repo
    );
    format!(
        r##"class Perllsp < Formula
  desc "Native Rust language server and debug adapter for Perl"
  homepage "https://github.com/{owner}/{repo}"
  version "{version}"
  license any_of: ["MIT", "Apache-2.0"]

  on_macos do
    if Hardware::CPU.arm?
      url "{base}/{mac_arm}"
      sha256 "{mac_arm_sha}"
    else
      url "{base}/{mac_x64}"
      sha256 "{mac_x64_sha}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "{base}/{linux_arm}"
      sha256 "{linux_arm_sha}"
    else
      url "{base}/{linux_x64}"
      sha256 "{linux_x64_sha}"
    end
  end

  def install
    extracted_dir = Dir.glob("perllsp-*").find {{ |dir| File.directory?(dir) }}

    if extracted_dir
      bin.install "#{{extracted_dir}}/perllsp"
      bin.install "#{{extracted_dir}}/perl-dap" if File.exist?("#{{extracted_dir}}/perl-dap")
    else
      bin.install "perllsp"
      bin.install "perl-dap" if File.exist?("perl-dap")
    end
  end

  test do
    output = shell_output("#{{bin}}/perllsp --version")
    assert_match "perllsp", output
    assert_match version.to_s, output
  end
end
"##,
        owner = config.owner,
        repo = config.repo,
        version = version,
        base = base,
        mac_arm = artifact_filename(&config.prefix, version, MAC_ARM),
        mac_x64 = artifact_filename(&config.prefix, version, MAC_X64),
        linux_arm = artifact_filename(&config.prefix, version, LIN_ARM),
        linux_x64 = artifact_filename(&config.prefix, version, LIN_X64),
        mac_arm_sha = checksums.mac_arm,
        mac_x64_sha = checksums.mac_x64,
        linux_arm_sha = checksums.linux_arm,
        linux_x64_sha = checksums.linux_x64,
    )
}

fn artifact_filename(prefix: &str, version: &str, artifact: &str) -> String {
    format!("{prefix}-{version}-{artifact}")
}

fn write_formula(path: &std::path::Path, content: &str) -> Result<()> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory for {}", path.display()))?;
    }
    fs::write(path, format!("{content}\n"))
        .with_context(|| format!("failed to write Homebrew formula to {}", path.display()))?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> UpdateHomebrewConfig {
        UpdateHomebrewConfig {
            version: "v0.13.0".to_string(),
            owner: "EffortlessMetrics".to_string(),
            repo: "perl-lsp".to_string(),
            prefix: "perllsp".to_string(),
            output: PathBuf::from("Formula/perllsp.rb"),
        }
    }

    #[test]
    fn generated_formula_uses_perllsp_artifacts() {
        let formula = build_brew_formula(
            &config(),
            "v0.13.0",
            "0.13.0",
            &Checksums { mac_arm: "a", mac_x64: "b", linux_arm: "c", linux_x64: "d" },
        );
        assert!(formula.contains("perllsp-0.13.0-x86_64-apple-darwin.tar.gz"));
        assert!(formula.contains("perllsp-0.13.0-aarch64-apple-darwin.tar.gz"));
        assert!(formula.contains("perllsp-0.13.0-x86_64-unknown-linux-gnu.tar.gz"));
        assert!(formula.contains("perllsp-0.13.0-aarch64-unknown-linux-gnu.tar.gz"));
        assert!(!formula.contains("perl-lsp-0.13.0-"));
    }

    #[test]
    fn generated_formula_installs_perllsp_and_optional_perl_dap() {
        let formula = build_brew_formula(
            &config(),
            "v0.13.0",
            "0.13.0",
            &Checksums { mac_arm: "a", mac_x64: "b", linux_arm: "c", linux_x64: "d" },
        );
        assert!(formula.contains("class Perllsp < Formula"));
        assert!(formula.contains("Dir.glob(\"perllsp-*\")"));
        assert!(formula.contains("bin.install \"#{extracted_dir}/perllsp\""));
        assert!(formula.contains("bin.install \"#{extracted_dir}/perl-dap\" if File.exist?(\"#{extracted_dir}/perl-dap\")"));
        assert!(!formula.contains("__RELEASE_VERSION__"));
        assert!(!formula.contains("__SHA256_"));
    }

    #[test]
    fn default_artifact_prefix_matches_release_workflow() {
        assert_eq!(
            artifact_filename("perllsp", "0.13.0", MAC_X64),
            "perllsp-0.13.0-x86_64-apple-darwin.tar.gz"
        );
    }
}
