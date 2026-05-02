use color_eyre::eyre::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

pub(super) fn create_spinner() -> Result<ProgressBar> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .context("Failed to create progress spinner template")?,
    );
    Ok(spinner)
}
