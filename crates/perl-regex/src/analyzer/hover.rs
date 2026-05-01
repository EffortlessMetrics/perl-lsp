use super::{capture::extract_named_captures, modifiers::describe_modifier};

pub(crate) fn hover_text_for_regex(pattern: &str, modifiers: &str) -> String {
    let mut parts: Vec<String> = Vec::new();

    if !pattern.is_empty() {
        parts.push(format!("Regex: `{pattern}`"));
    }

    let captures = extract_named_captures(pattern);
    if !captures.is_empty() {
        parts.push("Named captures:".to_string());
        for cap in &captures {
            parts.push(format!(
                "  ${{{name}}} (capture {index}): `{pat}`",
                name = cap.name,
                index = cap.index,
                pat = cap.pattern,
            ));
        }
    }

    let mut seen_modifiers: Vec<char> = Vec::new();
    let mut modifier_notes: Vec<&str> = Vec::new();
    let mut unknown_modifiers: Vec<char> = Vec::new();
    for modifier in modifiers.chars() {
        if seen_modifiers.contains(&modifier) {
            continue;
        }
        seen_modifiers.push(modifier);
        match describe_modifier(modifier) {
            Some(description) => modifier_notes.push(description),
            None => unknown_modifiers.push(modifier),
        }
    }

    if !modifier_notes.is_empty() {
        parts.push("Modifiers:".to_string());
        for note in modifier_notes {
            parts.push(format!("  {note}"));
        }
    }

    if !unknown_modifiers.is_empty() {
        let unknown: String = unknown_modifiers.into_iter().collect();
        parts.push(format!("Unknown modifiers: `{unknown}`"));
    }

    parts.join("\n")
}
