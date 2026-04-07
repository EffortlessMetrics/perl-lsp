//! Partial parse and anti-pattern AST for Perl
//!
//! This crate extends the standard AST with nodes that can represent
//! unparseable or problematic constructs, phase-aware parsing, and
//! tree-sitter compatibility adapters.

pub mod edge_case_handler;
pub mod partial_parse_ast;
pub mod phase_aware_parser;
pub mod tree_sitter_adapter;
pub mod understanding_parser;
