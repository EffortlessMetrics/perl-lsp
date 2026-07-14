use super::super::super::source_path_from_uri;
use perl_module::resolution::use_lib::{
    no_lib_cancelled_paths_at_offset, resolve_use_lib_paths_from_source,
    resolve_use_lib_paths_from_source_at_offset,
};
use std::collections::HashSet;
use std::path::{Path, PathBuf};

pub(super) fn lexical_paths(
    doc_uri: Option<&str>,
    doc_text: Option<&str>,
    doc_offset: Option<usize>,
    root: &Path,
) -> Vec<String> {
    let Some(text) = doc_text else {
        return Vec::new();
    };

    let file_dir = file_dir(doc_uri);
    if file_dir.is_none() && doc_uri.is_some() {
        tracing::trace!("Effective @INC context failed to resolve doc_uri: {:?}", doc_uri);
    }

    if let Some(offset) = doc_offset {
        resolve_use_lib_paths_from_source_at_offset(text, offset, root, file_dir.as_deref())
    } else {
        resolve_use_lib_paths_from_source(text, root, file_dir.as_deref())
    }
}

pub(super) fn include_paths_with_cancellations(
    doc_uri: Option<&str>,
    doc_text: Option<&str>,
    doc_offset: Option<usize>,
    root: &Path,
    raw_include_paths: Vec<String>,
) -> Vec<String> {
    let (Some(offset), Some(text)) = (doc_offset, doc_text) else {
        return raw_include_paths;
    };

    let file_dir = file_dir(doc_uri);
    let cancelled = no_lib_cancelled_paths_at_offset(text, offset, root, file_dir.as_deref());
    if cancelled.is_empty() {
        raw_include_paths
    } else {
        let cancelled_keys: HashSet<String> =
            cancelled.iter().map(|path| include_path_key(path)).collect();
        raw_include_paths
            .into_iter()
            .filter(|path| !cancelled_keys.contains(&include_path_key(path)))
            .collect()
    }
}

fn include_path_key(path: &str) -> String {
    Path::new(path)
        .components()
        .fold(PathBuf::new(), |mut acc, component| {
            match component {
                std::path::Component::CurDir => {}
                std::path::Component::RootDir
                | std::path::Component::Prefix(_)
                | std::path::Component::ParentDir
                | std::path::Component::Normal(_) => acc.push(component.as_os_str()),
            }
            acc
        })
        .to_string_lossy()
        .replace('\\', "/")
        .trim_end_matches('/')
        .to_string()
}

fn file_dir(doc_uri: Option<&str>) -> Option<PathBuf> {
    doc_uri
        .and_then(source_path_from_uri)
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
}
