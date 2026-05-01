use crate::heredoc_anti_patterns::model::Location;

pub(crate) fn build_line_starts(code: &str) -> Vec<usize> {
    let mut line_starts = Vec::new();
    line_starts.push(0);

    for (idx, byte) in code.bytes().enumerate() {
        if byte == b'\n' {
            line_starts.push(idx + 1);
        }
    }

    line_starts
}

pub(crate) fn location_from_start(line_starts: &[usize], offset: usize, start: usize) -> Location {
    let insertion = line_starts.partition_point(|&line_start| line_start <= start);
    let line = insertion.saturating_sub(1);
    let line_start = line_starts.get(line).copied().unwrap_or(0);
    let column = start.saturating_sub(line_start);

    Location { line, column, offset: offset + start }
}

pub(crate) fn mask_non_code_regions(code: &str) -> String {
    fn push_masked_char(masked: &mut String, ch: char) {
        for _ in 0..ch.len_utf8() {
            masked.push(' ');
        }
    }

    let mut masked = String::with_capacity(code.len());
    let mut in_single_quote = false;
    let mut in_double_quote = false;
    let mut in_line_comment = false;
    let mut escaped = false;

    for ch in code.chars() {
        if in_line_comment {
            if ch == '\n' {
                in_line_comment = false;
                masked.push('\n');
            } else {
                push_masked_char(&mut masked, ch);
            }
            continue;
        }

        if in_single_quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '\'' {
                in_single_quote = false;
            }
            push_masked_char(&mut masked, ch);
            continue;
        }

        if in_double_quote {
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_double_quote = false;
            }
            push_masked_char(&mut masked, ch);
            continue;
        }

        match ch {
            '#' => {
                in_line_comment = true;
                push_masked_char(&mut masked, ch);
            }
            '\'' => {
                in_single_quote = true;
                push_masked_char(&mut masked, ch);
            }
            '"' => {
                in_double_quote = true;
                push_masked_char(&mut masked, ch);
            }
            _ => masked.push(ch),
        }
    }

    masked
}
