//! Range builder for pragma state snapshots.
//!
//! This module owns AST traversal and delegates pragma directive semantics to
//! `directives`, keeping the public tracker facade focused on querying.

mod directives;
mod walk;

pub(crate) use walk::build_ranges;
