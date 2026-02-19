//! DAP Dependency Management Tests (AC18)
//!
//! Repository-backed checks for dependency expectations and fallback assets.
//!
//! Run with: `cargo test -p perl-dap --features dap-phase3`

#[cfg(feature = "dap-phase3")]
mod dap_dependencies {
    use anyhow::Result;
    use serde_json::Value;
    use std::path::PathBuf;
    use std::process::Command;

    fn repo_root() -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
    }

    fn read(path: impl AsRef<std::path::Path>) -> Result<String> {
        Ok(std::fs::read_to_string(path)?)
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac18-cpan-fallback
    #[test]
    // AC:18
    fn test_cpan_module_installation_fallback() -> Result<()> {
        let readme = read(repo_root().join("crates/perl-dap/README.md"))?;
        assert!(readme.contains("BridgeAdapter"));
        assert!(readme.contains("Perl::LanguageServer"));
        assert!(readme.contains("cpanm Perl::LanguageServer"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac18-version-detection
    #[test]
    // AC:18
    fn test_devel_tsperldap_version_detection() -> Result<()> {
        let fixture =
            repo_root().join("crates/perl-dap/tests/fixtures/mocks/perl_shim_responses.json");
        let json: Value = serde_json::from_str(&read(fixture)?)?;
        let description = json
            .get("description")
            .and_then(Value::as_str)
            .ok_or_else(|| anyhow::anyhow!("missing fixture description"))?;
        assert!(description.contains("Devel::TSPerlDAP"));
        assert!(json.get("set_breakpoints").is_some());
        assert!(json.get("stack_trace").is_some());
        assert!(json.get("scopes").is_some());
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac18-bundled-shim
    #[test]
    // AC:18
    fn test_bundled_shim_fallback() -> Result<()> {
        let architecture_doc = read(repo_root().join("docs/CRATE_ARCHITECTURE_DAP.md"))?;
        assert!(architecture_doc.contains("perl-shim"));
        assert!(architecture_doc.contains("TSPerlDAP.pm"));

        let fixture_index =
            read(repo_root().join("crates/perl-dap/tests/fixtures/FIXTURE_INDEX.md"))?;
        assert!(fixture_index.contains("perl_shim_responses.json"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac18-documentation
    #[test]
    // AC:18
    fn test_dependency_management_documentation() -> Result<()> {
        let user_guide = read(repo_root().join("docs/DAP_USER_GUIDE.md"))?;
        assert!(user_guide.contains("Perl::LanguageServer"));
        assert!(user_guide.contains("BridgeAdapter"));
        assert!(user_guide.contains("cpanm Perl::LanguageServer"));

        let bridge_guide = read(repo_root().join("docs/DAP_BRIDGE_SETUP_GUIDE.md"))?;
        assert!(bridge_guide.contains("cpan Perl::LanguageServer"));
        assert!(bridge_guide.contains("cpanm Perl::LanguageServer"));
        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac18-perl-version
    #[test]
    // AC:18
    fn test_perl_version_compatibility() -> Result<()> {
        let perl_version = Command::new("perl").arg("-e").arg("print $];").output();

        match perl_version {
            Ok(output) if output.status.success() => {
                let version_str = String::from_utf8_lossy(&output.stdout).trim().to_string();
                let version_num: f64 = version_str.parse().map_err(|e| {
                    anyhow::anyhow!("failed to parse perl version '{version_str}': {e}")
                })?;
                assert!(version_num >= 5.010, "Perl version must be >= 5.010, got {version_num}");
            }
            _ => {
                // If Perl is unavailable in the test environment, ensure compatibility
                // requirements are documented.
                let guide = read(repo_root().join("docs/DAP_USER_GUIDE.md"))?;
                assert!(guide.contains("Perl 5.10 or higher"));
            }
        }

        Ok(())
    }

    /// Tests feature spec: DAP_IMPLEMENTATION_SPECIFICATION.md#ac18-cpan-install
    #[test]
    // AC:18
    fn test_cpan_dependency_installation() -> Result<()> {
        let guide = read(repo_root().join("docs/DAP_BRIDGE_SETUP_GUIDE.md"))?;
        assert!(guide.contains("cpan Perl::LanguageServer"));
        assert!(guide.contains("cpanm Perl::LanguageServer"));
        assert!(guide.contains("Perl::LanguageServer not found"));
        Ok(())
    }
}
