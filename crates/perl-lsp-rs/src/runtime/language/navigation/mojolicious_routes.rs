//! Mojolicious route-to-controller resolution helpers.
//!
//! This module owns framework-specific parsing so the generic navigation
//! handler does not need to understand every supported route declaration shape.

use crate::protocol::JsonRpcError;
use std::sync::OnceLock;

static MOJO_STRING_ROUTE_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();
static MOJO_KV_ROUTE_RE: OnceLock<Result<regex::Regex, regex::Error>> = OnceLock::new();
static MOJO_KV_ROUTE_RE_ACTION_FIRST: OnceLock<Result<regex::Regex, regex::Error>> =
    OnceLock::new();

fn get_mojo_string_route_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    MOJO_STRING_ROUTE_RE
        .get_or_init(|| {
            regex::Regex::new(
                r##"->\s*to\s*\(\s*['"](?P<controller>[^'"#]+)#(?P<action>[^'"]+)['"]\s*\)"##,
            )
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize Mojolicious route regex: {err}"
            ))
        })
}

fn get_mojo_kv_route_regex() -> Result<&'static regex::Regex, JsonRpcError> {
    MOJO_KV_ROUTE_RE
        .get_or_init(|| {
            regex::Regex::new(
                r#"->\s*to\s*\(\s*controller\s*=>\s*['"](?P<controller>[^'"]+)['"]\s*,\s*action\s*=>\s*['"](?P<action>[^'"]+)['"]\s*\)"#,
            )
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize Mojolicious route regex: {err}"
            ))
        })
}

fn get_mojo_kv_route_regex_action_first() -> Result<&'static regex::Regex, JsonRpcError> {
    MOJO_KV_ROUTE_RE_ACTION_FIRST
        .get_or_init(|| {
            regex::Regex::new(
                r#"->\s*to\s*\(\s*action\s*=>\s*['"](?P<action>[^'"]+)['"]\s*,\s*controller\s*=>\s*['"](?P<controller>[^'"]+)['"]\s*\)"#,
            )
        })
        .as_ref()
        .map_err(|err| {
            crate::protocol::internal_error(&format!(
                "Failed to initialize Mojolicious route regex: {err}"
            ))
        })
}

fn mojolicious_app_root(current_package: &str) -> Option<String> {
    let package = current_package.trim();
    if package.is_empty() {
        return None;
    }

    Some(package.strip_suffix("::App").unwrap_or(package).to_string())
}

fn normalize_mojolicious_controller_name(raw: &str) -> Option<String> {
    let normalized = raw.trim().trim_matches('\'').trim_matches('"').trim();
    if normalized.is_empty() {
        return None;
    }

    let mut segments = Vec::new();
    for segment in
        normalized.split("::").flat_map(|part| part.split('/')).flat_map(|part| part.split('-'))
    {
        let segment = segment.trim();
        if segment.is_empty() {
            continue;
        }
        let mut normalized_segment = String::new();
        let mut capitalize_next = true;
        for ch in segment.chars() {
            if ch == '_' {
                capitalize_next = true;
                continue;
            }

            if capitalize_next {
                normalized_segment.extend(ch.to_uppercase());
                capitalize_next = false;
            } else {
                normalized_segment.push(ch);
            }
        }

        if normalized_segment.is_empty() {
            continue;
        }
        segments.push(normalized_segment);
    }

    if segments.is_empty() { None } else { Some(segments.join("::")) }
}

pub(super) fn resolve_mojolicious_route_definition(
    workspace_index: &crate::workspace_index::WorkspaceIndex,
    current_package: &str,
    text_around: &str,
    cursor_in_text: usize,
) -> Option<crate::workspace_index::Location> {
    let app_root = mojolicious_app_root(current_package)?;

    let try_route = |controller: &str, action: &str| {
        let controller_name = normalize_mojolicious_controller_name(controller)?;
        let package_name = format!("{app_root}::Controller::{controller_name}");
        super::find_workspace_definition_location(workspace_index, &package_name, action)
    };

    let string_re = get_mojo_string_route_regex().ok()?;
    for cap in string_re.captures_iter(text_around) {
        let Some(full_match) = cap.get(0) else {
            continue;
        };
        if cursor_in_text < full_match.start() || cursor_in_text >= full_match.end() {
            continue;
        }

        let Some(controller_match) = cap.name("controller") else {
            continue;
        };
        let Some(action_match) = cap.name("action") else {
            continue;
        };

        if (cursor_in_text >= controller_match.start() && cursor_in_text < controller_match.end())
            || (cursor_in_text >= action_match.start() && cursor_in_text < action_match.end())
        {
            if let Some(location) = try_route(controller_match.as_str(), action_match.as_str()) {
                return Some(location);
            }
        }
    }

    for kv_re in [get_mojo_kv_route_regex().ok()?, get_mojo_kv_route_regex_action_first().ok()?] {
        for cap in kv_re.captures_iter(text_around) {
            let Some(full_match) = cap.get(0) else {
                continue;
            };
            if cursor_in_text < full_match.start() || cursor_in_text >= full_match.end() {
                continue;
            }

            let Some(controller_match) = cap.name("controller") else {
                continue;
            };
            let Some(action_match) = cap.name("action") else {
                continue;
            };

            if (cursor_in_text >= controller_match.start()
                && cursor_in_text < controller_match.end())
                || (cursor_in_text >= action_match.start() && cursor_in_text < action_match.end())
            {
                if let Some(location) = try_route(controller_match.as_str(), action_match.as_str())
                {
                    return Some(location);
                }
            }
        }
    }

    None
}
