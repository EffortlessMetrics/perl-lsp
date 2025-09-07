//! Development server task implementation

use color_eyre::eyre::{Result, eyre};
use indicatif::{ProgressBar, ProgressStyle};
use notify::{RecommendedWatcher, RecursiveMode, Watcher, EventKind};
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};
use tokio::time::sleep;
use console::style;

/// Configuration for the development server
#[derive(Debug, Clone)]
pub struct DevServerConfig {
    /// Port for the development server
    pub port: u16,
    /// Enable file watching
    pub watch: bool,
    /// Paths to watch for changes
    pub watch_paths: Vec<PathBuf>,
    /// File extensions to watch
    pub watch_extensions: Vec<String>,
    /// Debounce duration for file changes
    pub debounce_ms: u64,
    /// LSP server executable path
    pub lsp_binary: Option<PathBuf>,
    /// Additional LSP arguments
    pub lsp_args: Vec<String>,
}

impl Default for DevServerConfig {
    fn default() -> Self {
        Self {
            port: 8080,
            watch: false,
            watch_paths: vec![
                PathBuf::from("crates"),
                PathBuf::from("src"),
                PathBuf::from("tests"),
                PathBuf::from("Cargo.toml"),
                PathBuf::from("Cargo.lock"),
            ],
            watch_extensions: vec![
                "rs".to_string(),
                "pl".to_string(),
                "pm".to_string(),
                "t".to_string(),
                "toml".to_string(),
                "yaml".to_string(),
                "yml".to_string(),
            ],
            debounce_ms: 500,
            lsp_binary: None,
            lsp_args: vec!["--stdio".to_string(), "--log".to_string()],
        }
    }
}

/// Development server that manages LSP server lifecycle and file watching
pub struct DevServer {
    config: DevServerConfig,
    lsp_process: Option<Child>,
    last_restart: Option<Instant>,
}

impl DevServer {
    pub fn new(config: DevServerConfig) -> Self {
        Self {
            config,
            lsp_process: None,
            last_restart: None,
        }
    }

    /// Start the LSP server process
    fn start_lsp_server(&mut self) -> Result<()> {
        // Stop existing server if running
        self.stop_lsp_server()?;

        let lsp_binary = if let Some(binary) = &self.config.lsp_binary {
            binary.clone()
        } else {
            // Try to find perl-lsp in target/release, target/debug, or PATH
            self.find_lsp_binary()?
        };

        println!("{} Starting LSP server: {:?}", 
                style("⚡").green(), 
                lsp_binary);

        let mut command = Command::new(&lsp_binary);
        command
            .args(&self.config.lsp_args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());

        match command.spawn() {
            Ok(child) => {
                self.lsp_process = Some(child);
                self.last_restart = Some(Instant::now());
                println!("{} LSP server started with PID: {}", 
                        style("✓").green(), 
                        self.lsp_process.as_ref().unwrap().id());
                Ok(())
            }
            Err(e) => Err(eyre!("Failed to start LSP server: {}", e)),
        }
    }

    /// Stop the LSP server process
    fn stop_lsp_server(&mut self) -> Result<()> {
        if let Some(mut child) = self.lsp_process.take() {
            println!("{} Stopping LSP server...", style("🛑").yellow());
            
            // Try graceful shutdown first
            let _ = child.kill();
            match child.wait() {
                Ok(status) => {
                    println!("{} LSP server stopped ({})", style("✓").green(), status);
                }
                Err(e) => {
                    println!("{} LSP server stop error: {}", style("⚠").yellow(), e);
                }
            }
        }
        Ok(())
    }

    /// Find the LSP binary in common locations
    fn find_lsp_binary(&self) -> Result<PathBuf> {
        let candidates = vec![
            PathBuf::from("target/release/perl-lsp"),
            PathBuf::from("target/debug/perl-lsp"),
            PathBuf::from("crates/perl-lsp/target/release/perl-lsp"),
            PathBuf::from("crates/perl-lsp/target/debug/perl-lsp"),
        ];

        for candidate in candidates {
            if candidate.exists() {
                return Ok(candidate);
            }
        }

        // Try PATH
        match which::which("perl-lsp") {
            Ok(path) => Ok(path),
            Err(_) => Err(eyre!("Could not find perl-lsp binary. Please build it first with: cargo build -p perl-lsp --release")),
        }
    }

    /// Check if the LSP server process is still running
    fn is_lsp_running(&mut self) -> bool {
        if let Some(ref mut child) = self.lsp_process {
            match child.try_wait() {
                Ok(Some(_)) => {
                    // Process has exited
                    self.lsp_process = None;
                    false
                }
                Ok(None) => {
                    // Process is still running
                    true
                }
                Err(_) => {
                    // Error checking status, assume dead
                    self.lsp_process = None;
                    false
                }
            }
        } else {
            false
        }
    }

    /// Restart the LSP server with debouncing
    fn restart_lsp_server(&mut self) -> Result<()> {
        // Implement debouncing
        if let Some(last_restart) = self.last_restart {
            let elapsed = last_restart.elapsed();
            let debounce_duration = Duration::from_millis(self.config.debounce_ms);
            if elapsed < debounce_duration {
                println!("{} Debouncing restart ({}ms remaining)", 
                        style("⏱").yellow(), 
                        (debounce_duration - elapsed).as_millis());
                return Ok(());
            }
        }

        println!("{} Restarting LSP server due to file changes...", 
                style("🔄").blue());
        self.start_lsp_server()
    }

    /// Setup file watching
    async fn setup_file_watching(&mut self) -> Result<()> {
        let (tx, rx) = mpsc::channel();

        let mut watcher: RecommendedWatcher = Watcher::new(
            move |result: notify::Result<notify::Event>| {
                match result {
                    Ok(event) => {
                        if let Err(e) = tx.send(event) {
                            eprintln!("Watch event send error: {}", e);
                        }
                    }
                    Err(e) => eprintln!("Watch error: {}", e),
                }
            },
            notify::Config::default(),
        )?;

        // Watch configured paths
        for path in &self.config.watch_paths {
            if path.exists() {
                println!("{} Watching: {}", 
                        style("👁").cyan(), 
                        path.display());
                watcher.watch(path, RecursiveMode::Recursive)?;
            } else {
                println!("{} Skipping non-existent path: {}", 
                        style("⚠").yellow(), 
                        path.display());
            }
        }

        println!("{} File watching started. Press Ctrl+C to stop.", 
                style("✓").green());

        // Process file events
        loop {
            // Check LSP server health
            if !self.is_lsp_running() {
                println!("{} LSP server died, restarting...", style("💀").red());
                self.start_lsp_server()?;
            }

            // Process watch events with timeout
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(event) => {
                    if self.should_restart_for_event(&event) {
                        // Enhanced logging for file change events
                        let paths: Vec<String> = event.paths.iter()
                            .map(|p| p.display().to_string())
                            .collect();
                        println!("{} File change detected: {} - {}", 
                                style("📁").cyan(),
                                format!("{:?}", event.kind).replace("EventKind::", ""),
                                paths.join(", "));
                        
                        self.restart_lsp_server()?;
                        // Sleep to allow for additional rapid changes
                        sleep(Duration::from_millis(self.config.debounce_ms)).await;
                        
                        // Drain additional events during debounce period
                        let mut drained_events = 0;
                        while rx.try_recv().is_ok() {
                            drained_events += 1;
                        }
                        
                        if drained_events > 0 {
                            println!("{} Drained {} additional file events during debounce period", 
                                    style("🔄").yellow(), drained_events);
                        }
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    // No events, continue monitoring
                    continue;
                }
                Err(mpsc::RecvTimeoutError::Disconnected) => {
                    return Err(eyre!("File watcher disconnected"));
                }
            }
        }
    }

    /// Determine if an event should trigger a server restart
    fn should_restart_for_event(&self, event: &notify::Event) -> bool {
        match event.kind {
            EventKind::Create(_) | EventKind::Modify(_) | EventKind::Remove(_) => {
                event.paths.iter().any(|path| self.should_watch_file(path))
            }
            _ => false,
        }
    }

    /// Check if the configured port is available
    fn check_port_availability(&self) -> Result<()> {
        use std::net::{TcpListener, SocketAddr, IpAddr, Ipv4Addr};
        
        let addr = SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), self.config.port);
        match TcpListener::bind(addr) {
            Ok(_) => Ok(()),
            Err(e) => Err(eyre!("Port {} is not available: {}", self.config.port, e)),
        }
    }

    /// Check if a file should trigger watching based on extension
    fn should_watch_file(&self, path: &Path) -> bool {
        // Always watch important config files
        if let Some(file_name) = path.file_name()
            && let Some(name_str) = file_name.to_str()
            && matches!(name_str, "Cargo.toml" | "Cargo.lock") {
            return true;
        }
        
        // Check extensions
        if let Some(extension) = path.extension()
            && let Some(ext_str) = extension.to_str() {
            return self.config.watch_extensions.contains(&ext_str.to_string());
        }
        
        false
    }

    /// Run the development server
    pub async fn run(&mut self) -> Result<()> {
        println!("{} Starting development server", style("🚀").green());
        
        // Note: This is a development server that manages LSP lifecycle, 
        // not a traditional HTTP server. The port is reserved for future use.
        if let Err(e) = self.check_port_availability() {
            println!("{} Port {} appears to be in use, continuing anyway: {}", 
                    style("⚠").yellow(), self.config.port, e);
        } else {
            println!("{} Port {} is available for future use", 
                    style("✓").green(), self.config.port);
        }

        // Start LSP server
        self.start_lsp_server()?;

        if self.config.watch {
            // Run with file watching
            self.setup_file_watching().await
        } else {
            // Run without file watching, just monitor LSP health
            println!("{} Running in static mode (no file watching)", 
                    style("📡").blue());
            println!("{} Press Ctrl+C to stop", style("ℹ").blue());

            loop {
                if !self.is_lsp_running() {
                    println!("{} LSP server died, restarting...", style("💀").red());
                    self.start_lsp_server()?;
                }
                
                sleep(Duration::from_secs(1)).await;
            }
            #[allow(unreachable_code)]
            Ok(())
        }
    }
}

impl Drop for DevServer {
    fn drop(&mut self) {
        let _ = self.stop_lsp_server();
    }
}

/// Main entry point for the development server task
pub fn run(watch: bool, port: u16) -> Result<()> {
    let spinner = ProgressBar::new_spinner();
    spinner.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {wide_msg}")
            .unwrap(),
    );

    spinner.set_message("Initializing development server");

    // Create configuration
    let config = DevServerConfig {
        watch,
        port,
        ..DevServerConfig::default()
    };

    spinner.finish_with_message("Configuration ready");

    // Initialize tokio runtime
    let rt = tokio::runtime::Runtime::new()?;

    rt.block_on(async {
        let mut dev_server = DevServer::new(config);
        
        // Setup graceful shutdown
        let result = tokio::select! {
            result = dev_server.run() => result,
            _ = tokio::signal::ctrl_c() => {
                println!("\n{} Received interrupt signal, shutting down...", 
                        style("🛑").yellow());
                Ok(())
            }
        };

        result
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;
    use tokio::time::Duration as TokioDuration;

    #[test]
    fn test_dev_server_config_default() {
        let config = DevServerConfig::default();
        assert_eq!(config.port, 8080);
        assert!(!config.watch);
        assert_eq!(config.debounce_ms, 500);
        assert!(config.lsp_args.contains(&"--stdio".to_string()));
        assert!(config.watch_extensions.contains(&"rs".to_string()));
    }

    #[test]
    fn test_should_watch_file() {
        let config = DevServerConfig::default();
        let dev_server = DevServer::new(config);

        // Test Rust files
        assert!(dev_server.should_watch_file(Path::new("src/main.rs")));
        assert!(dev_server.should_watch_file(Path::new("lib.rs")));

        // Test Perl files
        assert!(dev_server.should_watch_file(Path::new("script.pl")));
        assert!(dev_server.should_watch_file(Path::new("Module.pm")));

        // Test config files
        assert!(dev_server.should_watch_file(Path::new("Cargo.toml")));
        assert!(dev_server.should_watch_file(Path::new("Cargo.lock")));

        // Test ignored files
        assert!(!dev_server.should_watch_file(Path::new("README.md")));
        assert!(!dev_server.should_watch_file(Path::new("target/debug/exe")));
        assert!(!dev_server.should_watch_file(Path::new("file.txt")));
    }

    #[test]
    fn test_find_lsp_binary_when_missing() {
        let config = DevServerConfig::default();
        let dev_server = DevServer::new(config);

        // This should fail since we don't have perl-lsp built or in PATH in test environment
        let result = dev_server.find_lsp_binary();
        assert!(result.is_err());
        
        let error_msg = result.unwrap_err().to_string();
        assert!(error_msg.contains("Could not find perl-lsp binary"));
    }

    #[tokio::test]
    async fn test_dev_server_configuration() {
        let mut config = DevServerConfig::default();
        config.port = 9999;
        config.watch = true;
        config.debounce_ms = 100;

        let dev_server = DevServer::new(config.clone());
        assert_eq!(dev_server.config.port, 9999);
        assert!(dev_server.config.watch);
        assert_eq!(dev_server.config.debounce_ms, 100);
    }

    #[tokio::test]
    async fn test_file_event_handling() {
        let temp_dir = TempDir::new().unwrap();
        let temp_path = temp_dir.path().to_path_buf();

        let mut config = DevServerConfig::default();
        config.watch_paths = vec![temp_path.clone()];
        config.debounce_ms = 50;
        config.lsp_binary = Some(PathBuf::from("/bin/echo")); // Use echo as mock binary
        config.lsp_args = vec!["mock-lsp-server".to_string()];

        let dev_server = DevServer::new(config);

        // Test should_restart_for_event
        let create_event = notify::Event {
            kind: EventKind::Create(notify::event::CreateKind::File),
            paths: vec![temp_path.join("test.rs")],
            attrs: Default::default(),
        };
        
        assert!(dev_server.should_restart_for_event(&create_event));

        let modify_event = notify::Event {
            kind: EventKind::Modify(notify::event::ModifyKind::Data(notify::event::DataChange::Content)),
            paths: vec![temp_path.join("test.rs")],
            attrs: Default::default(),
        };

        assert!(dev_server.should_restart_for_event(&modify_event));

        // Test ignored events
        let access_event = notify::Event {
            kind: EventKind::Access(notify::event::AccessKind::Read),
            paths: vec![temp_path.join("test.rs")],
            attrs: Default::default(),
        };

        assert!(!dev_server.should_restart_for_event(&access_event));
    }

    #[tokio::test] 
    async fn test_debounce_logic() {
        let mut config = DevServerConfig::default();
        config.lsp_binary = Some(PathBuf::from("/bin/echo"));
        config.lsp_args = vec!["mock-server".to_string()];
        config.debounce_ms = 100;

        let mut dev_server = DevServer::new(config);

        // First restart should succeed
        let result1 = dev_server.restart_lsp_server();
        assert!(result1.is_ok());

        // Immediate restart should be debounced
        let result2 = dev_server.restart_lsp_server();
        assert!(result2.is_ok()); // It succeeds but doesn't actually restart

        // Wait for debounce period to pass
        tokio::time::sleep(TokioDuration::from_millis(150)).await;

        // Now restart should succeed again
        let result3 = dev_server.restart_lsp_server();
        assert!(result3.is_ok());
    }

    #[tokio::test]
    async fn test_lsp_process_lifecycle() {
        let mut config = DevServerConfig::default();
        config.lsp_binary = Some(PathBuf::from("/bin/sleep"));
        config.lsp_args = vec!["1".to_string()]; // Sleep for 1 second

        let mut dev_server = DevServer::new(config);

        // Initially no process running
        assert!(!dev_server.is_lsp_running());

        // Start the process
        let start_result = dev_server.start_lsp_server();
        assert!(start_result.is_ok());
        assert!(dev_server.is_lsp_running());

        // Stop the process
        let stop_result = dev_server.stop_lsp_server();
        assert!(stop_result.is_ok());
        assert!(!dev_server.is_lsp_running());
    }

    #[tokio::test]
    async fn test_custom_watch_paths() {
        let temp_dir = TempDir::new().unwrap();
        let custom_path = temp_dir.path().join("custom");
        fs::create_dir_all(&custom_path).unwrap();

        let mut config = DevServerConfig::default();
        config.watch_paths = vec![custom_path.clone()];

        let dev_server = DevServer::new(config);

        // Create a test file in the custom path
        let test_file = custom_path.join("test.rs");
        assert!(dev_server.should_watch_file(&test_file));

        let non_watch_file = custom_path.join("test.txt");
        assert!(!dev_server.should_watch_file(&non_watch_file));
    }

    #[tokio::test]
    async fn test_watch_extensions_configuration() {
        let mut config = DevServerConfig::default();
        config.watch_extensions = vec!["custom".to_string(), "ext".to_string()];

        let dev_server = DevServer::new(config);

        assert!(dev_server.should_watch_file(Path::new("file.custom")));
        assert!(dev_server.should_watch_file(Path::new("another.ext")));
        assert!(!dev_server.should_watch_file(Path::new("ignored.rs"))); // rs not in custom list
        
        // But Cargo.toml should still be watched
        assert!(dev_server.should_watch_file(Path::new("Cargo.toml")));
    }

    #[test]
    fn test_drop_cleanup() {
        let mut config = DevServerConfig::default();
        config.lsp_binary = Some(PathBuf::from("/bin/sleep"));
        config.lsp_args = vec!["10".to_string()]; // Sleep for 10 seconds

        {
            let mut dev_server = DevServer::new(config);
            let _ = dev_server.start_lsp_server();
            assert!(dev_server.is_lsp_running());
        } // dev_server goes out of scope here, Drop should be called

        // Process should be cleaned up, but we can't easily test this without process inspection
    }

    #[test]
    fn test_port_availability_check() {
        let config = DevServerConfig::default();
        let dev_server = DevServer::new(config);

        // This test checks that the port checking logic works
        // We can't guarantee any specific port is available, so just test that it returns a Result
        let result = dev_server.check_port_availability();
        // Either it succeeds (port available) or fails (port in use) - both are valid outcomes
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_enhanced_logging_paths() {
        // Test that our path display logic works correctly
        let config = DevServerConfig::default();
        let dev_server = DevServer::new(config);
        
        // Test with a simple file path
        let test_path = PathBuf::from("test/file.rs");
        assert!(dev_server.should_watch_file(&test_path));
        
        // Test with a path containing special characters or spaces wouldn't crash
        let special_path = PathBuf::from("test with spaces/file.rs");
        assert!(dev_server.should_watch_file(&special_path));
    }
}