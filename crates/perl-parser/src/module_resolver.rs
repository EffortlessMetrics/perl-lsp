use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// Resolve a module name to a file path URI.
///
/// The generic `D` allows this resolver to work with any document state
/// representation, as only the document keys (URIs) are inspected.
pub fn resolve_module_to_path<D>(
    documents: &Arc<Mutex<HashMap<String, D>>>,
    workspace_folders: &Arc<Mutex<Vec<String>>>,
    module_name: &str,
) -> Option<String> {
    // Convert Module::Name to Module/Name.pm
    let relative_path = format!("{}.pm", module_name.replace("::", "/"));

    // Fast path: check already-open documents
    let documents = documents.lock().unwrap();
    for (uri, _doc) in documents.iter() {
        if uri.ends_with(&relative_path) {
            return Some(uri.clone());
        }
    }
    drop(documents);

    // Time-limited filesystem search
    let start_time = Instant::now();
    let timeout = Duration::from_millis(50);

    let workspace_folders = workspace_folders.lock().unwrap().clone();
    let search_dirs = ["lib", ".", "local/lib/perl5"];

    for workspace_folder in workspace_folders.iter() {
        if start_time.elapsed() > timeout {
            return None;
        }

        let workspace_path = if workspace_folder.starts_with("file://") {
            workspace_folder.strip_prefix("file://").unwrap_or(workspace_folder)
        } else {
            workspace_folder
        };

        for dir in &search_dirs {
            let full_path = if *dir == "." {
                format!("{}/{}", workspace_path, relative_path)
            } else {
                format!("{}/{}/{}", workspace_path, dir, relative_path)
            };

            if start_time.elapsed() > timeout {
                return None;
            }

            if let Ok(meta) = std::fs::metadata(&full_path) {
                if meta.is_file() {
                    return Some(format!("file://{}", full_path));
                }
            }

            if start_time.elapsed() > timeout {
                return None;
            }
        }
    }

    // Don't search system paths to avoid blocking on network filesystems
    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn resolves_existing_module() {
        let dir = tempdir().unwrap();
        let module_dir = dir.path().join("lib").join("Foo");
        std::fs::create_dir_all(&module_dir).unwrap();
        std::fs::write(module_dir.join("Bar.pm"), "1;").unwrap();

        let documents = Arc::new(Mutex::new(HashMap::<String, ()>::new()));
        let workspace_folders =
            Arc::new(Mutex::new(vec![format!("file://{}", dir.path().display())]));

        let path = resolve_module_to_path(&documents, &workspace_folders, "Foo::Bar");
        assert!(path.is_some(), "module should resolve");
    }

    #[test]
    fn missing_module_returns_none() {
        let dir = tempdir().unwrap();
        let documents = Arc::new(Mutex::new(HashMap::<String, ()>::new()));
        let workspace_folders =
            Arc::new(Mutex::new(vec![format!("file://{}", dir.path().display())]));

        let path = resolve_module_to_path(&documents, &workspace_folders, "No::Such::Module");
        assert!(path.is_none(), "module should not resolve");
    }
}
