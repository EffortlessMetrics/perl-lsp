use std::collections::HashSet;

use super::{capture::extract_named_captures, modifiers::describe_modifier};

pub(crate) fn hover_text_for_regex(pattern: &str, modifiers: &str) -> String {
    let mut parts = Vec::new();
    if !pattern.is_empty() {
        parts.push(format!("Regex: `{pattern}`"));
    }
    let captures = extract_named_captures(pattern);
    if !captures.is_empty() {
        parts.push("Named captures:".to_string());
        for cap in &captures {
            parts.push(format!("  ${{{}}} (capture {}): `{}`", cap.name, cap.index, cap.pattern));
        }
    }
    let mut seen = HashSet::new();
    let mut notes = Vec::new();
    let mut unknown = Vec::new();
    for m in modifiers.chars() {
        if m.is_whitespace() {
            continue;
        }
        if !seen.insert(m) {
            continue;
        }
        if let Some(d) = describe_modifier(m) {
            notes.push(d);
        } else {
            unknown.push(m);
        }
    }
    if !notes.is_empty() {
        parts.push("Modifiers:".to_string());
        for n in notes {
            parts.push(format!("  {n}"));
        }
    }
    if !unknown.is_empty() {
        parts.push(format!("Unknown modifiers: `{}`", unknown.into_iter().collect::<String>()));
    }
    parts.join("\n")
}
