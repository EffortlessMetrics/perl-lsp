//! Public API re-exports for `perl-lexer` post-collapse.
//!
//! This module defines the public surface contributed by the Wave C-absorbed
//! satellites (keywords, builtins, tokenizer). Lexer-native items
//! (Token/TokenType/StringPart, LexerMode, Checkpointable, Position, etc.)
//! continue to be re-exported from `lib.rs` directly.
//!
//! All re-exports here are explicit named items (no wildcards) so the
//! public contract is reviewable at a glance.

// keywords module
pub use crate::keywords::{
    DAP_COMPLETION_KEYWORDS, KEYWORDS, LEXER_KEYWORDS, LSP_COMPLETION_KEYWORDS,
    LSP_RUNTIME_COMPLETION_KEYWORDS, PARSER_LSP_KEYWORDS, RENAME_KEYWORDS,
    is_dap_completion_keyword, is_keyword, is_lexer_keyword, is_lsp_completion_keyword,
    is_lsp_runtime_completion_keyword, is_parser_lsp_keyword, is_rename_keyword,
};

// builtins module
pub use crate::builtins::builtin_signatures::{BuiltinSignature, create_builtin_signatures};
pub use crate::builtins::phf_lookup::{
    BUILTIN_FULL_SIGS, BUILTIN_SIGS, builtin_count, get_param_names, is_builtin,
};

// tokenizer module (AST-agnostic slice only; TokenStream lives in
// perl-parser-core because it depends on perl-error)
pub use crate::tokenizer::token_wrapper::{PositionTracker, TokenWithPosition};
#[allow(deprecated)]
pub use crate::tokenizer::util::{code_slice, find_data_marker_byte, find_data_marker_byte_lexed};
