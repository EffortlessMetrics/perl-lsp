//! Bindings generation task implementation

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use bindgen::Builder;
use color_eyre::eyre::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};

/// Generate Rust bindings from C headers using bindgen.
pub fn run(header: PathBuf, output: PathBuf) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}")?);

    spinner.set_message("Generating bindings");

    // Ensure output directory exists
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).context("failed to create output directory")?;
    }

    let header_dir = header.parent().map(Path::to_path_buf).unwrap_or_else(|| PathBuf::from("."));

    // Run bindgen on the provided header - aligned with build.rs configuration
    let bindings = Builder::default()
        .header(header.to_string_lossy())
        .clang_arg(format!("-I{}", header_dir.display()))
        .allowlist_function("tree_sitter_perl.*")
        .allowlist_type("TS.*")
        .allowlist_var("TREE_SITTER_LANGUAGE_VERSION")
        .parse_callbacks(Box::new(bindgen::CargoCallbacks::new()))
        .generate_inline_functions(true)
        .size_t_is_usize(true)
        // For Rust 2024 compatibility: generate safe extern blocks
        .wrap_unsafe_ops(true)
        // Additional setting for external functions
        .default_macro_constant_type(bindgen::MacroTypeVariation::Signed)
        .generate()
        .context("unable to generate bindings")?;

    bindings.write_to_file(&output).context("failed to write bindings")?;

    // Format the generated bindings if rustfmt is available
    let _ = Command::new("rustfmt").arg(&output).status();

    spinner.finish_with_message("✅ Bindings generated");
    Ok(())
}
