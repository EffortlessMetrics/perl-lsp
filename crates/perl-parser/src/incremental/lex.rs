use crate::incremental::LineIndex;
use crate::incremental::checkpoint::LexCheckpoint;
use perl_lexer::{LexerMode, Token, TokenType};

pub(crate) fn create_lex_checkpoints(
    tokens: &[Token],
    line_index: &LineIndex,
) -> Vec<LexCheckpoint> {
    let mut checkpoints =
        vec![LexCheckpoint { byte: 0, mode: LexerMode::ExpectTerm, line: 0, column: 0 }];
    let mut mode = LexerMode::ExpectTerm;
    for token in tokens {
        mode = match token.token_type {
            TokenType::Semicolon | TokenType::LeftBrace | TokenType::RightBrace => {
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
                    mode: LexerMode::ExpectTerm,
                    line,
                    column,
                });
                LexerMode::ExpectTerm
            }
            TokenType::Identifier(_) | TokenType::Number(_) | TokenType::StringLiteral => {
                LexerMode::ExpectOperator
            }
            TokenType::Operator(_) => LexerMode::ExpectTerm,
            _ => mode,
        }
    }
    checkpoints
}
