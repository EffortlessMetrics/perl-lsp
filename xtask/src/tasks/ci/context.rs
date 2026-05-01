use color_eyre::eyre::{Context, Result};

use crate::utils::{constrained_env_vars, project_root};

pub(super) fn prepare_environment() -> Result<()> {
    let root = project_root()?;
    std::env::set_current_dir(&root).context("Failed to change to project root")?;

    let env_vars = constrained_env_vars();
    // SAFETY: We're in a single-threaded xtask binary with no concurrent environment access
    for (key, value) in &env_vars {
        unsafe {
            std::env::set_var(key, value);
        }
    }

    Ok(())
}
