impl<'a> Parser<'a> {
    /// Parse variable declaration (my, our, local, state)
    fn parse_variable_declaration(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?;
        let declarator = declarator_token.text.to_string();

        // Check if we have a list declaration like `my ($x, $y)`
        if self.peek_kind() == Some(TokenKind::LeftParen) {
            self.consume_token()?; // consume (

            let mut variables = Vec::new();

            // Parse comma-separated list of variables with their individual attributes
            while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                let var = self.parse_variable()?;

                // Parse optional attributes for this specific variable
                let mut var_attributes = Vec::new();
                while self.peek_kind() == Some(TokenKind::Colon) {
                    self.tokens.next()?; // consume colon
                    let attr_token = self.expect(TokenKind::Identifier)?;
                    var_attributes.push(attr_token.text.to_string());
                }

                // Create a node that includes both the variable and its attributes
                let var_with_attrs = if var_attributes.is_empty() {
                    var
                } else {
                    let start = var.location.start;
                    let end = self.previous_position();
                    Node::new(
                        NodeKind::VariableWithAttributes {
                            variable: Box::new(var),
                            attributes: var_attributes,
                        },
                        SourceLocation { start, end },
                    )
                };

                variables.push(var_with_attrs);

                if self.peek_kind() == Some(TokenKind::Comma) {
                    self.consume_token()?; // consume comma
                } else if self.peek_kind() != Some(TokenKind::RightParen) {
                    return Err(ParseError::syntax(
                        "Expected comma or closing parenthesis in variable list",
                        self.current_position(),
                    ));
                }
            }

            self.expect(TokenKind::RightParen)?; // consume )

            // No longer parse attributes here - they're parsed per variable above
            let attributes = Vec::new();

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                self.tokens.next()?; // consume =
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            // Don't consume semicolon here - let parse_statement handle it uniformly

            let end = self.previous_position();
            let node = Node::new(
                NodeKind::VariableListDeclaration {
                    declarator,
                    variables,
                    attributes,
                    initializer,
                },
                SourceLocation { start, end },
            );
            Ok(node)
        } else {
            // Single variable declaration
            // For 'local', we need to parse lvalue expressions (not just simple variables)
            // because local can take complex forms like local $ENV{PATH}
            let variable = if declarator == "local" {
                // For local, parse a general lvalue expression
                self.parse_assignment()?
            } else {
                // For my/our/state, parse a simple variable
                self.parse_variable()?
            };

            // Parse optional attributes
            let mut attributes = Vec::new();
            while self.peek_kind() == Some(TokenKind::Colon) {
                self.tokens.next()?; // consume colon
                let attr_token = self.expect(TokenKind::Identifier)?;
                attributes.push(attr_token.text.to_string());
            }

            let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
                self.tokens.next()?; // consume =
                Some(Box::new(self.parse_expression()?))
            } else {
                None
            };

            // Don't consume semicolon here - let parse_statement handle it uniformly

            let end = self.previous_position();
            let node = Node::new(
                NodeKind::VariableDeclaration {
                    declarator,
                    variable: Box::new(variable),
                    attributes,
                    initializer,
                },
                SourceLocation { start, end },
            );
            Ok(node)
        }
    }

    /// Parse local statement (can localize any lvalue, not just simple variables)
    fn parse_local_statement(&mut self) -> ParseResult<Node> {
        let start = self.current_position();
        let declarator_token = self.consume_token()?; // consume 'local'
        let declarator = declarator_token.text.to_string();

        // Parse the lvalue expression that's being localized
        let variable = Box::new(self.parse_expression()?);

        let initializer = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            Some(Box::new(self.parse_expression()?))
        } else {
            None
        };

        let end = self.previous_position();
        let node = Node::new(
            NodeKind::VariableDeclaration {
                declarator,
                variable,
                attributes: Vec::new(),
                initializer,
            },
            SourceLocation { start, end },
        );
        Ok(node)
    }

    /// Parse a variable ($foo, @bar, %baz)
    fn parse_variable(&mut self) -> ParseResult<Node> {
        // If the next token is a sigil token, delegate to parse_variable_from_sigil
        // This handles cases where the lexer splits sigil and name (e.g. "%" "hash" vs "%hash")
        // Also handles operators that can act as sigils in this context (%, &, *)
        if let Some(kind) = self.peek_kind() {
            match kind {
                TokenKind::ScalarSigil
                | TokenKind::ArraySigil
                | TokenKind::HashSigil
                | TokenKind::SubSigil
                | TokenKind::GlobSigil
                | TokenKind::Percent     // %hash
                | TokenKind::BitwiseAnd  // &sub
                | TokenKind::Star => {   // *glob
                    return self.parse_variable_from_sigil();
                }
                _ => {}
            }
        }

        let token = self.consume_token()?;

        // The lexer returns variables as identifiers like "$x", "@array", etc.
        // We need to split the sigil from the name
        let text = &token.text;

        // Special handling for @{ and %{ (array/hash dereference)
        if &**text == "@{" || &**text == "%{" {
            let sigil = text
                .chars()
                .next()
                .ok_or_else(|| {
                    ParseError::syntax("Empty token text for array/hash dereference", token.start)
                })?
                .to_string();
            let start = token.start;

            // Parse the expression inside the braces
            let expr = self.parse_expression()?;

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(expr) },
                SourceLocation { start, end },
            ));
        }

        let (sigil, name) = if let Some(rest) = text.strip_prefix('$') {
            ("$".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('@') {
            ("@".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('%') {
            ("%".to_string(), rest.to_string())
        } else if let Some(rest) = text.strip_prefix('&') {
            ("&".to_string(), rest.to_string())
        } else if text.starts_with('*') && text.len() > 1 {
            let rest = &text[1..];
            ("*".to_string(), rest.to_string())
        } else {
            return Err(ParseError::syntax(
                format!("Expected variable, found '{}'", text),
                token.start,
            ));
        };

        // Check if the variable name is followed by :: for package-qualified variables
        let mut full_name = name;
        let mut end = token.end;

        // Handle :: in package-qualified variables
        while self.peek_kind() == Some(TokenKind::DoubleColon) {
            self.tokens.next()?; // consume ::
            full_name.push_str("::");

            // The next part might be an identifier or another variable
            if self.peek_kind() == Some(TokenKind::Identifier) {
                let name_token = self.tokens.next()?;
                full_name.push_str(&name_token.text);
                end = name_token.end;
            } else {
                // Handle cases like $Foo::$bar
                return Err(ParseError::syntax(
                    "Expected identifier after :: in package-qualified variable",
                    self.current_position(),
                ));
            }
        }

        if sigil == "*" {
            Ok(Node::new(
                NodeKind::Typeglob { name: full_name },
                SourceLocation { start: token.start, end },
            ))
        } else {
            Ok(Node::new(
                NodeKind::Variable { sigil, name: full_name },
                SourceLocation { start: token.start, end },
            ))
        }
    }

    /// Parse a variable when we have a sigil token first
    fn parse_variable_from_sigil(&mut self) -> ParseResult<Node> {
        let sigil_token = self.consume_token()?;
        let sigil = match sigil_token.kind {
            TokenKind::BitwiseAnd => "&".to_string(), // Handle & as sigil
            _ => sigil_token.text.to_string(),
        };
        let start = sigil_token.start;

        // Check if next token is an identifier or a keyword that should be treated as identifier
        let next_kind = self.peek_kind();
        let can_be_sub_name = |k: TokenKind| {
            matches!(
                k,
                TokenKind::Sub
                    | TokenKind::My
                    | TokenKind::Our
                    | TokenKind::If
                    | TokenKind::Unless
                    | TokenKind::While
                    | TokenKind::For
                    | TokenKind::Return
                    | TokenKind::Do
                    | TokenKind::Eval
                    | TokenKind::Use
                    | TokenKind::Package
            )
        };

        let (name, end) = if next_kind == Some(TokenKind::Identifier) ||
                             // Keywords that can be used as subroutine names with & sigil
                             (sigil == "&" && matches!(next_kind, Some(k) if can_be_sub_name(k)))
        {
            let name_token = self.tokens.next()?;
            let mut name = name_token.text.to_string();
            let mut end = name_token.end;

            // Handle :: in package-qualified variables
            while self.peek_kind() == Some(TokenKind::DoubleColon) {
                self.tokens.next()?; // consume ::
                name.push_str("::");

                if self.peek_kind() == Some(TokenKind::Identifier) {
                    let next_token = self.tokens.next()?;
                    name.push_str(&next_token.text);
                    end = next_token.end;
                } else {
                    return Err(ParseError::syntax(
                        "Expected identifier after :: in package-qualified variable",
                        self.current_position(),
                    ));
                }
            }

            (name, end)
        } else {
            // Handle special variables like $$, $@, $!, $?, etc.
            match self.peek_kind() {
                Some(TokenKind::ScalarSigil) => {
                    // $$ - process ID
                    let token = self.tokens.next()?;
                    ("$".to_string(), token.end)
                }
                Some(TokenKind::ArraySigil) => {
                    // $@ - eval error
                    let token = self.tokens.next()?;
                    ("@".to_string(), token.end)
                }
                Some(TokenKind::Not) => {
                    // $! - system error
                    let token = self.tokens.next()?;
                    ("!".to_string(), token.end)
                }
                Some(TokenKind::Unknown) => {
                    // Could be $?, $^, $#, or other special
                    let token = self.tokens.peek()?;
                    match token.text.as_ref() {
                        "?" => {
                            let token = self.tokens.next()?;
                            ("?".to_string(), token.end)
                        }
                        "^" => {
                            // Handle $^X variables
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                (format!("^{}", var_token.text), var_token.end)
                            } else {
                                ("^".to_string(), token.end)
                            }
                        }
                        "#" => {
                            // Handle $# (array length)
                            let token = self.tokens.next()?;
                            if self.peek_kind() == Some(TokenKind::Identifier) {
                                let var_token = self.tokens.next()?;
                                (format!("#{}", var_token.text), var_token.end)
                            } else {
                                // Just $# by itself
                                ("#".to_string(), token.end)
                            }
                        }
                        _ => {
                            return Err(ParseError::syntax(
                                format!("Unexpected character after sigil: {}", token.text),
                                token.start,
                            ));
                        }
                    }
                }
                Some(TokenKind::Number) => {
                    // $0, $1, $2, etc. - numbered capture groups
                    let num_token = self.tokens.next()?;
                    (num_token.text.to_string(), num_token.end)
                }
                _ => {
                    // Empty variable name (just the sigil)
                    (String::new(), self.previous_position())
                }
            }
        };

        // Special handling for @ or % sigil followed by { - array/hash dereference
        if (sigil == "@" || sigil == "%") && self.peek_kind() == Some(TokenKind::LeftBrace) {
            self.tokens.next()?; // consume {

            // Parse the expression inside the braces
            let expr = self.parse_expression()?;

            self.expect(TokenKind::RightBrace)?;
            let end = self.previous_position();

            let op = format!("{}{{}}", sigil);
            return Ok(Node::new(
                NodeKind::Unary { op, operand: Box::new(expr) },
                SourceLocation { start, end },
            ));
        }

        // Special handling for & sigil - it's a function call
        if sigil == "&" {
            // Check if there are parentheses for arguments
            let args = if self.peek_kind() == Some(TokenKind::LeftParen) {
                self.consume_token()?; // consume (
                let mut args = vec![];

                // EOF guard to prevent infinite loop on truncated input
                while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
                    args.push(self.parse_expression()?);

                    if self.peek_kind() == Some(TokenKind::Comma) {
                        self.consume_token()?; // consume comma
                    } else if self.peek_kind() != Some(TokenKind::RightParen)
                        && !self.tokens.is_eof()
                    {
                        return Err(ParseError::syntax(
                            "Expected comma or right parenthesis",
                            self.current_position(),
                        ));
                    }
                }

                let right_paren = self.expect(TokenKind::RightParen)?;
                let _end = right_paren.end;
                args
            } else {
                vec![]
            };

            Ok(Node::new(NodeKind::FunctionCall { name, args }, SourceLocation { start, end }))
        } else if sigil == "*" {
            Ok(Node::new(NodeKind::Typeglob { name }, SourceLocation { start, end }))
        } else {
            Ok(Node::new(NodeKind::Variable { sigil, name }, SourceLocation { start, end }))
        }
    }

    /// Parse subroutine signature
    fn parse_signature(&mut self) -> ParseResult<Vec<Node>> {
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut params = Vec::new();

        while self.peek_kind() != Some(TokenKind::RightParen) && !self.tokens.is_eof() {
            // Parse parameter
            let param = self.parse_signature_param()?;
            params.push(param);

            // Check for comma or end of signature
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.tokens.next()?; // consume comma
            } else if self.peek_kind() == Some(TokenKind::RightParen) {
                break;
            } else {
                return Err(ParseError::syntax(
                    "Expected comma or closing parenthesis in signature",
                    self.current_position(),
                ));
            }
        }

        self.expect(TokenKind::RightParen)?; // consume )
        Ok(params)
    }

    /// Parse a single signature parameter
    fn parse_signature_param(&mut self) -> ParseResult<Node> {
        let start = self.current_position();

        // Check for named parameter (:$name)
        let named = if self.peek_kind() == Some(TokenKind::Colon) {
            self.tokens.next()?; // consume :
            true
        } else {
            false
        };

        // Check for type constraint (Type $var)
        let _type_constraint = if self.peek_kind() == Some(TokenKind::Identifier) {
            // Look ahead to see if this is a type constraint
            let token = self.tokens.peek()?;
            if !token.text.starts_with('$')
                && !token.text.starts_with('@')
                && !token.text.starts_with('%')
                && !token.text.starts_with('&')
            {
                // It's likely a type constraint
                Some(self.tokens.next()?.text.to_string())
            } else {
                None
            }
        } else {
            None
        };

        // Parse the variable
        let variable = self.parse_variable()?;

        // Check for default value (= expression)
        let default_value = if self.peek_kind() == Some(TokenKind::Assign) {
            self.tokens.next()?; // consume =
            // Parse a primary expression for default value to avoid parsing too far
            Some(Box::new(self.parse_primary()?))
        } else {
            None
        };

        let end = if let Some(ref default) = default_value {
            default.location.end
        } else {
            variable.location.end
        };

        // Check if variable is slurpy (@args or %hash)
        let is_slurpy = matches!(&variable.kind, NodeKind::Variable { sigil, .. } if sigil == "@" || sigil == "%");

        // Create the appropriate parameter node type
        let param_kind = if named {
            NodeKind::NamedParameter { variable: Box::new(variable) }
        } else if is_slurpy {
            NodeKind::SlurpyParameter { variable: Box::new(variable) }
        } else if let Some(default) = default_value {
            NodeKind::OptionalParameter { variable: Box::new(variable), default_value: default }
        } else {
            NodeKind::MandatoryParameter { variable: Box::new(variable) }
        };

        Ok(Node::new(param_kind, SourceLocation { start, end }))
    }

    /// Check if the parenthesized content after sub name is a prototype (not a signature)
    #[allow(dead_code)]
    fn is_prototype(&mut self) -> bool {
        // Peek at the next token after (
        match self.tokens.peek_second() {
            Ok(token) => {
                // Check if it starts with prototype characters or looks like a prototype
                matches!(token.kind,
                    TokenKind::ScalarSigil | TokenKind::ArraySigil |
                    TokenKind::HashSigil | TokenKind::SubSigil |
                    TokenKind::Star | TokenKind::Semicolon |
                    TokenKind::Backslash) ||
                // Check for special vars that look like prototypes ($$, $#, etc)
                (token.kind == TokenKind::Identifier &&
                 token.text.chars().all(|c| matches!(c, '$' | '@' | '%' | '*' | '&' | ';' | '\\')))
            }
            Err(_) => false,
        }
    }

    /// Check if the parentheses likely contain a prototype rather than a signature
    fn is_likely_prototype(&mut self) -> ParseResult<bool> {
        // We need to peek past the opening paren without consuming
        // First, ensure we're at a left paren
        if self.tokens.peek()?.kind != TokenKind::LeftParen {
            return Ok(false);
        }

        // Use peek_second to look at the token after the paren
        match self.tokens.peek_second() {
            Ok(token) => {
                Ok(match token.kind {
                    // These are unambiguously prototype tokens
                    TokenKind::Star
                    | TokenKind::Backslash
                    | TokenKind::Semicolon
                    | TokenKind::BitwiseAnd
                    | TokenKind::SubSigil
                    | TokenKind::GlobSigil => true,
                    // Sigils: peek past to distinguish prototype ($;@%) from signature ($x, @rest)
                    TokenKind::ScalarSigil
                    | TokenKind::ArraySigil
                    | TokenKind::HashSigil => {
                        match self.tokens.peek_third() {
                            Ok(third) => !matches!(third.kind, TokenKind::Identifier),
                            Err(_) => true, // default to prototype on error
                        }
                    }
                    // Empty prototype
                    TokenKind::RightParen => true,
                    // Colon indicates named parameter (:$foo), so it's a signature
                    TokenKind::Colon => false,
                    // Identifiers usually mean signature, but could be a special case
                    TokenKind::Identifier => {
                        // Check if it's a sigil-only identifier like "$" or "@"
                        // or the special underscore prototype
                        &*token.text == "_"
                            || token.text.chars().all(|c| matches!(c, '$' | '@' | '%' | '*' | '&'))
                    }
                    // Anything else suggests a signature
                    _ => false,
                })
            }
            Err(_) => Ok(false),
        }
    }

    /// Parse old-style prototype
    fn parse_prototype(&mut self) -> ParseResult<String> {
        self.expect(TokenKind::LeftParen)?; // consume (
        let mut prototype = String::new();

        while !self.tokens.is_eof() {
            let token = self.tokens.next()?;

            match token.kind {
                TokenKind::RightParen => {
                    // End of prototype
                    break;
                }
                TokenKind::ScalarSigil => prototype.push('$'),
                TokenKind::ArraySigil => prototype.push('@'),
                TokenKind::HashSigil => prototype.push('%'),
                TokenKind::GlobSigil | TokenKind::Star => prototype.push('*'),
                TokenKind::SubSigil | TokenKind::BitwiseAnd => prototype.push('&'),
                TokenKind::Semicolon => prototype.push(';'),
                TokenKind::Backslash => prototype.push('\\'),
                _ => {
                    // For any other token, just add its text
                    // This handles cases where sigils might be parsed differently
                    prototype.push_str(&token.text);
                }
            }
        }

        Ok(prototype)
    }

}

#[cfg(test)]
mod prototype_heuristic_tests {
    use super::*;

    /// Helper: parse code and extract the first Subroutine node.
    fn parse_sub(code: &str) -> Option<Node> {
        let mut parser = Parser::new(code);
        let ast = parser.parse().ok()?;
        if let NodeKind::Program { statements } = ast.kind {
            statements.into_iter().next()
        } else {
            None
        }
    }

    #[test]
    fn signature_with_named_params() {
        let node = parse_sub("sub foo($x) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($x) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { signature, prototype, .. } = &node.kind {
            assert!(signature.is_some(), "sub foo($x) should have a signature");
            assert!(prototype.is_none(), "sub foo($x) should not have a prototype");
        }
    }

    #[test]
    fn signature_with_multiple_params() {
        let node = parse_sub("sub foo($x, $y) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($x, $y) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { signature, .. } = &node.kind {
            assert!(signature.is_some(), "sub foo($x, $y) should have a signature");
        }
    }

    #[test]
    fn prototype_single_sigil() {
        let node = parse_sub("sub foo($) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, signature, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo($) should have a prototype");
            assert!(signature.is_none(), "sub foo($) should not have a signature");
        }
    }

    #[test]
    fn prototype_with_semicolon() {
        let node = parse_sub("sub foo($;@) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo($;@) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo($;@) should have a prototype");
        }
    }

    #[test]
    fn prototype_empty() {
        let node = parse_sub("sub foo() {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo() {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo() should have a prototype (empty)");
        }
    }

    #[test]
    fn prototype_with_sub_sigil() {
        let node = parse_sub("sub foo(&) {}");
        assert!(node.is_some(), "expected parsed subroutine for `sub foo(&) {{}}`");
        let Some(node) = node else {
            return;
        };
        assert!(
            matches!(&node.kind, NodeKind::Subroutine { .. }),
            "expected Subroutine node, got {}",
            node.kind.kind_name()
        );

        if let NodeKind::Subroutine { prototype, .. } = &node.kind {
            assert!(prototype.is_some(), "sub foo(&) should have a prototype");
        }
    }
}
