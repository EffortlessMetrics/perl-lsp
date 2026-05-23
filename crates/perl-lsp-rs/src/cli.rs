//! Shared CLI entrypoint for the perl-lsp binaries.

#![deny(clippy::option_env_unwrap)]
// cli.rs is user-facing CLI output — eprintln!/println! are intentional here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use crate::LspServer;
use perl_lsp_rs_core::runtime::launcher::{
    LaunchAction, LaunchConfig, StartupTimer, TransportMode, format_health_output,
    format_info_output, format_startup_banner, help_text, init_logging, log_server_startup,
    logging_filter, parse_args, port_in_use_message, shell_completion, should_enable_logging,
    should_use_ansi_stdout,
};
use perl_lsp_rs_core::tooling::native_compat::{
    classify_perlcritic_profile, classify_perltidy_profile, render_perlcritic_compat_markdown,
    render_perltidy_compat_markdown,
};
use std::env;
use std::path::Path;
use std::process;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};

mod check_project;

/// Run the shared perl-lsp CLI and return the process exit code.
pub fn run_cli<I>(args: I) -> i32
where
    I: IntoIterator,
    I::Item: Into<std::ffi::OsString> + Clone,
{
    let collected_args: Vec<std::ffi::OsString> = args.into_iter().map(Into::into).collect();
    let command_name = invocation_name(&collected_args);

    let launch_plan = match parse_args(collected_args) {
        Ok(plan) => plan,
        Err(error) => {
            eprintln!("{error}");
            eprintln!("{}", render_help_text(&command_name));
            return 1;
        }
    };

    match launch_plan.action {
        LaunchAction::Run => {
            run_server(&command_name, launch_plan.config);
            0
        }
        LaunchAction::Health => {
            let use_color = should_use_ansi_stdout();
            println!("{}", format_health_output(env!("CARGO_PKG_VERSION"), use_color));
            0
        }
        LaunchAction::Info => {
            let use_color = should_use_ansi_stdout();
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
            0
        }
        LaunchAction::Check => run_check(&command_name, &launch_plan.files),
        LaunchAction::CheckProject { ref dir } => check_project::run_check_project(dir),
        LaunchAction::Completion { ref shell } => {
            if let Some(script) = shell_completion(shell) {
                print!("{}", render_shell_completion(script, &command_name));
                0
            } else {
                eprintln!("Unknown shell: {shell}. Supported: bash, zsh, fish, powershell");
                1
            }
        }
        LaunchAction::Version => {
            print_version(&command_name);
            0
        }
        LaunchAction::FeaturesJson => {
            println!("{}", launch_plan.config.features_json());
            0
        }
        LaunchAction::PerltidyCompatReport { ref profile } => run_perltidy_compat_report(profile),
        LaunchAction::PerlcriticCompatReport { ref profile } => {
            run_perlcritic_compat_report(profile)
        }
        LaunchAction::Help => {
            println!("{}", render_help_text(&command_name));
            0
        }
    }
}

fn invocation_name(args: &[std::ffi::OsString]) -> String {
    args.first()
        .and_then(|arg| Path::new(arg).file_stem())
        .and_then(|name| name.to_str())
        .filter(|name| !name.is_empty())
        .unwrap_or("perllsp")
        .to_string()
}

fn render_help_text(command_name: &str) -> String {
    let placeholder = "__PERL_LSP_COMMAND_NAME__";
    help_text()
        .replace("perl-lsp", placeholder)
        .replace("perllsp", placeholder)
        .replace(placeholder, command_name)
}

fn render_shell_completion(script: &str, command_name: &str) -> String {
    let function_name = command_name.replace('-', "_");
    script.replace("_perl_lsp", &format!("_{function_name}")).replace("perl-lsp", command_name)
}

fn run_perltidy_compat_report(profile: &str) -> i32 {
    let raw = match std::fs::read_to_string(profile) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{profile}: error reading perltidy profile: {error}");
            return 1;
        }
    };

    let report = classify_perltidy_profile(&raw);
    print!("{}", render_perltidy_compat_markdown(profile, &report));
    0
}

fn run_perlcritic_compat_report(profile: &str) -> i32 {
    let raw = match std::fs::read_to_string(profile) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("{profile}: error reading perlcritic profile: {error}");
            return 1;
        }
    };

    let report = classify_perlcritic_profile(&raw);
    print!("{}", render_perlcritic_compat_markdown(profile, &report));
    0
}

/// Spawn a blocking reader thread that reads LSP messages from `reader` and
/// forwards them to `tx`. The thread exits when the channel closes or the
/// reader returns EOF or an error.
fn spawn_reader_thread<R: std::io::Read + Send + 'static>(
    reader: R,
    tx: tokio::sync::mpsc::Sender<crate::JsonRpcRequest>,
) {
    use crate::transport::ContentLengthMessageReader;
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
                Ok(None) => break,
                Err(error) => {
                    tracing::debug!(
                        error = %error,
                        "Stopping reader thread after transport read failure"
                    );
                    break;
                }
            }
        }
    });
}

fn run_check(command_name: &str, files: &[String]) -> i32 {
    if files.is_empty() {
        eprintln!("Usage: {command_name} --check <file.pl> [file2.pm ...]");
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
                for detail in format_parse_error_context(&source, &e) {
                    println!("{detail}");
                }
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

fn format_parse_error_context(source: &str, error: &perl_parser::ParseError) -> Vec<String> {
    let contexts = perl_parser::error::get_error_contexts(std::slice::from_ref(error), source);
    let Some(context) = contexts.first() else {
        return Vec::new();
    };

    let line_number = context.line + 1;
    let column_number = context.column + 1;
    let mut lines = vec![format!("  --> line {line_number}, column {column_number}")];

    if !context.source_line.is_empty() {
        let gutter = line_number.to_string();
        lines.push(format!("  {gutter} | {}", context.source_line));
        lines.push(format!("  {} | {}^", " ".repeat(gutter.len()), " ".repeat(context.column)));
    }

    if let Some(suggestion) = &context.suggestion {
        lines.push(format!("  help: {suggestion}"));
    }

    lines
}

fn run_server(command_name: &str, launch_config: LaunchConfig) {
    let command_name = command_name.to_string();
    let mut startup_timer = StartupTimer::new();
    let logging_enabled = should_enable_logging(launch_config.enable_logging);
    if logging_enabled {
        init_logging(&logging_filter(
            launch_config.enable_logging,
            "perl_lsp=info,perl_lsp_rs_core=info,info",
            "warn",
        ));
    }
    startup_timer.checkpoint("logging_init");

    if std::env::var("PERL_LSP_QUIET").is_err() {
        eprintln!(
            "{}",
            format_startup_banner(
                env!("CARGO_PKG_VERSION"),
                launch_config.feature_profile,
                launch_config.transport.is_socket(),
            )
            .replace("perl-lsp", &command_name)
        );
    }

    match launch_config.transport {
        TransportMode::Stdio => {
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!(
                        "Perl Language Server failed to start: could not initialize the async \
                         runtime ({e}). This is usually caused by system resource limits. \
                         Try restarting VS Code or increasing your OS thread limits."
                    );
                    process::exit(1);
                }
            };

            rt.block_on(async {
                startup_timer.checkpoint("runtime_setup");
                let server = Arc::new(LspServer::new_with_feature_profile_and_tuning(
                    launch_config.feature_profile,
                    launch_config.runtime_tuning,
                ));
                startup_timer.checkpoint("server_construction");

                let (tx, rx) = tokio::sync::mpsc::channel(64);
                spawn_reader_thread(std::io::stdin(), tx);

                if logging_enabled {
                    let report = startup_timer.finish();
                    log_server_startup(
                        &command_name,
                        env!("CARGO_PKG_VERSION"),
                        launch_config.transport,
                        Some(launch_config.feature_profile),
                        Some(&report),
                    );
                }

                server.serve_async(rx).await;
            });
        }
        TransportMode::Socket { port } => {
            let addr = format!("127.0.0.1:{port}");
            let feature_profile = launch_config.feature_profile;
            let runtime_tuning = launch_config.runtime_tuning;
            let rt = match Runtime::new() {
                Ok(rt) => rt,
                Err(e) => {
                    eprintln!(
                        "Perl Language Server failed to start: could not initialize the async \
                         runtime ({e}). This is usually caused by system resource limits. \
                         Try restarting VS Code or increasing your OS thread limits."
                    );
                    process::exit(1);
                }
            };

            rt.block_on(async {
                let listener = match TcpListener::bind(&addr).await {
                    Ok(l) => l,
                    Err(e) => {
                        if e.kind() == std::io::ErrorKind::AddrInUse {
                            eprintln!(
                                "{}",
                                port_in_use_message(port).replace("perl-lsp", &command_name)
                            );
                        } else {
                            eprintln!(
                                "Perl Language Server could not listen on {addr}: {e}. \
                                 Try a different port with --port or check firewall settings."
                            );
                        }
                        process::exit(1);
                    }
                };
                let local_addr = match listener.local_addr() {
                    Ok(a) => a,
                    Err(e) => {
                        eprintln!(
                            "Perl Language Server started but could not determine its \
                             listening address: {e}."
                        );
                        process::exit(1);
                    }
                };
                if logging_enabled {
                    tracing::info!(server = %command_name, address = %local_addr, "server listening");
                }

                loop {
                    match listener.accept().await {
                        Ok((stream, peer_addr)) => {
                            if logging_enabled {
                                tracing::info!(server = %command_name, peer = %peer_addr, "accepted connection");
                            }
                            let command_name = command_name.clone();
                            tokio::spawn(async move {
                                let std_stream = match stream.into_std() {
                                    Ok(std_stream) => std_stream,
                                    Err(error) => {
                                        tracing::error!(
                                            %error,
                                            "failed to convert socket stream to std stream"
                                        );
                                        return;
                                    }
                                };

                                if let Err(e) = std_stream.set_nonblocking(false) {
                                    tracing::error!(
                                        error = %e,
                                        "failed to set socket stream blocking mode"
                                    );
                                    return;
                                }

                                let writer = match std_stream.try_clone() {
                                    Ok(w) => w,
                                    Err(e) => {
                                        tracing::error!(error = %e, "failed to clone socket stream");
                                        return;
                                    }
                                };
                                let reader = std_stream;
                                let profile = feature_profile;

                                let output = Arc::new(parking_lot::Mutex::new(
                                    Box::new(writer) as Box<dyn std::io::Write + Send>
                                ));

                                let mut conn_timer = StartupTimer::new();
                                let server =
                                    Arc::new(LspServer::with_output_feature_profile_and_tuning(
                                        output,
                                        profile,
                                        runtime_tuning,
                                    ));
                                conn_timer.checkpoint("server_construction");

                                let (tx, rx) = tokio::sync::mpsc::channel(64);
                                spawn_reader_thread(reader, tx);

                                if logging_enabled {
                                    let report = conn_timer.finish();
                                    log_server_startup(
                                        &command_name,
                                        env!("CARGO_PKG_VERSION"),
                                        launch_config.transport,
                                        Some(profile),
                                        Some(&report),
                                    );
                                }

                                server.serve_async(rx).await;
                            });
                        }
                        Err(e) => {
                            tracing::error!(server = %command_name, error = %e, "socket accept error");
                            sleep(Duration::from_millis(100)).await;
                        }
                    }
                }
            });
        }
    }
}

fn print_version(command_name: &str) {
    println!("{command_name} {}", env!("CARGO_PKG_VERSION"));
    println!("Git tag: {}", env!("GIT_TAG"));
    println!("Perl Language Server using perl-parser v3");
}

#[cfg(test)]
mod tests {
    use super::{
        format_parse_error_context, invocation_name, render_help_text, render_shell_completion,
    };
    use std::ffi::OsString;

    #[test]
    fn invocation_name_uses_file_stem_from_first_arg() {
        let args = vec![OsString::from("/usr/local/bin/perl-lsp-rs"), OsString::from("--help")];
        assert_eq!(invocation_name(&args), "perl-lsp-rs");
    }

    #[test]
    fn invocation_name_falls_back_when_first_arg_is_empty() {
        let args = vec![OsString::from("")];
        assert_eq!(invocation_name(&args), "perllsp");
    }

    #[test]
    fn render_help_text_rewrites_usage_examples_for_invocation_name() {
        let rendered = render_help_text("perl-lsp-rs");

        assert!(rendered.contains("Usage: perl-lsp-rs [options]"));
        assert!(rendered.contains("perl-lsp-rs --check lib/MyModule.pm"));
        assert!(!rendered.contains("Usage: perllsp"));
        assert!(!rendered.contains("perllsp --"));
    }

    #[test]
    fn render_help_text_does_not_rewrite_inserted_command_name() {
        let rendered = render_help_text("perllsp-dev");

        assert!(rendered.contains("Usage: perllsp-dev [options]"));
        assert!(!rendered.contains("perllsp-dev-dev"));
    }

    #[test]
    fn render_shell_completion_rewrites_function_and_command_names() {
        let script =
            "complete -F _perl_lsp perl-lsp\n# shell completion for perl-lsp and _perl_lsp";
        let rendered = render_shell_completion(script, "my-perl-lsp");

        assert!(rendered.contains("_my_perl_lsp"));
        assert!(rendered.contains("my-perl-lsp"));
        assert!(!rendered.contains("complete -F _perl_lsp perl-lsp"));
    }

    #[test]
    fn format_parse_error_context_adds_line_column_and_caret()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $ok = 1;\nmy $broken = ;\n";
        let error = perl_parser::ParseError::unexpected("expression", ";", 25);
        let rendered = format_parse_error_context(source, &error);

        assert!(rendered.iter().any(|line| line == "  --> line 2, column 14"));
        assert!(rendered.iter().any(|line| line == "  2 | my $broken = ;"));
        assert!(rendered.iter().any(|line| line == "    |              ^"));
        Ok(())
    }

    #[test]
    fn format_parse_error_context_includes_suggestions_when_available()
    -> Result<(), Box<dyn std::error::Error>> {
        let source = "my $x = 1\n";
        let error = perl_parser::ParseError::unexpected(";", "end of input", source.len());
        let rendered = format_parse_error_context(source, &error);

        assert!(
            rendered
                .iter()
                .any(|line| line == "  help: add a semicolon ';' at the end of the statement")
        );
        Ok(())
    }
}
