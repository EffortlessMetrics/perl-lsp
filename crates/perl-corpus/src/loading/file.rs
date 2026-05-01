use crate::metadata::{Section, parser::parse_sections};
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub fn parse_file(path: &Path) -> Result<Vec<Section>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(parse_sections(&text, path))
}
