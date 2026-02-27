//! Token-based Perl parser using chumsky
//!
//! This module provides a token-based parser implementation that consumes
//! tokens from the logos lexer and produces the same AST as the Pest parser.

use chumsky::prelude::*;
use std::sync::Arc;

use crate::logos_lexer::{PerlLexer, Token};
use perl_parser_pest::AstNode;

/// Type alias for parser results
type ParserResult<'a, T> = Result<T, Vec<Simple<Token>>>;

/// Main parser struct
pub struct TokenParser;

impl TokenParser {
    /// Parse a complete Perl program
    pub fn parse(input: &str) -> ParserResult<AstNode> {
        let mut lexer = PerlLexer::new(input);
        let tokens = Self::collect_tokens(&mut lexer);

        let parser = Self::program();
        parser.parse(tokens)
    }

    /// Collect all tokens from the lexer
    fn collect_tokens(lexer: &mut PerlLexer) -> Vec<Token> {
        let mut tokens = Vec::new();
        while let Some((token, _span)) = lexer.next() {
            tokens.push(token);
        }
        tokens.push(Token::Eof);
        tokens
    }

    /// Parser for a complete program
    fn program() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        Self::statement().repeated().map(|statements| AstNode::Program(statements))
    }

    /// Parser for statements
    fn statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        choice((
            Self::variable_declaration(),
            Self::sub_declaration(),
            Self::format_declaration(),
            Self::package_declaration(),
            Self::use_statement(),
            Self::require_statement(),
            Self::if_statement(),
            Self::unless_statement(),
            Self::while_statement(),
            Self::for_statement(),
            Self::expression_statement(),
        ))
        .map(|node| AstNode::Statement(Box::new(node)))
    }

    /// Parser for variable declarations (my, our, local, state)
    fn variable_declaration() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        let scope = choice((
            just(Token::My).to("my"),
            just(Token::Our).to("our"),
            just(Token::Local).to("local"),
            just(Token::State).to("state"),
        ));

        let var_list = Self::variable().separated_by(just(Token::Comma)).allow_trailing();

        scope
            .then(var_list)
            .then(just(Token::Assign).ignore_then(Self::expression()).or_not())
            .then_ignore(just(Token::Semicolon).or_not())
            .map(|((scope, variables), initializer)| AstNode::VariableDeclaration {
                scope: Arc::from(scope),
                variables,
                initializer: initializer.map(Box::new),
            })
    }

    /// Parser for subroutine declarations
    fn sub_declaration() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        just(Token::Sub)
            .ignore_then(Self::identifier())
            .then(Self::prototype().or_not())
            .then(Self::attributes().or_not())
            .then(Self::block())
            .map(|(((name, prototype), attributes), body)| AstNode::SubDeclaration {
                name: Arc::from(name),
                prototype: prototype.map(Arc::from),
                attributes: attributes.unwrap_or_default().into_iter().map(Arc::from).collect(),
                body: Box::new(body),
            })
    }

    /// Parser for format declarations
    fn format_declaration() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        just(Token::Format)
            .ignore_then(Self::identifier().or_not())
            .then_ignore(just(Token::Assign))
            .then_ignore(just(Token::Newline))
            .then(filter(|t| !matches!(t, Token::Dot)).repeated().collect::<Vec<_>>())
            .then_ignore(just(Token::Dot))
            .then_ignore(just(Token::Newline).or_not())
            .map(|(name, lines)| AstNode::FormatDeclaration {
                name: Arc::from(name.unwrap_or_else(|| "STDOUT".to_string())),
                format_lines: lines.into_iter().map(|t| Arc::from(t.to_string())).collect(),
            })
    }

    /// Parser for package declarations
    fn package_declaration() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        just(Token::Package)
            .ignore_then(Self::qualified_name())
            .then(Self::version().or_not())
            .then(choice((
                just(Token::Semicolon).to(None),
                Self::block().map(|b| Some(Box::new(b))),
            )))
            .map(|((name, version), block)| AstNode::PackageDeclaration {
                name: Arc::from(name),
                version: version.map(Arc::from),
                block,
            })
    }

    /// Parser for use statements
    fn use_statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        just(Token::Use)
            .ignore_then(Self::qualified_name())
            .then(Self::version().or_not())
            .then(Self::import_list().or_not())
            .then_ignore(just(Token::Semicolon).or_not())
            .map(|((module, version), import_list)| AstNode::UseStatement {
                module: Arc::from(module),
                version: version.map(Arc::from),
                import_list: import_list.unwrap_or_default().into_iter().map(Arc::from).collect(),
            })
    }

    /// Parser for require statements
    fn require_statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        just(Token::Require)
            .ignore_then(Self::qualified_name())
            .then_ignore(just(Token::Semicolon).or_not())
            .map(|module| AstNode::RequireStatement { module: Arc::from(module) })
    }

    /// Parser for if statements
    fn if_statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        let elsif_clause = just(Token::Elsif).ignore_then(Self::expression()).then(Self::block());

        let else_clause = just(Token::Else).ignore_then(Self::block());

        just(Token::If)
            .ignore_then(just(Token::LeftParen).or_not())
            .ignore_then(Self::expression())
            .then_ignore(just(Token::RightParen).or_not())
            .then(Self::block())
            .then(elsif_clause.repeated())
            .then(else_clause.or_not())
            .map(|(((condition, then_block), elsif_clauses), else_block)| AstNode::IfStatement {
                condition: Box::new(condition),
                then_block: Box::new(then_block),
                elsif_clauses,
                else_block: else_block.map(Box::new),
            })
    }

    /// Parser for unless statements
    fn unless_statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        just(Token::Unless)
            .ignore_then(just(Token::LeftParen).or_not())
            .ignore_then(Self::expression())
            .then_ignore(just(Token::RightParen).or_not())
            .then(Self::block())
            .then(just(Token::Else).ignore_then(Self::block()).or_not())
            .map(|((condition, block), else_block)| AstNode::UnlessStatement {
                condition: Box::new(condition),
                block: Box::new(block),
                else_block: else_block.map(Box::new),
            })
    }

    /// Parser for while statements
    fn while_statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        Self::label()
            .or_not()
            .then_ignore(just(Token::While))
            .then(just(Token::LeftParen).or_not())
            .then(Self::expression())
            .then_ignore(just(Token::RightParen).or_not())
            .then(Self::block())
            .map(|(((label, _), condition), block)| AstNode::WhileStatement {
                label: label.map(Arc::from),
                condition: Box::new(condition),
                block: Box::new(block),
            })
    }

    /// Parser for for/foreach statements
    fn for_statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        Self::label()
            .or_not()
            .then(choice((just(Token::For), just(Token::Foreach))))
            .then(
                // C-style for loop
                just(Token::LeftParen)
                    .ignore_then(Self::expression().or_not())
                    .then_ignore(just(Token::Semicolon))
                    .then(Self::expression().or_not())
                    .then_ignore(just(Token::Semicolon))
                    .then(Self::expression().or_not())
                    .then_ignore(just(Token::RightParen))
                    .map(|((init, cond), update)| (init, cond, update, None))
                    .or(
                        // Foreach style
                        Self::variable()
                            .or_not()
                            .then(just(Token::LeftParen).or_not())
                            .then(Self::expression())
                            .then_ignore(just(Token::RightParen).or_not())
                            .map(|((var, _), list)| (None, None, None, Some((var, list)))),
                    ),
            )
            .then(Self::block())
            .try_map(|(((label, _), for_parts), block), span| {
                match for_parts {
                    (Some(init), cond, update, None) => Ok(AstNode::ForStatement {
                        label: label.map(Arc::from),
                        init: Some(Box::new(init)),
                        condition: cond.map(Box::new),
                        update: update.map(Box::new),
                        block: Box::new(block),
                    }),
                    (None, None, None, Some((var, list))) => Ok(AstNode::ForeachStatement {
                        label: label.map(Arc::from),
                        variable: var.map(Box::new),
                        list: Box::new(list),
                        block: Box::new(block),
                    }),
                    // Error: Invalid for-loop structure detected
                    // Valid structures:
                    //   - C-style: for (init; condition; update) { body }
                    //   - Foreach: for my $var (list) { body }
                    // This error indicates the parser found an incompatible combination of for-loop
                    // components that doesn't match either of the valid Perl for-loop patterns.
                    // Common causes: Missing semicolons in C-style for, wrong parenthesis placement,
                    // or mixing C-style and foreach syntax.
                    _ => Err(Simple::custom(
                        span,
                        "Invalid for-loop structure: expected either (init; condition; update) \
                         for C-style loops or (variable in list) for foreach loops. \
                         Check for missing semicolons or mixed syntax.",
                    )),
                }
            })
    }

    /// Parser for expression statements
    fn expression_statement() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        Self::expression().then_ignore(just(Token::Semicolon).or_not())
    }

    /// Parser for expressions (with Pratt precedence)
    fn expression() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        recursive(|expr| {
            let primary = choice((
                Self::number(),
                Self::string(),
                Self::variable(),
                Self::function_call(expr.clone()),
                just(Token::LeftParen)
                    .ignore_then(expr.clone())
                    .then_ignore(just(Token::RightParen)),
            ));

            // Build Pratt parser for operators
            Self::pratt_parser(primary, expr)
        })
    }

    /// Pratt parser for operator precedence
    fn pratt_parser(
        primary: impl Parser<Token, AstNode, Error = Simple<Token>> + Clone,
        expr: impl Parser<Token, AstNode, Error = Simple<Token>> + Clone,
    ) -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        use Token::*;

        // Define operator precedence and associativity
        let op = |t: Token| just(t).map(move |_| t.clone());

        primary
            .pratt((
                // Assignment operators (right associative)
                op(Assign).infix_right(1),
                op(PlusAssign).infix_right(1),
                op(MinusAssign).infix_right(1),
                op(StarAssign).infix_right(1),
                op(SlashAssign).infix_right(1),
                // Ternary operator
                op(Question).then_ignore(expr.clone()).then(op(Colon)).infix_right(2),
                // Logical OR
                op(OrOr).infix_left(3),
                op(DefinedOr).infix_left(3),
                // Logical AND
                op(AndAnd).infix_left(4),
                // Equality
                op(NumEq).infix_left(7),
                op(NumNe).infix_left(7),
                op(StrEq).infix_left(7),
                op(StrNe).infix_left(7),
                op(SmartMatch).infix_left(7),
                // Relational
                op(Lt).infix_left(8),
                op(Gt).infix_left(8),
                op(Le).infix_left(8),
                op(Ge).infix_left(8),
                op(StrLt).infix_left(8),
                op(StrGt).infix_left(8),
                op(StrLe).infix_left(8),
                op(StrGe).infix_left(8),
                // Additive
                op(Plus).infix_left(12),
                op(Minus).infix_left(12),
                op(Dot).infix_left(12),
                // Multiplicative
                op(Star).infix_left(13),
                op(Slash).infix_left(13),
                op(Mod).infix_left(13),
                op(StringRepeat).infix_left(13),
                // Unary operators
                op(Bang).prefix(15),
                op(UnaryMinus).prefix(15),
                op(UnaryPlus).prefix(15),
                op(BitNot).prefix(15),
                // Postfix
                op(Incr).postfix(16),
                op(Decr).postfix(16),
                // Arrow (method calls, dereferences)
                op(Arrow).infix_left(17),
            ))
            .map_infix(|left, op, right| {
                match op {
                    // Defensive programming: The Pratt parser should handle ternary operators (?)
                    // at the appropriate precedence level through the dedicated ternary operator
                    // configuration. If we reach this point in the infix position handler, it
                    // indicates a bug in the Pratt parser precedence configuration or operator
                    // routing logic.
                    Question => AstNode::ErrorNode {
                        message: Arc::from(format!(
                            "Unexpected ternary operator '?' in infix position. \
                             This should be handled by the Pratt parser precedence system. \
                             Found: operator={:?}, left={:?}, right={:?}. \
                             This error indicates a potential bug in the parser implementation.",
                            op, left, right
                        )),
                        content: Arc::from("?"),
                    },
                    _ => AstNode::BinaryOp {
                        op: Arc::from(op.to_string()),
                        left: Box::new(left),
                        right: Box::new(right),
                    },
                }
            })
            .map_prefix(|op, operand| AstNode::UnaryOp {
                op: Arc::from(op.to_string()),
                operand: Box::new(operand),
            })
            .map_postfix(|operand, op| AstNode::UnaryOp {
                op: Arc::from(op.to_string()),
                operand: Box::new(operand),
            })
    }

    /// Parser for blocks
    fn block() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        just(Token::LeftBrace)
            .ignore_then(Self::statement().repeated())
            .then_ignore(just(Token::RightBrace))
            .map(AstNode::Block)
    }

    /// Parser for variables
    fn variable() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        choice((
            filter_map(|span, token| match token {
                Token::ScalarVar(name) => Ok(AstNode::ScalarVariable(Arc::from(name))),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }),
            filter_map(|span, token| match token {
                Token::ArrayVar(name) => Ok(AstNode::ArrayVariable(Arc::from(name))),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }),
            filter_map(|span, token| match token {
                Token::HashVar(name) => Ok(AstNode::HashVariable(Arc::from(name))),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }),
            filter_map(|span, token| match token {
                Token::GlobVar(name) => Ok(AstNode::TypeglobVariable(Arc::from(name))),
                _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
            }),
        ))
    }

    /// Parser for numbers
    fn number() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        filter_map(|span, token| match token {
            Token::Number(n) | Token::HexNumber(n) | Token::BinNumber(n) | Token::OctNumber(n) => {
                Ok(AstNode::Number(Arc::from(n)))
            }
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    }

    /// Parser for strings
    fn string() -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        filter_map(|span, token| match token {
            Token::SingleString(s) | Token::DoubleString(s) | Token::Backtick(s) => {
                Ok(AstNode::String(Arc::from(s)))
            }
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    }

    /// Parser for identifiers
    fn identifier() -> impl Parser<Token, String, Error = Simple<Token>> {
        filter_map(|span, token| match token {
            Token::Identifier(name) => Ok(name),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    }

    /// Parser for qualified names (Foo::Bar)
    fn qualified_name() -> impl Parser<Token, String, Error = Simple<Token>> {
        Self::identifier()
            .then(just(Token::PackageSep).ignore_then(Self::identifier()).repeated())
            .map(
                |(first, rest)| {
                    if rest.is_empty() { first } else { format!("{}::{}", first, rest.join("::")) }
                },
            )
    }

    /// Parser for version numbers
    fn version() -> impl Parser<Token, String, Error = Simple<Token>> {
        filter_map(|span, token| match token {
            Token::Number(n) => Ok(n),
            _ => Err(Simple::expected_input_found(span, vec![], Some(token))),
        })
    }

    /// Parser for import lists
    fn import_list() -> impl Parser<Token, Vec<String>, Error = Simple<Token>> {
        just(Token::LeftParen)
            .ignore_then(Self::identifier().separated_by(just(Token::Comma)).allow_trailing())
            .then_ignore(just(Token::RightParen))
    }

    /// Parser for subroutine prototypes
    fn prototype() -> impl Parser<Token, String, Error = Simple<Token>> {
        just(Token::LeftParen)
            .ignore_then(filter(|t| !matches!(t, Token::RightParen)).repeated().collect::<Vec<_>>())
            .then_ignore(just(Token::RightParen))
            .map(|tokens| tokens.into_iter().map(|t| t.to_string()).collect::<String>())
    }

    /// Parser for attributes
    fn attributes() -> impl Parser<Token, Vec<String>, Error = Simple<Token>> {
        just(Token::Colon)
            .ignore_then(
                Self::identifier()
                    .then(
                        just(Token::LeftParen)
                            .ignore_then(
                                filter(|t| !matches!(t, Token::RightParen))
                                    .repeated()
                                    .collect::<String>(),
                            )
                            .then_ignore(just(Token::RightParen))
                            .or_not(),
                    )
                    .map(
                        |(name, args)| {
                            if let Some(args) = args { format!("{}({})", name, args) } else { name }
                        },
                    )
                    .repeated()
                    .at_least(1),
            )
            .or_not()
            .map(|attrs| attrs.unwrap_or_default())
    }

    /// Parser for labels
    fn label() -> impl Parser<Token, String, Error = Simple<Token>> {
        Self::identifier().then_ignore(just(Token::Colon))
    }

    /// Parser for function calls
    fn function_call(
        expr: impl Parser<Token, AstNode, Error = Simple<Token>> + Clone,
    ) -> impl Parser<Token, AstNode, Error = Simple<Token>> {
        Self::identifier()
            .then(
                just(Token::LeftParen)
                    .ignore_then(expr.separated_by(just(Token::Comma)).allow_trailing())
                    .then_ignore(just(Token::RightParen)),
            )
            .map(|(name, args)| AstNode::FunctionCall {
                function: Box::new(AstNode::Identifier(Arc::from(name))),
                args,
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_variable_declaration() {
        let result = TokenParser::parse("my $x = 42;");
        assert!(result.is_ok());

        let result = TokenParser::parse("our @array = (1, 2, 3);");
        assert!(result.is_ok());
    }

    #[test]
    fn test_if_statement() {
        let result = TokenParser::parse("if ($x > 0) { print $x; }");
        assert!(result.is_ok());
    }

    #[test]
    fn test_expressions() {
        let result = TokenParser::parse("$x = 2 + 3 * 4;");
        assert!(result.is_ok());

        let result = TokenParser::parse("$y = $x > 0 ? $x : -$x;");
        assert!(result.is_ok());
    }
}
