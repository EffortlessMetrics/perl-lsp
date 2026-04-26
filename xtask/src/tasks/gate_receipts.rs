//! Stubs for control-plane gate receipt commands.

use color_eyre::eyre::Result;
use std::path::PathBuf;

/// Run `cargo xtask gate-receipts list`.
pub fn list() -> Result<()> {
    println!("[stub] gate-receipts list is not implemented yet");
    Ok(())
}

/// Run `cargo xtask gate-receipts validate <path>`.
pub fn validate(path: PathBuf) -> Result<()> {
    println!(
        "[stub] gate-receipts validate is not implemented yet (path: {})",
        path.display()
    );
    Ok(())
}
