use crate::error::ParseError;

/// Validator for Perl regular expressions to prevent security and performance issues
pub struct RegexValidator {
    max_nesting: usize,
}

impl RegexValidator {
    /// Create a new validator with default safety limits
    pub fn new() -> Self {
        Self {
            // Default limits from issue #461
            max_nesting: 10,
        }
    }

    /// Validate a regex pattern for potential performance or security risks
    pub fn validate(&self, pattern: &str, start_pos: usize) -> Result<(), ParseError> {
        self.check_lookbehind_complexity(pattern, start_pos)
    }

    fn check_lookbehind_complexity(&self, pattern: &str, start_pos: usize) -> Result<(), ParseError> {
        let mut chars = pattern.char_indices().peekable();
        // Stack stores whether the current group is a lookbehind group
        let mut stack: Vec<bool> = Vec::new();
        
        while let Some((idx, ch)) = chars.next() {
            match ch {
                '\\' => {
                    // Skip escaped character
                    chars.next();
                }
                '(' => {
                    let mut is_lookbehind = false;
                    
                    // Check for extension syntax (?...)
                    if let Some((_, '?')) = chars.peek() {
                        chars.next(); // consume ?
                        
                        // Check for < (lookbehind or named capture)
                        if let Some((_, '<')) = chars.peek() {
                            chars.next(); // consume <
                            
                            // Check for = or ! (lookbehind)
                            if matches!(chars.peek(), Some((_, '=')) | Some((_, '!'))) {
                                chars.next(); // consume = or !
                                is_lookbehind = true;
                            }
                            // Otherwise it's likely a named capture (?<name>...) or condition (?<...)
                            // which we treat as a normal group (not lookbehind)
                        }
                    }
                    
                    if is_lookbehind {
                        // Calculate current lookbehind depth (number of true values in stack)
                        let lookbehind_depth = stack.iter().filter(|&&x| x).count();
                        if lookbehind_depth >= self.max_nesting {
                             return Err(ParseError::syntax(
                                "Regex lookbehind nesting too deep",
                                start_pos + idx
                            ));
                        }
                        stack.push(true);
                    } else {
                        stack.push(false);
                    }
                }
                ')' => {
                    stack.pop();
                }
                '[' => {
                    // Skip character class [ ... ]
                    // Need to handle escaping inside []
                    while let Some((_, c)) = chars.next() {
                        if c == '\\'
                         {
                            chars.next();
                        } else if c == ']'
                         {
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

impl Default for RegexValidator {
    fn default() -> Self {
        Self::new()
    }
}
