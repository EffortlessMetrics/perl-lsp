# Error Recovery Investigation for v3 Perl Parser

## Executive Summary

Error recovery is crucial for IDE support, allowing the parser to continue processing after encountering syntax errors and produce partial ASTs. This enables features like syntax highlighting, code folding, and partial semantic analysis even in the presence of errors.

## Current Error Handling

### Current Approach
The parser currently uses a "fail-fast" strategy:
```rust
// Current error handling stops immediately
match self.parse_expression() {
    Ok(expr) => expr,
    Err(e) => return Err(e), // Stops parsing
}
```

### Error Types
```rust
pub enum ParseError {
    UnexpectedEof,
    UnexpectedToken { expected: String, found: String, location: usize },
    SyntaxError { message: String, location: usize },
    LexerError { message: String },
    RecursionLimit,
    InvalidNumber { literal: String },
    InvalidString,
    UnclosedDelimiter { delimiter: char },
    InvalidRegex { message: String },
}
```

## Error Recovery Strategies

### 1. Panic Mode Recovery
Skip tokens until reaching a synchronization point.

**Synchronization Points in Perl:**
- Statement terminators: `;`, `}`
- Block boundaries: `{`, `}`
- Keywords: `sub`, `if`, `while`, `for`, `my`, `our`
- Package/class declarations: `package`, `class`

### 2. Phrase-Level Recovery
Try to parse partial constructs and create error nodes.

**Examples:**
- `if ($x` → Create partial if node with error condition
- `sub foo {` → Create partial subroutine with missing closing brace
- `my $x =` → Create declaration with missing initializer

### 3. Error Productions
Add grammar rules specifically for common errors.

**Common Perl Errors:**
- Missing semicolons
- Unclosed strings/regexes
- Missing parentheses in conditionals
- Incorrect operator precedence

## Implementation Design

### Phase 1: Error Node Infrastructure

#### 1.1 Enhanced NodeKind
```rust
pub enum NodeKind {
    // ... existing variants ...
    
    // Error recovery nodes
    Error {
        message: String,
        expected: Vec<TokenKind>,
        found: Option<Token>,
        partial: Option<Box<Node>>,
    },
    
    // Missing elements
    MissingExpression,
    MissingStatement,
    MissingIdentifier,
    MissingBlock,
}
```

#### 1.2 Parser State for Recovery
```rust
pub struct Parser<'a> {
    // ... existing fields ...
    
    // Error recovery state
    errors: Vec<ParseError>,
    recovery_mode: bool,
    synchronization_points: Vec<TokenKind>,
    max_errors: usize,
}

impl<'a> Parser<'a> {
    pub fn with_error_recovery(input: &'a str) -> Self {
        let mut parser = Parser::new(input);
        parser.max_errors = 100;
        parser.synchronization_points = vec![
            TokenKind::Semicolon,
            TokenKind::RightBrace,
            TokenKind::Sub,
            TokenKind::Package,
            TokenKind::If,
            TokenKind::While,
            TokenKind::For,
            TokenKind::My,
            TokenKind::Our,
        ];
        parser
    }
}
```

### Phase 2: Recovery Functions

#### 2.1 Basic Recovery
```rust
impl<'a> Parser<'a> {
    fn recover_to_synchronization_point(&mut self) -> Option<Token> {
        while let Ok(token) = self.tokens.peek() {
            if self.synchronization_points.contains(&token.kind) {
                return Some(token.clone());
            }
            self.tokens.next().ok();
        }
        None
    }
    
    fn create_error_node(&mut self, message: String, expected: Vec<TokenKind>) -> Node {
        let start = self.current_position();
        let found = self.tokens.peek().ok().cloned();
        
        Node::new(
            NodeKind::Error {
                message,
                expected,
                found,
                partial: None,
            },
            SourceLocation { start, end: start }
        )
    }
}
```

#### 2.2 Statement Recovery
```rust
fn parse_statement_with_recovery(&mut self) -> ParseResult<Node> {
    match self.parse_statement() {
        Ok(stmt) => Ok(stmt),
        Err(e) => {
            // Store the error
            self.errors.push(e.clone());
            
            // Try to recover
            if self.errors.len() < self.max_errors {
                self.recovery_mode = true;
                
                // Create error node
                let error_node = self.create_error_node(
                    format!("Failed to parse statement: {}", e),
                    vec![] // Could be enhanced with expected tokens
                );
                
                // Skip to next statement
                self.recover_to_synchronization_point();
                
                self.recovery_mode = false;
                Ok(error_node)
            } else {
                Err(ParseError::Other("Too many errors".to_string()))
            }
        }
    }
}
```

#### 2.3 Expression Recovery
```rust
fn parse_expression_with_recovery(&mut self) -> ParseResult<Node> {
    match self.parse_expression() {
        Ok(expr) => Ok(expr),
        Err(e) => {
            if self.recovery_mode {
                // Already in recovery, don't recurse
                return Err(e);
            }
            
            self.errors.push(e.clone());
            
            // Try common recovery strategies
            match self.peek_kind() {
                Some(TokenKind::Semicolon) | Some(TokenKind::RightParen) | 
                Some(TokenKind::RightBrace) | Some(TokenKind::Comma) => {
                    // Likely missing expression
                    Ok(Node::new(
                        NodeKind::MissingExpression,
                        SourceLocation { 
                            start: self.current_position(), 
                            end: self.current_position() 
                        }
                    ))
                }
                _ => {
                    // Skip one token and try again
                    self.tokens.next().ok();
                    self.parse_expression_with_recovery()
                }
            }
        }
    }
}
```

### Phase 3: Specific Recovery Strategies

#### 3.1 Block Recovery
```rust
fn parse_block_with_recovery(&mut self) -> ParseResult<Node> {
    let start = self.current_position();
    
    // Expect opening brace
    if let Err(e) = self.expect(TokenKind::LeftBrace) {
        self.errors.push(e);
        // Create empty block error node
        return Ok(Node::new(
            NodeKind::Error {
                message: "Expected block".to_string(),
                expected: vec![TokenKind::LeftBrace],
                found: self.peek_token().ok().cloned(),
                partial: None,
            },
            SourceLocation { start, end: self.current_position() }
        ));
    }
    
    let mut statements = Vec::new();
    
    while self.peek_kind() != Some(TokenKind::RightBrace) && !self.tokens.is_eof() {
        match self.parse_statement_with_recovery() {
            Ok(stmt) => statements.push(stmt),
            Err(_) => {
                // Skip to next potential statement start
                self.recover_to_synchronization_point();
            }
        }
    }
    
    // Handle missing closing brace
    let end = if self.peek_kind() == Some(TokenKind::RightBrace) {
        self.tokens.next().ok();
        self.previous_position()
    } else {
        self.errors.push(ParseError::UnclosedDelimiter { delimiter: '}' });
        self.current_position()
    };
    
    Ok(Node::new(
        NodeKind::Block { statements },
        SourceLocation { start, end }
    ))
}
```

#### 3.2 If Statement Recovery
```rust
fn parse_if_with_recovery(&mut self) -> ParseResult<Node> {
    let start = self.current_position();
    self.tokens.next()?; // consume 'if'
    
    // Parse condition with recovery
    let condition = match self.expect(TokenKind::LeftParen) {
        Ok(_) => {
            let cond = self.parse_expression_with_recovery()?;
            if let Err(e) = self.expect(TokenKind::RightParen) {
                self.errors.push(e);
                // Continue anyway
            }
            cond
        }
        Err(e) => {
            self.errors.push(e);
            // Try to parse condition without parens
            self.parse_expression_with_recovery()?
        }
    };
    
    // Parse then block
    let then_branch = self.parse_block_with_recovery()?;
    
    // Parse optional elsif/else
    let mut elsif_branches = Vec::new();
    let mut else_branch = None;
    
    while self.peek_kind() == Some(TokenKind::Elsif) {
        self.tokens.next()?; // consume 'elsif'
        
        let elsif_cond = match self.expect(TokenKind::LeftParen) {
            Ok(_) => {
                let cond = self.parse_expression_with_recovery()?;
                self.expect(TokenKind::RightParen).ok(); // Ignore error
                cond
            }
            Err(_) => self.parse_expression_with_recovery()?
        };
        
        let elsif_block = self.parse_block_with_recovery()?;
        elsif_branches.push((elsif_cond, elsif_block));
    }
    
    if self.peek_kind() == Some(TokenKind::Else) {
        self.tokens.next()?; // consume 'else'
        else_branch = Some(self.parse_block_with_recovery()?);
    }
    
    Ok(Node::new(
        NodeKind::If {
            condition: Box::new(condition),
            then_branch: Box::new(then_branch),
            elsif_branches,
            else_branch: else_branch.map(Box::new),
        },
        SourceLocation { start, end: self.previous_position() }
    ))
}
```

### Phase 4: Error Reporting

#### 4.1 Error Collection
```rust
pub struct ParseResult {
    pub ast: Option<Node>,
    pub errors: Vec<ParseError>,
}

impl Parser<'_> {
    pub fn parse_with_recovery(&mut self) -> ParseResult {
        let ast = match self.parse_program_with_recovery() {
            Ok(node) => Some(node),
            Err(e) => {
                self.errors.push(e);
                None
            }
        };
        
        ParseResult {
            ast,
            errors: self.errors.clone(),
        }
    }
}
```

#### 4.2 Error Context
```rust
#[derive(Debug, Clone)]
pub struct ErrorContext {
    pub error: ParseError,
    pub line: usize,
    pub column: usize,
    pub source_line: String,
    pub suggestion: Option<String>,
}

impl Parser<'_> {
    pub fn get_error_contexts(&self, source: &str) -> Vec<ErrorContext> {
        self.errors.iter().map(|error| {
            let location = error.location();
            let (line, column) = self.position_to_line_col(source, location);
            let source_line = source.lines().nth(line - 1).unwrap_or("").to_string();
            
            let suggestion = match error {
                ParseError::UnexpectedToken { expected, .. } => {
                    Some(format!("Expected: {}", expected))
                }
                ParseError::UnclosedDelimiter { delimiter } => {
                    Some(format!("Add closing '{}'", delimiter))
                }
                _ => None,
            };
            
            ErrorContext {
                error: error.clone(),
                line,
                column,
                source_line,
                suggestion,
            }
        }).collect()
    }
}
```

## Integration with IDEs

### Language Server Protocol Support
```rust
pub fn parse_for_lsp(source: &str) -> LspParseResult {
    let mut parser = Parser::with_error_recovery(source);
    let result = parser.parse_with_recovery();
    
    LspParseResult {
        ast: result.ast,
        diagnostics: result.errors.into_iter().map(|e| {
            Diagnostic {
                range: error_to_range(&e, source),
                severity: DiagnosticSeverity::Error,
                message: e.to_string(),
                source: Some("perl-parser".to_string()),
            }
        }).collect(),
    }
}
```

### Partial AST Features
1. **Syntax Highlighting**: Use partial AST to highlight valid portions
2. **Code Folding**: Identify block structures even with errors
3. **Outline View**: Show document structure from partial AST
4. **Semantic Tokens**: Provide token types for valid portions

## Testing Strategy

### 1. Unit Tests
```rust
#[test]
fn test_missing_semicolon_recovery() {
    let input = "my $x = 42\nmy $y = 43;";
    let result = parse_with_recovery(input);
    
    assert!(result.ast.is_some());
    assert_eq!(result.errors.len(), 1);
    assert!(matches!(result.errors[0], ParseError::UnexpectedToken { .. }));
}

#[test]
fn test_unclosed_block_recovery() {
    let input = "if ($x) { print 'hello'";
    let result = parse_with_recovery(input);
    
    assert!(result.ast.is_some());
    assert_eq!(result.errors.len(), 1);
    assert!(matches!(result.errors[0], ParseError::UnclosedDelimiter { .. }));
}
```

### 2. Integration Tests
- Test recovery across multiple error types
- Verify partial AST structure
- Check error positions and messages

### 3. Fuzzing
- Generate random mutations of valid Perl code
- Ensure parser doesn't crash
- Verify some AST is always produced

## Performance Considerations

### 1. Error Limit
Set maximum error count to prevent infinite loops:
```rust
const MAX_ERRORS: usize = 100;
```

### 2. Recovery Caching
Cache synchronization points for faster recovery:
```rust
struct RecoveryCache {
    sync_points: Vec<(usize, TokenKind)>,
}
```

### 3. Lazy Error Messages
Generate detailed error messages only when requested:
```rust
pub enum ErrorDetail {
    Simple(String),
    Detailed(Box<dyn Fn() -> String>),
}
```

## Implementation Timeline

### Phase 1: Basic Infrastructure (1 week)
- [ ] Add error node types
- [ ] Implement error collection
- [ ] Create recovery mode flag

### Phase 2: Core Recovery (2 weeks)
- [ ] Implement panic mode recovery
- [ ] Add synchronization points
- [ ] Create basic recovery functions

### Phase 3: Specific Constructs (2-3 weeks)
- [ ] Block recovery
- [ ] If/while/for recovery
- [ ] Expression recovery
- [ ] Function call recovery

### Phase 4: Testing & Refinement (1-2 weeks)
- [ ] Comprehensive test suite
- [ ] Performance optimization
- [ ] Error message improvements

Total: 6-8 weeks

## Success Metrics

1. **Robustness**: Parser completes on 99% of invalid inputs
2. **Accuracy**: 90% of valid code portions correctly parsed
3. **Performance**: < 20% overhead vs non-recovery parsing
4. **Usefulness**: IDE features work on 95% of incomplete code

## Conclusion

Error recovery is essential for IDE integration and requires:
1. Infrastructure for error nodes and collection
2. Multiple recovery strategies (panic, phrase-level, productions)
3. Careful handling of Perl's complex syntax
4. Extensive testing with real-world error patterns

The implementation can be done incrementally, with each phase adding value. The key challenge is balancing recovery accuracy with performance.