//! Shared CLI entrypoint for the perl-lsp binaries.

#![deny(clippy::option_env_unwrap)]
// cli.rs is user-facing CLI output — eprintln!/println! are intentional here.
#![allow(clippy::print_stderr, clippy::print_stdout)]

use crate::LspServer;
use perl_lsp_launcher::{
    LaunchAction, LaunchConfig, StartupTimer, TransportMode, format_health_output,
    format_info_output, format_startup_banner, help_text, init_logging, log_server_startup,
    logging_filter, parse_args, port_in_use_message, shell_completion, should_enable_logging,
};
use std::env;
use std::path::{Path, PathBuf};
use std::process;
use std::sync::Arc;
use tokio::net::TcpListener;
use tokio::runtime::Runtime;
use tokio::time::{Duration, sleep};

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
            let use_color = is_terminal_stdout();
            println!("{}", format_health_report(env!("CARGO_PKG_VERSION"), use_color));
            0
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
            0
        }
        LaunchAction::Check => run_check(&command_name, &launch_plan.files),
        LaunchAction::CheckProject { ref dir } => run_check_project(dir),
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
        LaunchAction::Help => {
            println!("{}", render_help_text(&command_name));
            0
        }
    }
}

fn format_health_report(version: &str, use_color: bool) -> String {
    let mut lines = vec![format_health_output(version, use_color), String::new()];

    let cwd = match env::current_dir() {
        Ok(cwd) => cwd,
        Err(error) => {
            lines.push(format!("Perl::Critic: unavailable (failed to read cwd: {error})"));
            return lines.join("\n");
        }
    };

    let mut config = perl_lsp_config::ServerConfig::default();
    let mut config_error = None;
    match perl_lsp_config::load_project_config(&cwd) {
        Ok(Some(project)) => {
            project.apply_to_server_config(&mut config);
        }
        Ok(None) => {}
        Err(error) => {
            config_error = Some(error);
        }
    }

    let perlcritic_path = command_path("perlcritic");
    let configured_profile =
        config.perlcritic_profile.as_ref().map(|profile| resolve_profile_path(&cwd, profile));
    let discovered_profile =
        if configured_profile.is_none() { discover_perlcritic_profile(&cwd) } else { None };
    let selected_profile = configured_profile
        .as_ref()
        .or(discovered_profile.as_ref())
        .map(|path| path.display().to_string());

    let status = if !config.perlcritic_enabled {
        "disabled".to_string()
    } else if perlcritic_path.is_none() {
        "enabled (binary missing)".to_string()
    } else if let Some(profile) = configured_profile.as_ref() {
        if profile.exists() {
            "enabled (healthy)".to_string()
        } else {
            "enabled (bad profile)".to_string()
        }
    } else {
        "enabled (healthy)".to_string()
    };

    lines.push("Perl::Critic:".to_string());
    lines.push(format!("  status: {status}"));
    lines.push(format!("  enabled: {}", config.perlcritic_enabled));
    lines.push(format!(
        "  binary: {}",
        perlcritic_path.unwrap_or_else(|| "not found on PATH".to_string())
    ));
    lines.push(format!("  severity: {}", config.perlcritic_severity));
    lines.push(format!(
        "  configured_profile: {}",
        config.perlcritic_profile.unwrap_or_else(|| "<none>".to_string())
    ));
    lines.push(format!(
        "  resolved_profile: {}",
        selected_profile.unwrap_or_else(|| "<none>".to_string())
    ));
    lines.push(format!(
        "  walkup_discovery: {}",
        if discovered_profile.is_some() { "found" } else { "not used/found" }
    ));

    if let Some(profile) = configured_profile.as_ref() {
        if !profile.exists() {
            lines
                .push(format!("  last_check: profile path does not exist ({})", profile.display()));
        }
    }

    if let Some(error) = config_error {
        lines.push(format!("  config_warning: {error}"));
    }

    lines.join("\n")
}

fn resolve_profile_path(cwd: &Path, profile: &str) -> PathBuf {
    let configured = PathBuf::from(profile);
    if configured.is_absolute() { configured } else { cwd.join(configured) }
}

fn discover_perlcritic_profile(start: &Path) -> Option<PathBuf> {
    let mut current = Some(start.to_path_buf());
    while let Some(dir) = current {
        let candidate = dir.join(".perlcriticrc");
        if candidate.exists() {
            return Some(candidate);
        }
        current = dir.parent().map(std::path::Path::to_path_buf);
    }
    None
}

fn command_path(program: &str) -> Option<String> {
    if Path::new(program).is_file() {
        return Some(program.to_string());
    }
    env::var_os("PATH").and_then(|paths| {
        env::split_paths(&paths).find_map(|path| {
            #[cfg(windows)]
            {
                let exe = path.join(format!("{program}.exe"));
                if exe.is_file() {
                    return Some(exe.to_string_lossy().to_string());
                }
                let bat = path.join(format!("{program}.bat"));
                if bat.is_file() {
                    return Some(bat.to_string_lossy().to_string());
                }
            }

            let candidate = path.join(program);
            if candidate.is_file() { Some(candidate.to_string_lossy().to_string()) } else { None }
        })
    })
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
    help_text().replace("perl-lsp", command_name)
}

fn render_shell_completion(script: &str, command_name: &str) -> String {
    let function_name = command_name.replace('-', "_");
    script.replace("_perl_lsp", &format!("_{function_name}")).replace("perl-lsp", command_name)
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

fn is_terminal_stdout() -> bool {
    use std::io::IsTerminal;
    env::var("NO_COLOR").is_err() && std::io::stdout().is_terminal()
}

struct FileError {
    path: String,
    errors: Vec<String>,
}

fn run_check_project(dir: &str) -> i32 {
    let extensions: &[&str] = &["pm", "pl", "t"];
    let walker = walkdir::WalkDir::new(dir).follow_links(true).into_iter();

    let mut total = 0usize;
    let mut clean = 0usize;
    let mut file_errors: Vec<FileError> = Vec::new();
    let mut category_counts: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();

    for entry in walker.filter_map(|e| e.ok()) {
        let path = entry.path();
        if !path.is_file() {
            continue;
        }

        let ext_match = path
            .extension()
            .and_then(|e| e.to_str())
            .map(|e| extensions.contains(&e))
            .unwrap_or(false);
        if !ext_match {
            continue;
        }

        total += 1;
        let path_str = path.display().to_string();

        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                file_errors
                    .push(FileError { path: path_str, errors: vec![format!("read error: {e}")] });
                category_counts.entry("IO error".to_string()).and_modify(|c| *c += 1).or_insert(1);
                continue;
            }
        };

        let mut parser = perl_parser::Parser::new(&source);
        let parse_result = parser.parse();
        let recovered_errors = parser.errors();

        let mut errors_for_file: Vec<String> = Vec::new();

        for err in recovered_errors {
            let cat = categorize_error(&format!("{err}"));
            category_counts.entry(cat).and_modify(|c| *c += 1).or_insert(1);
            errors_for_file.push(format!("{err}"));
        }

        if let Err(ref e) = parse_result {
            let cat = categorize_error(&format!("{e}"));
            category_counts.entry(cat).and_modify(|c| *c += 1).or_insert(1);
            errors_for_file.push(format!("{e}"));
        }

        if errors_for_file.is_empty() {
            clean += 1;
        } else {
            file_errors.push(FileError { path: path_str, errors: errors_for_file });
        }
    }

    println!("Perl Project Parsability Report");
    println!("===============================");
    println!();
    println!("Directory: {dir}");
    println!("Files scanned: {total}");

    if total == 0 {
        println!();
        println!("No Perl files (.pm, .pl, .t) found.");
        return 0;
    }

    let pct = if total > 0 { (clean as f64 / total as f64) * 100.0 } else { 0.0 };

    println!("Clean parses: {clean}/{total} ({pct:.1}%)");
    println!();

    if !file_errors.is_empty() {
        println!("Parse errors:");
        for fe in &file_errors {
            for err in &fe.errors {
                println!("  {}: {err}", fe.path);
            }
        }
        println!();
    }

    if !category_counts.is_empty() {
        let mut cats: Vec<_> = category_counts.into_iter().collect();
        cats.sort_by(|a, b| b.1.cmp(&a.1));
        println!("Top issue categories:");
        for (cat, count) in &cats {
            println!("  {cat}: {count}");
        }
        println!();
    }

    if pct >= 80.0 {
        println!("Assessment: PASS ({pct:.1}% parsable)");
        0
    } else {
        println!("Assessment: FAIL ({pct:.1}% parsable, threshold 80%)");
        1
    }
}

fn categorize_error(msg: &str) -> String {
    if msg.contains("Unexpected end of input") {
        "Unexpected EOF".to_string()
    } else if msg.contains("expected") && msg.contains("found") {
        "Unexpected token".to_string()
    } else if msg.contains("Invalid syntax") {
        "Syntax error".to_string()
    } else if msg.contains("Lexer error") {
        "Lexer error".to_string()
    } else if msg.contains("recursion") || msg.contains("Recursion") {
        "Recursion limit".to_string()
    } else if msg.contains("read error") {
        "IO error".to_string()
    } else {
        "Other".to_string()
    }
}

fn run_server(command_name: &str, launch_config: LaunchConfig) {
    let command_name = command_name.to_string();
    let mut startup_timer = StartupTimer::new();
    let logging_enabled = should_enable_logging(launch_config.enable_logging);
    if logging_enabled {
        init_logging(&logging_filter(
            launch_config.enable_logging,
            "perl_lsp=info,perl_lsp_launcher=info,info",
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
                let server =
                    Arc::new(LspServer::new_with_feature_profile(launch_config.feature_profile));
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
                                let server = Arc::new(LspServer::with_output_and_feature_profile(
                                    output, profile,
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
