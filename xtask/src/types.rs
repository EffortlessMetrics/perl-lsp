//! Shared types for xtask

#[derive(Clone, clap::ValueEnum)]
pub enum TestSuite {
    Unit,
    Integration,
    Property,
    Performance,
    Heredoc,
    All,
}

#[derive(Clone, clap::ValueEnum)]
pub enum ScannerType {
    C,
    Rust,
    Both,
}

#[derive(Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Csv,
}
