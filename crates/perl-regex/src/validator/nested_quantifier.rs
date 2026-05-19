use crate::syntax::cursor::quoted_literal_end;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct GroupFrame {
    has_backtracking_quantifier: bool,
    is_atomic: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LastAtom {
    None,
    Other,
    Group { has_backtracking_quantifier: bool, is_atomic: bool },
}

pub(crate) fn detect_nested_quantifiers(pattern: &str) -> bool {
    find_nested_quantifier(pattern, 0).is_some()
}

pub(crate) fn find_nested_quantifier(pattern: &str, start_pos: usize) -> Option<usize> {
    let bytes = pattern.as_bytes();
    let mut i = 0;
    let mut group_stack = Vec::new();
    let mut last_atom = LastAtom::None;

    while i < bytes.len() {
        match bytes[i] {
            b'\\' => {
                if let Some(end) = quoted_literal_end(bytes, i) {
                    i = end;
                    last_atom = LastAtom::None;
                    continue;
                }
                i += 2;
                last_atom = LastAtom::None;
                continue;
            }
            b'[' => {
                i = skip_char_class(bytes, i + 1);
                last_atom = LastAtom::Other;
                continue;
            }
            b'(' => {
                let (next, is_atomic) = skip_group_prefix(bytes, i);
                group_stack.push(GroupFrame { has_backtracking_quantifier: false, is_atomic });
                i = next;
                last_atom = LastAtom::None;
                continue;
            }
            b')' => {
                if let Some(frame) = group_stack.pop() {
                    let has_backtracking_quantifier =
                        frame.has_backtracking_quantifier && !frame.is_atomic;
                    if has_backtracking_quantifier {
                        if let Some(parent) = group_stack.last_mut() {
                            parent.has_backtracking_quantifier = true;
                        }
                    }
                    last_atom =
                        LastAtom::Group { has_backtracking_quantifier, is_atomic: frame.is_atomic };
                } else {
                    last_atom = LastAtom::None;
                }
                i += 1;
                continue;
            }
            b'+' | b'*' | b'?' | b'{' => {
                if let Some(quantifier) = quantifier_at(bytes, i) {
                    if !quantifier.is_possessive {
                        if let LastAtom::Group { has_backtracking_quantifier: true, .. } = last_atom
                        {
                            return Some(start_pos + i);
                        }

                        if let Some(parent) = group_stack.last_mut() {
                            if !matches!(last_atom, LastAtom::Group { is_atomic: true, .. }) {
                                parent.has_backtracking_quantifier = true;
                            }
                        }
                    }
                    i += quantifier.len;
                    last_atom = LastAtom::None;
                    continue;
                }
            }
            _ => {}
        }

        last_atom = LastAtom::Other;
        i += 1;
    }

    None
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Quantifier {
    len: usize,
    is_possessive: bool,
}

fn quantifier_at(bytes: &[u8], i: usize) -> Option<Quantifier> {
    let quantifier_len = match bytes.get(i).copied() {
        Some(b'+' | b'*' | b'?') => 1,
        Some(b'{') => brace_quantifier_len(bytes, i)?,
        _ => return None,
    };
    let is_possessive = bytes.get(i + quantifier_len) == Some(&b'+');
    Some(Quantifier { len: quantifier_len + usize::from(is_possessive), is_possessive })
}

fn brace_quantifier_len(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i = start + 1;
    let mut has_digit = false;
    let mut has_comma = false;

    while i < bytes.len() {
        match bytes[i] {
            ch if ch.is_ascii_digit() => has_digit = true,
            b',' if !has_comma => has_comma = true,
            b'}' if has_digit => return Some(i - start + 1),
            _ => return None,
        }
        i += 1;
    }

    None
}

fn skip_group_prefix(bytes: &[u8], start: usize) -> (usize, bool) {
    let mut i = start + 1;
    let mut is_atomic = false;

    if bytes.get(i) == Some(&b'?') {
        i += 1;
        if bytes.get(i) == Some(&b'>') {
            is_atomic = true;
            i += 1;
        } else if i < bytes.len()
            && matches!(bytes[i], b':' | b'=' | b'!' | b'<' | b'|' | b'P' | b'#')
        {
            i += 1;
        }
    }

    (i, is_atomic)
}

fn skip_char_class(bytes: &[u8], mut i: usize) -> usize {
    while i < bytes.len() {
        match bytes[i] {
            b'\\' => i += 2,
            b']' => return i + 1,
            _ => i += 1,
        }
    }
    i
}
