//! Development server task implementation

use color_eyre::eyre::{Context, Result};
use indicatif::{ProgressBar, ProgressStyle};
use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use std::io::{Read, Write};
use std::net::TcpListener;
use std::path::Path;
use std::process::{Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread;

pub fn run(watch: bool, port: u16) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap_or_else(|_| ProgressStyle::default_spinner()),
    );

    spinner.set_message("Starting development server setup...");

    // Shared state for the current child process
    let current_process: Arc<Mutex<Option<Child>>> = Arc::new(Mutex::new(None));

    // Setup file watcher if requested
    let _watcher = if watch {
        spinner.set_message("Setting up file watcher...");
        let current_process = current_process.clone();

        // Watch the 'crates' directory
        let path = Path::new("crates");

        let mut watcher: RecommendedWatcher = Watcher::new(
            move |res: notify::Result<notify::Event>| {
                match res {
                    Ok(event) => {
                        // Any modification, creation, or removal triggers a restart
                        if event.kind.is_modify()
                            || event.kind.is_create()
                            || event.kind.is_remove()
                        {
                            // Kill the current process if it exists
                            if let Ok(mut process_guard) = current_process.lock()
                                && let Some(child) = process_guard.as_mut()
                            {
                                let _ = child.kill(); // Ignore error if already dead
                                let _ = child.wait(); // Clean up zombies
                                *process_guard = None;
                            }
                            // Killing the process breaks the pipe, which closes the TCP connection.
                            // The client will reconnect, and we will spawn a new process (which includes recompilation).
                        }
                    }
                    Err(e) => eprintln!("watch error: {:?}", e),
                }
            },
            Config::default(),
        )?;

        watcher.watch(path, RecursiveMode::Recursive)?;

        spinner.set_message(format!("Watching {} for changes...", path.display()));
        Some(watcher)
    } else {
        None
    };

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))
        .wrap_err_with(|| format!("Failed to bind to port {}", port))?;

    spinner.finish_with_message(format!(
        "✅ Development server listening on 127.0.0.1:{}. {}",
        port,
        if watch { "Watching for changes." } else { "Not watching for changes." }
    ));

    println!("Connect your editor to this TCP port to use the dev server.");
    println!("The server will rebuild and spawn 'perl-lsp --stdio' for each connection.");

    // Accept connections
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                println!("New connection received.");

                // Spawn the lsp process
                println!("Spawning perl-lsp...");

                // We use cargo run to ensure it builds if needed
                let mut cmd = Command::new("cargo");
                cmd.args(["run", "-q", "-p", "perl-lsp", "--", "--stdio"])
                    .stdin(Stdio::piped())
                    .stdout(Stdio::piped())
                    .stderr(Stdio::inherit()); // Let stderr go to console for logs

                let mut child = match cmd.spawn() {
                    Ok(c) => c,
                    Err(e) => {
                        eprintln!("Failed to spawn perl-lsp: {}", e);
                        continue;
                    }
                };

                let Some(mut stdin) = child.stdin.take() else {
                    eprintln!("Failed to open stdin");
                    continue;
                };
                let Some(mut stdout) = child.stdout.take() else {
                    eprintln!("Failed to open stdout");
                    continue;
                };

                // Store the child process so watcher can kill it
                if let Ok(mut guard) = current_process.lock() {
                    if let Some(mut old_child) = guard.take() {
                        let _ = old_child.kill();
                        let _ = old_child.wait();
                    }
                    *guard = Some(child);
                }

                let Ok(mut stream_clone) = stream.try_clone() else {
                    eprintln!("Failed to clone stream");
                    continue;
                };
                let Ok(mut stream_clone2) = stream.try_clone() else {
                    eprintln!("Failed to clone stream");
                    continue;
                };

                // Thread: TCP -> Process Stdin
                let _t1 = thread::spawn(move || {
                    let mut buffer = [0; 1024];
                    loop {
                        match stream_clone.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                if stdin.write_all(&buffer[..n]).is_err() {
                                    break;
                                }
                                if stdin.flush().is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                });

                // Thread: Process Stdout -> TCP
                let _t2 = thread::spawn(move || {
                    let mut buffer = [0; 1024];
                    loop {
                        match stdout.read(&mut buffer) {
                            Ok(0) => break, // EOF
                            Ok(n) => {
                                if stream_clone2.write_all(&buffer[..n]).is_err() {
                                    break;
                                }
                                if stream_clone2.flush().is_err() {
                                    break;
                                }
                            }
                            Err(_) => break,
                        }
                    }
                    let _ = stream_clone2.shutdown(std::net::Shutdown::Both);
                });
            }
            Err(e) => {
                eprintln!("Connection failed: {}", e);
            }
        }
    }

    Ok(())
}
