fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(perl_ripr_facts::run_cli(std::env::args()) as u8)
}
