use color_eyre::eyre::Result;
use std::path::PathBuf;

pub fn list() -> Result<()> {
    println!("gate-receipts list is not implemented yet");
    Ok(())
}

pub fn validate(path: PathBuf) -> Result<()> {
    println!("gate-receipts validate is not implemented yet (path: {})", path.display());
    Ok(())
}
