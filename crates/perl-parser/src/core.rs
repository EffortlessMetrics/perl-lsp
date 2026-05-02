//! Facade over `perl-parser-core`.
//!
//! Canonical parser kernel implementation lives in `perl-parser-core`.

pub use perl_parser_core::ast::{Node, NodeKind, SourceLocation};
pub use perl_parser_core::error::{ParseError, ParseOutput, ParseResult};
pub use perl_parser_core::parser::Parser;

pub use perl_parser_core::engine;
pub use perl_parser_core::engine::{error, parser, position};
