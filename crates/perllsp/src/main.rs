fn main() -> std::process::ExitCode {
    std::process::ExitCode::from(perllsp::run_cli(std::env::args()) as u8)
}
