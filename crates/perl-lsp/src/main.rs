//! Perl Language Server binary.
//!
//! This binary wires startup options to the reusable launcher microcrate and
//! delegates command parsing, profile compatibility, and CLI interoperability to a
//! shared package boundary.
//!
//! Both stdio and TCP modes use the same async dispatch path (`serve_async`),
//! with a blocking reader thread feeding an `mpsc` channel.

#![deny(clippy::option_env_unwrap)]
use perl_lsp::LspServer;
use perl_lsp_launcher::{
    LaunchAction, LaunchConfig, TransportMode, format_health_output, format_info_output, help_text,
    parse_args, port_in_use_message, shell_completion,
};
use std::env;
use std::io;
use std::io::IsTerminal;
use std::process;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};
use tracing::{debug, error, info, warn};
use tracing_subscriber::{EnvFilter, fmt};

/// Spawn a blocking reader thread that reads LSP messages from `reader` and
/// forwards them to `tx`. The thread exits when the channel closes or the
/// reader returns EOF or an error.
fn spawn_reader_thread<R: std::io::Read + Send + 'static>(
    reader: R,
    tx: tokio::sync::mpsc::Sender<perl_lsp::JsonRpcRequest>,
) {
    use perl_lsp::transport::ContentLengthMessageReader;
    std::thread::spawn(move || {
        let mut msg_reader = ContentLengthMessageReader::new();
        let mut buf_reader = std::io::BufReader::new(reader);
        loop {
            match msg_reader.read_next(&mut buf_reader) {
                Ok(Some(request)) => {
                    if tx.blocking_send(request).is_err() {
                        break;
                    }
                }
                Ok(None) => break, // EOF
                Err(error) => {
                    debug!(error = %error, "Stopping reader thread after transport read failure");
                    break;
                }
            }
        }
    });
}

fn logging_env_directive() -> Option<String> {
    env::var("PERL_LSP_LOG").ok().or_else(|| env::var("RUST_LOG").ok())
}

fn should_enable_logging(launch_config: &LaunchConfig) -> bool {
    launch_config.enable_logging || logging_env_directive().is_some()
}

fn logging_filter(launch_config: &LaunchConfig) -> String {
    logging_env_directive().unwrap_or_else(|| {
        if launch_config.enable_logging {
            "perl_lsp=info,perl_lsp_launcher=info,info".to_string()
        } else {
            "warn".to_string()
        }
    })
}

fn init_logging(launch_config: &LaunchConfig) {
    if !should_enable_logging(launch_config) {
        return;
    }

    let use_ansi = env::var("NO_COLOR").is_err() && std::io::stderr().is_terminal();
    let filter = EnvFilter::try_new(logging_filter(launch_config))
        .unwrap_or_else(|_| EnvFilter::new("perl_lsp=info,info"));

    let _ = fmt()
        .with_writer(io::stderr)
        .with_env_filter(filter)
        .with_ansi(use_ansi)
        .with_target(true)
        .try_init();
}

fn main() {
    let launch_plan = match parse_args(env::args()) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", help_text());
            process::exit(1);
        }
    };

    match launch_plan.action {
        LaunchAction::Run => run_server(launch_plan.config),
        LaunchAction::Health => {
            let use_color = is_terminal_stdout();
            println!("{}", format_health_output(env!("CARGO_PKG_VERSION"), use_color));
            process::exit(0);
        }
        LaunchAction::Info => {
            let use_color = is_terminal_stdout();
            let exe_path = env::current_exe()
                .map(|p| p.display().to_string())
                .unwrap_or_else(|_| "<unknown>".to_string());
            print!(
                "{}",
                format_info_output(
                    env!("CARGO_PKG_VERSION"),
                    env!("GIT_TAG"),
                    &exe_path,
                    launch_plan.config.feature_profile,
                    use_color,
                )
            );
            process::exit(0);
        }
        LaunchAction::Check => {
            let exit_code = run_check(&launch_plan.files);
            process::exit(exit_code);
        }
        LaunchAction::Completion { ref shell } => {
            if let Some(script) = shell_completion(shell) {
                print!("{script}");
                process::exit(0);
            } else {
                eprintln!("Unknown shell: {shell}. Supported: bash, zsh, fish");
                process::exit(1);
            }
        }
        LaunchAction::Version => {
            print_version();
            process::exit(0);
        }
        LaunchAction::FeaturesJson => {
            println!("{}", launch_plan.config.features_json());
            process::exit(0);
        }
        LaunchAction::Help => {
            println!("{}", help_text());
            process::exit(0);
        }
    }
}

/// Run the `--check` batch mode: parse the given files and report errors.
fn run_check(files: &[String]) -> i32 {
    if files.is_empty() {
        eprintln!("Usage: perl-lsp --check <file.pl> [file2.pm ...]");
        eprintln!("No files specified.");
        return 1;
    }

    let mut total = 0usize;
    let mut errors = 0usize;

    for path in files {
        total += 1;
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{path}: error reading file: {e}");
                errors += 1;
                continue;
            }
        };

        let mut parser = perl_parser::Parser::new(&source);
        match parser.parse() {
            Ok(_) => {
                println!("{path}: ok");
            }
            Err(e) => {
                println!("{path}: FAIL - {e}");
                errors += 1;
            }
        }
    }

    if total > 1 {
        println!();
        println!("{total} files checked, {errors} with errors");
    }

    if errors > 0 { 1 } else { 0 }
}

/// Detect whether stdout is a terminal (for colored output).
///
/// Respects `NO_COLOR` (<https://no-color.org/>) and checks the actual
/// file descriptor via `std::io::IsTerminal` (stable since Rust 1.70).
fn is_terminal_stdout() -> bool {
    use std::io::IsTerminal;
    env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal()
}

fn run_server(launch_config: LaunchConfig) {
    init_logging(&launch_config);

    if should_enable_logging(&launch_config) {
        info!(
            version = env!("CARGO_PKG_VERSION"),
            git_tag = env!("GIT_TAG"),
            transport = launch_config.transport.label(),
            port = launch_config.transport.port(),
            feature_profile = launch_config.feature_profile.as_str(),
            pid = process::id(),
            "Perl Language Server starting"
        );
    }

    match launch_config.transport {
        TransportMode::Stdio => {
            // Stdio uses the same async dispatch path as TCP: a blocking reader
            // thread feeds an mpsc channel into serve_async().
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create Tokio runtime: {e}");
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let server =
                    Arc::new(LspServer::new_with_feature_profile(launch_config.feature_profile));

                // Spawn a blocking reader thread for stdin
                let (tx, rx) = tokio::sync::mpsc::channel(64);
                spawn_reader_thread(std::io::stdin(), tx);

                // Same async dispatch path as TCP
                server.serve_async(rx).await;
            });
        }
        TransportMode::Socket { port } => {
            let addr = format!("127.0.0.1:{port}");
            let feature_profile = launch_config.feature_profile;
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!("Failed to create Tokio runtime: {e}");
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::AddrInUse {
                            eprintln!("{}", port_in_use_message(port));
                        } else {
                            eprintln!("Failed to bind to {addr}: {e}");
                        }
                        process::exit(1);
                    }
                };
                let local_addr = match listener.local_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!("Failed to get local address: {e}");
                        process::exit(1);
                    }
                };
                info!(address = %local_addr, "Perl LSP listening");

                loop {
                    match listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            info!(peer = %peer_addr, "Accepted LSP socket connection");
                            tokio::spawn(async move {
                                let std_stream = match stream.into_std() {
                                    Ok(std_stream) => std_stream,
                                    Err(error) => {
                                        error!(error = %error, "Failed to convert Tokio stream to std stream");
                                        return;
                                    }
                                };

                                if let Err(e) = std_stream.set_nonblocking(false) {
                                    error!(error = %e, "Failed to set accepted socket to blocking mode");
                                    return;
                                }

                                let writer = match std_stream.try_clone() {
                                    Ok(w) => w,
                                    Err(e) => {
                                        error!(error = %e, "Failed to clone accepted socket stream");
                                        return;
                                    }
                                };
                                let reader = std_stream;
                                let profile = feature_profile;

                                let output = Arc::new(parking_lot::Mutex::new(
                                    Box::new(writer) as Box<dyn std::io::Write + Send>
                                ));

                                // Create server, wrap in Arc for concurrent async dispatch
                                let server = Arc::new(LspServer::with_output_and_feature_profile(
                                    output, profile,
                                ));

                                // Spawn a blocking reader thread that feeds an async channel
                                let (tx, rx) = tokio::sync::mpsc::channel(64);
                                spawn_reader_thread(reader, tx);

                                // Run async serve loop with concurrent dispatch
                                server.serve_async(rx).await;
                            });
                        }
                        Err(e) => {
                            warn!(error = %e, "Failed to accept incoming LSP connection");
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            });
        }
    }
}

fn print_version() {
    println!("perl-lsp {}", env!("CARGO_PKG_VERSION"));
    // build.rs always sets GIT_TAG (falls back to "unknown"), so env! is safe.
    println!("Git tag: {}", env!("GIT_TAG"));
    println!("Perl Language Server using perl-parser v3");
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_lsp_feature_governance::FeatureProfile;

    #[test]
    fn logging_disabled_by_default() {
        let _perl = EnvVarGuard::set("PERL_LSP_LOG", None);
        let _rust = EnvVarGuard::set("RUST_LOG", None);
        let config = LaunchConfig::new(FeatureProfile::current());
        assert!(!should_enable_logging(&config));
    }

    #[test]
    fn explicit_log_flag_enables_logging() {
        let _perl = EnvVarGuard::set("PERL_LSP_LOG", None);
        let _rust = EnvVarGuard::set("RUST_LOG", None);
        let mut config = LaunchConfig::new(FeatureProfile::current());
        config.enable_logging = true;
        assert!(should_enable_logging(&config));
        assert_eq!(logging_filter(&config), "perl_lsp=info,perl_lsp_launcher=info,info");
    }

    #[test]
    fn env_logging_overrides_default_filter() {
        let _rust = EnvVarGuard::set("RUST_LOG", Some("warn"));
        let _guard = EnvVarGuard::set("PERL_LSP_LOG", Some("perl_lsp=debug"));
        let config = LaunchConfig::new(FeatureProfile::current());
        assert!(should_enable_logging(&config));
        assert_eq!(logging_filter(&config), "perl_lsp=debug");
    }

    #[test]
    fn perl_lsp_log_takes_precedence_over_rust_log() {
        let _rust = EnvVarGuard::set("RUST_LOG", Some("warn"));
        let _perl = EnvVarGuard::set("PERL_LSP_LOG", Some("perl_lsp=trace"));
        let config = LaunchConfig::new(FeatureProfile::current());
        assert_eq!(logging_filter(&config), "perl_lsp=trace");
    }

    struct EnvVarGuard {
        key: &'static str,
        old: Option<String>,
    }

    impl EnvVarGuard {
        fn set(key: &'static str, value: Option<&str>) -> Self {
            let old = env::var(key).ok();
            match value {
                Some(value) => unsafe { env::set_var(key, value) },
                None => unsafe { env::remove_var(key) },
            }
            Self { key, old }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            match self.old.as_deref() {
                Some(value) => unsafe { env::set_var(self.key, value) },
                None => unsafe { env::remove_var(self.key) },
            }
        }
    }
}
