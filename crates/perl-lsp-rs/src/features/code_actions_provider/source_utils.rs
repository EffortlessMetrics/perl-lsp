use super::CodeActionsProvider;

pub(super) fn ranges_overlap(r1: (usize, usize), r2: (usize, usize)) -> bool {
    r1.0 < r2.1 && r2.0 < r1.1
}

pub(super) fn extract_quoted_value(message: &str) -> Option<String> {
    ['\'', '`', '"']
        .into_iter()
        .filter_map(|delimiter| extract_between(message, delimiter))
        .min_by_key(|(start, _)| *start)
        .map(|(_, value)| value)
}

pub(super) fn find_declaration_position(provider: &CodeActionsProvider, near: usize) -> usize {
    provider.source()[..near].rfind('\n').map(|idx| idx + 1).unwrap_or(0)
}

pub(super) fn find_declaration_range(
    provider: &CodeActionsProvider,
    var_name: &str,
    near: usize,
) -> Option<(usize, usize)> {
    let search_pattern = format!("my {}", var_name);
    let source = provider.source();
    let line_start = source[..near].rfind('\n').map(|idx| idx + 1).unwrap_or(0);
    let line_end = source[near..].find('\n').map(|offset| near + offset).unwrap_or(source.len());

    if let Some(pos) = source[line_start..line_end]
        .match_indices(&search_pattern)
        .map(|(offset, _)| line_start + offset)
        .filter(|pos| *pos <= near)
        .max()
    {
        return Some((pos, declaration_end(source, pos, &search_pattern)));
    }

    if let Some(pos) = source[..near].rfind(&search_pattern) {
        return Some((pos, declaration_end(source, pos, &search_pattern)));
    }

    None
}

pub(super) fn find_line_end(provider: &CodeActionsProvider, pos: usize) -> usize {
    provider.source()[pos..]
        .find('\n')
        .map(|offset| pos + offset)
        .unwrap_or(provider.source().len())
}

pub(super) fn detect_quote_char(provider: &CodeActionsProvider, pos: usize) -> char {
    let source = provider.source();
    let before = &source[pos.saturating_sub(10)..pos];
    if before.contains('\'') { '\'' } else { '"' }
}

pub(super) fn make_unused_name(name: &str) -> String {
    if let Some(stripped) = name.strip_prefix('$') {
        format!("$_{}", stripped)
    } else if let Some(stripped) = name.strip_prefix('@') {
        format!("@_{}", stripped)
    } else if let Some(stripped) = name.strip_prefix('%') {
        format!("%_{}", stripped)
    } else {
        format!("_{}", name)
    }
}

pub(super) fn split_sigil(name: &str) -> (&str, &str) {
    let bare = name.trim_start_matches(['$', '@', '%']);
    let sigil_len = name.len() - bare.len();
    (&name[..sigil_len], bare)
}

fn extract_between(message: &str, delimiter: char) -> Option<(usize, String)> {
    let start = message.find(delimiter)?;
    let end = message[start + 1..].find(delimiter)?;
    Some((start, message[start + 1..start + 1 + end].to_string()))
}

fn declaration_end(source: &str, pos: usize, search_pattern: &str) -> usize {
    source[pos..]
        .find(';')
        .map(|offset| {
            let semicolon = pos + offset + 1;
            if semicolon < source.len() && source.as_bytes()[semicolon] == b'\n' {
                semicolon + 1
            } else {
                semicolon
            }
        })
        .unwrap_or(pos + search_pattern.len())
}
