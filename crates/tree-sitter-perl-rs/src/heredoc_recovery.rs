//! Heredoc recovery mechanism for handling dynamic delimiters
//!
//! This module provides heuristics and static analysis to recover from
//! unresolved dynamic heredoc delimiters during the parsing phase.

use crate::perl_lexer::{Token, TokenType};
use regex::Regex;
use std::collections::HashMap;
use std::sync::{Arc, LazyLock};

/// Result of a heredoc recovery operation
#[derive(Debug, Clone)]
pub struct RecoveryResult {
    /// Resolved delimiter if any
    pub delimiter: Option<Arc<str>>,
    /// Confidence score (0.0 to 1.0)
    pub confidence: f32,
    /// Recovery method used
    pub method: RecoveryMethod,
    /// Potential alternatives
    pub alternatives: Vec<Arc<str>>,
    /// Diagnostics and warnings
    pub diagnostics: Vec<String>,
    /// Whether to insert an error node
    pub error_node: bool,
}

/// Methods used for heredoc recovery
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecoveryMethod {
    /// Resolved via static analysis of previous tokens
    StaticAnalysis,
    /// Resolved via common pattern matching
    PatternMatch,
    /// Resolved via global context analysis
    ContextAnalysis,
    /// User-provided hint
    UserHint,
    /// Fallback default
    Fallback,
}

/// Configuration for heredoc recovery
#[derive(Debug, Clone)]
pub struct RecoveryConfig {
    pub enable_heuristics: bool,
    pub enable_pattern_matching: bool,
    pub enable_context_analysis: bool,
    pub confidence_threshold: f32,
}

impl Default for RecoveryConfig {
    fn default() -> Self {
        Self {
            enable_heuristics: true,
            enable_pattern_matching: true,
            enable_context_analysis: true,
            confidence_threshold: 0.6,
        }
    }
}

static DYNAMIC_DELIMITER: LazyLock<Regex> = LazyLock::new(|| match Regex::new(r"<<$(\w+)") {
    Ok(re) => re,
    Err(_) => unreachable!("DYNAMIC_DELIMITER regex failed to compile"),
});
#[allow(dead_code)]
static EXPR_DELIMITER: LazyLock<Regex> = LazyLock::new(|| match Regex::new(r"<<\$\{([^}]+)\}") {
    Ok(re) => re,
    Err(_) => unreachable!("EXPR_DELIMITER regex failed to compile"),
});
#[allow(dead_code)]
static SPACED_DELIMITER: LazyLock<Regex> = LazyLock::new(|| match Regex::new(r"<<\s+\$(\w+)") {
    Ok(re) => re,
    Err(_) => unreachable!("SPACED_DELIMITER regex failed to compile"),
});
static METHOD_DELIMITER: LazyLock<Regex> =
    LazyLock::new(|| match Regex::new(r"<<$(\w+)->(\w+)\(\)") {
        Ok(re) => re,
        Err(_) => unreachable!("METHOD_DELIMITER regex failed to compile"),
    });
#[allow(dead_code)]
static CONCAT_DELIMITER: LazyLock<Regex> = LazyLock::new(|| match Regex::new(r"<<\(([^)]+)\)") {
    Ok(re) => re,
    Err(_) => unreachable!("CONCAT_DELIMITER regex failed to compile"),
});

/// Heredoc recovery infrastructure
pub struct HeredocRecovery {
    /// Configuration options
    pub config: RecoveryConfig,
    /// Cached resolved delimiters
    pub delimiter_cache: HashMap<String, Arc<str>>,
    /// Context-aware delimiter recovery
    pub delimiter_recovery: crate::dynamic_delimiter_recovery::DynamicDelimiterRecovery,
}

impl HeredocRecovery {
    pub fn new(config: RecoveryConfig) -> Self {
        Self {
            config,
            delimiter_cache: HashMap::new(),
            delimiter_recovery: crate::dynamic_delimiter_recovery::DynamicDelimiterRecovery::new(
                crate::dynamic_delimiter_recovery::RecoveryMode::BestGuess,
            ),
        }
    }

    /// Attempt to recover a dynamic heredoc delimiter
    pub fn recover_dynamic_heredoc(
        &mut self,
        input: &str,
        position: usize,
        tokens: &[Token],
    ) -> RecoveryResult {
        let mut result = RecoveryResult {
            delimiter: None,
            confidence: 0.0,
            method: RecoveryMethod::Fallback,
            alternatives: Vec::new(),
            diagnostics: Vec::new(),
            error_node: true,
        };

        let expr_end = self.find_expression_end(input, position);
        let expression = &input[position..expr_end];

        // 1. Check cache first
        if let Some(cached) = self.delimiter_cache.get(expression) {
            result.delimiter = Some(cached.clone());
            result.confidence = 1.0;
            result.method = RecoveryMethod::StaticAnalysis;
            result.error_node = false;
            return result;
        }

        // 2. Try static analysis with lookahead
        if self.config.enable_heuristics
            && let Some((delimiter, confidence)) = self.try_static_analysis(input, position, tokens)
            && confidence >= self.config.confidence_threshold
        {
            result.delimiter = Some(delimiter.clone());
            result.confidence = confidence;
            result.method = RecoveryMethod::StaticAnalysis;
            result.error_node = false;
            self.delimiter_cache.insert(expression.to_string(), delimiter);
            return result;
        }

        // 3. Try pattern matching
        if self.config.enable_pattern_matching
            && let Some((delimiter, confidence)) = self.try_pattern_matching(expression)
        {
            if confidence >= self.config.confidence_threshold {
                result.delimiter = Some(delimiter.clone());
                result.confidence = confidence;
                result.method = RecoveryMethod::PatternMatch;
                result.error_node = false;

                // Still collect alternatives even if we found a good match
                let alternatives = self.apply_heuristics(expression);
                for alt in alternatives {
                    if alt.as_ref() != delimiter.as_ref()
                        && !result.alternatives.iter().any(|a| a.as_ref() == alt.as_ref())
                    {
                        result.alternatives.push(alt);
                    }
                }

                self.delimiter_cache.insert(expression.to_string(), delimiter);
                return result;
            } else {
                // Add as alternative
                result.alternatives.push(delimiter);
            }
        }

        // 4. Try context analysis
        if self.config.enable_context_analysis {
            let context = self.build_context(tokens, position);
            let analysis = self.delimiter_recovery.analyze_dynamic_delimiter(expression, &context);

            if let Some(delim) = analysis.delimiter
                && analysis.confidence >= self.config.confidence_threshold
            {
                let delimiter: Arc<str> = Arc::from(delim);
                result.delimiter = Some(delimiter.clone());
                result.confidence = analysis.confidence;
                result.method = RecoveryMethod::ContextAnalysis;
                result.error_node = false;
                self.delimiter_cache.insert(expression.to_string(), delimiter);
                return result;
            }

            // Add alternatives from analysis
            for alt in analysis.alternatives {
                result.alternatives.push(Arc::from(alt));
            }
            result.diagnostics.extend(analysis.warnings);
        }

        // 5. Final fallback - provide best guesses as alternatives
        let fallback_guesses = self.apply_heuristics(expression);
        for guess in fallback_guesses {
            if !result.alternatives.iter().any(|a| a.as_ref() == guess.as_ref()) {
                result.alternatives.push(guess);
            }
        }

        result
    }

    /// Find the end of a heredoc expression
    fn find_expression_end(&self, input: &str, start: usize) -> usize {
        let mut pos = start;
        let bytes = input.as_bytes();

        // Skip leading <<
        if pos + 2 <= bytes.len() && &bytes[pos..pos + 2] == b"<<" {
            pos += 2;
        }

        // Skip whitespace
        while pos < bytes.len() && bytes[pos].is_ascii_whitespace() && bytes[pos] != b'\n' {
            pos += 1;
        }

        // Simple scanner for expression end (stops at semicolon, newline, or unnested closing bracket)
        let mut brace_depth = 0;
        let mut paren_depth = 0;

        while pos < bytes.len() {
            match bytes[pos] {
                b';' | b'\n' if brace_depth == 0 && paren_depth == 0 => break,
                b'{' => brace_depth += 1,
                b'}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    } else if paren_depth == 0 {
                        break;
                    }
                }
                b'(' => paren_depth += 1,
                b')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    } else if brace_depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            pos += 1;
        }

        pos
    }

    /// Try static analysis by looking for variable assignments
    fn try_static_analysis(
        &mut self,
        input: &str,
        position: usize,
        tokens: &[Token],
    ) -> Option<(Arc<str>, f32)> {
        // Extract variable info from heredoc expression
        let expr_end = self.find_expression_end(input, position);
        let expression = &input[position..expr_end];

        // Check for array element access like $markers[1]
        static ARRAY_PATTERN: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r"\$(\w+)\[(\d+)\]") {
                Ok(re) => re,
                Err(_) => unreachable!("ARRAY_PATTERN regex failed to compile"),
            });
        if let Some(cap) = ARRAY_PATTERN.captures(expression) {
            let var_name = cap.get(1)?.as_str();
            let index: usize = cap.get(2)?.as_str().parse().ok()?;

            // Look for array assignment
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type
                    && name.as_ref() == format!("@{}", var_name)
                {
                    // Found array variable, look for list assignment
                    if i + 2 < tokens.len()
                        && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                    {
                        // Look for the list values
                        let mut list_values = Vec::new();
                        let mut j = i + 2;
                        let mut in_list = false;

                        while j < tokens.len() {
                            match &tokens[j].token_type {
                                TokenType::LeftParen => in_list = true,
                                TokenType::RightParen => break,
                                TokenType::StringLiteral if in_list => {
                                    if let Some(value) =
                                        self.extract_string_literal(&tokens[j].text)
                                    {
                                        list_values.push(value);
                                    }
                                }
                                _ => {}
                            }
                            j += 1;
                        }

                        // Return the value at the requested index
                        if index < list_values.len() {
                            return Some((Arc::from(list_values[index].clone()), 0.9));
                        }
                    }
                }
            }
        }

        // Check for package variables like $My::Pkg::var
        static PKG_VAR_PATTERN: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r"\$((?:\w+::)*\w+)") {
                Ok(re) => re,
                Err(_) => unreachable!("PKG_VAR_PATTERN regex failed to compile"),
            });
        if let Some(cap) = PKG_VAR_PATTERN.captures(expression) {
            let var_name = cap.get(1)?.as_str();

            // Look for 'our' declaration or direct assignment
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type
                    && name.as_ref() == format!("${}", var_name)
                {
                    // Check if next tokens form an assignment
                    if i + 2 < tokens.len()
                        && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                        && matches!(tokens[i + 2].token_type, TokenType::StringLiteral)
                    {
                        // Extract the string value
                        let text = tokens[i + 2].text.as_ref();
                        if let Some(delimiter) = self.extract_string_literal(text) {
                            return Some((Arc::from(delimiter), 0.9));
                        }
                    }
                }
            }
        }

        // Check for braced variable access like ${var}
        static BRACE_VAR_PATTERN: LazyLock<Regex> =
            LazyLock::new(|| match Regex::new(r"\$\{(.+)\}") {
                Ok(re) => re,
                Err(_) => unreachable!("BRACE_VAR_PATTERN regex failed to compile"),
            });
        if let Some(cap) = BRACE_VAR_PATTERN.captures(expression) {
            let var_name = cap.get(1)?.as_str();

            // Look for assignment in previous tokens
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type
                    && name.as_ref() == format!("${}", var_name)
                {
                    // Check if next tokens form an assignment
                    if i + 2 < tokens.len()
                        && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                        && matches!(tokens[i + 2].token_type, TokenType::StringLiteral)
                    {
                        // Extract the string value
                        let text = tokens[i + 2].text.as_ref();
                        if let Some(delimiter) = self.extract_string_literal(text) {
                            return Some((Arc::from(delimiter), 0.9));
                        }
                    }
                }
            }
        }

        // Check for simple scalar variables like $var
        static VAR_PATTERN: LazyLock<Regex> = LazyLock::new(|| match Regex::new(r"\$(\w+)") {
            Ok(re) => re,
            Err(_) => unreachable!("VAR_PATTERN regex failed to compile"),
        });
        if let Some(cap) = VAR_PATTERN.captures(expression) {
            let mut _current_var = cap.get(1)?.as_str();

            // Look for assignment of current variable
            for i in (0..tokens.len()).rev() {
                if let TokenType::Identifier(name) = &tokens[i].token_type
                    && name.as_ref() == format!("${}", _current_var)
                {
                    // Check if next tokens form an assignment
                    if i + 2 < tokens.len()
                        && matches!(tokens[i + 1].token_type, TokenType::Operator(ref op) if op.as_ref() == "=")
                    {
                        match &tokens[i + 2].token_type {
                            TokenType::StringLiteral => {
                                // Found a string literal value
                                let text = tokens[i + 2].text.as_ref();
                                return self
                                    .extract_string_literal(text)
                                    .map(|d| (Arc::from(d), 0.9));
                            }
                            TokenType::Identifier(next_var) => {
                                // Variable assigned to another variable
                                if let Some(stripped) = next_var.strip_prefix('$') {
                                    _current_var = stripped;
                                    // continue loop to look for next_var's assignment
                                }
                            }
                            _ => {}
                        }
                    }
                }
            }
        }

        None
    }

    /// Extract literal value from a quoted string
    fn extract_string_literal(&self, text: &str) -> Option<String> {
        let trimmed = text.trim();
        if ((trimmed.starts_with('"') && trimmed.ends_with('"'))
            || (trimmed.starts_with('\'') && trimmed.ends_with('\'')))
            && trimmed.len() >= 2
        {
            return Some(trimmed[1..trimmed.len() - 1].to_string());
        }
        None
    }

    /// Try to resolve delimiter using common patterns
    fn try_pattern_matching(&self, expression: &str) -> Option<(Arc<str>, f32)> {
        let region = expression;

        // 1. Simple variable delimiter: <<$var
        if let Some(cap) = DYNAMIC_DELIMITER.captures(region)
            && let Some(var_name) = cap.get(1)
        {
            let name = var_name.as_str().to_lowercase();
            if name.contains("eof") {
                return Some((Arc::from("EOF"), 0.5));
            }
            if name.contains("end") {
                return Some((Arc::from("END"), 0.5));
            }
        }

        // 2. Method call: <<$obj->method()
        if let Some(cap) = METHOD_DELIMITER.captures(region)
            && let Some(method_match) = cap.get(2)
        {
            let method = method_match.as_str();
            if method == "to_string" || method == "as_string" {
                return Some((Arc::from("EOF"), 0.4));
            }
        }

        None
    }

    /// Apply heuristics to guess common delimiters
    fn apply_heuristics(&self, expression: &str) -> Vec<Arc<str>> {
        let mut delimiters = Vec::new();

        // Most common Perl heredoc delimiters
        let common = ["EOF", "END", "EOT", "EOD", "HERE", "DATA", "TEXT"];

        // Special handling for special variables
        // Strip << prefix if present
        let expr = expression.strip_prefix("<<").map(|s| s.trim()).unwrap_or(expression);

        if expr == "$_" || expr == "$@" || expr == "$!" || expr == "$?" {
            // For special variables, return common delimiters in priority order
            delimiters.push(Arc::from("EOF"));
            delimiters.push(Arc::from("END"));
            delimiters.push(Arc::from("EOT"));
            delimiters.push(Arc::from("EOD"));
            delimiters.push(Arc::from("DONE"));
            return delimiters;
        }

        // If expression contains hints, prioritize those
        let lower = expression.to_lowercase();

        // Check for specific patterns
        if lower.contains("eof") {
            delimiters.push(Arc::from("EOF"));
            // Also add END as it's commonly confused with EOF
            delimiters.push(Arc::from("END"));
        }
        if lower.contains("end") && !lower.contains("eof") {
            delimiters.push(Arc::from("END"));
            delimiters.push(Arc::from("EOF"));
        }
        if lower.contains("sql") {
            delimiters.push(Arc::from("SQL"));
        }

        // Check other delimiters
        for delim in &common {
            let delim_lower = delim.to_lowercase();
            if delim_lower != "eof" && delim_lower != "end" && lower.contains(&delim_lower) {
                delimiters.push(Arc::from(*delim));
            }
        }

        // Add remaining common delimiters
        for delim in &common {
            if !delimiters.iter().any(|d: &Arc<str>| d.as_ref() == *delim) {
                delimiters.push(Arc::from(*delim));
            }
        }

        delimiters
    }

    /// Build parse context from tokens
    fn build_context(
        &self,
        _tokens: &[Token],
        _position: usize,
    ) -> crate::dynamic_delimiter_recovery::ParseContext {
        // Scan tokens for context clues
        // (simplified - could scan for package/sub keywords if needed)

        crate::dynamic_delimiter_recovery::ParseContext {
            current_package: None,
            imported_modules: Vec::new(),
            in_subroutine: None,
            file_type_hint: None,
        }
    }

    /// Generate error token for unrecoverable heredoc
    pub fn generate_error_token(
        &self,
        input: &str,
        start: usize,
        result: &RecoveryResult,
    ) -> Token {
        let end = self.find_expression_end(input, start);
        let text = &input[start..end];

        let mut error_msg = format!("Unresolved dynamic heredoc delimiter: {}", text);
        if !result.diagnostics.is_empty() {
            error_msg.push_str(&format!(" ({})", result.diagnostics.join("; ")));
        }
        if !result.alternatives.is_empty() {
            error_msg.push_str(&format!(
                " - possible delimiters: {}",
                result
                    .alternatives
                    .iter()
                    .map(|d| format!("'{}'", d))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
        }

        Token {
            token_type: TokenType::Error(Arc::from(error_msg)),
            text: Arc::from(text),
            start,
            end,
        }
    }

    /// Parse a complete delimiter expression, handling nested structures
    pub fn parse_delimiter_expression(
        &self,
        input: &str,
        start_pos: usize,
    ) -> Option<(String, usize)> {
        let bytes = input.as_bytes();
        let mut pos = start_pos;
        let mut brace_depth = 0;
        let mut bracket_depth = 0;
        let mut paren_depth = 0;
        let mut in_method_call = false;

        // Skip leading sigil if present ($ or @)
        if pos < bytes.len() && (bytes[pos] == b'$' || bytes[pos] == b'@' || bytes[pos] == b'%') {
            pos += 1;

            // Handle special variables like $_ and $@
            if pos < bytes.len() {
                match bytes[pos] {
                    b'_' | b'@' | b'!' | b'?' | b'$' | b'*' | b'#' | b'[' | b']' => {
                        pos += 1;
                        return Some((input[start_pos..pos].to_string(), pos));
                    }
                    _ => {}
                }
            }
        }

        while pos < bytes.len() {
            match bytes[pos] {
                b'{' => brace_depth += 1,
                b'}' => {
                    if brace_depth > 0 {
                        brace_depth -= 1;
                    } else if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                b'[' => bracket_depth += 1,
                b']' => {
                    if bracket_depth > 0 {
                        bracket_depth -= 1;
                    } else if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                b'(' => paren_depth += 1,
                b')' => {
                    if paren_depth > 0 {
                        paren_depth -= 1;
                    } else if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                b'-' if pos + 1 < bytes.len() && bytes[pos + 1] == b'>' => {
                    // Method call arrow
                    in_method_call = true;
                    pos += 1; // Will increment again at loop end
                }
                b':' if pos + 1 < bytes.len() && bytes[pos + 1] == b':' => {
                    // Package separator
                    pos += 1; // Will increment again at loop end
                }
                // Stop at semicolon, newline or whitespace if we're not inside any delimiters
                b';' | b'\n' | b' ' | b'\t'
                    if brace_depth == 0
                        && bracket_depth == 0
                        && paren_depth == 0
                        && !in_method_call =>
                {
                    break;
                }
                // For method calls, reset the flag after the identifier
                b' ' | b';' | b'\n' if in_method_call => {
                    in_method_call = false;
                    if brace_depth == 0 && bracket_depth == 0 && paren_depth == 0 {
                        break;
                    }
                }
                _ => {}
            }
            pos += 1;
        }

        if pos > start_pos { Some((input[start_pos..pos].to_string(), pos)) } else { None }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_expression_end_detection() {
        let recovery = HeredocRecovery::new(RecoveryConfig::default());

        // Simple variable
        let input = "<<$delimiter;";
        let end = recovery.find_expression_end(input, 0);
        assert_eq!(&input[0..end], "<<$delimiter");

        // Expression with braces
        let input = "<<${foo};";
        let end = recovery.find_expression_end(input, 0);
        assert_eq!(&input[0..end], "<<${foo}");
    }

    #[test]
    fn test_heuristics() {
        let recovery = HeredocRecovery::new(RecoveryConfig::default());

        let delims = recovery.apply_heuristics("$end_delimiter");
        assert!(!delims.is_empty());
        assert_eq!(delims[0].as_ref(), "END");

        let delims = recovery.apply_heuristics("$eof");
        assert!(!delims.is_empty());
        assert_eq!(delims[0].as_ref(), "EOF");
    }
}
