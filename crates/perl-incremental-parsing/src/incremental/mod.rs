//! Compatibility re-exports for incremental parsing.
//!
//! Source of truth: `perl_parser::incremental`.

#![allow(missing_docs)]

pub use perl_parser::incremental::*;
pub use perl_parser::incremental::{
    incremental_advanced_reuse, incremental_checkpoint, incremental_document, incremental_edit,
    incremental_integration, incremental_simple, incremental_v2,
};
