//! Tooling integration for Perl LSP.
//!
//! Provides abstractions for integrating with external Perl tooling such as
//! perltidy (formatting) and perlcritic (linting), plus subprocess runtime
//! and performance utilities.
//!
//! Previously the standalone `perl-lsp-tooling` crate; absorbed into
//! `perl-lsp-rs-core::tooling` in Wave G3 (#4535).

/// Performance optimizations: AST caching, incremental parsing, parallel indexing.
pub mod performance {
    pub use crate::performance::*;
}

/// Perl::Critic integration for code quality analysis.
pub mod perl_critic;

/// Perltidy integration for code formatting.
pub mod perltidy {
    pub use perl_lsp_perltidy::*;
}

/// Subprocess runtime abstraction.
pub use perl_subprocess_runtime::{SubprocessError, SubprocessOutput, SubprocessRuntime};

/// OS-native subprocess runtime (non-WASM only).
#[cfg(not(target_arch = "wasm32"))]
pub use perl_subprocess_runtime::OsSubprocessRuntime;

/// Test mock implementations for subprocess runtimes.
pub mod mock {
    pub use perl_subprocess_runtime::mock::*;
}
