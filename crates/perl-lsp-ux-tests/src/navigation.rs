use serde_json::Value;

/// Canonical navigation target extracted from either `Location` or
/// `LocationLink` LSP payloads.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NavigationTarget {
    pub uri: String,
    pub start_line: u64,
    pub start_character: u64,
}

/// Return true when every entry is either a `Location` or a `LocationLink`.
pub fn all_locations_or_links(entries: &[Value]) -> bool {
    entries.iter().all(|entry| navigation_target_from_value(entry).is_some())
}

/// Extract all parseable navigation targets from raw LSP response entries.
pub fn collect_navigation_targets(entries: &[Value]) -> Vec<NavigationTarget> {
    entries.iter().filter_map(navigation_target_from_value).collect()
}

/// Return true when any parsed target points to a URI ending with `file_suffix`.
pub fn has_target_in_file(entries: &[Value], file_suffix: &str) -> bool {
    collect_navigation_targets(entries).iter().any(|target| target.uri.ends_with(file_suffix))
}

/// Return true when any parsed target starts at `line` (0-indexed).
pub fn has_target_start_line(entries: &[Value], line: u64) -> bool {
    collect_navigation_targets(entries).iter().any(|target| target.start_line == line)
}

fn navigation_target_from_value(entry: &Value) -> Option<NavigationTarget> {
    let (uri, range) = if let Some(uri) = entry.get("targetUri").and_then(Value::as_str) {
        let range = entry.get("targetSelectionRange").or_else(|| entry.get("targetRange"))?;
        (uri, range)
    } else {
        let uri = entry.get("uri").and_then(Value::as_str)?;
        let range = entry.get("range")?;
        (uri, range)
    };

    let start = range.get("start")?;
    let start_line = start.get("line")?.as_u64()?;
    let start_character = start.get("character")?.as_u64()?;

    Some(NavigationTarget { uri: uri.to_string(), start_line, start_character })
}

#[cfg(test)]
mod tests {
    use super::{all_locations_or_links, has_target_in_file, has_target_start_line};
    use anyhow::Result;
    use serde_json::json;

    #[test]
    fn navigation_helpers_accept_location_and_location_link_shapes() -> Result<()> {
        let entries = vec![
            json!({
                "uri": "file:///workspace/sample.pl",
                "range": {
                    "start": {"line": 3, "character": 2},
                    "end": {"line": 3, "character": 8}
                }
            }),
            json!({
                "targetUri": "file:///workspace/lib/Sample.pm",
                "targetRange": {
                    "start": {"line": 10, "character": 0},
                    "end": {"line": 12, "character": 1}
                },
                "targetSelectionRange": {
                    "start": {"line": 11, "character": 4},
                    "end": {"line": 11, "character": 10}
                }
            }),
        ];

        assert!(all_locations_or_links(&entries));
        assert!(has_target_in_file(&entries, "sample.pl"));
        assert!(has_target_in_file(&entries, "Sample.pm"));
        assert!(has_target_start_line(&entries, 3));
        assert!(has_target_start_line(&entries, 11));

        Ok(())
    }

    #[test]
    fn navigation_helpers_reject_malformed_entries() -> Result<()> {
        let malformed = vec![json!({"uri": "file:///workspace/sample.pl"})];
        assert!(!all_locations_or_links(&malformed));
        Ok(())
    }
}
