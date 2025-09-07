//! Task implementations for xtask automation

pub mod bench;
pub mod bindings;
pub mod build;
pub mod bump_version;
pub mod check;
pub mod clean;
pub mod compare;
#[cfg(feature = "legacy")]
pub mod compare_parsers;
#[cfg(feature = "legacy")]
pub mod corpus;
pub mod dev;
pub mod doc;
pub mod edge_cases;
pub mod features;
pub mod fmt;
pub mod highlight;
pub mod parse_rust;
pub mod publish;
pub mod release;
pub mod test;
pub mod test_lsp;
