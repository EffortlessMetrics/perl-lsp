//! Generate Homebrew formula and VS Code asset metadata from cargo-dist checksums.

use color_eyre::eyre::{Context, Result, eyre};
use serde_json::{Map, Value, json};
use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

/// Arguments for `cargo xtask inject-sha-assets`.
pub struct InjectShaAssetsConfig {
    /// Version tag used by release artifacts (for example `v0.8.3`).
    pub version: String,
    /// GitHub organization/owner.
    pub owner: String,
    /// GitHub repository name.
    pub repo: String,
    /// Release artifact filename prefix.
    pub prefix: String,
    /// Path to cargo-dist checksums JSON map.
    pub checksums: PathBuf,
    /// Optional path for generated Homebrew formula.
    pub brew_out: Option<PathBuf>,
    /// Optional path for generated VS Code extension asset map.
    pub asset_map_out: Option<PathBuf>,
}

const MAC_ARM: &str = "aarch64-apple-darwin.tar.gz";
const MAC_X64: &str = "x86_64-apple-darwin.tar.gz";
const LIN_ARM: &str = "aarch64-unknown-linux-musl.tar.gz";
const LIN_X64: &str = "x86_64-unknown-linux-musl.tar.gz";
const WIN_X64: &str = "x86_64-pc-windows-msvc.zip";
const WIN_ARM: &str = "aarch64-pc-windows-msvc.zip";

pub fn run(config: InjectShaAssetsConfig) -> Result<()> {
    let checksums = load_checksums(&config.checksums)?;

    let mac_sha_arm = checksum_for(&config, MAC_ARM, &checksums)?;
    let mac_sha_x64 = checksum_for(&config, MAC_X64, &checksums)?;
    let lin_sha_arm = checksum_for(&config, LIN_ARM, &checksums)?;
    let lin_sha_x64 = checksum_for(&config, LIN_X64, &checksums)?;
    let win_sha_x64 = checksum_for(&config, WIN_X64, &checksums)?;
    let win_sha_arm = checksum_for(&config, WIN_ARM, &checksums)?;

    let brew_formula = build_brew_formula(
        &config,
        &AssetShaMap {
            mac_arm: &mac_sha_arm,
            mac_x64: &mac_sha_x64,
            lin_arm: &lin_sha_arm,
            lin_x64: &lin_sha_x64,
            win_x64: &win_sha_x64,
            win_arm: &win_sha_arm,
        },
    );
    let asset_map = build_asset_map(
        &config,
        &AssetShaMap {
            mac_arm: &mac_sha_arm,
            mac_x64: &mac_sha_x64,
            lin_arm: &lin_sha_arm,
            lin_x64: &lin_sha_x64,
            win_x64: &win_sha_x64,
            win_arm: &win_sha_arm,
        },
    )?;

    write_output(config.brew_out.as_deref(), &brew_formula, "brew formula")?;
    write_output(config.asset_map_out.as_deref(), &asset_map, "asset map")?;

    Ok(())
}

fn load_checksums(path: &Path) -> Result<BTreeMap<String, String>> {
    let raw = fs::read_to_string(path)
        .with_context(|| format!("failed to read checksums file {}", path.display()))?;
    serde_json::from_str(&raw)
        .with_context(|| format!("expected JSON map in checksums file {}", path.display()))
}

fn checksum_for<'a>(
    config: &InjectShaAssetsConfig,
    artifact_suffix: &'a str,
    checksums: &'a BTreeMap<String, String>,
) -> Result<String> {
    let filename = format!("{}-{}-{}", config.prefix, config.version, artifact_suffix);
    checksums
        .get(&filename)
        .cloned()
        .ok_or_else(|| eyre!("missing checksum for {}", filename))
}

struct AssetShaMap<'a> {
    mac_arm: &'a str,
    mac_x64: &'a str,
    lin_arm: &'a str,
    lin_x64: &'a str,
    win_x64: &'a str,
    win_arm: &'a str,
}

fn build_brew_formula(config: &InjectShaAssetsConfig, assets: &AssetShaMap<'_>) -> String {
    let base = release_base_url(config);

    let mac_arm_filename = artifact_filename(&config.prefix, &config.version, MAC_ARM);
    let mac_x64_filename = artifact_filename(&config.prefix, &config.version, MAC_X64);
    let lin_arm_filename = artifact_filename(&config.prefix, &config.version, LIN_ARM);
    let lin_x64_filename = artifact_filename(&config.prefix, &config.version, LIN_X64);

    format!(
        r##"class PerlLsp < Formula
  desc "Perl language server"
  homepage "https://github.com/{owner}/{repo}"
  version "{version}"
  license "MIT"

  on_macos do
    on_arm do
      url "{base}/{mac_arm_filename}"
      sha256 "{mac_arm_sha}"
    end
    on_intel do
      url "{base}/{mac_x64_filename}"
      sha256 "{mac_x64_sha}"
    end
  end

  on_linux do
    on_arm do
      url "{base}/{lin_arm_filename}"
      sha256 "{lin_arm_sha}"
    end
    on_intel do
      url "{base}/{lin_x64_filename}"
      sha256 "{lin_x64_sha}"
    end
  end

  def install
    bin.install "perllsp"
  end

  test do
    assert_match "perllsp", shell_output("#{{bin}}/perllsp --version")
  end
end
"##,
        owner = config.owner,
        repo = config.repo,
        version = config.version,
        base = base,
        mac_arm_filename = mac_arm_filename,
        mac_x64_filename = mac_x64_filename,
        lin_arm_filename = lin_arm_filename,
        lin_x64_filename = lin_x64_filename,
        mac_arm_sha = assets.mac_arm,
        mac_x64_sha = assets.mac_x64,
        lin_arm_sha = assets.lin_arm,
        lin_x64_sha = assets.lin_x64,
    )
}

fn build_asset_map(config: &InjectShaAssetsConfig, assets: &AssetShaMap<'_>) -> Result<String> {
    let base = release_base_url(config);
    let linux_x64 = artifact_filename(&config.prefix, &config.version, LIN_X64);
    let linux_arm = artifact_filename(&config.prefix, &config.version, LIN_ARM);
    let mac_x64 = artifact_filename(&config.prefix, &config.version, MAC_X64);
    let mac_arm = artifact_filename(&config.prefix, &config.version, MAC_ARM);
    let win_x64 = artifact_filename(&config.prefix, &config.version, WIN_X64);
    let win_arm = artifact_filename(&config.prefix, &config.version, WIN_ARM);

    let url = |file: &str| format!("{base}/{file}");
    let mut payload = Map::new();
    payload.insert("v".to_string(), json!(&config.version));
    payload.insert(
        "linux-x64".to_string(),
        json!({ "url": url(&linux_x64), "sha256": assets.lin_x64 }),
    );
    payload.insert(
        "linux-arm64".to_string(),
        json!({ "url": url(&linux_arm), "sha256": assets.lin_arm }),
    );
    payload.insert(
        "macos-x64".to_string(),
        json!({ "url": url(&mac_x64), "sha256": assets.mac_x64 }),
    );
    payload.insert(
        "macos-arm64".to_string(),
        json!({ "url": url(&mac_arm), "sha256": assets.mac_arm }),
    );
    payload.insert(
        "win-x64".to_string(),
        json!({ "url": url(&win_x64), "sha256": assets.win_x64 }),
    );
    payload.insert(
        "win-arm64".to_string(),
        json!({ "url": url(&win_arm), "sha256": assets.win_arm }),
    );
    let payload = Value::Object(payload);
    serde_json::to_string_pretty(&payload).map_err(Into::into)
}

fn release_base_url(config: &InjectShaAssetsConfig) -> String {
    format!(
        "https://github.com/{}/{}/releases/download/{}",
        config.owner, config.repo, config.version
    )
}

fn artifact_filename(prefix: &str, version: &str, artifact_suffix: &str) -> String {
    format!("{prefix}-{version}-{artifact_suffix}")
}

fn write_output(path: Option<&Path>, content: &str, name: &str) -> Result<()> {
    let content = content.trim_end_matches('\n');
    if let Some(path) = path {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent)
                .with_context(|| format!("failed to create directory for {}", path.display()))?;
        }
        fs::write(path, format!("{content}\n"))
            .with_context(|| format!("failed to write {name} {}", path.display()))?;
        println!("[inject] wrote {}", path.display());
    } else {
        println!("{content}");
    }
    Ok(())
}
