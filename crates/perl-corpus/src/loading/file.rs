use crate::metadata::{self, Section};
use anyhow::{Context, Result};
use std::{fs, path::Path};

pub fn parse_file(path: &Path) -> Result<Vec<Section>> {
    let text = fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))?;
    Ok(metadata::parser::parse_sections(&text, path))
}
