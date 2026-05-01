use crate::incremental::checkpoint::{LexCheckpoint, ParseCheckpoint, ScopeSnapshot};
use crate::incremental::lex::create_lex_checkpoints;
use perl_lexer::{PerlLexer, Token, TokenType};
use perl_line_index::LineIndex;
use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
use perl_parser_core::parser::Parser;
use ropey::Rope;

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
    pub fn new(source: String) -> Self {
        let rope = Rope::from_str(&source);
        let line_index = LineIndex::new(&source);
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
        let mut lexer = PerlLexer::new(&source);
        let mut tokens = Vec::new();
        while let Some(token) = lexer.next_token() {
            if token.token_type == TokenType::EOF {
                break;
            }
            tokens.push(token);
        }
        let lex_checkpoints = create_lex_checkpoints(&tokens, &line_index);
        let parse_checkpoints = Self::create_parse_checkpoints(&ast);
        Self { rope, line_index, lex_checkpoints, parse_checkpoints, ast, tokens, source }
    }
    pub fn find_lex_checkpoint(&self, byte: usize) -> Option<&LexCheckpoint> {
        self.lex_checkpoints.iter().rev().find(|cp| cp.byte <= byte)
    }
    pub fn find_parse_checkpoint(&self, byte: usize) -> Option<&ParseCheckpoint> {
        self.parse_checkpoints.iter().rev().find(|cp| cp.byte <= byte)
    }

    pub(crate) fn create_parse_checkpoints(ast: &Node) -> Vec<ParseCheckpoint> {
        let mut checkpoints = vec![];
        let mut scope = ScopeSnapshot::default();
        walk_ast_for_checkpoints(ast, &mut checkpoints, &mut scope, 0);
        checkpoints
    }
}

fn walk_ast_for_checkpoints(
    node: &Node,
    checkpoints: &mut Vec<ParseCheckpoint>,
    scope: &mut ScopeSnapshot,
    node_id: usize,
) {
    match &node.kind {
        NodeKind::Package { name, .. } => {
            scope.package_name = name.clone();
            checkpoints.push(ParseCheckpoint {
                byte: node.location.start,
                scope_snapshot: scope.clone(),
                node_id,
            });
        }
        NodeKind::Subroutine { .. } | NodeKind::Block { .. } => checkpoints.push(ParseCheckpoint {
            byte: node.location.start,
            scope_snapshot: scope.clone(),
            node_id,
        }),
        NodeKind::VariableDeclaration { variable, .. } => {
            if let NodeKind::Variable { name, sigil, .. } = &variable.kind {
                scope.locals.push(format!("{}{}", sigil, name));
            }
        }
        NodeKind::VariableListDeclaration { variables, .. } => {
            for var in variables {
                if let NodeKind::Variable { name, sigil, .. } = &var.kind {
                    scope.locals.push(format!("{}{}", sigil, name));
                }
            }
        }
        _ => {}
    }
    match &node.kind {
        NodeKind::Program { statements } | NodeKind::Block { statements } => {
            for (i, stmt) in statements.iter().enumerate() {
                walk_ast_for_checkpoints(
                    stmt,
                    checkpoints,
                    scope,
                    node_id.wrapping_mul(101).wrapping_add(i),
                );
            }
        }
        _ => {}
    }
}
