//! Shared types for xtask

#[derive(Clone, clap::ValueEnum)]
pub enum TestSuite {
    Unit,
    Integration,
    Property,
    Performance,
    All,
}

#[derive(Clone, clap::ValueEnum)]
pub enum ScannerType {
    C,
    Rust,
    Both,
}
