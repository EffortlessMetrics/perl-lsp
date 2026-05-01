use crate::loading::file::parse_file;
use crate::metadata::Section;
use anyhow::Result;
use std::path::Path;

pub fn parse_dir(dir: &Path) -> Result<Vec<Section>> {
    let mut all = Vec::new();
    let pattern = format!("{}/**/*.txt", dir.display());

    for entry in glob::glob(&pattern)? {
        let path = entry?;
        let filename = path.file_name().unwrap_or_default().to_string_lossy();
        if filename.starts_with('_') || filename.starts_with('.') {
            continue;
        }
        all.extend(parse_file(&path)?);
    }

    all.sort_by(|a, b| a.file.cmp(&b.file).then_with(|| a.id.cmp(&b.id)));
    Ok(all)
}
