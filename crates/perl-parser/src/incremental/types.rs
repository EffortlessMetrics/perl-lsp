use perl_lexer::LexerMode;

/// Stable restart points to avoid re-lexing the whole world
#[derive(Clone, Copy, Debug)]
pub struct LexCheckpoint {
    pub byte: usize,
    pub mode: LexerMode,
    pub line: usize,
    pub column: usize,
}

/// Scope information at a parse checkpoint
#[derive(Clone, Debug, Default)]
pub struct ScopeSnapshot {
    pub package_name: String,
    pub locals: Vec<String>,
    pub our_vars: Vec<String>,
    pub parent_isa: Vec<String>,
}

/// Parse checkpoint with scope context
#[derive(Clone, Debug)]
pub struct ParseCheckpoint {
    pub byte: usize,
    pub scope_snapshot: ScopeSnapshot,
    pub node_id: usize, // ID of AST node at this point
}

