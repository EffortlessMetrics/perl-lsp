fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(perl_lsp::run_cli(std::env::args()) as u8)
}
