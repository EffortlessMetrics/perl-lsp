/// Replace `old_module::` namespace prefixes in `line` with `new_module::`.
#[must_use]
pub fn replace_module_name_prefix(line: &str, old_module: &str, new_module: &str) -> String {
    if old_module.is_empty() || new_module.is_empty() || line.is_empty() {
        return line.to_string();
    }
    let trimmed = line.trim_start();
    if trimmed.starts_with("package ")
        || trimmed.starts_with("use ")
        || trimmed.starts_with("require ")
        || trimmed.starts_with("no ")
    {
        return line.to_string();
    }

    let mut out = line.to_string();

    for separator in ["::", "'"] {
        let needle = format!("{old_module}{separator}");
        let replacement = format!("{new_module}{separator}");
        let needle_bytes = needle.as_bytes();
        let needle_len = needle_bytes.len();
        let line_bytes = out.as_bytes();

        if line_bytes.len() < needle_len {
            continue;
        }

        let mut replaced = String::with_capacity(out.len());
        let mut cursor = 0usize;

        while cursor + needle_len <= line_bytes.len() {
            let Some(rel) = out[cursor..].find(needle.as_str()) else {
                break;
            };
            let abs = cursor + rel;
            let after = abs + needle_len;

            let before_ok = abs == 0 || {
                let ch = line_bytes[abs - 1] as char;
                !ch.is_alphanumeric() && ch != '_' && ch != ':'
            };

            let after_ok = after < line_bytes.len() && {
                let ch = line_bytes[after] as char;
                ch.is_alphabetic() || ch == '_'
            };

            if before_ok && after_ok && !index_is_in_quote_or_comment(&out, abs) {
                replaced.push_str(&out[cursor..abs]);
                replaced.push_str(&replacement);
                cursor = after;
            } else {
                replaced.push_str(&out[cursor..abs + 1]);
                cursor = abs + 1;
            }
        }

        replaced.push_str(&out[cursor..]);
        out = replaced;
    }

    out
}

pub(super) fn index_is_in_quote_or_comment(line: &str, index: usize) -> bool {
    let bytes = line.as_bytes();
    if index >= bytes.len() {
        return false;
    }

    let mut in_single = false;
    let mut in_double = false;
    let mut escaped = false;

    for (i, &byte) in bytes.iter().enumerate() {
        if i == index {
            return in_single || in_double;
        }

        let ch = byte as char;
        if escaped {
            escaped = false;
            continue;
        }

        if in_single {
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '\'' {
                in_single = false;
            }
            continue;
        }

        if in_double {
            if ch == '\\' {
                escaped = true;
                continue;
            }
            if ch == '"' {
                in_double = false;
            }
            continue;
        }

        if ch == '#' {
            return i < index;
        }

        if ch == '\'' {
            in_single = true;
            continue;
        }

        if ch == '"' {
            in_double = true;
        }
    }

    false
}
