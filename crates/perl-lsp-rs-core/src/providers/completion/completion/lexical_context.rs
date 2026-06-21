use perl_parser_core::syntax::text_line::is_identifier_byte;
use std::cmp::Ordering;

/// Simple heuristic to check if position is in a string.
pub(super) fn is_in_string(source: &str, position: usize) -> bool {
    let before = &source[..position];
    let single_quotes = before.matches('\'').count();
    let double_quotes = before.matches('"').count();

    single_quotes % 2 == 1 || double_quotes % 2 == 1
}

/// Heuristic to check if position is inside a regex literal.
pub(super) fn is_in_regex(source: &str, position: usize) -> bool {
    let before = &source[..position];

    let Some(last_slash) = before.rfind('/') else {
        return false;
    };

    let pre_slash = before[..last_slash].trim_end();
    if pre_slash.ends_with("=~") || pre_slash.ends_with("!~") {
        return true;
    }

    if pre_slash_has_regex_op(pre_slash) {
        return true;
    }

    if matches!(
        pre_slash.split_ascii_whitespace().next_back(),
        Some("or") | Some("and") | Some("not")
    ) {
        return true;
    }

    if let Some(last_char) = pre_slash.chars().next_back()
        && matches!(last_char, '(' | ',' | '=' | '!' | '&' | '|' | ';' | '{' | '~')
    {
        return true;
    }

    pre_slash.is_empty()
}

/// Return true when the text immediately before a `/` is one of the explicit regex operators.
fn pre_slash_has_regex_op(pre_slash: &str) -> bool {
    let trimmed = pre_slash.trim_end();
    for op in &["qr", "m", "s", "tr", "y"] {
        if let Some(before_op) = trimmed.strip_suffix(op) {
            let boundary_ok = before_op
                .chars()
                .next_back()
                .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
            if boundary_ok {
                return true;
            }
        }
    }
    false
}

/// Return true when the cursor is positioned in the flag region after a closing regex delimiter.
pub(crate) fn is_in_regex_flags(source: &str, position: usize) -> bool {
    if position == 0 || position > source.len() {
        return false;
    }
    let before = &source[..position];
    let flag_chars: &[char] = &['g', 'i', 'm', 's', 'x', 'e', 'r', 'a', 'd', 'u', 'p', 'l', 'c'];
    let without_flags = before.trim_end_matches(|c: char| flag_chars.contains(&c));
    let close_pos = without_flags.len();
    if close_pos >= 2 && without_flags.ends_with('/') && is_in_regex(source, close_pos - 1) {
        return true;
    }

    let body = without_flags.trim();
    is_multi_delim_regex_at_close(body)
}

fn is_multi_delim_regex_at_close(text: &str) -> bool {
    let (op_len, required_slashes) = if text.starts_with("tr/") || text.starts_with("y/") {
        let op = if text.starts_with("tr/") { 2 } else { 1 };
        (op, 3usize)
    } else if text.starts_with("s/") {
        (1, 3usize)
    } else if text.starts_with("m") || text.starts_with("qr") {
        return is_m_or_qr_closed(text);
    } else {
        let stripped = text
            .find("=~")
            .map(|p| text[p + 2..].trim_start())
            .or_else(|| text.find("!~").map(|p| text[p + 2..].trim_start()));
        if let Some(rhs) = stripped {
            return is_multi_delim_regex_at_close(rhs);
        }
        return false;
    };
    let body_after_op = &text[op_len..];
    let slash_count = count_unescaped_slashes(body_after_op);
    slash_count == required_slashes
}

fn is_m_or_qr_closed(text: &str) -> bool {
    let (op_len, delim_pos) =
        if text.starts_with("qr") { (2usize, 2usize) } else { (1usize, 1usize) };
    let Some(delim) = text[delim_pos..].chars().next() else {
        return false;
    };
    if delim.is_ascii_alphanumeric() || delim.is_ascii_whitespace() {
        return false;
    }
    let close_delim = matching_close_delimiter(delim);
    let body = &text[(op_len + delim.len_utf8())..];
    body.ends_with(close_delim)
}

fn matching_close_delimiter(open: char) -> char {
    match open {
        '(' => ')',
        '{' => '}',
        '[' => ']',
        '<' => '>',
        _ => open,
    }
}

fn count_unescaped_slashes(s: &str) -> usize {
    let mut count = 0usize;
    let mut escaped = false;
    for ch in s.chars() {
        if escaped {
            escaped = false;
        } else if ch == '\\' {
            escaped = true;
        } else if ch == '/' {
            count += 1;
        }
    }
    count
}

/// Simple heuristic to check if position is in a comment.
pub(super) fn is_in_comment(source: &str, position: usize) -> bool {
    let line_start = source[..position].rfind('\n').map_or(0, |p| p + 1);
    let line = &source[line_start..position];
    line.contains('#')
}

/// Check if position is inside a heredoc literal.
///
/// A heredoc starts with `<<DELIMITER` or `<<'DELIMITER'` or `<<"DELIMITER"` and
/// ends when a line contains only the delimiter (and optional trailing whitespace/newline).
pub(super) fn is_in_heredoc(source: &str, position: usize) -> bool {
    is_in_heredoc_with_boundary(source, position, false)
}

fn is_in_heredoc_or_closing_line(source: &str, position: usize) -> bool {
    is_in_heredoc_with_boundary(source, position, true)
}

fn is_in_heredoc_with_boundary(source: &str, position: usize, include_closing_line: bool) -> bool {
    if position == 0 {
        return false;
    }

    let position = position.min(source.len());
    let mut active_delimiters: std::collections::VecDeque<HeredocDelimiter> =
        std::collections::VecDeque::new();
    let mut in_pod_block = false;
    let mut literal_state = LiteralScanState::default();
    let mut line_start = 0usize;

    for raw_line in source.split_inclusive('\n') {
        let line_end = line_start + raw_line.len();
        let line = strip_line_ending(raw_line);

        if let Some(delimiter) = active_delimiters.front() {
            if delimiter.matches_close(line) {
                if position >= line_start && position < line_end {
                    return include_closing_line;
                }
                active_delimiters.pop_front();
            } else if position >= line_start && position < line_end {
                return true;
            }
        } else {
            if is_pod_end_marker(line) {
                in_pod_block = false;
                if position >= line_start && position < line_end {
                    return false;
                }
                line_start = line_end;
                continue;
            }

            if in_pod_block {
                if position >= line_start && position < line_end {
                    return false;
                }
                line_start = line_end;
                continue;
            }

            let started_in_literal = literal_state.is_active();
            if is_pod_start_marker(line) && !started_in_literal {
                in_pod_block = true;
                if position >= line_start && position < line_end {
                    return false;
                }
                line_start = line_end;
                continue;
            }

            if position >= line_start && position < line_end {
                return false;
            }

            let resumed_code_index =
                literal_state.scan_segment(source.as_bytes(), line_start, line_end);
            if started_in_literal {
                if let Some(resumed_code_index) = resumed_code_index {
                    active_delimiters.extend(extract_heredoc_delimiters_from_source_line(
                        source,
                        line,
                        line_end,
                        resumed_code_index - line_start,
                    ));
                }
            } else {
                active_delimiters
                    .extend(extract_heredoc_delimiters_from_source_line(source, line, line_end, 0));
            }
        }

        line_start = line_end;
    }

    !active_delimiters.is_empty() && position >= line_start
}

fn strip_line_ending(line: &str) -> &str {
    let without_lf = line.strip_suffix('\n').unwrap_or(line);
    without_lf.strip_suffix('\r').unwrap_or(without_lf)
}

fn extract_heredoc_delimiters_from_code_line_from(
    line: &str,
    start_index: usize,
) -> Vec<HeredocDelimiter> {
    let bytes = line.as_bytes();
    let mut delimiters = Vec::new();
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_backtick = false;
    let mut literal: Option<ActiveLiteral> = None;
    let mut escaped = false;
    let mut index = start_index;

    while index + 1 < bytes.len() {
        let byte = bytes[index];

        if escaped {
            escaped = false;
            index += 1;
            continue;
        }

        if let Some(active_literal) = literal.as_mut() {
            if active_literal.advance(byte, &mut escaped) {
                literal = None;
            }
            index += 1;
            continue;
        }

        match byte {
            b'\\' if in_single_quote || in_double_quote || in_backtick => {
                escaped = true;
            }
            b'\'' if !in_double_quote && !in_backtick => {
                in_single_quote = !in_single_quote;
            }
            b'"' if !in_single_quote && !in_backtick => {
                in_double_quote = !in_double_quote;
            }
            b'`' if !in_single_quote && !in_double_quote => {
                in_backtick = !in_backtick;
            }
            b'#' if !in_single_quote && !in_double_quote && !in_backtick => {
                break;
            }
            _ if !in_single_quote && !in_double_quote && !in_backtick => {
                if let Some(literal_start) = quote_like_literal_start(bytes, index) {
                    let consumed = literal_start.consumed;
                    literal = Some(ActiveLiteral::new(literal_start));
                    index += consumed;
                    continue;
                }
                if let Some(literal_start) = slash_regex_literal_start(bytes, index) {
                    let consumed = literal_start.consumed;
                    literal = Some(ActiveLiteral::new(literal_start));
                    index += consumed;
                    continue;
                }

                if byte == b'<' && bytes[index + 1] == b'<' {
                    let after_marker = &line[index + 2..];
                    if is_heredoc_operator_context(line, index)
                        && let Some(mut delimiter) = extract_heredoc_delimiter(after_marker)
                    {
                        let no_space_bareword = no_space_bareword_before_marker(line, index);
                        let no_space_requires_future =
                            is_ambiguous_no_space_output_filehandle_context(line, index)
                                || no_space_bareword
                                    .is_some_and(is_ambiguous_no_space_bareword_word);
                        delimiter.requires_future_close = no_space_requires_future
                            || is_ambiguous_spaced_keyword_bareword_context(line, index);
                        delimiter.ignore_future_body_heredocs =
                            is_no_space_candidate_body_context(line, index);
                        delimiter.no_space_bareword = no_space_bareword.map(str::to_string);
                        delimiters.push(delimiter);
                    }
                    index += 2;
                    continue;
                }
            }
            _ => {}
        }

        index += 1;
    }

    delimiters
}

fn extract_heredoc_delimiters_from_source_line(
    source: &str,
    line: &str,
    line_end: usize,
    start_index: usize,
) -> Vec<HeredocDelimiter> {
    extract_heredoc_delimiters_from_code_line_from(line, start_index)
        .into_iter()
        .filter(|delimiter| {
            !delimiter.requires_future_close
                || (!delimiter
                    .no_space_bareword
                    .as_deref()
                    .is_some_and(|word| has_constant_declaration_before(source, line_end, word))
                    && has_future_heredoc_close(source, line_end, delimiter))
        })
        .collect()
}

fn has_future_heredoc_close(source: &str, line_end: usize, delimiter: &HeredocDelimiter) -> bool {
    let Some(rest) = source.get(line_end..) else {
        return false;
    };

    let mut active_delimiters: std::collections::VecDeque<HeredocDelimiter> =
        std::collections::VecDeque::new();
    let mut literal_state = LiteralScanState::default();
    let mut in_pod_block = false;
    let mut line_start = line_end;

    for raw_line in rest.split_inclusive('\n') {
        let next_line_start = line_start + raw_line.len();
        let line = strip_line_ending(raw_line);

        if let Some(active_delimiter) = active_delimiters.front() {
            if active_delimiter.matches_close(line) {
                active_delimiters.pop_front();
            }
            line_start = next_line_start;
            continue;
        }

        let started_in_literal = literal_state.is_active();
        let mut ignored_pod_line = in_pod_block;

        if !started_in_literal {
            if is_pod_end_marker(line) {
                in_pod_block = false;
                ignored_pod_line = true;
            } else if is_pod_start_marker(line) {
                in_pod_block = true;
                ignored_pod_line = true;
            }
        }

        if !started_in_literal && !ignored_pod_line && delimiter.matches_close(line) {
            return true;
        }

        let resumed_code_index =
            literal_state.scan_segment(source.as_bytes(), line_start, next_line_start);
        if !ignored_pod_line && !delimiter.ignore_future_body_heredocs {
            if started_in_literal {
                if let Some(resumed_code_index) = resumed_code_index {
                    active_delimiters.extend(extract_heredoc_delimiters_from_source_line(
                        source,
                        line,
                        next_line_start,
                        resumed_code_index - line_start,
                    ));
                }
            } else {
                active_delimiters.extend(extract_heredoc_delimiters_from_source_line(
                    source,
                    line,
                    next_line_start,
                    0,
                ));
            }
        }
        line_start = next_line_start;
    }

    false
}

fn is_heredoc_operator_context(line: &str, marker_index: usize) -> bool {
    let raw_before_marker = &line[..marker_index];
    let has_space_before_marker = raw_before_marker.ends_with([' ', '\t']);
    let before_marker = raw_before_marker.trim_end_matches([' ', '\t']);
    let Some(last_char) = before_marker.chars().next_back() else {
        return true;
    };

    if is_expression_prefix_char(last_char) {
        return true;
    }

    match last_char {
        '}' => is_braced_output_filehandle_context(before_marker),
        ')' | ']' | '\'' | '"' | '`' | '0'..='9' => false,
        'A'..='Z' | 'a'..='z' | '_' => {
            let word_start = ascii_word_start(before_marker);
            let before_word = &before_marker[..word_start];
            let word = &before_marker[word_start..];

            if before_word
                .chars()
                .next_back()
                .is_some_and(|ch| matches!(ch, '$' | '@' | '%' | '&' | '*'))
            {
                return is_output_filehandle_context(before_word);
            }

            if is_bareword_output_filehandle_context(before_word) {
                return true;
            }

            if has_space_before_marker && is_bareword_shift_operand_context(before_word) {
                return false;
            }

            !word.is_empty()
        }
        _ => true,
    }
}

fn is_ambiguous_no_space_output_filehandle_context(line: &str, marker_index: usize) -> bool {
    let raw_before_marker = &line[..marker_index];
    if raw_before_marker.ends_with([' ', '\t']) {
        return false;
    }

    let before_marker = raw_before_marker.trim_end_matches([' ', '\t']);
    let word_start = before_marker
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_alphanumeric() && *ch != '_')
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    let before_word = &before_marker[..word_start];
    let word = &before_marker[word_start..];

    is_bareword_output_filehandle_context(before_word) && !word.is_empty()
}

fn is_ambiguous_no_space_bareword_context(line: &str, marker_index: usize) -> bool {
    no_space_bareword_before_marker(line, marker_index)
        .is_some_and(is_ambiguous_no_space_bareword_word)
}

fn no_space_bareword_before_marker(line: &str, marker_index: usize) -> Option<&str> {
    let raw_before_marker = &line[..marker_index];
    if raw_before_marker.ends_with([' ', '\t']) {
        return None;
    }

    let before_marker = raw_before_marker.trim_end_matches([' ', '\t']);
    let word_start = ascii_word_start(before_marker);
    let word = &before_marker[word_start..];

    (!word.is_empty()).then_some(word)
}

fn is_ambiguous_no_space_bareword_word(word: &str) -> bool {
    !is_no_space_heredoc_keyword(word)
}

fn is_no_space_candidate_body_context(line: &str, marker_index: usize) -> bool {
    if is_ambiguous_no_space_output_filehandle_context(line, marker_index) {
        return true;
    }
    if !is_ambiguous_no_space_bareword_context(line, marker_index) {
        return false;
    }

    let raw_before_marker = &line[..marker_index];
    let before_marker = raw_before_marker.trim_end_matches([' ', '\t']);
    let word_start = before_marker
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_alphanumeric() && *ch != '_')
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    let before_word = before_marker[..word_start].trim_end_matches([' ', '\t']);

    before_word.is_empty()
        || before_word.chars().next_back().is_some_and(|ch| matches!(ch, ';' | '{' | '}'))
}

fn is_ambiguous_spaced_keyword_bareword_context(line: &str, marker_index: usize) -> bool {
    let raw_before_marker = &line[..marker_index];
    if !raw_before_marker.ends_with([' ', '\t']) {
        return false;
    }

    let before_marker = raw_before_marker.trim_end_matches([' ', '\t']);
    let word_start = before_marker
        .char_indices()
        .rev()
        .find(|(_, ch)| !ch.is_ascii_alphanumeric() && *ch != '_')
        .map_or(0, |(index, ch)| index + ch.len_utf8());
    let before_word = before_marker[..word_start].trim_end_matches([' ', '\t']);
    let word = &before_marker[word_start..];

    !word.is_empty()
        && matches!(
            before_word.split_ascii_whitespace().next_back(),
            Some("return" | "if" | "unless" | "while" | "until" | "for" | "foreach")
        )
}

fn is_braced_output_filehandle_context(before_marker: &str) -> bool {
    let Some(open_brace_index) = before_marker.rfind('{') else {
        return false;
    };
    let prefix = before_marker[..open_brace_index].trim_end_matches([' ', '\t']);
    is_output_filehandle_keyword(prefix.split_ascii_whitespace().next_back())
}

fn is_output_filehandle_context(before_word: &str) -> bool {
    let prefix = before_word.trim_end_matches([' ', '\t']);
    let Some((sigil_index, _)) = prefix
        .char_indices()
        .next_back()
        .filter(|(_, ch)| matches!(ch, '$' | '@' | '%' | '&' | '*'))
    else {
        return false;
    };
    let prefix = &prefix[..sigil_index];
    is_output_filehandle_keyword(prefix.split_ascii_whitespace().next_back())
}

fn is_output_filehandle_keyword(keyword: Option<&str>) -> bool {
    matches!(keyword, Some("print" | "say" | "printf"))
}

fn is_bareword_output_filehandle_context(before_word: &str) -> bool {
    is_output_filehandle_keyword(before_word.split_ascii_whitespace().next_back())
}

fn is_no_space_heredoc_keyword(word: &str) -> bool {
    matches!(word, "print" | "say" | "warn" | "die")
}

fn is_bareword_shift_operand_context(before_word: &str) -> bool {
    let prefix = before_word.trim_end_matches([' ', '\t']);
    if prefix.is_empty() || prefix.split_ascii_whitespace().next_back() == Some("print") {
        return false;
    }

    prefix.chars().next_back().is_some_and(is_expression_prefix_char)
}

fn is_expression_prefix_char(ch: char) -> bool {
    matches!(
        ch,
        '=' | '('
            | '['
            | '{'
            | ','
            | ';'
            | ':'
            | '?'
            | '!'
            | '~'
            | '+'
            | '-'
            | '*'
            | '/'
            | '%'
            | '&'
            | '|'
            | '^'
    )
}

struct QuoteLikeLiteral {
    opener: u8,
    closer: u8,
    sections: usize,
    consumed: usize,
}

struct ActiveLiteral {
    opener: u8,
    closer: u8,
    sections_remaining: usize,
    depth: usize,
    awaiting_section_opener: bool,
}

impl ActiveLiteral {
    fn new(literal: QuoteLikeLiteral) -> Self {
        Self {
            opener: literal.opener,
            closer: literal.closer,
            sections_remaining: literal.sections,
            depth: 1,
            awaiting_section_opener: false,
        }
    }

    fn advance(&mut self, byte: u8, escaped: &mut bool) -> bool {
        if *escaped {
            *escaped = false;
            return false;
        }

        if byte == b'\\' {
            *escaped = true;
            return false;
        }

        if self.awaiting_section_opener {
            if byte == b';' {
                return true;
            } else if let Some(closer) = quote_like_closer(byte) {
                self.opener = byte;
                self.closer = closer;
                self.awaiting_section_opener = false;
                self.depth = 1;
            }
            return false;
        }

        if self.opener != self.closer && byte == self.opener {
            self.depth += 1;
            return false;
        }

        if byte != self.closer {
            return false;
        }

        self.depth = self.depth.saturating_sub(1);
        if self.depth > 0 {
            return false;
        }

        self.sections_remaining = self.sections_remaining.saturating_sub(1);
        if self.sections_remaining == 0 {
            return true;
        }

        if self.opener == self.closer {
            self.depth = 1;
        } else {
            self.awaiting_section_opener = true;
        }
        false
    }
}

#[derive(Default)]
struct LiteralScanState {
    in_single_quote: bool,
    in_double_quote: bool,
    in_backtick: bool,
    literal: Option<ActiveLiteral>,
    pending_literal_body_start: Option<usize>,
    escaped: bool,
}

impl LiteralScanState {
    fn is_active(&self) -> bool {
        self.in_single_quote || self.in_double_quote || self.in_backtick || self.literal.is_some()
    }

    fn scan_segment(&mut self, bytes: &[u8], mut index: usize, end: usize) -> Option<usize> {
        let started_active = self.is_active();
        let mut resumed_code_index = None;

        loop {
            if let Some(body_start) = self.pending_literal_body_start {
                match index.cmp(&body_start) {
                    Ordering::Less => match body_start.cmp(&end) {
                        Ordering::Less => index = body_start,
                        Ordering::Equal | Ordering::Greater => break,
                    },
                    Ordering::Equal | Ordering::Greater => {}
                }
                self.pending_literal_body_start = None;
            }

            let Some(byte) = bytes.get(index..end).and_then(|remaining| remaining.first()).copied()
            else {
                break;
            };

            if self.escaped {
                self.escaped = false;
                index += 1;
                continue;
            }

            if let Some(active_literal) = self.literal.as_mut() {
                if active_literal.advance(byte, &mut self.escaped) {
                    self.literal = None;
                    if started_active && resumed_code_index.is_none() {
                        resumed_code_index = Some(index + 1);
                    }
                }
                index += 1;
                continue;
            }

            match byte {
                b'\\' if self.in_single_quote || self.in_double_quote || self.in_backtick => {
                    self.escaped = true
                }
                b'\'' if !self.in_double_quote && !self.in_backtick => {
                    self.in_single_quote = !self.in_single_quote;
                    if started_active && !self.in_single_quote && resumed_code_index.is_none() {
                        resumed_code_index = Some(index + 1);
                    }
                }
                b'"' if !self.in_single_quote && !self.in_backtick => {
                    self.in_double_quote = !self.in_double_quote;
                    if started_active && !self.in_double_quote && resumed_code_index.is_none() {
                        resumed_code_index = Some(index + 1);
                    }
                }
                b'`' if !self.in_single_quote && !self.in_double_quote => {
                    self.in_backtick = !self.in_backtick;
                    if started_active && !self.in_backtick && resumed_code_index.is_none() {
                        resumed_code_index = Some(index + 1);
                    }
                }
                b'#' if !self.in_single_quote && !self.in_double_quote && !self.in_backtick => {
                    break;
                }
                _ if !self.in_single_quote && !self.in_double_quote && !self.in_backtick => {
                    if let Some(literal_start) = quote_like_literal_start(bytes, index) {
                        let consumed = literal_start.consumed;
                        self.literal = Some(ActiveLiteral::new(literal_start));
                        let body_start = index + consumed;
                        match body_start.cmp(&end) {
                            Ordering::Greater => {
                                self.pending_literal_body_start = Some(body_start);
                                index = end;
                            }
                            Ordering::Less | Ordering::Equal => index = body_start,
                        }
                        continue;
                    }
                    if let Some(literal_start) = slash_regex_literal_start(bytes, index) {
                        let consumed = literal_start.consumed;
                        self.literal = Some(ActiveLiteral::new(literal_start));
                        index += consumed;
                        continue;
                    }
                }
                _ => {}
            }

            index += 1;
        }

        resumed_code_index
    }
}

fn quote_like_literal_start(bytes: &[u8], index: usize) -> Option<QuoteLikeLiteral> {
    if !quote_like_operator_boundary(bytes, index) {
        return None;
    }
    if quote_like_follows_sub_declaration(bytes, index) {
        return None;
    }
    if quote_like_follows_method_or_qualified_name(bytes, index) {
        return None;
    }
    if quote_like_is_file_test_s_operator(bytes, index) {
        return None;
    }

    let (delimiter_index, sections, allow_space) = match bytes.get(index).copied()? {
        b'q' if bytes.get(index + 1) == Some(&b'r') => (index + 2, 1, true),
        b'q' if matches!(bytes.get(index + 1), Some(b'q' | b'w' | b'x')) => (index + 2, 1, true),
        b't' if bytes.get(index + 1) == Some(&b'r') => (index + 2, 2, true),
        b'q' => (index + 1, 1, true),
        b'm' => (index + 1, 1, true),
        b's' | b'y' => (index + 1, 2, true),
        _ => return None,
    };

    let delimiter_index =
        if allow_space { skip_ascii_space(bytes, delimiter_index) } else { delimiter_index };
    if bytes.get(delimiter_index..delimiter_index + 2) == Some(b"=>") {
        return None;
    }

    let opener = bytes.get(delimiter_index).copied()?;
    let closer = quote_like_closer(opener)?;
    Some(QuoteLikeLiteral { opener, closer, sections, consumed: delimiter_index + 1 - index })
}

fn quote_like_is_file_test_s_operator(bytes: &[u8], index: usize) -> bool {
    if bytes.get(index) != Some(&b's') {
        return false;
    }

    let mut before = index;
    while before > 0 && matches!(bytes.get(before - 1), Some(b' ' | b'\t')) {
        before -= 1;
    }

    if bytes.get(before.saturating_sub(1)) != Some(&b'-') {
        return false;
    }

    before <= 1
        || bytes.get(before - 2).is_none_or(|byte| !byte.is_ascii_alphanumeric() && *byte != b'_')
}

fn quote_like_follows_sub_declaration(bytes: &[u8], index: usize) -> bool {
    let mut before = index;
    while before > 0 && matches!(bytes.get(before - 1), Some(b' ' | b'\t')) {
        before -= 1;
    }

    let word_end = before;
    while before > 0
        && bytes.get(before - 1).is_some_and(|byte| byte.is_ascii_alphanumeric() || *byte == b'_')
    {
        before -= 1;
    }

    before < word_end && bytes.get(before..word_end) == Some(b"sub")
}

fn quote_like_follows_method_or_qualified_name(bytes: &[u8], index: usize) -> bool {
    let mut before = index;
    while before > 0 && matches!(bytes.get(before - 1), Some(b' ' | b'\t')) {
        before -= 1;
    }

    before >= 2 && matches!(bytes.get(before - 2..before), Some(b"->") | Some(b"::"))
}

fn slash_regex_literal_start(bytes: &[u8], index: usize) -> Option<QuoteLikeLiteral> {
    if bytes.get(index) != Some(&b'/')
        || !(slash_follows_binding_operator(bytes, index)
            || slash_starts_bare_regex_literal(bytes, index))
    {
        return None;
    }

    Some(QuoteLikeLiteral { opener: b'/', closer: b'/', sections: 1, consumed: 1 })
}

fn slash_follows_binding_operator(bytes: &[u8], index: usize) -> bool {
    let mut before = index;
    while before > 0 && matches!(bytes.get(before - 1), Some(b' ' | b'\t')) {
        before -= 1;
    }

    before >= 2
        && matches!(bytes.get(before - 2), Some(b'=' | b'!'))
        && bytes.get(before - 1) == Some(&b'~')
}

fn slash_starts_bare_regex_literal(bytes: &[u8], index: usize) -> bool {
    let before = bytes[..index]
        .iter()
        .rposition(|byte| !matches!(byte, b' ' | b'\t'))
        .map_or(0, |position| position + 1);

    if let 0 = before {
        return true;
    }

    if bytes.get(before - 1).is_some_and(|byte| {
        matches!(*byte, b'(' | b',' | b'=' | b'!' | b'&' | b'|' | b';' | b'{' | b'~')
    }) {
        return true;
    }

    let word_start = bytes[..before]
        .iter()
        .rposition(|byte| !is_identifier_byte(*byte))
        .map_or(0, |position| position + 1);
    matches!(
        bytes.get(word_start..before),
        Some(
            b"and"
                | b"do"
                | b"eval"
                | b"for"
                | b"foreach"
                | b"given"
                | b"grep"
                | b"if"
                | b"map"
                | b"not"
                | b"or"
                | b"return"
                | b"split"
                | b"unless"
                | b"until"
                | b"when"
                | b"while"
        )
    )
}

fn ascii_word_start(text: &str) -> usize {
    text.as_bytes()
        .iter()
        .rposition(|byte| !is_identifier_byte(*byte))
        .map_or(0, |position| position + 1)
}

fn quote_like_operator_boundary(bytes: &[u8], index: usize) -> bool {
    index == 0
        || bytes.get(index.saturating_sub(1)).is_none_or(|byte| {
            !byte.is_ascii_alphanumeric()
                && *byte != b'_'
                && !matches!(*byte, b'$' | b'@' | b'%' | b'&' | b'*')
        })
}

fn skip_ascii_space(bytes: &[u8], mut index: usize) -> usize {
    while bytes.get(index).is_some_and(u8::is_ascii_whitespace) {
        index += 1;
    }
    index
}

fn quote_like_closer(opener: u8) -> Option<u8> {
    match opener {
        b'/' => Some(b'/'),
        b'{' => Some(b'}'),
        b'[' => Some(b']'),
        b'(' => Some(b')'),
        b'<' => Some(b'>'),
        _ if opener.is_ascii_punctuation() => Some(opener),
        _ => None,
    }
}

struct HeredocDelimiter {
    label: String,
    allow_indented_close: bool,
    requires_future_close: bool,
    ignore_future_body_heredocs: bool,
    no_space_bareword: Option<String>,
}

impl HeredocDelimiter {
    fn matches_close(&self, line: &str) -> bool {
        if self.allow_indented_close {
            line.trim_start_matches([' ', '\t']) == self.label
        } else {
            line == self.label
        }
    }
}

/// Extract the heredoc delimiter from text immediately after `<<`
fn extract_heredoc_delimiter(text: &str) -> Option<HeredocDelimiter> {
    let (text, has_leading_space) = strip_horizontal_space(text);
    let (text, allow_indented_close, has_label_leading_space) = if has_leading_space {
        (text, false, true)
    } else if let Some(rest) = text.strip_prefix('~') {
        let (rest, has_space_after_tilde) = strip_horizontal_space(rest);
        (rest, true, has_space_after_tilde)
    } else {
        (text, false, false)
    };
    let first_char = text.chars().next()?;

    let label = match first_char {
        // Quoted forms: <<'EOF', <<"EOF", <<`EOF`, etc.
        '\'' | '"' | '`' => parse_quoted_heredoc_label(text, first_char)?,
        // Non-interpolating bare form: <<\EOF
        '\\' if !has_label_leading_space => {
            let rest = &text[first_char.len_utf8()..];
            let first_rest_char = rest.chars().next()?;
            if !first_rest_char.is_ascii_alphanumeric() && first_rest_char != '_' {
                return None;
            }
            let end =
                rest.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(rest.len());
            rest[..end].to_string()
        }
        // Bare form: <<EOF
        _ if !has_label_leading_space
            && (first_char.is_ascii_alphanumeric() || first_char == '_') =>
        {
            let end =
                text.find(|c: char| !c.is_ascii_alphanumeric() && c != '_').unwrap_or(text.len());
            text[..end].to_string()
        }
        _ => return None,
    };

    Some(HeredocDelimiter {
        label,
        allow_indented_close,
        requires_future_close: false,
        ignore_future_body_heredocs: false,
        no_space_bareword: None,
    })
}

fn has_constant_declaration_before(source: &str, position: usize, name: &str) -> bool {
    source[..position.min(source.len())].lines().any(|line| line_declares_constant(line, name))
}

fn line_declares_constant(line: &str, name: &str) -> bool {
    let line = line.split_once('#').map_or(line, |(code, _)| code);
    let Some(rest) = line.trim_start().strip_prefix("use") else {
        return false;
    };
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return false;
    }
    let Some(rest) = rest.trim_start().strip_prefix("constant") else {
        return false;
    };
    if !rest.chars().next().is_some_and(char::is_whitespace) {
        return false;
    }
    let rest = rest.trim_start();

    constant_statement_declares_name(rest, name)
}

fn constant_statement_declares_name(rest: &str, name: &str) -> bool {
    if let Some(map_body) =
        rest.strip_prefix('{').and_then(|after_open| after_open.split('}').next())
    {
        return map_body.split(',').any(|entry| constant_map_entry_declares_name(entry, name));
    }

    strip_constant_name_prefix(rest, name)
        .is_some_and(|after_name| after_name.chars().next().is_none_or(is_constant_name_boundary))
}

fn constant_map_entry_declares_name(entry: &str, name: &str) -> bool {
    strip_constant_name_prefix(entry.trim_start(), name).is_some_and(|after_name| {
        after_name
            .trim_start()
            .strip_prefix("=>")
            .is_some_and(|_| after_name.chars().next().is_none_or(is_constant_name_boundary))
    })
}

fn strip_constant_name_prefix<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    if let Some(after_name) = text.strip_prefix(name) {
        return Some(after_name);
    }

    for quote in ['"', '\''] {
        if let Some(quoted_name) = text
            .strip_prefix(quote)
            .and_then(|rest| rest.strip_prefix(name))
            .and_then(|rest| rest.strip_prefix(quote))
        {
            return Some(quoted_name);
        }
    }

    None
}

fn is_constant_name_boundary(ch: char) -> bool {
    !ch.is_ascii_alphanumeric() && ch != '_'
}

fn parse_quoted_heredoc_label(text: &str, quote: char) -> Option<String> {
    let mut label = String::new();
    let mut escaped = false;

    for ch in text[quote.len_utf8()..].chars() {
        if escaped {
            if ch == quote || ch == '\\' {
                label.push(ch);
            } else {
                label.push('\\');
                label.push(ch);
            }
            escaped = false;
            continue;
        }

        if ch == '\\' {
            escaped = true;
        } else if ch == quote {
            return Some(label);
        } else {
            label.push(ch);
        }
    }

    None
}

fn strip_horizontal_space(text: &str) -> (&str, bool) {
    let trimmed = text.trim_start_matches([' ', '\t']);
    (trimmed, trimmed.len() != text.len())
}

/// Check if position is inside a POD block.
///
/// POD blocks start with a POD directive like `=pod`, `=head1`, `=head2`, etc.
/// at the **beginning** of a line (column 0) and end with `=cut` at the beginning
/// of a line.  Per the perlpod spec, leading whitespace disqualifies a line from
/// being a POD command — `  =pod` is plain text, not a POD directive.
///
/// Lines that fall inside a heredoc are skipped so that heredoc bodies containing
/// `=pod`-like content cannot corrupt the POD state machine.
pub(super) fn is_in_pod(source: &str, position: usize) -> bool {
    if position == 0 {
        return false;
    }

    let position = position.min(source.len());
    let before = &source[..position];
    let mut line_end = before.len();

    while line_end > 0 {
        let line_start = before[..line_end].rfind('\n').map_or(0, |newline| newline + 1);
        let line = strip_line_ending(&before[line_start..line_end]);

        if is_pod_end_marker(line) {
            let in_ignored_context = is_ignored_code_context(source, line_start);
            if !in_ignored_context || has_real_pod_start_before(source, line_start) {
                return false;
            }
        } else if is_pod_start_marker(line) {
            let in_ignored_context = is_ignored_code_context(source, line_start);
            if !in_ignored_context {
                return true;
            }
        }

        if line_start == 0 {
            break;
        }
        line_end = line_start - 1;
    }

    false
}

fn is_ignored_code_context(source: &str, line_start: usize) -> bool {
    is_in_heredoc_or_closing_line(source, line_start) || is_in_multiline_literal(source, line_start)
}

fn has_real_pod_start_before(source: &str, position: usize) -> bool {
    let before = &source[..position.min(source.len())];
    let mut line_end = before.len();

    while line_end > 0 {
        let line_start = before[..line_end].rfind('\n').map_or(0, |newline| newline + 1);
        let line = strip_line_ending(&before[line_start..line_end]);

        if is_pod_end_marker(line) {
            let in_ignored_context = is_ignored_code_context(source, line_start);
            if !in_ignored_context {
                return false;
            }
        }

        if is_pod_start_marker(line) {
            let in_ignored_context = is_ignored_code_context(source, line_start);
            if !in_ignored_context {
                return true;
            }
        }

        if line_start == 0 {
            break;
        }
        line_end = line_start - 1;
    }

    false
}

fn is_in_multiline_literal(source: &str, position: usize) -> bool {
    let bytes = source.as_bytes();
    let position = position.min(bytes.len());
    let mut active_delimiters: std::collections::VecDeque<HeredocDelimiter> =
        std::collections::VecDeque::new();
    let mut in_pod_block = false;
    let mut state = LiteralScanState::default();
    let mut line_start = 0usize;

    while line_start < position {
        let line_end = bytes[line_start..position]
            .iter()
            .position(|candidate| *candidate == b'\n')
            .map_or(position, |newline_offset| line_start + newline_offset + 1);
        let line = source.get(line_start..line_end).map(strip_line_ending).unwrap_or_default();

        if let Some(delimiter) = active_delimiters.front() {
            if delimiter.matches_close(line) {
                active_delimiters.pop_front();
            }
            line_start = line_end;
            continue;
        }

        if is_pod_end_marker(line) {
            in_pod_block = false;
            line_start = line_end;
            continue;
        }

        if in_pod_block {
            line_start = line_end;
            continue;
        }

        let started_in_literal = state.is_active();
        if is_pod_start_marker(line) && !started_in_literal {
            in_pod_block = true;
            line_start = line_end;
            continue;
        }

        let segment_end = line_end.min(position);
        let resumed_code_index = state.scan_segment(bytes, line_start, segment_end);

        if !started_in_literal && line_end <= position {
            active_delimiters
                .extend(extract_heredoc_delimiters_from_source_line(source, line, line_end, 0));
        } else if let Some(resumed_code_index) = resumed_code_index
            && line_end <= position
        {
            active_delimiters.extend(extract_heredoc_delimiters_from_source_line(
                source,
                line,
                line_end,
                resumed_code_index - line_start,
            ));
        }

        line_start = line_end;
    }

    state.is_active()
}

fn is_pod_start_marker(line: &str) -> bool {
    if is_pod_end_marker(line) {
        return false;
    }

    // Per perlpod: the command paragraph must begin at column 0.
    // Do NOT use trim_start() here — indented `=pod` is not a POD directive.
    line.strip_prefix('=')
        .and_then(|rest| rest.chars().next())
        .is_some_and(|command| command.is_ascii_alphabetic())
}

fn is_pod_end_marker(line: &str) -> bool {
    // Per perlpod: `=cut` must also appear at column 0.
    line.strip_prefix("=cut")
        .is_some_and(|rest| rest.chars().next().is_none_or(char::is_whitespace))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn future_close_probe_rejects_out_of_range_start() {
        let delimiter = HeredocDelimiter {
            label: "EOF".to_string(),
            allow_indented_close: false,
            requires_future_close: true,
            ignore_future_body_heredocs: false,
            no_space_bareword: None,
        };

        assert!(!has_future_heredoc_close("<<EOF", 99, &delimiter));
    }

    #[test]
    fn future_close_probe_ignores_pod_blocks() {
        let delimiter = HeredocDelimiter {
            label: "bar".to_string(),
            allow_indented_close: false,
            requires_future_close: true,
            ignore_future_body_heredocs: false,
            no_space_bareword: None,
        };
        let source = "return foo <<bar;\n=pod\nbar\n=cut\n";
        let line_end = "return foo <<bar;\n".len();

        assert!(!has_future_heredoc_close(source, line_end, &delimiter));
    }

    #[test]
    fn heredoc_operator_context_accepts_start_of_line_marker() {
        assert!(is_heredoc_operator_context("<<EOF", 0));
    }

    #[test]
    fn constant_declaration_probe_finds_scalar_and_map_constants() {
        let source = "use constant FOO => 2;\nuse constant \"QUOTED\" => 4;\nuse constant 'SINGLE' => 5;\nuse constant { BAR => 1, BAZ => 3, \"MAPQ\" => 6, 'MAPS' => 7 };\n";

        assert!(has_constant_declaration_before(source, source.len(), "FOO"));
        assert!(has_constant_declaration_before(source, source.len(), "QUOTED"));
        assert!(has_constant_declaration_before(source, source.len(), "SINGLE"));
        assert!(has_constant_declaration_before(source, source.len(), "BAR"));
        assert!(has_constant_declaration_before(source, source.len(), "BAZ"));
        assert!(has_constant_declaration_before(source, source.len(), "MAPQ"));
        assert!(has_constant_declaration_before(source, source.len(), "MAPS"));
        assert!(!has_constant_declaration_before(source, source.len(), "BA"));
    }

    #[test]
    fn heredoc_operator_context_accepts_braced_filehandle() {
        assert_eq!(is_heredoc_operator_context("print {$fh} <<EOF", 12), true);
        assert_eq!(is_heredoc_operator_context("my $value = {$fh} <<EOF", 18), false);
    }

    #[test]
    fn heredoc_operator_context_accepts_underscore_filehandle_word() {
        assert_eq!(is_heredoc_operator_context("print OUT_FH <<EOF", 13), true);
    }

    #[test]
    fn heredoc_operator_context_rejects_sigiled_underscore_term_outside_print() {
        assert_eq!(is_heredoc_operator_context("my $_ <<EOF", 6), false);
        assert_eq!(is_heredoc_operator_context("my $out_fh <<EOF", 11), false);
    }

    #[test]
    fn filehandle_context_helpers_reject_missing_shapes() {
        assert!(!is_braced_output_filehandle_context("print }"));
        assert!(!is_output_filehandle_context("print"));
    }

    #[test]
    fn active_literal_escape_consumes_next_byte() {
        let literal = QuoteLikeLiteral { opener: b'!', closer: b'!', sections: 1, consumed: 2 };
        let mut active = ActiveLiteral::new(literal);
        let mut escaped = true;

        assert!(!active.advance(b'!', &mut escaped));
        assert!(!escaped);
    }

    #[test]
    fn pending_literal_body_past_segment_waits_for_next_scan() {
        let mut state = LiteralScanState {
            pending_literal_body_start: Some(10),
            ..LiteralScanState::default()
        };

        assert_eq!(state.scan_segment(b"q", 0, 1), None);
        assert_eq!(state.pending_literal_body_start, Some(10));
    }

    #[test]
    fn pending_literal_body_at_segment_start_resumes_scanning() {
        let mut state =
            LiteralScanState { pending_literal_body_start: Some(0), ..LiteralScanState::default() };

        assert_eq!(state.scan_segment(b"'", 0, 1), None);
        assert_eq!(state.pending_literal_body_start, None);
        assert!(state.in_single_quote);
    }

    #[test]
    fn pending_literal_body_at_segment_end_waits_for_next_scan() {
        let mut state =
            LiteralScanState { pending_literal_body_start: Some(1), ..LiteralScanState::default() };

        assert_eq!(state.scan_segment(b"q", 0, 1), None);
        assert_eq!(state.pending_literal_body_start, Some(1));
    }

    #[test]
    fn quote_like_literal_body_start_eq_segment_end_stays_active_without_pending() {
        let mut state = LiteralScanState::default();

        assert_eq!(state.scan_segment(b"q!", 0, 2), None);
        assert!(state.literal.is_some());
        assert_eq!(state.pending_literal_body_start, None);
    }

    #[test]
    fn literal_scan_index_eq_end_is_noop() {
        let mut state = LiteralScanState { in_single_quote: true, ..LiteralScanState::default() };

        assert_eq!(state.scan_segment(b"'", 1, 1), None);
        assert!(state.in_single_quote);
    }

    #[test]
    fn literal_scan_records_single_quote_resume() {
        let mut state = LiteralScanState { in_single_quote: true, ..LiteralScanState::default() };

        assert_eq!(state.scan_segment(b"abc' <<EOF", 0, 10), Some(4));
        assert!(!state.in_single_quote);
    }

    #[test]
    fn slash_regex_before_eq_zero_starts_literal() {
        assert_eq!(slash_starts_bare_regex_literal(b"/<<EOF/", 0), true);
    }

    #[test]
    fn slash_regex_can_start_after_leading_spaces() {
        assert_eq!(slash_starts_bare_regex_literal(b"   /<<EOF/", 3), true);
    }

    #[test]
    fn slash_regex_keyword_probe_keeps_underscore_inside_word() {
        assert_eq!(slash_starts_bare_regex_literal(b"not_if /<<EOF/", 7), false);
    }

    #[test]
    fn angle_quote_like_delimiter_uses_angle_closer() {
        assert_eq!(quote_like_closer(b'<'), Some(b'>'));
    }

    #[test]
    fn quoted_heredoc_label_preserves_non_quote_escape() {
        assert_eq!(parse_quoted_heredoc_label(r#""EO\nF""#, '"'), Some(r"EO\nF".to_string()));
    }
}
