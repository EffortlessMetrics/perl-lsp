//! Parser for Perl debugger variable output.
//!
//! This module provides utilities for parsing variable output from the Perl debugger
//! into structured [`PerlValue`] representations.

use crate::PerlValue;
use once_cell::sync::Lazy;
use regex::Regex;
use thiserror::Error;

/// Errors that can occur during variable parsing.
#[derive(Debug, Error)]
pub enum VariableParseError {
    /// The input format was not recognized.
    #[error("unrecognized variable format: {0}")]
    UnrecognizedFormat(String),

    /// A nested structure was too deep.
    #[error("maximum nesting depth exceeded ({0})")]
    MaxDepthExceeded(usize),

    /// A string literal was not properly terminated.
    #[error("unterminated string literal")]
    UnterminatedString,

    /// An array or hash was not properly closed.
    #[error("unterminated collection")]
    UnterminatedCollection,

    /// A regex pattern failed to compile.
    #[error("regex error: {0}")]
    RegexError(#[from] regex::Error),
}

/// Compiled regex patterns for variable parsing.
static SCALAR_VAR_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\s*(?P<name>[\$\@\%][\w:]+)\s*=\s*(?P<value>.*?)$")
        .unwrap_or_else(|_| panic!("Failed to compile SCALAR_VAR_RE"))
});

static UNDEF_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^undef$").unwrap_or_else(|_| panic!("Failed to compile UNDEF_RE")));

static INTEGER_RE: Lazy<Regex> =
    Lazy::new(|| Regex::new(r"^-?\d+$").unwrap_or_else(|_| panic!("Failed to compile INTEGER_RE")));

static NUMBER_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^-?(?:\d+\.?\d*|\.\d+)(?:[eE][+-]?\d+)?$")
        .unwrap_or_else(|_| panic!("Failed to compile NUMBER_RE"))
});

static QUOTED_STRING_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r#"^'(?:[^'\\]|\\.)*'|^"(?:[^"\\]|\\.)*""#)
        .unwrap_or_else(|_| panic!("Failed to compile QUOTED_STRING_RE"))
});

static ARRAY_REF_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^ARRAY\(0x[0-9a-fA-F]+\)$")
        .unwrap_or_else(|_| panic!("Failed to compile ARRAY_REF_RE"))
});

static HASH_REF_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^HASH\(0x[0-9a-fA-F]+\)$")
        .unwrap_or_else(|_| panic!("Failed to compile HASH_REF_RE"))
});

static CODE_REF_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^CODE\(0x[0-9a-fA-F]+\)$")
        .unwrap_or_else(|_| panic!("Failed to compile CODE_REF_RE"))
});

static OBJECT_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?P<class>[\w:]+)=(?P<type>ARRAY|HASH|SCALAR|GLOB)\(0x[0-9a-fA-F]+\)$")
        .unwrap_or_else(|_| panic!("Failed to compile OBJECT_RE"))
});

static GLOB_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^\*(?P<name>[\w:]+)$").unwrap_or_else(|_| panic!("Failed to compile GLOB_RE"))
});

/// Regex for parsing compiled regexp values (reserved for future use)
#[allow(dead_code)]
static REGEX_RE: Lazy<Regex> = Lazy::new(|| {
    Regex::new(r"^(?:\(\?(?P<flags>[xism-]*)(?:-[xism]+)?:)?(?P<pattern>.*?)\)?$")
        .unwrap_or_else(|_| panic!("Failed to compile REGEX_RE"))
});

/// Parser for Perl debugger variable output.
///
/// This parser converts text output from the Perl debugger's variable
/// inspection commands into structured [`PerlValue`] representations.
#[derive(Debug, Default)]
pub struct VariableParser {
    /// Maximum nesting depth for recursive parsing
    max_depth: usize,
}

impl VariableParser {
    /// Creates a new variable parser with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self { max_depth: 50 }
    }

    /// Sets the maximum nesting depth for parsing.
    #[must_use]
    pub fn with_max_depth(mut self, depth: usize) -> Self {
        self.max_depth = depth;
        self
    }

    /// Parses a variable assignment line from debugger output.
    ///
    /// # Arguments
    ///
    /// * `line` - A line like "$var = value" or "@arr = (1, 2, 3)"
    ///
    /// # Returns
    ///
    /// A tuple of (variable name, parsed value) if successful.
    ///
    /// # Errors
    ///
    /// Returns a [`VariableParseError`] if the line cannot be parsed.
    pub fn parse_assignment(&self, line: &str) -> Result<(String, PerlValue), VariableParseError> {
        if let Some(caps) = SCALAR_VAR_RE.captures(line) {
            let name = caps
                .name("name")
                .ok_or_else(|| VariableParseError::UnrecognizedFormat(line.to_string()))?
                .as_str()
                .to_string();
            let value_str = caps
                .name("value")
                .ok_or_else(|| VariableParseError::UnrecognizedFormat(line.to_string()))?
                .as_str();
            let value = self.parse_value(value_str, 0)?;
            Ok((name, value))
        } else {
            Err(VariableParseError::UnrecognizedFormat(line.to_string()))
        }
    }

    /// Parses a value string from debugger output.
    ///
    /// # Arguments
    ///
    /// * `text` - The value portion of debugger output
    ///
    /// # Returns
    ///
    /// The parsed [`PerlValue`].
    ///
    /// # Errors
    ///
    /// Returns a [`VariableParseError`] if the value cannot be parsed.
    pub fn parse_value(&self, text: &str, depth: usize) -> Result<PerlValue, VariableParseError> {
        if depth > self.max_depth {
            return Err(VariableParseError::MaxDepthExceeded(self.max_depth));
        }

        let text = text.trim();

        // Check for undef
        if UNDEF_RE.is_match(text) {
            return Ok(PerlValue::Undef);
        }

        // Check for integer
        if INTEGER_RE.is_match(text) {
            if let Ok(i) = text.parse::<i64>() {
                return Ok(PerlValue::Integer(i));
            }
        }

        // Check for number
        if NUMBER_RE.is_match(text) {
            if let Ok(n) = text.parse::<f64>() {
                return Ok(PerlValue::Number(n));
            }
        }

        // Check for quoted string
        if QUOTED_STRING_RE.is_match(text) {
            let unquoted = self.unquote_string(text)?;
            return Ok(PerlValue::Scalar(unquoted));
        }

        // Check for array reference notation
        if ARRAY_REF_RE.is_match(text) {
            return Ok(PerlValue::Array(vec![]));
        }

        // Check for hash reference notation
        if HASH_REF_RE.is_match(text) {
            return Ok(PerlValue::Hash(vec![]));
        }

        // Check for code reference
        if CODE_REF_RE.is_match(text) {
            return Ok(PerlValue::Code { name: None });
        }

        // Check for blessed object
        if let Some(caps) = OBJECT_RE.captures(text) {
            let class = caps
                .name("class")
                .ok_or_else(|| VariableParseError::UnrecognizedFormat(text.to_string()))?
                .as_str()
                .to_string();
            let type_str = caps
                .name("type")
                .ok_or_else(|| VariableParseError::UnrecognizedFormat(text.to_string()))?
                .as_str();
            let inner = match type_str {
                "ARRAY" => PerlValue::Array(vec![]),
                "HASH" => PerlValue::Hash(vec![]),
                _ => PerlValue::Scalar(String::new()),
            };
            return Ok(PerlValue::Object { class, value: Box::new(inner) });
        }

        // Check for glob
        if let Some(caps) = GLOB_RE.captures(text) {
            let name = caps
                .name("name")
                .ok_or_else(|| VariableParseError::UnrecognizedFormat(text.to_string()))?
                .as_str()
                .to_string();
            return Ok(PerlValue::Glob(name));
        }

        // Check for array literal
        if text.starts_with('(') && text.ends_with(')') {
            return self.parse_array_literal(text, depth);
        }

        // Check for array bracket literal
        if text.starts_with('[') && text.ends_with(']') {
            return self.parse_array_literal(text, depth);
        }

        // Check for hash literal
        if text.starts_with('{') && text.ends_with('}') {
            return self.parse_hash_literal(text, depth);
        }

        // Treat as unquoted scalar (bareword or other)
        Ok(PerlValue::Scalar(text.to_string()))
    }

    /// Parses an array literal like (1, 2, 3) or [1, 2, 3].
    fn parse_array_literal(
        &self,
        text: &str,
        depth: usize,
    ) -> Result<PerlValue, VariableParseError> {
        // Remove outer delimiters (works for both '(' and '[')
        let inner = &text[1..text.len() - 1];

        if inner.trim().is_empty() {
            return Ok(PerlValue::Array(vec![]));
        }

        let elements = self.split_elements(inner)?;
        let parsed: Result<Vec<PerlValue>, _> =
            elements.iter().map(|e| self.parse_value(e, depth + 1)).collect();

        Ok(PerlValue::Array(parsed?))
    }

    /// Parses a hash literal like {key => value, ...}.
    fn parse_hash_literal(
        &self,
        text: &str,
        depth: usize,
    ) -> Result<PerlValue, VariableParseError> {
        // Remove outer braces
        let inner = &text[1..text.len() - 1];

        if inner.trim().is_empty() {
            return Ok(PerlValue::Hash(vec![]));
        }

        let elements = self.split_elements(inner)?;
        let mut pairs = Vec::new();

        for element in elements {
            if let Some((key, value)) = element.split_once("=>") {
                let key = self.unquote_key(key.trim());
                let value = self.parse_value(value.trim(), depth + 1)?;
                pairs.push((key, value));
            } else {
                // Treat as key with undef value
                let key = self.unquote_key(element.trim());
                pairs.push((key, PerlValue::Undef));
            }
        }

        Ok(PerlValue::Hash(pairs))
    }

    /// Splits a comma-separated list while respecting nested structures.
    fn split_elements(&self, text: &str) -> Result<Vec<String>, VariableParseError> {
        let mut elements = Vec::new();
        let mut current = String::new();
        let mut paren_depth: u32 = 0;
        let mut bracket_depth: u32 = 0;
        let mut brace_depth: u32 = 0;
        let mut in_string = false;
        let mut string_char = ' ';
        let mut escape_next = false;

        for ch in text.chars() {
            if escape_next {
                current.push(ch);
                escape_next = false;
                continue;
            }

            if ch == '\\' {
                current.push(ch);
                escape_next = true;
                continue;
            }

            if in_string {
                current.push(ch);
                if ch == string_char {
                    in_string = false;
                }
                continue;
            }

            match ch {
                '"' | '\'' => {
                    current.push(ch);
                    in_string = true;
                    string_char = ch;
                }
                '(' => {
                    current.push(ch);
                    paren_depth += 1;
                }
                ')' => {
                    current.push(ch);
                    paren_depth = paren_depth.saturating_sub(1);
                }
                '[' => {
                    current.push(ch);
                    bracket_depth += 1;
                }
                ']' => {
                    current.push(ch);
                    bracket_depth = bracket_depth.saturating_sub(1);
                }
                '{' => {
                    current.push(ch);
                    brace_depth += 1;
                }
                '}' => {
                    current.push(ch);
                    brace_depth = brace_depth.saturating_sub(1);
                }
                ',' if paren_depth == 0 && bracket_depth == 0 && brace_depth == 0 => {
                    let trimmed = current.trim().to_string();
                    if !trimmed.is_empty() {
                        elements.push(trimmed);
                    }
                    current = String::new();
                }
                _ => {
                    current.push(ch);
                }
            }
        }

        // Add the last element
        let trimmed = current.trim().to_string();
        if !trimmed.is_empty() {
            elements.push(trimmed);
        }

        Ok(elements)
    }

    /// Removes quotes from a string value.
    fn unquote_string(&self, text: &str) -> Result<String, VariableParseError> {
        if text.len() < 2 {
            return Err(VariableParseError::UnterminatedString);
        }

        let first = text.chars().next();
        let last = text.chars().next_back();

        match (first, last) {
            (Some('"'), Some('"')) | (Some('\''), Some('\'')) => {
                let inner = &text[1..text.len() - 1];
                Ok(self.unescape_string(inner))
            }
            _ => Ok(text.to_string()),
        }
    }

    /// Removes quotes from a hash key (or returns as-is if not quoted).
    fn unquote_key(&self, text: &str) -> String {
        if text.len() >= 2 {
            let first = text.chars().next();
            let last = text.chars().next_back();

            match (first, last) {
                (Some('"'), Some('"')) | (Some('\''), Some('\'')) => {
                    return self.unescape_string(&text[1..text.len() - 1]);
                }
                _ => {}
            }
        }
        text.to_string()
    }

    /// Unescapes common escape sequences in a string.
    fn unescape_string(&self, text: &str) -> String {
        let mut result = String::with_capacity(text.len());
        let mut chars = text.chars().peekable();

        while let Some(ch) = chars.next() {
            if ch == '\\' {
                match chars.next() {
                    Some('n') => result.push('\n'),
                    Some('r') => result.push('\r'),
                    Some('t') => result.push('\t'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some('\'') => result.push('\''),
                    Some(other) => {
                        result.push('\\');
                        result.push(other);
                    }
                    None => result.push('\\'),
                }
            } else {
                result.push(ch);
            }
        }

        result
    }

    /// Parses multiple variable lines (e.g., from 'V' command output).
    ///
    /// # Arguments
    ///
    /// * `output` - Multi-line debugger output
    ///
    /// # Returns
    ///
    /// A vector of (name, value) pairs for successfully parsed variables.
    pub fn parse_variables(&self, output: &str) -> Vec<(String, PerlValue)> {
        output.lines().filter_map(|line| self.parse_assignment(line).ok()).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_undef() {
        let parser = VariableParser::new();
        let result = parser.parse_value("undef", 0);
        assert!(matches!(result, Ok(PerlValue::Undef)));
    }

    #[test]
    fn test_parse_integer() {
        let parser = VariableParser::new();

        let result = parser.parse_value("42", 0);
        assert!(matches!(result, Ok(PerlValue::Integer(42))));

        let result = parser.parse_value("-17", 0);
        assert!(matches!(result, Ok(PerlValue::Integer(-17))));
    }

    #[test]
    fn test_parse_number() {
        let parser = VariableParser::new();

        let result = parser.parse_value("3.25", 0);
        assert!(matches!(result, Ok(PerlValue::Number(n)) if (n - 3.25).abs() < 0.001));

        let result = parser.parse_value("1.5e10", 0);
        assert!(matches!(result, Ok(PerlValue::Number(_))));
    }

    #[test]
    fn test_parse_quoted_string() {
        let parser = VariableParser::new();

        let result = parser.parse_value("\"hello\"", 0);
        assert!(matches!(result, Ok(PerlValue::Scalar(s)) if s == "hello"));

        let result = parser.parse_value("'world'", 0);
        assert!(matches!(result, Ok(PerlValue::Scalar(s)) if s == "world"));
    }

    #[test]
    fn test_parse_string_with_escapes() {
        let parser = VariableParser::new();

        let result = parser.parse_value("\"line1\\nline2\"", 0);
        assert!(matches!(result, Ok(PerlValue::Scalar(s)) if s.contains('\n')));
    }

    #[test]
    fn test_parse_array_reference() {
        let parser = VariableParser::new();

        let result = parser.parse_value("ARRAY(0x1234abcd)", 0);
        assert!(matches!(result, Ok(PerlValue::Array(_))));
    }

    #[test]
    fn test_parse_hash_reference() {
        let parser = VariableParser::new();

        let result = parser.parse_value("HASH(0x5678abcd)", 0);
        assert!(matches!(result, Ok(PerlValue::Hash(_))));
    }

    #[test]
    fn test_parse_code_reference() {
        let parser = VariableParser::new();

        let result = parser.parse_value("CODE(0xdeadbeef)", 0);
        assert!(matches!(result, Ok(PerlValue::Code { name: None })));
    }

    #[test]
    fn test_parse_object() {
        let parser = VariableParser::new();

        let result = parser.parse_value("My::Class=HASH(0x1234)", 0);
        assert!(matches!(result, Ok(PerlValue::Object { class, .. }) if class == "My::Class"));
    }

    #[test]
    fn test_parse_glob() {
        let parser = VariableParser::new();

        let result = parser.parse_value("*main::foo", 0);
        assert!(matches!(result, Ok(PerlValue::Glob(name)) if name == "main::foo"));
    }

    #[test]
    fn test_parse_array_literal() {
        let parser = VariableParser::new();

        let result = parser.parse_value("(1, 2, 3)", 0);
        assert!(matches!(result, Ok(PerlValue::Array(arr)) if arr.len() == 3));

        let result = parser.parse_value("[1, 2, 3]", 0);
        assert!(matches!(result, Ok(PerlValue::Array(arr)) if arr.len() == 3));

        let result = parser.parse_value("()", 0);
        assert!(matches!(result, Ok(PerlValue::Array(arr)) if arr.is_empty()));
    }

    #[test]
    fn test_parse_hash_literal() {
        let parser = VariableParser::new();

        let result = parser.parse_value("{foo => 1, bar => 2}", 0);
        assert!(matches!(result, Ok(PerlValue::Hash(pairs)) if pairs.len() == 2));

        let result = parser.parse_value("{}", 0);
        assert!(matches!(result, Ok(PerlValue::Hash(pairs)) if pairs.is_empty()));
    }

    #[test]
    fn test_parse_assignment() {
        let parser = VariableParser::new();

        let result = parser.parse_assignment("$x = 42");
        assert!(matches!(result, Ok((name, PerlValue::Integer(42))) if name == "$x"));

        let result = parser.parse_assignment("@arr = (1, 2, 3)");
        assert!(matches!(result, Ok((name, PerlValue::Array(_))) if name == "@arr"));

        let result = parser.parse_assignment("%hash = {a => 1}");
        assert!(matches!(result, Ok((name, PerlValue::Hash(_))) if name == "%hash"));
    }

    #[test]
    fn test_parse_variables_multi_line() {
        let parser = VariableParser::new();

        let output = "$x = 1\n$y = 2\n$z = \"hello\"";
        let vars = parser.parse_variables(output);

        assert_eq!(vars.len(), 3);
        assert_eq!(vars[0].0, "$x");
        assert_eq!(vars[1].0, "$y");
        assert_eq!(vars[2].0, "$z");
    }

    #[test]
    fn test_max_depth_exceeded() {
        let parser = VariableParser::new().with_max_depth(2);

        // Create deeply nested structure
        let result = parser.parse_value("(((1)))", 0);
        assert!(matches!(result, Err(VariableParseError::MaxDepthExceeded(_))));
    }

    #[test]
    fn test_parse_nested_structure() {
        let parser = VariableParser::new();

        let result = parser.parse_value("{arr => [1, 2], hash => {a => 1}}", 0);
        assert!(matches!(result, Ok(PerlValue::Hash(pairs)) if pairs.len() == 2));
    }
}
