//! Development server task implementation

use color_eyre::eyre::Result;
use indicatif::{ProgressBar, ProgressStyle};
use std::path::Path;
use std::sync::{Arc, mpsc::channel};
use std::thread;

use notify::{Config, RecommendedWatcher, RecursiveMode, Watcher};
use tiny_http::{Response, Server};

/// Run the development server.
///
/// * `watch` - If true, watches the repository for changes and restarts
///   the server on modification.
/// * `port` - The port to bind the HTTP server to.
pub fn run(watch: bool, port: u16) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner().template("{spinner:.green} {wide_msg}").unwrap(),
    );

    let addr = format!("127.0.0.1:{port}");
    spinner.set_message(format!("Starting development server on {addr}"));

    // Helper to start the HTTP server
    fn start_server(addr: &str) -> color_eyre::Result<Arc<Server>> {
        Ok(Arc::new(Server::http(addr).map_err(|e| color_eyre::eyre::eyre!(e))?))
    }

    let mut server = start_server(&addr)?;
    spinner.finish_with_message(format!("✅ Development server started on http://{addr}"));

    // Handle requests on a separate thread so we can optionally watch for changes.
    let serve = |srv: Arc<Server>| {
        thread::spawn(move || {
            for request in srv.incoming_requests() {
                let _ = request.respond(Response::from_string("tree-sitter-perl dev server"));
            }
        })
    };

    if watch {
        // Setup file watcher
        let (tx, rx) = channel();
        let mut watcher = RecommendedWatcher::new(tx, Config::default())?;
        watcher.watch(Path::new("crates"), RecursiveMode::Recursive)?;

        loop {
            let handle = serve(server.clone());

            match rx.recv() {
                Ok(event) => {
                    spinner.println(format!("🔄 File change detected: {:?}. Restarting...", event));
                    // Stop serving and restart
                    server.unblock();
                    let _ = handle.join();
                    server = start_server(&addr)?;
                }
                Err(err) => {
                    server.unblock();
                    let _ = handle.join();
                    return Err(err.into());
                }
            }
        }
    } else {
        // If not watching, just serve indefinitely.
        let handle = serve(server.clone());
        handle.join().map_err(|e| color_eyre::eyre::eyre!("server thread error: {:?}", e))?;
        Ok(())
    }
}
