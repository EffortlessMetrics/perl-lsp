use crate::error::ParseError;

/// Validator for Perl regular expressions to prevent security and performance issues
pub struct RegexValidator {
    max_nesting: usize,
    max_unicode_properties: usize,
}

impl RegexValidator {
    /// Create a new validator with default safety limits
    pub fn new() -> Self {
        Self {
            // Default limits from issue #461
            max_nesting: 10,
            // Limit from issue #460
            max_unicode_properties: 50,
        }
    }

    /// Validate a regex pattern for potential performance or security risks
    pub fn validate(&self, pattern: &str, start_pos: usize) -> Result<(), ParseError> {
        self.check_complexity(pattern, start_pos)
    }

    /// Check if the pattern contains embedded code constructs (?{...}) or (??{...})
    pub fn detects_code_execution(&self, pattern: &str) -> bool {
        let mut chars = pattern.char_indices().peekable();
        while let Some((_, ch)) = chars.next() {
            if ch == '\\' {
                chars.next(); // skip escaped
                continue;
            }
            if ch == '(' {
                if let Some((_, '?')) = chars.peek() {
                    chars.next(); // consume ?
                    // Check for { or ?{
                    if let Some((_, next)) = chars.peek() {
                        if *next == '{' {
                            return true; // (?{
                        } else if *next == '?' {
                            chars.next(); // consume second ?
                            if let Some((_, '{')) = chars.peek() {
                                return true; // (??{
                            }
                        }
                    }
                }
            }
        }
        false
    }

    fn check_complexity(&self, pattern: &str, start_pos: usize) -> Result<(), ParseError> {
        let mut chars = pattern.char_indices().peekable();
        // Stack stores the type of the current group
        let mut stack: Vec<GroupType> = Vec::new();
        let mut unicode_property_count = 0;
        
        while let Some((idx, ch)) = chars.next() {
            match ch {
                '\\' => {
                    // Check for escaped character
                    if let Some((_, next_char)) = chars.peek() {
                        match next_char {
                            'p' | 'P' => {
                                // Unicode property start \p or \P
                                // We consume the 'p'/'P'
                                chars.next();
                                
                                // Check if it's followed by {
                                if let Some((_, '{')) = chars.peek() {
                                    unicode_property_count += 1;
                                    if unicode_property_count > self.max_unicode_properties {
                                        return Err(ParseError::syntax(
                                            "Too many Unicode properties in regex (max 50)",
                                            start_pos + idx
                                        ));
                                    }
                                }
                            }
                            _ => {
                                // Just skip other escaped chars
                                chars.next();
                            }
                        }
                    }
                }
                '(' => {
                    let mut group_type = GroupType::Normal;
                    
                    // Check for extension syntax (?...)
                    if let Some((_, '?')) = chars.peek() {
                        chars.next(); // consume ?
                        
                        // Check for < (lookbehind or named capture)
                        if let Some((_, '<')) = chars.peek() {
                            chars.next(); // consume <
                            
                            // Check for = or ! (lookbehind)
                            if matches!(chars.peek(), Some((_, '=')) | Some((_, '!'))) {
                                chars.next(); // consume = or !
                                group_type = GroupType::Lookbehind;
                            }
                            // Otherwise it's likely a named capture (?<name>...) or condition (?<...)
                            // which we treat as a normal group
                        } else if let Some((_, '|')) = chars.peek() {
                            chars.next(); // consume |
                            group_type = GroupType::BranchReset { branch_count: 1 };
                        }
                    }
                    
                    match group_type {
                        GroupType::Lookbehind => {
                            // Calculate current lookbehind depth
                            let lookbehind_depth = stack.iter().filter(|g| matches!(g, GroupType::Lookbehind)).count();
                            if lookbehind_depth >= self.max_nesting {
                                    return Err(ParseError::syntax(
                                    "Regex lookbehind nesting too deep",
                                    start_pos + idx
                                ));
                            }
                        }
                        GroupType::BranchReset { .. } => {
                            // Calculate current branch reset nesting
                            let reset_depth = stack.iter().filter(|g| matches!(g, GroupType::BranchReset { .. })).count();
                            if reset_depth >= self.max_nesting { // Use same nesting limit for now
                                return Err(ParseError::syntax(
                                    "Regex branch reset nesting too deep",
                                    start_pos + idx
                                ));
                            }
                        }
                        _ => {}
                    }
                    stack.push(group_type);
                }
                '|' => {
                    // Check if we are in a branch reset group
                    if let Some(GroupType::BranchReset { branch_count }) = stack.last_mut() {
                        *branch_count += 1;
                        if *branch_count > 50 { // Max 50 branches
                            return Err(ParseError::syntax(
                                "Too many branches in branch reset group (max 50)",
                                start_pos + idx
                            ));
                        }
                    }
                }
                ')' => {
                    stack.pop();
                }
                '[' => {
                    // Skip character class [ ... ]
                    // Need to handle escaping inside []
                    while let Some((_, c)) = chars.next() {
                        if c == '\\' {
                            chars.next();
                        } else if c == ']' {
                            break;
                        }
                    }
                }
                _ => {}
            }
        }
        
        Ok(())
    }
}

enum GroupType {
    Normal,
    Lookbehind,
    BranchReset { branch_count: usize },
}

impl Default for RegexValidator {
    fn default() -> Self {
        Self::new()
    }
}
