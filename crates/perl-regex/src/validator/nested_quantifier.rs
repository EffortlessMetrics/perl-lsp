pub(crate) fn detect_nested_quantifiers(pattern: &str) -> bool {
    find_nested_quantifier(pattern, 0).is_some()
}

pub(crate) fn find_nested_quantifier(pattern: &str, start_pos: usize) -> Option<usize> {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut group_stack = Vec::new();
    let mut last_type = TokenType::Other;
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                i += 2;
                last_type = TokenType::Other;
                continue;
            }
            b'[' => {
                i += 1;
                while i < bytes.len() {
                    if bytes[i] == b'\\' {
                        i += 2;
                    } else if bytes[i] == b']' {
                        break;
                    } else {
                        i += 1;
                    }
                }
                last_type = TokenType::Other;
            }
            b'(' => {
                let (group, next) = parse_group_start(bytes, i);
                group_stack.push(group);
                i = next;
                last_type = TokenType::Other;
                continue;
            }
            b')' => {
                if let Some(group) = group_stack.pop() {
                    last_type = if group.has_backtracking_quantifier {
                        TokenType::QuantifiedGroup
                    } else {
                        TokenType::Other
                    };
                }
            }
            b'+' | b'*' | b'?' | b'{' => {
                let Some(quantifier) = parse_quantifier(bytes, i) else {
                    last_type = TokenType::Other;
                    i += 1;
                    continue;
                };

                if last_type == TokenType::QuantifiedGroup && quantifier.allows_backtracking {
                    return Some(start_pos + i);
                }

                if quantifier.allows_backtracking {
                    if let Some(last) = group_stack.last_mut() {
                        if !last.is_atomic {
                            last.has_backtracking_quantifier = true;
                        }
                    }
                }
                i = quantifier.next;
                last_type = TokenType::Quantifier;
                continue;
            }
            _ => last_type = TokenType::Other,
        }
        i += 1;
    }
    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TokenType {
    Other,
    Quantifier,
    QuantifiedGroup,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroupContext {
    has_backtracking_quantifier: bool,
    is_atomic: bool,
}

fn parse_group_start(bytes: &[u8], offset: usize) -> (GroupContext, usize) {
    let mut next = offset + 1;
    let mut is_atomic = false;

    if next < bytes.len() && bytes[next] == b'?' {
        next += 1;
        if next < bytes.len()
            && matches!(bytes[next], b':' | b'=' | b'!' | b'<' | b'|' | b'P' | b'#')
        {
            next += 1;
        } else if next < bytes.len() && bytes[next] == b'>' {
            is_atomic = true;
            next += 1;
        }
    }

    (GroupContext { has_backtracking_quantifier: false, is_atomic }, next)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Quantifier {
    allows_backtracking: bool,
    next: usize,
}

fn parse_quantifier(bytes: &[u8], offset: usize) -> Option<Quantifier> {
    match bytes[offset] {
        b'+' | b'*' | b'?' => Some(quantifier_with_suffix(bytes, offset + 1)),
        b'{' => {
            let mut next = offset + 1;
            if is_brace_quantifier(bytes, &mut next) {
                Some(quantifier_with_suffix(bytes, next))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn quantifier_with_suffix(bytes: &[u8], next: usize) -> Quantifier {
    match bytes.get(next) {
        Some(b'+') => Quantifier { allows_backtracking: false, next: next + 1 },
        Some(b'?') => Quantifier { allows_backtracking: true, next: next + 1 },
        _ => Quantifier { allows_backtracking: true, next },
    }
}

fn is_brace_quantifier(bytes: &[u8], i: &mut usize) -> bool {
    let mut has_digit = false;
    let mut has_comma = false;
    while *i < bytes.len() {
        let ch = bytes[*i];
        *i += 1;
        if ch.is_ascii_digit() {
            has_digit = true;
        } else if ch == b',' && !has_comma {
            has_comma = true;
        } else if ch == b'}' && has_digit {
            return true;
        } else {
            break;
        }
    }
    false
}
