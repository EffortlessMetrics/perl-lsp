//! Task implementations for xtask automation

pub mod bench;
pub mod bindings;
pub mod build;
pub mod check;
pub mod clean;
pub mod corpus;
pub mod dev;
pub mod doc;
pub mod fmt;
pub mod highlight;
pub mod release;
pub mod test;

pub use bench::*;
pub use bindings::*;
pub use build::*;
pub use check::*;
pub use clean::*;
pub use corpus::*;
pub use dev::*;
pub use doc::*;
pub use fmt::*;
pub use highlight::*;
pub use release::*;
pub use test::*;
