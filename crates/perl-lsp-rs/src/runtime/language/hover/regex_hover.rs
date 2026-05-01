use super::*;

impl LspServer {
    /// Build a hover response when the cursor is inside a Perl regex literal.
    ///
    /// Detects `/pattern/`, `m/pattern/`, `s/pattern/repl/`, and `qr/pattern/`
    /// operators (including paired-delimiter variants) and returns a Markdown
    /// table explaining each metacharacter in the pattern.
    pub(super) fn extract_regex_hover(text: &str, offset: usize) -> Option<Value> {
        let pattern = Self::find_regex_at_offset(text, offset)?;
        let entries = Self::explain_regex(&pattern);
        if entries.is_empty() {
            return None;
        }

        let mut table = "**Regex Pattern**\n\n".to_string();
        table.push_str("| Token | Meaning |\n|-------|-------|\n");
        for (tok, desc) in &entries {
            table.push_str(&format!("| `{}` | {} |\n", tok, desc));
        }

        Some(json!({
            "contents": {
                "kind": "markdown",
                "value": table,
            },
        }))
    }

    /// Return the inner pattern string if `offset` falls inside a regex literal.
    fn find_regex_at_offset(text: &str, offset: usize) -> Option<String> {
        // Find which line contains the offset and compute the column within it.
        let mut line_start = 0usize;
        for line in text.split('\n') {
            let line_end = line_start + line.len();
            if offset <= line_end {
                let col = offset - line_start;
                return Self::find_regex_in_line(line, col);
            }
            line_start = line_end + 1; // +1 for the '\n'
        }
        None
    }

    /// Scan `line` for Perl regex operators and return the inner pattern if
    /// `col` (0-based byte index into `line`) falls inside the pattern.
    fn find_regex_in_line(line: &str, col: usize) -> Option<String> {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0usize;

        while i < len {
            // --- bare /pattern/ ---
            if bytes[i] == b'/' {
                // Reject division: preceded by alphanumeric, `_`, `)`, `]`, `}`, `'`, `"`
                // e.g. `$x / 2` or `$hash{key}/2` should not trigger regex detection.
                let is_division = i > 0 && {
                    let prev = bytes[i - 1];
                    prev.is_ascii_alphanumeric()
                        || prev == b'_'
                        || prev == b')'
                        || prev == b']'
                        || prev == b'}'
                        || prev == b'\''
                        || prev == b'"'
                };
                if !is_division {
                    let open_delim = i;
                    let pattern_start = open_delim + 1;
                    let mut j = pattern_start;
                    while j < len {
                        if bytes[j] == b'\\' {
                            j += 2;
                            continue;
                        }
                        if bytes[j] == b'/' {
                            // col inside [pattern_start, j)?
                            if col >= pattern_start && col < j {
                                return Some(line[pattern_start..j].to_string());
                            }
                            i = j + 1;
                            break;
                        }
                        j += 1;
                    }
                    if j >= len {
                        // unterminated regex — skip
                        break;
                    }
                    continue;
                }
            }

            // --- m/.../, m{...}, m(...), m[...], m<...> ---
            // --- qr/.../, qr{...}, etc. ---
            // --- s/.../.../, s{...}{...}, etc. ---
            if i + 1 < len {
                let is_m = bytes[i] == b'm';
                let is_qr = bytes[i] == b'q' && i + 2 < len && bytes[i + 1] == b'r';
                let is_s = bytes[i] == b's';

                // Operator must be followed by a non-word character to avoid
                // matching variable names / identifiers like `$str`, `some`.
                let op_end = if is_qr { i + 2 } else { i + 1 };
                let delim_pos = op_end;

                if (is_m || is_qr || is_s)
                    && delim_pos < len
                    && !bytes[delim_pos].is_ascii_alphanumeric()
                    && bytes[delim_pos] != b'_'
                    // also make sure the operator itself starts a token
                    && (i == 0
                        || (!bytes[i - 1].is_ascii_alphanumeric()
                            && bytes[i - 1] != b'_'))
                {
                    let open = bytes[delim_pos];
                    let close = Self::matching_close(open);
                    let paired = open != close;
                    let pattern_start = delim_pos + 1;
                    let mut j = pattern_start;
                    let mut depth = 1usize;
                    while j < len {
                        if bytes[j] == b'\\' {
                            j += 2;
                            continue;
                        }
                        if paired {
                            if bytes[j] == open {
                                depth += 1;
                            } else if bytes[j] == close {
                                depth -= 1;
                                if depth == 0 {
                                    break;
                                }
                            }
                        } else if bytes[j] == close {
                            break;
                        }
                        j += 1;
                    }
                    if j <= len && col >= pattern_start && col < j {
                        return Some(line[pattern_start..j].to_string());
                    }
                    i = j + 1;
                    continue;
                }
            }

            i += 1;
        }
        None
    }

    /// For paired delimiters return the matching close; otherwise return `open`.
    fn matching_close(open: u8) -> u8 {
        match open {
            b'{' => b'}',
            b'(' => b')',
            b'[' => b']',
            b'<' => b'>',
            other => other,
        }
    }

    /// Walk `pattern` and return `(token, description)` pairs for each
    /// recognisable metacharacter or metacharacter sequence.
    fn explain_regex(pattern: &str) -> Vec<(String, String)> {
        let bytes = pattern.as_bytes();
        let len = bytes.len();
        let mut entries: Vec<(String, String)> = Vec::new();
        let mut i = 0usize;

        while i < len {
            let b = bytes[i];

            match b {
                b'\\' if i + 1 < len => {
                    let next = bytes[i + 1];
                    // Handle \p{...} and \P{...} Unicode property escapes.
                    if (next == b'p' || next == b'P') && i + 2 < len && bytes[i + 2] == b'{' {
                        let prop_start = i + 3;
                        let mut k = prop_start;
                        while k < len && bytes[k] != b'}' {
                            k += 1;
                        }
                        let prop_end = if k < len { k + 1 } else { k };
                        let prop_str = pattern.get(i..prop_end).unwrap_or(if next == b'p' {
                            r"\p{}"
                        } else {
                            r"\P{}"
                        });
                        let desc = if next == b'p' {
                            "Unicode property — matches characters with this property"
                        } else {
                            "Unicode property complement — matches characters WITHOUT this property"
                        };
                        i = prop_end;
                        let prop_owned = prop_str.to_string();
                        let (final_tok, final_desc) =
                            Self::apply_quantifier(prop_owned, desc.to_string(), bytes, &mut i);
                        entries.push((final_tok, final_desc));
                        continue;
                    }
                    // Handle \N{name} — named Unicode character.
                    if next == b'N' && i + 2 < len && bytes[i + 2] == b'{' {
                        let name_start = i + 3;
                        let mut k = name_start;
                        while k < len && bytes[k] != b'}' {
                            k += 1;
                        }
                        let name_end = if k < len { k + 1 } else { k };
                        let name_str = pattern.get(i..name_end).unwrap_or(r"\N{}");
                        i = name_end;
                        let name_owned = name_str.to_string();
                        entries.push((name_owned, "Named Unicode character".to_string()));
                        continue;
                    }
                    // Handle \g{name} and \g{n} — named/numbered backreference.
                    if next == b'g' && i + 2 < len && bytes[i + 2] == b'{' {
                        let ref_start = i + 3;
                        let mut k = ref_start;
                        while k < len && bytes[k] != b'}' {
                            k += 1;
                        }
                        let ref_end = if k < len { k + 1 } else { k };
                        let ref_str = pattern.get(i..ref_end).unwrap_or(r"\g{}");
                        i = ref_end;
                        let ref_owned = ref_str.to_string();
                        entries.push((ref_owned, "Named or numbered backreference".to_string()));
                        continue;
                    }
                    // Handle \k<name> — named backreference (angle-bracket form).
                    if next == b'k' && i + 2 < len && bytes[i + 2] == b'<' {
                        let name_start = i + 3;
                        let mut k = name_start;
                        while k < len && bytes[k] != b'>' {
                            k += 1;
                        }
                        let name_end = if k < len { k + 1 } else { k };
                        let ref_str = pattern.get(i..name_end).unwrap_or(r"\k<>");
                        i = name_end;
                        let ref_owned = ref_str.to_string();
                        entries.push((
                            ref_owned,
                            "Named backreference (angle-bracket form)".to_string(),
                        ));
                        continue;
                    }
                    let (tok, desc) = match next {
                        b'd' => (r"\d", "Any decimal digit (0-9)"),
                        b'D' => (r"\D", "Any non-digit character"),
                        b'w' => (r"\w", "Any word character (alphanumeric + underscore)"),
                        b'W' => (r"\W", "Any non-word character"),
                        b's' => (r"\s", "Any whitespace character (space, tab, newline, etc.)"),
                        b'S' => (r"\S", "Any non-whitespace character"),
                        b'b' => (r"\b", "Word boundary"),
                        b'B' => (r"\B", "Non-word boundary"),
                        b'A' => (r"\A", "Start of string (unaffected by multiline mode)"),
                        b'Z' => (r"\Z", "End of string (allows optional trailing newline)"),
                        b'z' => (r"\z", "Absolute end of string"),
                        b'G' => (r"\G", "Where the previous match left off (pos())"),
                        b'n' => (r"\n", "Newline character"),
                        b't' => (r"\t", "Tab character"),
                        b'r' => (r"\r", "Carriage return character"),
                        b'f' => (r"\f", "Form feed character"),
                        b'e' => (r"\e", "Escape character"),
                        b'a' => (r"\a", "Alarm (bell) character"),
                        b'0' => (r"\0", "Null character"),
                        b'h' => (r"\h", "Horizontal whitespace (space or tab)"),
                        b'H' => (r"\H", "Non-horizontal-whitespace character"),
                        b'v' => (r"\v", "Vertical whitespace character"),
                        b'V' => (r"\V", "Non-vertical-whitespace character"),
                        b'X' => (r"\X", "Extended Unicode grapheme cluster"),
                        b'1'..=b'9' => {
                            let n = (next - b'0') as usize;
                            let tok_s = format!("\\{}", n);
                            let desc_s = format!("Backreference to capture group {}", n);
                            i += 2;
                            let (final_tok, final_desc) =
                                Self::apply_quantifier(tok_s, desc_s, bytes, &mut i);
                            entries.push((final_tok, final_desc));
                            continue;
                        }
                        _ => {
                            // Escaped literal or unrecognised — skip silently.
                            i += 2;
                            continue;
                        }
                    };
                    let tok_s = tok.to_string();
                    let desc_s = desc.to_string();
                    i += 2;
                    let (final_tok, final_desc) =
                        Self::apply_quantifier(tok_s, desc_s, bytes, &mut i);
                    entries.push((final_tok, final_desc));
                }
                b'^' => {
                    // Outside a character class (handled separately by `[`), `^`
                    // is always an anchor — at position 0 it anchors to the start
                    // of the string/line, and after `(?` it can appear inside
                    // alternatives such as `(?:^foo|^bar)`.
                    i += 1;
                    entries.push((r"^".to_string(), "Start of string/line anchor".to_string()));
                }
                b'$' => {
                    i += 1;
                    entries.push((r"$".to_string(), "End of string/line anchor".to_string()));
                }
                b'.' => {
                    let tok_s = ".".to_string();
                    let desc_s = "Any character except newline".to_string();
                    i += 1;
                    let (final_tok, final_desc) =
                        Self::apply_quantifier(tok_s, desc_s, bytes, &mut i);
                    entries.push((final_tok, final_desc));
                }
                b'(' => {
                    // Check for non-capturing group, lookaround, or named capture.
                    if i + 2 < len && bytes[i + 1] == b'?' {
                        let kind = bytes[i + 2];
                        let (prefix, desc, advance) = match kind {
                            b':' => ("(?:", "Non-capturing group", 3),
                            b'=' => ("(?=", "Positive lookahead assertion", 3),
                            b'!' => ("(?!", "Negative lookahead assertion", 3),
                            b'<' if i + 3 < len && bytes[i + 3] == b'=' => {
                                ("(?<=", "Positive lookbehind assertion", 4)
                            }
                            b'<' if i + 3 < len && bytes[i + 3] == b'!' => {
                                ("(?<!", "Negative lookbehind assertion", 4)
                            }
                            b'<' => ("(?<name>", "Named capture group (angle-bracket form)", 3),
                            b'\'' => ("(?'name'", "Named capture group (single-quote form)", 3),
                            b'#' => ("(?#", "Comment — ignored by the regex engine", 3),
                            b'|' => (
                                "(?|",
                                "Branch reset group — resets capture numbering per branch",
                                3,
                            ),
                            b'>' => ("(?>", "Atomic group (no backtracking into this group)", 3),
                            _ => ("(?", "Special group (inline modifier or other extension)", 2),
                        };
                        // Advance past the full group-open prefix so we don't
                        // re-process `?`, `:`, `=`, etc. as standalone tokens.
                        i += advance;
                        entries.push((prefix.to_string(), desc.to_string()));
                    } else {
                        i += 1;
                        entries.push((
                            "(".to_string(),
                            "Capture group — captures matched text".to_string(),
                        ));
                    }
                }
                b'[' => {
                    // Collect up to the closing `]` to show the class
                    let start = i;
                    i += 1;
                    if i < len && bytes[i] == b'^' {
                        i += 1;
                    }
                    while i < len && bytes[i] != b']' {
                        if bytes[i] == b'\\' {
                            i += 1;
                        }
                        i += 1;
                    }
                    let end = if i < len { i + 1 } else { i };
                    let cls = pattern[start..end.min(len)].to_string();
                    let desc = if cls.starts_with("[^") {
                        "Negated character class"
                    } else {
                        "Character class"
                    };
                    i = end;
                    let (final_tok, final_desc) =
                        Self::apply_quantifier(cls, desc.to_string(), bytes, &mut i);
                    entries.push((final_tok, final_desc));
                }
                b'+' => {
                    i += 1;
                    entries.push(("+".to_string(), "Quantifier: one or more".to_string()));
                }
                b'*' => {
                    i += 1;
                    entries.push(("*".to_string(), "Quantifier: zero or more".to_string()));
                }
                b'?' => {
                    i += 1;
                    entries
                        .push(("?".to_string(), "Quantifier: zero or one (optional)".to_string()));
                }
                b'|' => {
                    i += 1;
                    entries.push(("| ".to_string(), "Alternation (OR)".to_string()));
                }
                _ => {
                    i += 1;
                }
            }
        }

        entries
    }

    /// If the next byte(s) in `bytes` starting at `*pos` are a quantifier
    /// (`+`, `*`, `?`, `{n,m}`), consume them and fold into the description.
    fn apply_quantifier(
        tok: String,
        desc: String,
        bytes: &[u8],
        pos: &mut usize,
    ) -> (String, String) {
        let len = bytes.len();
        if *pos >= len {
            return (tok, desc);
        }
        let suffix = match bytes[*pos] {
            b'+' => {
                *pos += 1;
                if *pos < len && bytes[*pos] == b'+' {
                    // `++` possessive quantifier — no backtracking
                    *pos += 1;
                    ", one or more (possessive — no backtracking)"
                } else if *pos < len && bytes[*pos] == b'?' {
                    // `+?` lazy quantifier — matches as few as possible
                    *pos += 1;
                    ", one or more (lazy — matches as few as possible)"
                } else {
                    ", one or more (greedy)"
                }
            }
            b'*' => {
                *pos += 1;
                if *pos < len && bytes[*pos] == b'?' {
                    // `*?` lazy quantifier
                    *pos += 1;
                    ", zero or more (lazy — matches as few as possible)"
                } else if *pos < len && bytes[*pos] == b'+' {
                    // `*+` possessive quantifier
                    *pos += 1;
                    ", zero or more (possessive — no backtracking)"
                } else {
                    ", zero or more (greedy)"
                }
            }
            b'?' => {
                *pos += 1;
                if *pos < len && bytes[*pos] == b'?' {
                    // `??` lazy optional
                    *pos += 1;
                    ", zero or one (lazy — prefers zero)"
                } else {
                    ", zero or one (optional, greedy)"
                }
            }
            b'{' => {
                // {n} or {n,m} — collect the quantifier text and fold it in.
                let brace_start = *pos;
                *pos += 1;
                while *pos < len && bytes[*pos] != b'}' {
                    *pos += 1;
                }
                if *pos < len {
                    *pos += 1; // consume '}'
                }
                let brace_end = *pos;
                // Check for lazy ({n,m}?) or possessive ({n,m}+) suffix.
                let counted_suffix = if *pos < len && bytes[*pos] == b'?' {
                    *pos += 1;
                    ", counted repetition (lazy)"
                } else if *pos < len && bytes[*pos] == b'+' {
                    *pos += 1;
                    ", counted repetition (possessive)"
                } else {
                    ", counted repetition"
                };
                // All bytes in {…} are ASCII so from_utf8 is infallible here.
                let range = std::str::from_utf8(&bytes[brace_start..brace_end]).unwrap_or("{n}");
                return (format!("{}{}", tok, range), format!("{}{}", desc, counted_suffix));
            }
            _ => return (tok, desc),
        };
        (tok, format!("{}{}", desc, suffix))
    }
}
