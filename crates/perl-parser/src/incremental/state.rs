use crate::incremental::{LexCheckpoint, ParseCheckpoint, ScopeSnapshot};
use perl_lexer::{PerlLexer, Token, TokenType};
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_parser_core::parser::Parser;
use ropey::Rope;

pub use perl_line_index::LineIndex;

/// Incremental parsing state
#[derive(Clone)]
pub struct IncrementalState {
    pub rope: Rope,
    pub line_index: LineIndex,
    pub lex_checkpoints: Vec<LexCheckpoint>,
    pub parse_checkpoints: Vec<ParseCheckpoint>,
    pub ast: Node,
    pub tokens: Vec<Token>,
    pub source: String,
}

impl IncrementalState {
    /// Initialize incremental state by doing a full parse of `source`.
    pub fn new(source: String) -> Self {
        let rope = Rope::from_str(&source);
        let line_index = LineIndex::new(&source);

        // Parse the initial document
        let mut parser = Parser::new(&source);
        let ast = match parser.parse() {
            Ok(ast) => ast,
            Err(e) => Node::new(
                NodeKind::Error {
                    message: e.to_string(),
                    expected: vec![],
                    found: None,
                    partial: None,
                },
                SourceLocation { start: 0, end: source.len() },
            ),
        };

        // Get tokens from lexer
        let mut lexer = PerlLexer::new(&source);
        let mut tokens = Vec::new();
        loop {
            match lexer.next_token() {
                Some(token) => {
                    if token.token_type == TokenType::EOF {
                        break;
                    }
                    tokens.push(token);
                }
                None => break,
            }
        }

        // Create initial checkpoints
        let lex_checkpoints = Self::create_lex_checkpoints(&tokens, &line_index);
        let parse_checkpoints = Self::create_parse_checkpoints(&ast);

        Self { rope, line_index, lex_checkpoints, parse_checkpoints, ast, tokens, source }
    }

    /// Create lexer checkpoints at safe boundaries
    pub(crate) fn create_lex_checkpoints(tokens: &[Token], line_index: &LineIndex) -> Vec<LexCheckpoint> {
        let mut checkpoints =
            vec![LexCheckpoint { byte: 0, mode: LexerMode::ExpectTerm, line: 0, column: 0 }];

        let mut mode = LexerMode::ExpectTerm;

        for token in tokens {
            // Update mode based on token
            mode = match token.token_type {
                TokenType::Semicolon | TokenType::LeftBrace | TokenType::RightBrace => {
                    // Safe boundary - reset to ExpectTerm
                    let (line, column) = line_index.byte_to_position(token.end);
                    checkpoints.push(LexCheckpoint {
                        byte: token.end,
                        mode: LexerMode::ExpectTerm,
                        line,
                        column,
                    });
                    LexerMode::ExpectTerm
                }
                TokenType::Keyword(ref kw) if kw.as_ref() == "sub" || kw.as_ref() == "package" => {
                    let (line, column) = line_index.byte_to_position(token.start);
                    checkpoints.push(LexCheckpoint {
                        byte: token.start,
                        mode: LexerMode::ExpectTerm, // ExpectIdentifier not available
                        line,
                        column,
                    });
                    LexerMode::ExpectTerm // ExpectIdentifier not available
                }
                TokenType::Identifier(_) | TokenType::Number(_) | TokenType::StringLiteral => {
                    LexerMode::ExpectOperator
                }
                TokenType::Operator(_) => LexerMode::ExpectTerm,
                _ => mode,
            };
        }

        checkpoints
    }

    /// Create parse checkpoints at statement boundaries
    pub(crate) fn create_parse_checkpoints(ast: &Node) -> Vec<ParseCheckpoint> {
        let mut checkpoints = vec![];
        let mut scope = ScopeSnapshot::default();

        Self::walk_ast_for_checkpoints(ast, &mut checkpoints, &mut scope, 0);
        checkpoints
    }

    fn walk_ast_for_checkpoints(
        node: &Node,
        checkpoints: &mut Vec<ParseCheckpoint>,
        scope: &mut ScopeSnapshot,
        node_id: usize,
    ) {
        // Process current node
        match &node.kind {
            NodeKind::Package { name, .. } => {
                scope.package_name = name.clone();
                checkpoints.push(ParseCheckpoint {
                    byte: node.location.start,
                    scope_snapshot: scope.clone(),
                    node_id,
                });
            }
            NodeKind::Subroutine { .. } | NodeKind::Block { .. } => {
                checkpoints.push(ParseCheckpoint {
                    byte: node.location.start,
                    scope_snapshot: scope.clone(),
                    node_id,
                });
            }
            NodeKind::VariableDeclaration { variable, .. } => {
                // Extract variable name from the variable node
                if let NodeKind::Variable { name, sigil, .. } = &variable.kind {
                    // Include sigil for proper variable tracking
                    scope.locals.push(format!("{}{}", sigil, name));
                }
            }
            NodeKind::VariableListDeclaration { variables, .. } => {
                // Handle list declarations like my ($x, $y, @z)
                for var in variables {
                    if let NodeKind::Variable { name, sigil, .. } = &var.kind {
                        scope.locals.push(format!("{}{}", sigil, name));
                    }
                }
            }
            _ => {}
        }

        // Recurse into children based on node kind
        match &node.kind {
            NodeKind::Program { statements } => {
                for (i, stmt) in statements.iter().enumerate() {
                    let child_id = node_id.wrapping_mul(101).wrapping_add(i);
                    Self::walk_ast_for_checkpoints(stmt, checkpoints, scope, child_id);
                }
            }
            NodeKind::Block { statements } => {
                // Enter new scope for blocks
                let mut local_scope = scope.clone();
                for (i, stmt) in statements.iter().enumerate() {
                    let child_id = node_id.wrapping_mul(101).wrapping_add(i);
                    Self::walk_ast_for_checkpoints(stmt, checkpoints, &mut local_scope, child_id);
                }
            }
            NodeKind::Subroutine { body, .. } => {
                // Subroutine body is a single node (Block), not Vec<Node>
                let mut local_scope = scope.clone();
                let child_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(body, checkpoints, &mut local_scope, child_id);
            }
            NodeKind::If { condition, then_branch, elsif_branches, else_branch } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(condition, checkpoints, scope, base_id);

                // then_branch is Box<Node>, not Option<Box<Node>>
                Self::walk_ast_for_checkpoints(
                    then_branch,
                    checkpoints,
                    scope,
                    base_id.wrapping_add(1),
                );

                // elsif_branches is Vec<(Box<Node>, Box<Node>)>
                for (i, (elsif_cond, elsif_block)) in elsif_branches.iter().enumerate() {
                    let elsif_base = base_id.wrapping_add((i + 2) * 2);
                    Self::walk_ast_for_checkpoints(elsif_cond, checkpoints, scope, elsif_base);
                    Self::walk_ast_for_checkpoints(
                        elsif_block,
                        checkpoints,
                        scope,
                        elsif_base.wrapping_add(1),
                    );
                }
                if let Some(else_br) = else_branch {
                    let else_id = base_id.wrapping_add((elsif_branches.len() + 2) * 2);
                    Self::walk_ast_for_checkpoints(else_br, checkpoints, scope, else_id);
                }
            }
            NodeKind::While { condition, body, .. } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(condition, checkpoints, scope, base_id);
                // body is Box<Node>, not Option<Box<Node>>
                Self::walk_ast_for_checkpoints(body, checkpoints, scope, base_id.wrapping_add(1));
            }
            NodeKind::For { init, condition, update, body, .. } => {
                let base_id = node_id.wrapping_mul(101);
                let mut offset = 0;
                if let Some(init) = init {
                    Self::walk_ast_for_checkpoints(
                        init,
                        checkpoints,
                        scope,
                        base_id.wrapping_add(offset),
                    );
                    offset += 1;
                }
                if let Some(cond) = condition {
                    Self::walk_ast_for_checkpoints(
                        cond,
                        checkpoints,
                        scope,
                        base_id.wrapping_add(offset),
                    );
                    offset += 1;
                }
                if let Some(upd) = update {
                    Self::walk_ast_for_checkpoints(
                        upd,
                        checkpoints,
                        scope,
                        base_id.wrapping_add(offset),
                    );
                    offset += 1;
                }
                // body is Box<Node>, not Option<Box<Node>>
                Self::walk_ast_for_checkpoints(
                    body,
                    checkpoints,
                    scope,
                    base_id.wrapping_add(offset),
                );
            }
            NodeKind::Binary { left, right, .. } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(left, checkpoints, scope, base_id);
                Self::walk_ast_for_checkpoints(right, checkpoints, scope, base_id.wrapping_add(1));
            }
            NodeKind::Assignment { lhs, rhs, .. } => {
                let base_id = node_id.wrapping_mul(101);
                Self::walk_ast_for_checkpoints(lhs, checkpoints, scope, base_id);
                Self::walk_ast_for_checkpoints(rhs, checkpoints, scope, base_id.wrapping_add(1));
            }
            NodeKind::VariableDeclaration { initializer, .. } => {
                if let Some(init) = initializer {
                    let child_id = node_id.wrapping_mul(101);
                    Self::walk_ast_for_checkpoints(init, checkpoints, scope, child_id);
                }
            }
            // Add more cases as needed
            _ => {}
        }
    }

    /// Find the best checkpoint before a given byte offset
    pub fn find_lex_checkpoint(&self, byte: usize) -> Option<&LexCheckpoint> {
        self.lex_checkpoints.iter().rev().find(|cp| cp.byte <= byte)
    }

    /// Find the best parse checkpoint before a given byte offset
    pub fn find_parse_checkpoint(&self, byte: usize) -> Option<&ParseCheckpoint> {
        self.parse_checkpoints.iter().rev().find(|cp| cp.byte <= byte)
    }
}

