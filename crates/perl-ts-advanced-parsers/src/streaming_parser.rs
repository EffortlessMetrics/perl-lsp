//! Streaming parser for large Perl files
//!
//! This module provides a streaming parser that can handle large files
//! without loading them entirely into memory. It processes files in chunks
//! and emits parse events as it goes.

use perl_ts_heredoc_parser::enhanced_heredoc_lexer::{HeredocDeclaration, process_with_enhanced_heredocs};
use perl_parser_pest::ParseError;
use perl_ts_heredoc_parser::lexer_adapter::LexerAdapter;
use perl_parser_pest::pure_rust_parser::{AstNode, PerlParser, Rule};
use pest::Parser;
use std::collections::VecDeque;
use std::io::{BufRead, BufReader, Read};

/// Events emitted by the streaming parser
#[derive(Debug, Clone)]
pub enum ParseEvent {
    /// Start of a new statement or declaration
    StatementStart { line: usize, kind: StatementKind },
    /// Complete AST node parsed
    Node(AstNode),
    /// End of statement
    StatementEnd { line: usize },
    /// Parse error encountered
    Error { line: usize, message: String },
    /// Special section found
    SpecialSection { kind: SectionKind, start_line: usize, content: String },
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatementKind {
    SubDeclaration,
    PackageDeclaration,
    UseStatement,
    Variable,
    Expression,
    ControlFlow,
    Other,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SectionKind {
    Pod,
    Data,
    End,
}

/// Configuration for the streaming parser
pub struct StreamConfig {
    /// Size of the buffer for reading chunks
    pub buffer_size: usize,
    /// Maximum size of a single statement before forcing a parse
    pub max_statement_size: usize,
    /// Whether to emit partial ASTs for incomplete statements
    pub emit_partial: bool,
}

impl Default for StreamConfig {
    fn default() -> Self {
        Self {
            buffer_size: 8192,         // 8KB chunks
            max_statement_size: 65536, // 64KB max statement
            emit_partial: false,
        }
    }
}

pub struct StreamingParser<R: Read> {
    reader: BufReader<R>,
    config: StreamConfig,
    #[allow(dead_code)]
    buffer: String,
    line_number: usize,
    in_pod: bool,
    in_data_section: bool,
    #[allow(dead_code)]
    pending_heredocs: VecDeque<HeredocDeclaration>,
    statement_buffer: String,
    statement_start_line: usize,
}

impl<R: Read> StreamingParser<R> {
    pub fn new(reader: R, config: StreamConfig) -> Self {
        Self {
            reader: BufReader::with_capacity(config.buffer_size, reader),
            config,
            buffer: String::new(),
            line_number: 0,
            in_pod: false,
            in_data_section: false,
            pending_heredocs: VecDeque::new(),
            statement_buffer: String::new(),
            statement_start_line: 0,
        }
    }

    /// Parse the input stream and yield events
    pub fn parse(&mut self) -> impl Iterator<Item = ParseEvent> + '_ {
        StreamIterator { parser: self }
    }

    fn next_event(&mut self) -> Option<ParseEvent> {
        // Read lines until we have a complete statement
        loop {
            let mut line = String::new();
            match self.reader.read_line(&mut line) {
                Ok(0) => {
                    // End of file
                    if !self.statement_buffer.is_empty() {
                        return self.flush_statement();
                    }
                    return None;
                }
                Ok(_) => {
                    self.line_number += 1;

                    // Check for special sections
                    if let Some(event) = self.check_special_sections(&line) {
                        return Some(event);
                    }

                    // Skip if in special section
                    if self.in_pod || self.in_data_section {
                        continue;
                    }

                    // Add to statement buffer
                    if self.statement_buffer.is_empty() {
                        self.statement_start_line = self.line_number;
                    }
                    self.statement_buffer.push_str(&line);

                    // Check if we have a complete statement
                    if self.is_complete_statement(&self.statement_buffer) {
                        return self.flush_statement();
                    }

                    // Force parse if statement is too large
                    if self.statement_buffer.len() > self.config.max_statement_size {
                        return self.flush_statement();
                    }
                }
                Err(e) => {
                    return Some(ParseEvent::Error {
                        line: self.line_number,
                        message: format!("Read error: {}", e),
                    });
                }
            }
        }
    }

    fn check_special_sections(&mut self, line: &str) -> Option<ParseEvent> {
        let trimmed = line.trim();

        // Check for POD start
        if !self.in_pod
            && trimmed.starts_with('=')
            && trimmed.len() > 1
            && trimmed.chars().nth(1).is_some_and(|c| c.is_alphabetic())
        {
            self.in_pod = true;
            return Some(ParseEvent::SpecialSection {
                kind: SectionKind::Pod,
                start_line: self.line_number,
                content: line.to_string(),
            });
        }

        // Check for POD end
        if self.in_pod && trimmed == "=cut" {
            self.in_pod = false;
            return None;
        }

        // Check for DATA/END section
        if !self.in_data_section && (trimmed == "__DATA__" || trimmed == "__END__") {
            self.in_data_section = true;
            let kind = if trimmed == "__DATA__" { SectionKind::Data } else { SectionKind::End };
            return Some(ParseEvent::SpecialSection {
                kind,
                start_line: self.line_number,
                content: String::new(),
            });
        }

        None
    }

    fn is_complete_statement(&self, buffer: &str) -> bool {
        // Simple heuristic: check for statement terminators
        // In a real implementation, we'd use more sophisticated parsing
        let trimmed = buffer.trim_end();

        // Check for common statement endings
        if trimmed.ends_with(';') || trimmed.ends_with('}') {
            // Make sure we're not in a string or regex
            if !self.in_string_or_regex(trimmed) {
                return true;
            }
        }

        // Check for block statements that don't need semicolons
        if trimmed.starts_with("sub ") && trimmed.ends_with('}') {
            return true;
        }

        if trimmed.starts_with("package ") && (trimmed.ends_with(';') || trimmed.ends_with('}')) {
            return true;
        }

        false
    }

    fn in_string_or_regex(&self, text: &str) -> bool {
        // Simplified check - in reality would need proper lexing
        let mut in_single_quote = false;
        let mut in_double_quote = false;
        let mut escaped = false;

        for ch in text.chars() {
            if escaped {
                escaped = false;
                continue;
            }

            match ch {
                '\\' => escaped = true,
                '\'' if !in_double_quote => in_single_quote = !in_single_quote,
                '"' if !in_single_quote => in_double_quote = !in_double_quote,
                _ => {}
            }
        }

        in_single_quote || in_double_quote
    }

    fn flush_statement(&mut self) -> Option<ParseEvent> {
        if self.statement_buffer.is_empty() {
            return None;
        }

        let statement = std::mem::take(&mut self.statement_buffer);
        let start_line = self.statement_start_line;

        // Determine statement kind
        let _kind = self.detect_statement_kind(&statement);

        // Try to parse the statement
        match self.parse_statement(&statement) {
            Ok(ast) => Some(ParseEvent::Node(ast)),
            Err(e) => Some(ParseEvent::Error {
                line: start_line,
                message: format!("Parse error: {:?}", e),
            }),
        }
    }

    fn detect_statement_kind(&self, statement: &str) -> StatementKind {
        let trimmed = statement.trim();

        if trimmed.starts_with("sub ") {
            StatementKind::SubDeclaration
        } else if trimmed.starts_with("package ") {
            StatementKind::PackageDeclaration
        } else if trimmed.starts_with("use ") || trimmed.starts_with("require ") {
            StatementKind::UseStatement
        } else if trimmed.starts_with("my ")
            || trimmed.starts_with("our ")
            || trimmed.starts_with("local ")
            || trimmed.starts_with("state ")
        {
            StatementKind::Variable
        } else if trimmed.starts_with("if ")
            || trimmed.starts_with("while ")
            || trimmed.starts_with("for ")
            || trimmed.starts_with("foreach ")
        {
            StatementKind::ControlFlow
        } else {
            StatementKind::Expression
        }
    }

    fn parse_statement(&mut self, statement: &str) -> Result<AstNode, ParseError> {
        // Process heredocs
        let (processed, _declarations) = process_with_enhanced_heredocs(statement);

        // Handle slash disambiguation
        let disambiguated = LexerAdapter::preprocess(&processed);

        // Parse with Pest
        let pairs = PerlParser::parse(Rule::statement, &disambiguated)
            .map_err(|_| ParseError::ParseFailed)?;

        // Build AST
        let mut parser = perl_parser_pest::PureRustPerlParser::new();
        for pair in pairs {
            if let Ok(Some(node)) = parser.build_node(pair) {
                return Ok(node);
            }
        }

        Err(ParseError::ParseFailed)
    }
}

struct StreamIterator<'a, R: Read> {
    parser: &'a mut StreamingParser<R>,
}

impl<'a, R: Read> Iterator for StreamIterator<'a, R> {
    type Item = ParseEvent;

    fn next(&mut self) -> Option<Self::Item> {
        self.parser.next_event()
    }
}

/// Stream parse a file and collect all events
pub fn stream_parse_file(path: &str) -> Result<Vec<ParseEvent>, std::io::Error> {
    let file = std::fs::File::open(path)?;
    let mut parser = StreamingParser::new(file, StreamConfig::default());
    Ok(parser.parse().collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_streaming_simple() {
        let input = r#"
my $x = 42;
print $x;
"#;

        let cursor = Cursor::new(input);
        let mut parser = StreamingParser::new(cursor, StreamConfig::default());
        let events: Vec<_> = parser.parse().collect();

        assert!(!events.is_empty());
        assert!(events.iter().any(|e| matches!(e, ParseEvent::Node(_))));
    }

    #[test]
    fn test_streaming_with_pod() {
        let input = r#"
print "Before POD\n";

=head1 NAME

Test

=cut

print "After POD\n";
"#;

        let cursor = Cursor::new(input);
        let mut parser = StreamingParser::new(cursor, StreamConfig::default());
        let events: Vec<_> = parser.parse().collect();

        assert!(
            events
                .iter()
                .any(|e| matches!(e, ParseEvent::SpecialSection { kind: SectionKind::Pod, .. }))
        );
    }

    #[test]
    fn test_statement_detection() {
        let parser = StreamingParser::new(Cursor::new(""), StreamConfig::default());

        assert_eq!(parser.detect_statement_kind("sub foo { }"), StatementKind::SubDeclaration);
        assert_eq!(parser.detect_statement_kind("package Foo;"), StatementKind::PackageDeclaration);
        assert_eq!(parser.detect_statement_kind("use strict;"), StatementKind::UseStatement);
        assert_eq!(parser.detect_statement_kind("my $x = 42;"), StatementKind::Variable);
        assert_eq!(parser.detect_statement_kind("if ($x) { }"), StatementKind::ControlFlow);
        assert_eq!(parser.detect_statement_kind("print $x;"), StatementKind::Expression);
    }
}
