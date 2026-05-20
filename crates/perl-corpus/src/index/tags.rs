use crate::meta::Section;
use anyhow::Result;
use std::{collections::BTreeMap, fs, path::Path};

pub(super) fn write_tag_index(dir: &Path, sections: &[Section]) -> Result<()> {
    let mut tagmap: BTreeMap<String, Vec<&str>> = BTreeMap::new();
    for section in sections {
        for tag in &section.tags {
            tagmap.entry(tag.clone()).or_default().push(&section.id);
        }
    }

    let tags_path = dir.join("_tags.json");
    fs::write(&tags_path, serde_json::to_vec_pretty(&tagmap)?)?;

    Ok(())
}
