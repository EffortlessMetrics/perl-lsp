//! Lightweight Mason navigation support.
//!
//! This helper intentionally keeps the Mason surface narrow:
//! - `.mason` / `.mas` recognition
//! - `<%method>` / `<%sub>` same-file goto-definition
//! - `<& component &>` component-file goto-definition
//!
//! It does not attempt full Mason parsing, highlighting, args semantics,
//! or embedded Perl diagnostics.

use super::super::*;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use url::Url;

use crate::position::{Position as EnginePosition, Range as EngineRange};
use crate::workspace_index::Location;

static MASON_DEF_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();
static MASON_COMPONENT_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();

fn get_mason_def_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    MASON_DEF_RE
        .get_or_init(|| regex::Regex::new(r"(?s)<%(?:method|sub)\s+([A-Za-z_][A-Za-z0-9_]*)\b"))
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize Mason definition regex: {err}"
            ))
        })
}

fn get_mason_component_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    MASON_COMPONENT_RE.get_or_init(|| regex::Regex::new(r"(?s)<&(?P<body>.*?)&>")).as_ref().map_err(
        |err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize Mason component regex: {err}"
            ))
        },
    )
}

fn is_mason_template_path(path: &Path) -> bool {
    path.extension().and_then(|ext| ext.to_str()).is_some_and(|ext| matches!(ext, "mason" | "mas"))
}

fn uri_to_path(uri: &str) -> Option<PathBuf> {
    Url::parse(uri).ok()?.to_file_path().ok()
}

fn engine_position(text: &str, offset: usize) -> EnginePosition {
    let (line, column) = byte_to_line_col(text, offset);

    EnginePosition { byte: offset, line, column }
}

fn name_range_in_text(text: &str, start: usize, end: usize) -> EngineRange {
    EngineRange { start: engine_position(text, start), end: engine_position(text, end) }
}

fn zero_range() -> EngineRange {
    EngineRange {
        start: EnginePosition { byte: 0, line: 0, column: 0 },
        end: EnginePosition { byte: 0, line: 0, column: 0 },
    }
}

fn component_token(body: &str) -> Option<(String, usize, usize)> {
    let trimmed = body.trim_start();
    let leading_ws = body.len().saturating_sub(trimmed.len());

    if trimmed.is_empty() || trimmed.starts_with('|') {
        return None;
    }

    let token_end = trimmed
        .find(|ch: char| ch.is_whitespace() || ch == ',' || ch == '|')
        .unwrap_or(trimmed.len());
    if token_end == 0 {
        return None;
    }

    let token = &trimmed[..token_end];
    Some((token.to_string(), leading_ws, leading_ws + token_end))
}

fn same_file_mason_definitions(
    text: &str,
    def_re: &regex::Regex,
) -> HashMap<String, (usize, usize)> {
    let mut definitions = HashMap::new();

    for cap in def_re.captures_iter(text) {
        let Some(name) = cap.get(1) else {
            continue;
        };
        definitions.entry(name.as_str().to_string()).or_insert((name.start(), name.end()));
    }

    definitions
}

fn resolve_mason_component_file(source_path: &Path, component_name: &str) -> Option<PathBuf> {
    let raw = Path::new(component_name);
    let candidates: Vec<PathBuf> = if raw.is_absolute()
        || raw
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| matches!(ext, "mason" | "mas"))
    {
        vec![raw.to_path_buf()]
    } else {
        vec![raw.with_extension("mason"), raw.with_extension("mas")]
    };

    let mut search_roots = Vec::new();
    let mut current = source_path.parent();
    while let Some(dir) = current {
        search_roots.push(dir.to_path_buf());
        current = dir.parent();
    }

    for root in search_roots {
        for candidate in &candidates {
            let path = root.join(candidate);
            if path.is_file() {
                return Some(path);
            }
        }
    }

    None
}

impl LspServer {
    /// Resolve Mason-specific definitions in `.mason` / `.mas` files.
    ///
    /// The resolution is intentionally narrow:
    /// - same-file `<%method>` / `<%sub>` names resolve to their own definition
    /// - `<& component &>` resolves to a same-file Mason definition first, then
    ///   to a sibling/ancestor component file with `.mason` or `.mas`
    pub(crate) fn resolve_mason_definition(
        &self,
        uri: &str,
        text: &str,
        offset: usize,
    ) -> Option<Location> {
        let source_path = uri_to_path(uri)?;
        if !is_mason_template_path(&source_path) {
            return None;
        }

        let def_re = get_mason_def_regex().ok()?;
        let component_re = get_mason_component_regex().ok()?;
        let definitions = same_file_mason_definitions(text, def_re);

        for cap in def_re.captures_iter(text) {
            let Some(name) = cap.get(1) else {
                continue;
            };

            if offset >= name.start() && offset < name.end() {
                return Some(Location {
                    uri: uri.to_string(),
                    range: name_range_in_text(text, name.start(), name.end()),
                });
            }
        }

        for cap in component_re.captures_iter(text) {
            let Some(body) = cap.name("body") else {
                continue;
            };
            let Some((component_name, rel_start, rel_end)) = component_token(body.as_str()) else {
                continue;
            };

            let name_start = body.start() + rel_start;
            let name_end = body.start() + rel_end;
            if offset < name_start || offset >= name_end {
                continue;
            }

            if let Some((def_start, def_end)) = definitions.get(&component_name) {
                return Some(Location {
                    uri: uri.to_string(),
                    range: name_range_in_text(text, *def_start, *def_end),
                });
            }

            if let Some(component_path) =
                resolve_mason_component_file(&source_path, &component_name)
            {
                if let Ok(component_uri) = Url::from_file_path(component_path) {
                    return Some(Location { uri: component_uri.to_string(), range: zero_range() });
                }
            }
        }

        None
    }
}
