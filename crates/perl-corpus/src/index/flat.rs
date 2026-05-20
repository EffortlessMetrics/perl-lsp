use crate::meta::Section;
use anyhow::Result;
use serde::Serialize;
use std::{fs, path::Path};

#[derive(Serialize)]
struct IndexEntry<'a> {
    id: &'a str,
    file: &'a str,
    title: &'a str,
    tags: &'a [String],
    perl: &'a Option<String>,
    flags: &'a [String],
}

pub(super) fn write_flat_index(dir: &Path, sections: &[Section]) -> Result<()> {
    let index: Vec<IndexEntry> = sections
        .iter()
        .map(|section| IndexEntry {
            id: &section.id,
            file: &section.file,
            title: &section.title,
            tags: &section.tags,
            perl: &section.perl,
            flags: &section.flags,
        })
        .collect();

    let idx_path = dir.join("_index.json");
    fs::write(&idx_path, serde_json::to_vec_pretty(&index)?)?;

    Ok(())
}
