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
    V3,
    V2PestMicrocrate,
    V2Parity,
}

#[derive(Clone, clap::ValueEnum)]
pub enum OutputFormat {
    Human,
    Json,
    Csv,
}
