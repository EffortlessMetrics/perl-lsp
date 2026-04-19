//! Generate Homebrew formula for release artifacts using SHA256SUMS.

use color_eyre::eyre::{Context, Result, eyre};
use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;
use std::process::Command;

use crate::utils::project_root;

/// Arguments for `cargo xtask update-homebrew`.
pub struct UpdateHomebrewConfig {
    /// Release tag used by the artifact URLs (for example `v0.8.3`).
    pub version: String,
    /// GitHub organization owning the release repository.
    pub owner: String,
    /// GitHub repository name.
    pub repo: String,
    /// Artifact filename prefix.
    pub prefix: String,
    /// Output path for generated Homebrew formula.
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
    println!();
    println!("Checksums:");
    println!("  macOS ARM64:  {mac_sha_arm}");
    println!("  macOS x86_64: {mac_sha_x64}");
    println!("  Linux ARM64:  {linux_sha_arm}");
    println!("  Linux x86_64: {linux_sha_x64}");
    println!();
    println!("Next steps:");
    println!("1. Review the formula: cat {}", output.display());
    println!("2. Copy to your homebrew-tap repository");
    println!("3. Commit and push the updated formula");
    println!();
    println!("Users can then install with:");
    println!("  brew tap effortlesssteven/tap");
    println!("  brew install perl-lsp");

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
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(eyre!("failed to fetch SHA256SUMS from {url}: {stderr}"));
    }

    String::from_utf8(output.stdout)
        .with_context(|| format!("response from {url} was not valid UTF-8"))
}

fn parse_sha256sums(raw: &str) -> Result<BTreeMap<String, String>> {
    let mut map = BTreeMap::new();
    for line in raw.lines() {
        let mut parts = line.split_whitespace();
        let hash = parts.next();
        let file = parts.next();

        if let (Some(hash), Some(file)) = (hash, file) {
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
    checksums
        .get(&filename)
        .cloned()
        .ok_or_else(|| eyre!("missing checksum for {filename}"))
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
    let mac_arm_filename = artifact_filename(&config.prefix, version, MAC_ARM);
    let mac_x64_filename = artifact_filename(&config.prefix, version, MAC_X64);
    let linux_arm_filename = artifact_filename(&config.prefix, version, LIN_ARM);
    let linux_x64_filename = artifact_filename(&config.prefix, version, LIN_X64);

    format!(
        r##"class PerlLsp < Formula
  desc "Fast, reliable Perl language server with 100% syntax coverage"
  homepage "https://github.com/{owner}/{repo}"
  version "{version}"
  license "MIT"

  on_macos do
    if Hardware::CPU.arm?
      url "{base}/{mac_arm_filename}"
      sha256 "{mac_arm_sha}"
    else
      url "{base}/{mac_x64_filename}"
      sha256 "{mac_x64_sha}"
    end
  end

  on_linux do
    if Hardware::CPU.arm?
      url "{base}/{linux_arm_filename}"
      sha256 "{linux_arm_sha}"
    else
      url "{base}/{linux_x64_filename}"
      sha256 "{linux_x64_sha}"
    end
  end

  def install
    # Find the extracted directory (should be perllsp-v*).
    extracted_dir = Dir.glob("perllsp-v*").first
    if extracted_dir && File.directory?(extracted_dir)
      bin.install "#{{extracted_dir}}/perllsp"
    else
      # Fallback: binary might be in the root
      bin.install "perllsp"
    end
  end

  def caveats
    <<~EOS
      To use perl-lsp with your editor:

      VS Code:
        Install the "Perl Language Server" extension from the marketplace

      Neovim (with lspconfig):
        require('lspconfig').perl_lsp.setup{{
          cmd = {{'#{{opt_bin}}/perllsp', '--stdio'}}
        }}

      Emacs (with lsp-mode):
        (lsp-register-client
         (make-lsp-client :new-connection (lsp-stdio-connection '#{{opt_bin}}/perllsp' "--stdio")
                          :activation-fn (lsp-activate-on "perl")
                          :server-id 'perl-lsp))
    EOS
  end

  test do
    # Test that the binary runs and responds to version request
    assert_match(/perllsp|Perl LSP/, shell_output("#{{bin}}/perllsp --version"))

    # Test LSP initialization
    input = '{{"jsonrpc":"2.0","id":1,"method":"initialize","params":{{}}}}'
    output = pipe_output("#{{bin}}/perllsp --stdio", input, 0)
    assert_match(/Content-Length/, output)
  end
end
"##,
        owner = config.owner,
        repo = config.repo,
        base = base,
        version = version,
        mac_arm_filename = mac_arm_filename,
        mac_x64_filename = mac_x64_filename,
        linux_arm_filename = linux_arm_filename,
        linux_x64_filename = linux_x64_filename,
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
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)
            .with_context(|| format!("failed to create directory for {}", path.display()))?;
    }

    fs::write(path, format!("{content}\n"))
        .with_context(|| format!("failed to write Homebrew formula to {}", path.display()))?;
    println!("[homebrew] wrote {}", path.display());

    Ok(())
}
