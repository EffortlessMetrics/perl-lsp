#[derive(Debug, Clone, PartialEq)]
pub struct CaptureGroup {
    pub name: String,
    pub index: usize,
    pub pattern: String,
}

/// Analysis utilities for Perl regex patterns: capture extraction and hover text.
pub struct RegexAnalyzer;

impl RegexAnalyzer {
    pub fn extract_named_captures(pattern: &str) -> Vec<CaptureGroup> {
        let mut result = Vec::new();
        let mut capture_index = 0usize;
        let bytes = pattern.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            if bytes[i] == b'\\' {
                i += 2;
                continue;
            }
            if bytes[i] == b'[' {
                i += 1;
                while i < len {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b']' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }

            if bytes[i] == b'(' {
                i += 1;
                if i < len && bytes[i] == b'?' {
                    i += 1;
                    if i < len && bytes[i] == b'<' {
                        i += 1;
                        if i < len && (bytes[i] == b'=' || bytes[i] == b'!') {
                            i += 1;
                            continue;
                        }

                        if let Some((name, next_pos)) = parse_named_capture_name_from(bytes, i, b'>') {
                            capture_index += 1;
                            i = next_pos;
                            let pattern_start = i;
                            let mut depth = 1usize;
                            while i < len && depth > 0 {
                                if bytes[i] == b'\\' {
                                    i += 2;
                                    continue;
                                }
                                if bytes[i] == b'[' {
                                    i += 1;
                                    while i < len {
                                        if bytes[i] == b'\\' {
                                            i += 2;
                                        } else if bytes[i] == b']' {
                                            i += 1;
                                            break;
                                        } else {
                                            i += 1;
                                        }
                                    }
                                    continue;
                                }
                                if bytes[i] == b'(' {
                                    depth += 1;
                                } else if bytes[i] == b')' {
                                    depth -= 1;
                                }
                                i += 1;
                            }
                            let sub: String = if i > 0 && pattern_start < i - 1 {
                                String::from_utf8_lossy(&bytes[pattern_start..i - 1]).into_owned()
                            } else {
                                String::new()
                            };

                            result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                            continue;
                        }
                    } else if i < len && bytes[i] == b'\'' {
                        if let Some((name, next_pos)) = parse_named_capture_name(bytes, i, b'\'', b'\'') {
                            capture_index += 1;
                            i = next_pos;
                            let pattern_start = i;
                            let mut depth = 1usize;
                            while i < len && depth > 0 {
                                if bytes[i] == b'\\' {
                                    i += 2;
                                    continue;
                                }
                                if bytes[i] == b'[' {
                                    i += 1;
                                    while i < len {
                                        if bytes[i] == b'\\' {
                                            i += 2;
                                        } else if bytes[i] == b']' {
                                            i += 1;
                                            break;
                                        } else {
                                            i += 1;
                                        }
                                    }
                                    continue;
                                }
                                if bytes[i] == b'(' {
                                    depth += 1;
                                } else if bytes[i] == b')' {
                                    depth -= 1;
                                }
                                i += 1;
                            }
                            let sub: String = if i > 0 && pattern_start < i - 1 {
                                String::from_utf8_lossy(&bytes[pattern_start..i - 1]).into_owned()
                            } else {
                                String::new()
                            };

                            result.push(CaptureGroup { name, index: capture_index, pattern: sub });
                            continue;
                        }
                    } else if i < len && matches!(bytes[i], b':' | b'=' | b'!' | b'>' | b'|' | b'P' | b'#') {
                        continue;
                    }
                    continue;
                }

                capture_index += 1;
                continue;
            }

            i += 1;
        }

        result
    }

    pub fn hover_text_for_regex(pattern: &str, modifiers: &str) -> String {
        let mut parts: Vec<String> = Vec::new();

        if !pattern.is_empty() {
            parts.push(format!("Regex: `{pattern}`"));
        }

        let captures = Self::extract_named_captures(pattern);
        if !captures.is_empty() {
            parts.push("Named captures:".to_string());
            for cap in &captures {
                parts.push(format!("  ${{{}}} (capture {}): `{}`", cap.name, cap.index, cap.pattern));
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
}

fn describe_modifier(modifier: char) -> Option<&'static str> {
    match modifier {
        'i' => Some("case-insensitive matching"),
        'm' => Some("multiline mode: ^ and $ match line boundaries"),
        's' => Some("single-line mode: dot matches newline"),
        'x' => Some("extended mode: whitespace and comments allowed"),
        'g' => Some("global: match all occurrences"),
        'a' => Some("ASCII-safe character classes"),
        'd' => Some("native platform character set semantics"),
        'l' => Some("locale-dependent character semantics"),
        'u' => Some("Unicode character semantics"),
        'n' => Some("non-capturing by default for unnamed groups"),
        'p' => Some("preserve string for ${^PREMATCH}, ${^MATCH}, ${^POSTMATCH}"),
        'r' => Some("non-destructive substitution result"),
        'c' => Some("keep current match position for /g scans"),
        'o' => Some("compile pattern only once"),
        'e' => Some("evaluate replacement as code in substitutions"),
        _ => None,
    }
}

fn parse_named_capture_name(bytes: &[u8], pos: usize, open_delim: u8, close_delim: u8) -> Option<(String, usize)> {
    if pos >= bytes.len() || bytes[pos] != open_delim {
        return None;
    }

    let mut i = pos + 1;
    let name_start = i;
    while i < bytes.len() && bytes[i] != close_delim {
        i += 1;
    }

    if i == name_start || i >= bytes.len() {
        return None;
    }

    let name = String::from_utf8_lossy(&bytes[name_start..i]).into_owned();
    Some((name, i + 1))
}

fn parse_named_capture_name_from(bytes: &[u8], start: usize, close_delim: u8) -> Option<(String, usize)> {
    if start >= bytes.len() {
        return None;
    }

    let mut i = start;
    while i < bytes.len() && bytes[i] != close_delim {
        i += 1;
    }

    if i == start || i >= bytes.len() {
        return None;
    }

    let name = String::from_utf8_lossy(&bytes[start..i]).into_owned();
    Some((name, i + 1))
}
