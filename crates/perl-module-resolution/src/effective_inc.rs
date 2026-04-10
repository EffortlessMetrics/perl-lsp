use std::path::{Path, PathBuf};
use url::Url;
use std::time::{Duration, Instant};

/// Represents an ordered sequence of include paths for module resolution.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EffectiveInc {
    pub roots: Vec<PathBuf>,
}

impl EffectiveInc {
    /// Creates a new, ordered `EffectiveInc` stack.
    ///
    /// The canonical order is:
    /// 1. File-local lexical roots (`use lib`, `FindBin` — if prepended)
    /// 2. Configured workspace roots (relative or absolute)
    /// 3. Startup `@INC` paths (if system inc is enabled)
    pub fn new(
        workspace_root: Option<&Path>,
        configured_paths: &[String],
        system_inc: &[PathBuf],
    ) -> Self {
        let mut roots = Vec::new();

        if let Some(root) = workspace_root {
            for base_str in configured_paths {
                let base_path = Path::new(base_str);
                if base_path.is_absolute() {
                    roots.push(base_path.to_path_buf());
                } else if base_str == "." {
                    roots.push(root.to_path_buf());
                } else {
                    roots.push(root.join(base_path));
                }
            }
        } else {
            // No workspace root: only absolute paths can be resolved
            for base_str in configured_paths {
                let base_path = Path::new(base_str);
                if base_path.is_absolute() {
                    roots.push(base_path.to_path_buf());
                }
            }
        }

        // Add system @INC paths
        roots.extend_from_slice(system_inc);

        Self { roots }
    }
}
