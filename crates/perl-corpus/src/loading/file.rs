use crate::metadata::{Section, parser};
use anyhow::{Context, Result};
use std::fs;
use std::path::Path;

pub fn parse_file(path: &Path) -> Result<Vec<Section>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(parser::parse_sections(&text, path))
}
