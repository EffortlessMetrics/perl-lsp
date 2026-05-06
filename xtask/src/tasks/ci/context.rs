use color_eyre::eyre::{Context, Result};

use crate::utils::project_root;

pub(super) fn prepare_environment() -> Result<()> {
    let root = project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    Ok(())
}
