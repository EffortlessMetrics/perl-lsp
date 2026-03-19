//! Human-readable regex explanation generator
//!
//! Produces a markdown-formatted breakdown of Perl regular expression patterns
//! for display in LSP hover tooltips.

use std::fmt::Write;

/// Generate a human-readable explanation of a Perl regex pattern.
///
/// Returns a markdown-formatted string describing each element of the pattern,
/// or `None` if the pattern is empty or trivially simple (just a literal string).
pub fn explain_regex(pattern: &str, modifiers: &str) -> Option<String> {
    if pattern.is_empty() {
        return None;
    }

    let parts = parse_pattern(pattern);

    // If every part is a literal character, the pattern is trivially simple
    if parts.iter().all(|p| matches!(p, Part::Literal(_))) {
        return None;
    }

    let mut out = String::new();

    for part in &parts {
        let _ = writeln!(out, "- {}", part.describe());
    }

    if !modifiers.is_empty() {
        let _ = writeln!(out);
        let _ = writeln!(out, "**Modifiers**: {}", explain_modifiers(modifiers));
    }

    Some(out)
}

// ---------------------------------------------------------------------------
// Internal representation
// ---------------------------------------------------------------------------

enum Part {
    Literal(String),
    Anchor(AnchorKind),
    Shorthand(ShorthandKind),
    Quantifier(QuantifierKind),
    CharClass { negated: bool, body: String },
    Group(GroupKind),
    GroupEnd,
    Alternation,
    Backreference(String),
    Dot,
}

enum AnchorKind {
    StartOfLine,
    EndOfLine,
    WordBoundary,
    NonWordBoundary,
    StartOfString,
    EndOfString,
    EndOfStringBeforeNewline,
}

enum ShorthandKind {
    Digit,
    NonDigit,
    Word,
    NonWord,
    Whitespace,
    NonWhitespace,
    HorizWhitespace,
    NonHorizWhitespace,
    VertWhitespace,
    NonVertWhitespace,
    Newline,
    Tab,
    Return,
    FormFeed,
    Alarm,
    Escape,
    UnicodeProperty(String),
    NegUnicodeProperty(String),
}

enum QuantifierKind {
    ZeroOrMore { lazy: bool },
    OneOrMore { lazy: bool },
    Optional { lazy: bool },
    Exact(String),
    Range(String),
}

enum GroupKind {
    Capturing(usize),
    NonCapturing,
    NamedCapture(String),
    LookaheadPositive,
    LookaheadNegative,
    LookbehindPositive,
    LookbehindNegative,
}

impl Part {
    fn describe(&self) -> String {
        match self {
            Part::Literal(s) => {
                if s.len() == 1 {
                    format!("`{}` — literal character", s)
                } else {
                    format!("`{}` — literal string", s)
                }
            }
            Part::Anchor(a) => match a {
                AnchorKind::StartOfLine => "`^` — start of line".into(),
                AnchorKind::EndOfLine => "`$` — end of line".into(),
                AnchorKind::WordBoundary => "`\\b` — word boundary".into(),
                AnchorKind::NonWordBoundary => "`\\B` — non-word boundary".into(),
                AnchorKind::StartOfString => "`\\A` — start of string".into(),
                AnchorKind::EndOfString => "`\\z` — end of string".into(),
                AnchorKind::EndOfStringBeforeNewline => {
                    "`\\Z` — end of string (before trailing newline)".into()
                }
            },
            Part::Shorthand(s) => match s {
                ShorthandKind::Digit => "`\\d` — digit `[0-9]`".into(),
                ShorthandKind::NonDigit => "`\\D` — non-digit".into(),
                ShorthandKind::Word => "`\\w` — word character `[a-zA-Z0-9_]`".into(),
                ShorthandKind::NonWord => "`\\W` — non-word character".into(),
                ShorthandKind::Whitespace => "`\\s` — whitespace".into(),
                ShorthandKind::NonWhitespace => "`\\S` — non-whitespace".into(),
                ShorthandKind::HorizWhitespace => "`\\h` — horizontal whitespace".into(),
                ShorthandKind::NonHorizWhitespace => "`\\H` — non-horizontal whitespace".into(),
                ShorthandKind::VertWhitespace => "`\\v` — vertical whitespace".into(),
                ShorthandKind::NonVertWhitespace => "`\\V` — non-vertical whitespace".into(),
                ShorthandKind::Newline => "`\\n` — newline".into(),
                ShorthandKind::Tab => "`\\t` — tab".into(),
                ShorthandKind::Return => "`\\r` — carriage return".into(),
                ShorthandKind::FormFeed => "`\\f` — form feed".into(),
                ShorthandKind::Alarm => "`\\a` — alarm (bell)".into(),
                ShorthandKind::Escape => "`\\e` — escape character".into(),
                ShorthandKind::UnicodeProperty(name) => {
                    format!("`\\p{{{}}}` — Unicode property '{}'", name, name)
                }
                ShorthandKind::NegUnicodeProperty(name) => {
                    format!("`\\P{{{}}}` — not Unicode property '{}'", name, name)
                }
            },
            Part::Quantifier(q) => match q {
                QuantifierKind::ZeroOrMore { lazy } => {
                    if *lazy {
                        "`*?` — zero or more (lazy)".into()
                    } else {
                        "`*` — zero or more".into()
                    }
                }
                QuantifierKind::OneOrMore { lazy } => {
                    if *lazy {
                        "`+?` — one or more (lazy)".into()
                    } else {
                        "`+` — one or more".into()
                    }
                }
                QuantifierKind::Optional { lazy } => {
                    if *lazy {
                        "`??` — optional (lazy)".into()
                    } else {
                        "`?` — optional".into()
                    }
                }
                QuantifierKind::Exact(n) => format!("`{{{}}}` — exactly {} times", n, n),
                QuantifierKind::Range(r) => format!("`{{{}}}` — {} times", r, r),
            },
            Part::CharClass { negated, body } => {
                if *negated {
                    format!("`[^{}]` — any character except: {}", body, describe_char_class(body))
                } else {
                    format!("`[{}]` — character class: {}", body, describe_char_class(body))
                }
            }
            Part::Group(g) => match g {
                GroupKind::Capturing(n) => format!("`(` — start capture group #{}", n),
                GroupKind::NonCapturing => "`(?:` — start non-capturing group".into(),
                GroupKind::NamedCapture(name) => {
                    format!("`(?<{}>` — start named capture '{}'", name, name)
                }
                GroupKind::LookaheadPositive => "`(?=` — positive lookahead".into(),
                GroupKind::LookaheadNegative => "`(?!` — negative lookahead".into(),
                GroupKind::LookbehindPositive => "`(?<=` — positive lookbehind".into(),
                GroupKind::LookbehindNegative => "`(?<!` — negative lookbehind".into(),
            },
            Part::GroupEnd => "`)` — end group".into(),
            Part::Alternation => "`|` — or".into(),
            Part::Backreference(n) => format!("`\\{}` — backreference to group #{}", n, n),
            Part::Dot => "`.` — any character (except newline)".into(),
        }
    }
}

fn describe_char_class(body: &str) -> String {
    let mut pieces = Vec::new();
    let mut chars = body.chars().peekable();

    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(&next) = chars.peek() {
                chars.next();
                match next {
                    'd' => pieces.push("digit".into()),
                    'D' => pieces.push("non-digit".into()),
                    'w' => pieces.push("word char".into()),
                    'W' => pieces.push("non-word char".into()),
                    's' => pieces.push("whitespace".into()),
                    'S' => pieces.push("non-whitespace".into()),
                    'n' => pieces.push("newline".into()),
                    't' => pieces.push("tab".into()),
                    _ => pieces.push(format!("\\{}", next)),
                }
            }
        } else if chars.peek() == Some(&'-') {
            chars.next(); // consume '-'
            if let Some(&end) = chars.peek() {
                chars.next();
                pieces.push(format!("{} to {}", c, end));
            } else {
                pieces.push(c.to_string());
                pieces.push("-".into());
            }
        } else {
            pieces.push(format!("'{}'", c));
        }
    }

    pieces.join(", ")
}

// ---------------------------------------------------------------------------
// Pattern parser
// ---------------------------------------------------------------------------

fn parse_pattern(pattern: &str) -> Vec<Part> {
    let mut parts = Vec::new();
    let mut chars = pattern.chars().peekable();
    let mut capture_count: usize = 0;
    let mut literal_buf = String::new();

    while let Some(c) = chars.next() {
        match c {
            '\\' => {
                flush_literal(&mut literal_buf, &mut parts);
                if let Some(&next) = chars.peek() {
                    chars.next();
                    match next {
                        'd' => parts.push(Part::Shorthand(ShorthandKind::Digit)),
                        'D' => parts.push(Part::Shorthand(ShorthandKind::NonDigit)),
                        'w' => parts.push(Part::Shorthand(ShorthandKind::Word)),
                        'W' => parts.push(Part::Shorthand(ShorthandKind::NonWord)),
                        's' => parts.push(Part::Shorthand(ShorthandKind::Whitespace)),
                        'S' => parts.push(Part::Shorthand(ShorthandKind::NonWhitespace)),
                        'h' => parts.push(Part::Shorthand(ShorthandKind::HorizWhitespace)),
                        'H' => parts.push(Part::Shorthand(ShorthandKind::NonHorizWhitespace)),
                        'v' => parts.push(Part::Shorthand(ShorthandKind::VertWhitespace)),
                        'V' => parts.push(Part::Shorthand(ShorthandKind::NonVertWhitespace)),
                        'n' => parts.push(Part::Shorthand(ShorthandKind::Newline)),
                        't' => parts.push(Part::Shorthand(ShorthandKind::Tab)),
                        'r' => parts.push(Part::Shorthand(ShorthandKind::Return)),
                        'f' => parts.push(Part::Shorthand(ShorthandKind::FormFeed)),
                        'a' => parts.push(Part::Shorthand(ShorthandKind::Alarm)),
                        'e' => parts.push(Part::Shorthand(ShorthandKind::Escape)),
                        'b' => parts.push(Part::Anchor(AnchorKind::WordBoundary)),
                        'B' => parts.push(Part::Anchor(AnchorKind::NonWordBoundary)),
                        'A' => parts.push(Part::Anchor(AnchorKind::StartOfString)),
                        'z' => parts.push(Part::Anchor(AnchorKind::EndOfString)),
                        'Z' => {
                            parts.push(Part::Anchor(AnchorKind::EndOfStringBeforeNewline));
                        }
                        'p' | 'P' => {
                            let neg = next == 'P';
                            let name = parse_unicode_property(&mut chars);
                            if neg {
                                parts
                                    .push(Part::Shorthand(ShorthandKind::NegUnicodeProperty(name)));
                            } else {
                                parts.push(Part::Shorthand(ShorthandKind::UnicodeProperty(name)));
                            }
                        }
                        '1'..='9' => {
                            parts.push(Part::Backreference(next.to_string()));
                        }
                        _ => {
                            // Escaped literal
                            literal_buf.push(next);
                        }
                    }
                }
            }
            '^' => {
                flush_literal(&mut literal_buf, &mut parts);
                parts.push(Part::Anchor(AnchorKind::StartOfLine));
            }
            '$' => {
                flush_literal(&mut literal_buf, &mut parts);
                parts.push(Part::Anchor(AnchorKind::EndOfLine));
            }
            '.' => {
                flush_literal(&mut literal_buf, &mut parts);
                parts.push(Part::Dot);
            }
            '*' => {
                flush_literal(&mut literal_buf, &mut parts);
                let lazy = chars.peek() == Some(&'?');
                if lazy {
                    chars.next();
                }
                parts.push(Part::Quantifier(QuantifierKind::ZeroOrMore { lazy }));
            }
            '+' => {
                flush_literal(&mut literal_buf, &mut parts);
                let lazy = chars.peek() == Some(&'?');
                if lazy {
                    chars.next();
                }
                parts.push(Part::Quantifier(QuantifierKind::OneOrMore { lazy }));
            }
            '?' => {
                flush_literal(&mut literal_buf, &mut parts);
                let lazy = chars.peek() == Some(&'?');
                if lazy {
                    chars.next();
                }
                parts.push(Part::Quantifier(QuantifierKind::Optional { lazy }));
            }
            '{' => {
                flush_literal(&mut literal_buf, &mut parts);
                let quant_body = collect_until(&mut chars, '}');
                if quant_body.contains(',') {
                    parts.push(Part::Quantifier(QuantifierKind::Range(quant_body)));
                } else {
                    parts.push(Part::Quantifier(QuantifierKind::Exact(quant_body)));
                }
            }
            '[' => {
                flush_literal(&mut literal_buf, &mut parts);
                let negated = chars.peek() == Some(&'^');
                if negated {
                    chars.next();
                }
                let body = collect_char_class(&mut chars);
                parts.push(Part::CharClass { negated, body });
            }
            '(' => {
                flush_literal(&mut literal_buf, &mut parts);
                if chars.peek() == Some(&'?') {
                    chars.next(); // consume '?'
                    match chars.peek() {
                        Some(&':') => {
                            chars.next();
                            parts.push(Part::Group(GroupKind::NonCapturing));
                        }
                        Some(&'=') => {
                            chars.next();
                            parts.push(Part::Group(GroupKind::LookaheadPositive));
                        }
                        Some(&'!') => {
                            chars.next();
                            parts.push(Part::Group(GroupKind::LookaheadNegative));
                        }
                        Some(&'<') => {
                            chars.next();
                            match chars.peek() {
                                Some(&'=') => {
                                    chars.next();
                                    parts.push(Part::Group(GroupKind::LookbehindPositive));
                                }
                                Some(&'!') => {
                                    chars.next();
                                    parts.push(Part::Group(GroupKind::LookbehindNegative));
                                }
                                _ => {
                                    // Named capture (?<name>...)
                                    let name = collect_until(&mut chars, '>');
                                    capture_count += 1;
                                    parts.push(Part::Group(GroupKind::NamedCapture(name)));
                                }
                            }
                        }
                        Some(&'P') => {
                            chars.next();
                            if chars.peek() == Some(&'<') {
                                chars.next();
                                let name = collect_until(&mut chars, '>');
                                capture_count += 1;
                                parts.push(Part::Group(GroupKind::NamedCapture(name)));
                            } else {
                                // Unknown (?P...) - treat as non-capturing
                                parts.push(Part::Group(GroupKind::NonCapturing));
                            }
                        }
                        _ => {
                            // Unknown (? extension — treat as non-capturing
                            parts.push(Part::Group(GroupKind::NonCapturing));
                        }
                    }
                } else {
                    capture_count += 1;
                    parts.push(Part::Group(GroupKind::Capturing(capture_count)));
                }
            }
            ')' => {
                flush_literal(&mut literal_buf, &mut parts);
                parts.push(Part::GroupEnd);
            }
            '|' => {
                flush_literal(&mut literal_buf, &mut parts);
                parts.push(Part::Alternation);
            }
            _ => {
                literal_buf.push(c);
            }
        }
    }
    flush_literal(&mut literal_buf, &mut parts);
    parts
}

fn flush_literal(buf: &mut String, parts: &mut Vec<Part>) {
    if !buf.is_empty() {
        parts.push(Part::Literal(buf.clone()));
        buf.clear();
    }
}

fn collect_until(chars: &mut std::iter::Peekable<std::str::Chars<'_>>, end: char) -> String {
    let mut s = String::new();
    for c in chars.by_ref() {
        if c == end {
            break;
        }
        s.push(c);
    }
    s
}

fn collect_char_class(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    let mut s = String::new();
    // Handle ] as first character in class (literal)
    if chars.peek() == Some(&']') {
        chars.next();
        s.push(']');
    }
    while let Some(c) = chars.next() {
        if c == '\\' {
            s.push(c);
            if let Some(&next) = chars.peek() {
                chars.next();
                s.push(next);
            }
        } else if c == ']' {
            break;
        } else {
            s.push(c);
        }
    }
    s
}

fn parse_unicode_property(chars: &mut std::iter::Peekable<std::str::Chars<'_>>) -> String {
    if chars.peek() == Some(&'{') {
        chars.next(); // consume '{'
        collect_until(chars, '}')
    } else if let Some(&c) = chars.peek() {
        chars.next();
        c.to_string()
    } else {
        String::new()
    }
}

fn explain_modifiers(modifiers: &str) -> String {
    let mut explanations = Vec::new();
    for c in modifiers.chars() {
        let desc = match c {
            'i' => "`i` case-insensitive",
            'm' => "`m` multiline (^ and $ match line boundaries)",
            's' => "`s` single-line (. matches newline)",
            'x' => "`x` extended (whitespace ignored, # comments)",
            'g' => "`g` global (match all occurrences)",
            'e' => "`e` evaluate replacement as code",
            'r' => "`r` return modified copy",
            'c' => "`c` keep current position after failed /g",
            'p' => "`p` preserve match variables",
            'a' => "`a` ASCII-only matching",
            'u' => "`u` Unicode matching",
            'l' => "`l` locale-dependent matching",
            'd' => "`d` default character set",
            'n' => "`n` non-capturing groups by default",
            _ => continue,
        };
        explanations.push(desc.to_string());
    }
    explanations.join(", ")
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_tdd_support::must_some;

    #[test]
    fn test_simple_literal_returns_none() {
        assert!(explain_regex("hello", "").is_none());
    }

    #[test]
    fn test_anchors_and_word_chars() {
        let result = must_some(explain_regex(r"^\w+$", ""));
        assert!(result.contains("start of line"));
        assert!(result.contains("word character"));
        assert!(result.contains("one or more"));
        assert!(result.contains("end of line"));
    }

    #[test]
    fn test_capture_groups() {
        let result = must_some(explain_regex(r"(\w+)\s*=>\s*(.*)", ""));
        assert!(result.contains("capture group #1"));
        assert!(result.contains("capture group #2"));
        assert!(result.contains("whitespace"));
        assert!(result.contains("zero or more"));
    }

    #[test]
    fn test_character_class() {
        let result = must_some(explain_regex("[a-zA-Z_]", ""));
        assert!(result.contains("character class"));
        assert!(result.contains("a to z"));
    }

    #[test]
    fn test_negated_character_class() {
        let result = must_some(explain_regex("[^0-9]", ""));
        assert!(result.contains("except"));
    }

    #[test]
    fn test_modifiers() {
        let result = must_some(explain_regex(r"\d+", "gi"));
        assert!(result.contains("case-insensitive"));
        assert!(result.contains("global"));
    }

    #[test]
    fn test_lookahead() {
        let result = must_some(explain_regex(r"foo(?=bar)", ""));
        assert!(result.contains("positive lookahead"));
    }

    #[test]
    fn test_lookbehind() {
        let result = must_some(explain_regex(r"(?<=foo)bar", ""));
        assert!(result.contains("positive lookbehind"));
    }

    #[test]
    fn test_named_capture() {
        let result = must_some(explain_regex(r"(?<name>\w+)", ""));
        assert!(result.contains("named capture 'name'"));
    }

    #[test]
    fn test_non_capturing() {
        let result = must_some(explain_regex(r"(?:foo|bar)", ""));
        assert!(result.contains("non-capturing group"));
        assert!(result.contains("or"));
    }

    #[test]
    fn test_quantifier_range() {
        let result = must_some(explain_regex(r"\d{2,4}", ""));
        assert!(result.contains("2,4"));
    }

    #[test]
    fn test_quantifier_exact() {
        let result = must_some(explain_regex(r"\d{3}", ""));
        assert!(result.contains("exactly 3 times"));
    }

    #[test]
    fn test_lazy_quantifiers() {
        let result = must_some(explain_regex(r".*?", ""));
        assert!(result.contains("lazy"));
    }

    #[test]
    fn test_backreference() {
        let result = must_some(explain_regex(r"(\w+)\s+\1", ""));
        assert!(result.contains("backreference to group #1"));
    }

    #[test]
    fn test_unicode_property() {
        let result = must_some(explain_regex(r"\p{Latin}", ""));
        assert!(result.contains("Unicode property 'Latin'"));
    }

    #[test]
    fn test_dot() {
        let result = must_some(explain_regex(".", ""));
        assert!(result.contains("any character"));
    }

    #[test]
    fn test_complex_real_world_pattern() {
        let result = must_some(explain_regex(r"^[\w.+-]+@[\w-]+\.[\w.]+$", ""));
        assert!(result.contains("start of line"));
        assert!(result.contains("end of line"));
        assert!(result.contains("character class"));
    }

    #[test]
    fn test_empty_pattern() {
        assert!(explain_regex("", "").is_none());
    }

    #[test]
    fn test_negative_lookahead() {
        let result = must_some(explain_regex(r"foo(?!bar)", ""));
        assert!(result.contains("negative lookahead"));
    }

    #[test]
    fn test_negative_lookbehind() {
        let result = must_some(explain_regex(r"(?<!foo)bar", ""));
        assert!(result.contains("negative lookbehind"));
    }
}
