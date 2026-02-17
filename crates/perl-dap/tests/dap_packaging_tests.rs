//! DAP Packaging Tests (AC19)
//!
//! Repository-backed checks for build/distribution packaging surfaces.
//!
//! Run with: `cargo test -p perl-dap --features dap-phase3`

#[cfg(feature = "dap-phase3")]
mod dap_packaging {
    use anyhow::Result;
    use serde_json::Value;
    use std::path::PathBuf;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn read(path: impl AsRef<std::path::Path>) -> Result<String> {
        Ok(std::fs::read_to_string(path)?)
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac19-platform-binaries
    #[test]
    // AC:19
    fn test_platform_binary_builds() -> Result<()> {
        let cargo_toml = read(repo_root().join("crates/perl-dap/Cargo.toml"))?;
        assert!(cargo_toml.contains("[[bin]]"));
        assert!(cargo_toml.contains("name = \"perl-dap\""));
        assert!(cargo_toml.contains("path = \"src/main.rs\""));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac19-github-releases
    #[test]
    // AC:19
    fn test_github_releases_distribution() -> Result<()> {
        let release_workflow = read(repo_root().join(".github/workflows/release.yml"))?;
        assert!(release_workflow.contains("action-gh-release"));
        assert!(release_workflow.contains("matrix"));
        assert!(release_workflow.contains("target: x86_64-unknown-linux-gnu"));

        let publish_workflow = read(repo_root().join(".github/workflows/publish-crates.yml"))?;
        assert!(publish_workflow.contains("- perl-dap"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac19-binary-size
    #[test]
    // AC:19
    fn test_binary_size_optimization() -> Result<()> {
        let workspace_cargo = read(repo_root().join("Cargo.toml"))?;
        assert!(workspace_cargo.contains("[profile.release]"));
        assert!(workspace_cargo.contains("lto = true"));
        assert!(workspace_cargo.contains("strip = \"debuginfo\""));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac19-vscode-bundling
    #[test]
    // AC:19
    fn test_vscode_extension_binary_bundling() -> Result<()> {
        let package_json: Value =
            serde_json::from_str(&read(repo_root().join("vscode-extension/package.json"))?)?;
        let debuggers = package_json["contributes"]["debuggers"]
            .as_array()
            .ok_or_else(|| anyhow::anyhow!("missing contributes.debuggers"))?;
        assert!(debuggers.iter().any(|dbg| dbg["type"] == "perl"));

        let package_text = read(repo_root().join("vscode-extension/package.json"))?;
        assert!(package_text.contains("autoDownload"));
        assert!(package_text.contains("downloadBaseUrl"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac19-unix-permissions
    #[test]
    // AC:19
    fn test_binary_permissions_unix() -> Result<()> {
        let install_script = repo_root().join("install.sh");
        let script_text = read(&install_script)?;
        assert!(script_text.contains("chmod +x"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac19-cross-compilation
    #[test]
    // AC:19
    fn test_cross_compilation_ci() -> Result<()> {
        let release_workflow = read(repo_root().join(".github/workflows/release.yml"))?;
        assert!(release_workflow.contains("x86_64-unknown-linux-gnu"));
        assert!(release_workflow.contains("aarch64-unknown-linux-gnu"));
        assert!(release_workflow.contains("x86_64-apple-darwin"));
        assert!(release_workflow.contains("x86_64-pc-windows-msvc"));
        assert!(release_workflow.contains("cross build"));
        Ok(())
    }
}
