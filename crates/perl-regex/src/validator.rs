use crate::error::RegexError;

enum GroupType {
    Normal,
    Lookbehind,
    BranchReset { branch_count: usize },
}

/// Validator for Perl regular expressions to prevent security and performance issues
pub struct RegexValidator {
    max_nesting: usize,
    max_unicode_properties: usize,
}

impl Default for RegexValidator {
    fn default() -> Self {
        Self::new()
    }
}

impl RegexValidator {
    /// Create a new validator with default safety limits
    pub fn new() -> Self {
        Self { max_nesting: 10, max_unicode_properties: 50 }
    }

    pub fn validate(&self, pattern: &str, start_pos: usize) -> Result<(), RegexError> {
        self.check_complexity(pattern, start_pos)
    }

    pub fn detects_code_execution(&self, pattern: &str) -> bool {
        let bytes = pattern.as_bytes();
        let mut i = 0;
        let len = bytes.len();
        while i < len {
            let ch = bytes[i];
            if ch == b'\\' {
                i += 2;
                continue;
            }
            if ch == b'[' {
                i += 1;
                while i < len {
                    let class_ch = bytes[i];
                    if class_ch == b'\\' {
                        i += 2;
                    } else if class_ch == b']' {
                        i += 1;
                        break;
                    } else {
                        i += 1;
                    }
                }
                continue;
            }
            if ch == b'(' && i + 1 < len && bytes[i + 1] == b'?' {
                i += 2;
                if i < len {
                    if bytes[i] == b'{' {
                        return true;
                    } else if bytes[i] == b'?' && i + 1 < len && bytes[i + 1] == b'{' {
                        return true;
                    }
                }
                continue;
            }
            i += 1;
        }
        false
    }

    pub fn detect_nested_quantifiers(&self, pattern: &str) -> bool {
        let bytes = pattern.as_bytes();
        let mut i = 0;
        let len = bytes.len();
        let mut group_stack = Vec::new();
        let mut last_type = 0;

        while i < len {
            let ch = bytes[i];
            match ch {
                b'\\' => {
                    i += 2;
                    last_type = 0;
                    continue;
                }
                b'(' => {
                    if i + 1 < len && bytes[i + 1] == b'?' {
                        i += 2;
                        if i < len
                            && matches!(
                                bytes[i],
                                b':' | b'=' | b'!' | b'<' | b'>' | b'|' | b'P' | b'#'
                            )
                        {
                            i += 1;
                        }
                    } else {
                        i += 1;
                    }
                    group_stack.push(false);
                    last_type = 0;
                    continue;
                }
                b')' => {
                    if let Some(has_quantifier) = group_stack.pop() {
                        last_type = if has_quantifier { 2 } else { 0 };
                    }
                }
                b'+' | b'*' | b'?' | b'{' => {
                    if last_type == 2 {
                        if ch == b'{' {
                            let mut peek_i = i + 1;
                            if Self::is_brace_quantifier(bytes, &mut peek_i) {
                                return true;
                            }
                            last_type = 0;
                            i += 1;
                            continue;
                        }
                        return true;
                    }

                    if let Some(last) = group_stack.last_mut() {
                        *last = true;
                    }
                    last_type = 1;
                }
                _ => {
                    last_type = 0;
                }
            }
            i += 1;
        }
        false
    }

    fn is_brace_quantifier(bytes: &[u8], i: &mut usize) -> bool {
        let mut has_digit = false;
        let mut has_comma = false;
        let len = bytes.len();

        while *i < len {
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

    fn check_complexity(&self, pattern: &str, start_pos: usize) -> Result<(), RegexError> {
        let bytes = pattern.as_bytes();
        let mut i = 0;
        let len = bytes.len();
        let mut stack: Vec<GroupType> = Vec::new();
        let mut unicode_property_count = 0;

        while i < len {
            let ch = bytes[i];
            match ch {
                b'\\' => {
                    if i + 1 < len {
                        let next_char = bytes[i + 1];
                        match next_char {
                            b'p' | b'P' => {
                                i += 2;
                                if i < len && bytes[i] == b'{' {
                                    unicode_property_count += 1;
                                    if unicode_property_count > self.max_unicode_properties {
                                        return Err(RegexError::syntax(
                                            "Too many Unicode properties in regex (max 50)",
                                            start_pos + i - 2,
                                        ));
                                    }
                                }
                                continue;
                            }
                            _ => {
                                i += 2;
                                continue;
                            }
                        }
                    }
                }
                b'[' => {
                    i += 1;
                    while i < len {
                        if bytes[i] == b'\\' {
                            i += 2;
                        } else if bytes[i] == b']' {
                            break;
                        } else {
                            i += 1;
                        }
                    }
                }
                b'(' => {
                    let mut group_type = GroupType::Normal;
                    if i + 1 < len && bytes[i + 1] == b'?' {
                        i += 2;
                        if i < len && bytes[i] == b'<' {
                            i += 1;
                            if i < len && (bytes[i] == b'=' || bytes[i] == b'!') {
                                i += 1;
                                group_type = GroupType::Lookbehind;
                            }
                        } else if i < len && bytes[i] == b'|' {
                            i += 1;
                            group_type = GroupType::BranchReset { branch_count: 1 };
                        }
                    } else {
                        i += 1;
                    }

                    match group_type {
                        GroupType::Lookbehind => {
                            let lookbehind_depth =
                                stack.iter().filter(|g| matches!(g, GroupType::Lookbehind)).count();
                            if lookbehind_depth >= self.max_nesting {
                                return Err(RegexError::syntax(
                                    "Regex lookbehind nesting too deep",
                                    start_pos + i - 1,
                                ));
                            }
                        }
                        GroupType::BranchReset { .. } => {
                            let reset_depth = stack
                                .iter()
                                .filter(|g| matches!(g, GroupType::BranchReset { .. }))
                                .count();
                            if reset_depth >= self.max_nesting {
                                return Err(RegexError::syntax(
                                    "Regex branch reset nesting too deep",
                                    start_pos + i - 1,
                                ));
                            }
                        }
                        GroupType::Normal => {}
                    }
                    stack.push(group_type);
                    continue;
                }
                b'|' => {
                    if let Some(GroupType::BranchReset { branch_count }) = stack.last_mut() {
                        *branch_count += 1;
                        if *branch_count > 50 {
                            return Err(RegexError::syntax(
                                "Too many branches in branch reset group (max 50)",
                                start_pos + i,
                            ));
                        }
                    }
                }
                b')' => {
                    stack.pop();
                }
                _ => {}
            }
            i += 1;
        }

        Ok(())
    }
}
