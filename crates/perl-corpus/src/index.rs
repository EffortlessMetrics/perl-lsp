use crate::meta::Section;
use anyhow::Result;
use std::path::Path;

mod coverage;
mod flat;
mod tags;

/// Write index files and coverage report
pub fn write_indices(dir: &Path, sections: &[Section]) -> Result<()> {
    flat::write_flat_index(dir, sections)?;
    tags::write_tag_index(dir, sections)?;
    coverage::write_coverage_summary(dir, sections)?;

    Ok(())
}
