use color_eyre::eyre::Result;
use std::path::Path;

pub fn list() -> Result<()> {
    println!("gate-receipts list: not implemented yet");
    Ok(())
}

pub fn validate(path: &Path) -> Result<()> {
    println!("gate-receipts validate {}: not implemented yet", path.display());
    Ok(())
}
