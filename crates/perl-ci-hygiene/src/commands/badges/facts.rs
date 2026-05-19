use color_eyre::eyre::{Context, Result};
use std::path::{Path, PathBuf};
use toml::Value as TomlValue;

pub(super) struct VscodeMarketplaceInstalls {
    pub(super) value: u32,
    pub(super) facts_file: PathBuf,
}

pub(super) fn read_vscode_marketplace_installs(
    repo_root: &Path,
) -> Result<VscodeMarketplaceInstalls> {
    let facts_file = repo_root.join("docs/project/publication-facts.toml");
    let facts_content = std::fs::read_to_string(&facts_file)
        .with_context(|| format!("reading facts file {:?}", facts_file))?;
    let facts: TomlValue = toml::from_str(&facts_content)
        .with_context(|| format!("parsing facts file {:?}", facts_file))?;

    let installs_i64 = facts
        .get("external")
        .and_then(|e| e.get("vscode_marketplace_installs"))
        .and_then(|v| v.get("value"))
        .and_then(|v| v.as_integer())
        .ok_or_else(|| {
            color_eyre::eyre::eyre!(
                "Could not read vscode_marketplace_installs.value from {}",
                facts_file.display()
            )
        })?;

    let value = u32::try_from(installs_i64).with_context(|| {
        format!("vscode_marketplace_installs value {} is out of range for u32", installs_i64)
    })?;

    Ok(VscodeMarketplaceInstalls { value, facts_file })
}
