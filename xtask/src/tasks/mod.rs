//! Task implementations for xtask automation

pub mod build;
pub mod test;
pub mod bench;
pub mod doc;
pub mod check;
pub mod fmt;
pub mod corpus;
pub mod highlight;
pub mod clean;
pub mod bindings;
pub mod dev;
pub mod release;

pub use build::*;
pub use test::*;
pub use bench::*;
pub use doc::*;
pub use check::*;
pub use fmt::*;
pub use corpus::*;
pub use highlight::*;
pub use clean::*;
pub use bindings::*;
pub use dev::*;
pub use release::*; 