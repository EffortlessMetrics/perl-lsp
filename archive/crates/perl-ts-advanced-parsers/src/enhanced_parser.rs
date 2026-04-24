//! Enhanced parser that automatically uses stateful parsing for heredocs

use crate::stateful_parser::StatefulPerlParser;
use perl_parser_pest::{AstNode, PureRustPerlParser};

/// Enhanced Perl parser that automatically handles heredocs and other stateful constructs
pub struct EnhancedPerlParser {
    use_stateful: bool,
}

impl Default for EnhancedPerlParser {
    fn default() -> Self {
        Self::new()
    }
}

impl EnhancedPerlParser {
    pub fn new() -> Self {
        Self {
            use_stateful: true, // Enable stateful parsing by default
        }
    }

    pub fn with_stateful(mut self, enabled: bool) -> Self {
        self.use_stateful = enabled;
        self
    }

    pub fn parse(&self, source: &str) -> Result<AstNode, Box<dyn std::error::Error>> {
        if self.use_stateful && source.contains("<<") {
            // Use stateful parser if heredocs are detected
            let mut parser = StatefulPerlParser::new();
            parser.parse(source)
        } else {
            // Use regular parser for simple cases
            let mut parser = PureRustPerlParser::new();
            parser.parse(source)
        }
    }

    pub fn to_sexp(&self, ast: &AstNode) -> String {
        PureRustPerlParser::new().to_sexp(ast)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_enhanced_parser_detects_heredocs() {
        let source = r#"my $x = <<EOF;
Hello world
EOF
"#;
        let parser = EnhancedPerlParser::new();
        use perl_tdd_support::must;
        let ast = must(parser.parse(source));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("Hello world"));
    }

    #[test]
    fn test_enhanced_parser_works_without_heredocs() {
        let source = "my $x = 42;";
        let parser = EnhancedPerlParser::new();
        use perl_tdd_support::must;
        let ast = must(parser.parse(source));
        let sexp = parser.to_sexp(&ast);
        assert!(sexp.contains("42"));
    }
}
