//! Development server task implementation

use color_eyre::eyre::{Context, Result};
use duct::cmd;
use indicatif::{ProgressBar, ProgressStyle};
use std::process::Child;

pub fn run(watch: bool, port: u16) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}").unwrap(),
    );

    spinner.set_message("Starting development server");

    let mut child = match spawn_server(port) {
        Ok(child) => {
            spinner.finish_with_message(format!("✅ Development server started on port {}", port));
            child
        }
        Err(e) => {
            spinner.finish_with_message("❌ Failed to start development server");
            return Err(e);
        }
    };

    if watch {
        use notify::{RecursiveMode, Watcher};
        use std::path::Path;
        use std::sync::mpsc::channel;

        let (tx, rx) = channel();
        let mut watcher = notify::recommended_watcher(move |res| {
            let _ = tx.send(res);
        })?;
        watcher.watch(Path::new("."), RecursiveMode::Recursive)?;

        for res in rx {
            match res {
                Ok(_) => {
                    spinner.println("🔄 Change detected. Restarting server...");
                    let _ = child.kill();
                    let _ = child.wait();
                    match spawn_server(port) {
                        Ok(c) => child = c,
                        Err(e) => {
                            spinner.println(format!("❌ Failed to restart server: {}", e));
                            break;
                        }
                    }
                }
                Err(e) => {
                    spinner.println(format!("watch error: {}", e));
                }
            }
        }
    } else {
        let status = child.wait().context("Failed to wait on development server")?;
        if !status.success() {
            return Err(color_eyre::eyre::eyre!(
                "Development server exited with status: {}",
                status
            ));
        }
    }

    Ok(())
}

fn spawn_server(port: u16) -> Result<Child> {
    let port_arg = port.to_string();
    let child = cmd("python", &["-m", "http.server", &port_arg])
        .start()
        .context("Failed to spawn development server")?;
    Ok(child)
}
