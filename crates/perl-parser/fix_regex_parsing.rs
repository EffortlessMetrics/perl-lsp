// Helper functions for parsing regex constructs

/// Extract modifiers from a regex/substitution/transliteration string
/// For example: /pattern/gimsx -> "gimsx"
fn extract_modifiers(text: &str) -> (String, String) {
    // Find the last delimiter (could be /, }, ], ), etc.)
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 2 {
        return (text.to_string(), String::new());
    }
    
    // Common regex delimiters
    let delimiters = [
        ('/', '/'),
        ('{', '}'),
        ('[', ']'),
        ('(', ')'),
        ('<', '>'),
        ('|', '|'),
        ('!', '!'),
        ('#', '#'),
        (',', ','),
        ('\'', '\''),
        ('"', '"'),
    ];
    
    // Find matching delimiter from the start
    let first_char = chars[0];
    let closing_delim = delimiters.iter()
        .find(|(open, _)| *open == first_char)
        .map(|(_, close)| *close)
        .unwrap_or(first_char);
    
    // Find the last occurrence of the closing delimiter
    let mut last_delim_pos = None;
    for (i, &ch) in chars.iter().enumerate().skip(1) {
        if ch == closing_delim {
            last_delim_pos = Some(i);
        }
    }
    
    if let Some(pos) = last_delim_pos {
        let pattern = chars[..=pos].iter().collect();
        let modifiers = chars[pos+1..].iter().collect();
        (pattern, modifiers)
    } else {
        (text.to_string(), String::new())
    }
}

/// Parse a substitution operator (s///)
/// Returns (pattern, replacement, modifiers)
fn parse_substitution_parts(text: &str) -> (String, String, String) {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 2 || chars[0] != 's' {
        return (text.to_string(), String::new(), String::new());
    }
    
    // Get the delimiter
    let delim = chars[1];
    let closing_delim = match delim {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        '<' => '>',
        _ => delim,
    };
    
    // Find pattern end
    let mut i = 2;
    let mut escaped = false;
    let mut pattern_end = None;
    
    while i < chars.len() {
        if !escaped && chars[i] == closing_delim {
            pattern_end = Some(i);
            break;
        }
        escaped = !escaped && chars[i] == '\\';
        i += 1;
    }
    
    let pattern_end = match pattern_end {
        Some(pos) => pos,
        None => return (text.to_string(), String::new(), String::new()),
    };
    
    // Check if we have mixed delimiters (e.g., s{foo}[bar])
    let replacement_start = pattern_end + 1;
    let (replacement_delim, replacement_closing) = if replacement_start < chars.len() {
        let next_char = chars[replacement_start];
        match next_char {
            '{' => ('{', '}'),
            '[' => ('[', ']'),
            '(' => ('(', ')'),
            '<' => ('<', '>'),
            _ => (closing_delim, closing_delim),
        }
    } else {
        (closing_delim, closing_delim)
    };
    
    // Find replacement end
    i = if replacement_delim != closing_delim { replacement_start + 1 } else { replacement_start };
    escaped = false;
    let mut replacement_end = None;
    
    while i < chars.len() {
        if !escaped && chars[i] == replacement_closing {
            replacement_end = Some(i);
            break;
        }
        escaped = !escaped && chars[i] == '\\';
        i += 1;
    }
    
    let replacement_end = match replacement_end {
        Some(pos) => pos,
        None => return (text.to_string(), String::new(), String::new()),
    };
    
    let pattern = chars[2..pattern_end].iter().collect();
    let replacement_range = if replacement_delim != closing_delim {
        (replacement_start + 1)..replacement_end
    } else {
        replacement_start..replacement_end
    };
    let replacement = chars[replacement_range].iter().collect();
    let modifiers = chars[replacement_end + 1..].iter().collect();
    
    (pattern, replacement, modifiers)
}

/// Parse a transliteration operator (tr/// or y///)
/// Returns (search_list, replace_list, modifiers)
fn parse_transliteration_parts(text: &str) -> (String, String, String) {
    let chars: Vec<char> = text.chars().collect();
    if chars.len() < 3 || (chars[0] != 't' && chars[0] != 'y') {
        return (text.to_string(), String::new(), String::new());
    }
    
    let start_pos = if chars[0] == 't' && chars[1] == 'r' { 2 } else { 1 };
    
    // Similar logic to substitution parsing
    let delim = chars[start_pos];
    let closing_delim = match delim {
        '{' => '}',
        '[' => ']',
        '(' => ')',
        '<' => '>',
        _ => delim,
    };
    
    // Find search list end
    let mut i = start_pos + 1;
    let mut search_end = None;
    
    while i < chars.len() {
        if chars[i] == closing_delim {
            search_end = Some(i);
            break;
        }
        i += 1;
    }
    
    let search_end = match search_end {
        Some(pos) => pos,
        None => return (text.to_string(), String::new(), String::new()),
    };
    
    // Check for mixed delimiters
    let replace_start = search_end + 1;
    let (replace_delim, replace_closing) = if replace_start < chars.len() {
        let next_char = chars[replace_start];
        match next_char {
            '{' => ('{', '}'),
            '[' => ('[', ']'),
            '(' => ('(', ')'),
            '<' => ('<', '>'),
            _ => (closing_delim, closing_delim),
        }
    } else {
        (closing_delim, closing_delim)
    };
    
    // Find replace list end
    i = if replace_delim != closing_delim { replace_start + 1 } else { replace_start };
    let mut replace_end = None;
    
    while i < chars.len() {
        if chars[i] == replace_closing {
            replace_end = Some(i);
            break;
        }
        i += 1;
    }
    
    let replace_end = match replace_end {
        Some(pos) => pos,
        None => return (text.to_string(), String::new(), String::new()),
    };
    
    let search = chars[(start_pos + 1)..search_end].iter().collect();
    let replace_range = if replace_delim != closing_delim {
        (replace_start + 1)..replace_end
    } else {
        replace_start..replace_end
    };
    let replace = chars[replace_range].iter().collect();
    let modifiers = chars[replace_end + 1..].iter().collect();
    
    (search, replace, modifiers)
}