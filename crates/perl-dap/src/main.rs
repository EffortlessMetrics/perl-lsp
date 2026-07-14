//! DAP adapter entry point
//!
//! This binary provides the Debug Adapter Protocol server for Perl debugging.
//! It follows the TDD approach with comprehensive test scaffolding for 19 acceptance criteria.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use clap::Parser;
use perl_dap::backend::capabilities::ControlMode;
use perl_dap::backend::external_peer::ExternalDebuggerPeerBackend;
use perl_dap::backend::peer_launch::{
    DEFAULT_LISTEN_HANDSHAKE_TIMEOUT, ENV_PEER_TOKEN, ExternalPeerLaunchConfig, PeerRendezvousMode,
    prepare_mirror_listen_session, run_mirror_listen_session_socket,
    run_mirror_listen_session_stdio,
};
use perl_dap::backend::{
    DapPeerBridge, run_external_peer_session, run_external_peer_session_stdio,
};
use perl_dap::model::{DebugSessionPacket, DebugSource};
use perl_dap::ptkdb_bootstrap::render_ptkdbrc;
use perl_dap::session_plan::DebugSessionPlanBuilder;
use perl_dap::{DapConfig, DapMode, DapServer};
use perl_lsp_rs_core::runtime::launcher::{init_logging, log_server_startup};

const DEFAULT_DAP_PORT: u16 = 13_603;

/// How long to wait for the external peer handshake / a session poll tick.
const EXTERNAL_PEER_TIMEOUT: Duration = Duration::from_secs(10);
const EXTERNAL_PEER_POLL: Duration = Duration::from_millis(50);

/// Run an external-peer DAP session: the editor connects to us on `editor_port`
/// (socket transport), we connect to the running debugger peer at `peer_addr`
/// (e.g. Devel::ptkdb), and bridge DAP ↔ the Perl Debugger Peer Protocol.
fn run_external_peer_bridge(editor_port: u16, peer_addr: &str) -> anyhow::Result<()> {
    use std::net::TcpListener;

    tracing::info!(port = editor_port, peer = peer_addr, "Starting external-peer DAP bridge");
    // Bind the editor-facing listener FIRST so the port is open (and the editor's
    // connect can queue in the listen backlog) while we connect to the peer, which
    // may take up to EXTERNAL_PEER_TIMEOUT. Otherwise an editor that spawns us and
    // immediately connects could fail before the port ever opened.
    let listener = TcpListener::bind(("127.0.0.1", editor_port))?;
    let backend = ExternalDebuggerPeerBackend::connect(peer_addr, EXTERNAL_PEER_TIMEOUT)
        .map_err(|e| anyhow::anyhow!("failed to connect to debugger peer {peer_addr}: {e}"))?;
    let bridge = DapPeerBridge::new(Box::new(backend));

    // Bound the editor accept: a plain blocking `accept()` would hang the whole
    // process forever (holding the peer connection open, recoverable only by
    // SIGKILL) if the editor crashes before connecting or `--port` is
    // misconfigured. Poll non-blocking against a deadline, matching
    // `ExternalDebuggerPeerBackend::listen`.
    listener.set_nonblocking(true)?;
    let deadline = Instant::now() + EXTERNAL_PEER_TIMEOUT;
    let editor = loop {
        match listener.accept() {
            Ok((stream, _)) => break stream,
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if Instant::now() >= deadline {
                    anyhow::bail!("no editor connected within {EXTERNAL_PEER_TIMEOUT:?}");
                }
                std::thread::sleep(Duration::from_millis(10));
            }
            Err(e) => return Err(anyhow::anyhow!("editor accept failed: {e}")),
        }
    };
    // Restore blocking mode; the session driver manages its own read timeout.
    editor.set_nonblocking(false)?;
    run_external_peer_session(editor, bridge, EXTERNAL_PEER_POLL)?;
    Ok(())
}

/// Run an external-peer DAP session over **stdio**: the editor spawns us as a
/// child process and drives DAP over our stdin/stdout, while we connect to the
/// running debugger peer at `peer_addr` and bridge DAP ↔ the Perl Debugger Peer
/// Protocol. This is the default transport when no `--socket`/`--port` is given.
fn run_external_peer_bridge_stdio(peer_addr: &str) -> anyhow::Result<()> {
    tracing::info!(peer = peer_addr, "Starting external-peer DAP bridge on stdio");
    let backend = ExternalDebuggerPeerBackend::connect(peer_addr, EXTERNAL_PEER_TIMEOUT)
        .map_err(|e| anyhow::anyhow!("failed to connect to debugger peer {peer_addr}: {e}"))?;
    let bridge = DapPeerBridge::new(Box::new(backend));
    run_external_peer_session_stdio(bridge, EXTERNAL_PEER_POLL)?;
    Ok(())
}

/// Parse a `HOST` or `HOST:PORT` bind spec for `--external-peer-listen`.
///
/// A bare host (or empty string) binds an ephemeral loopback port (`port = 0`).
/// A `HOST:PORT` binds the given port. An unparseable port falls back to `0`.
fn parse_listen_bind(spec: &str) -> (String, u16) {
    let spec = spec.trim();
    if spec.is_empty() {
        return ("127.0.0.1".to_string(), 0);
    }
    match spec.rsplit_once(':') {
        Some((host, port)) if !host.is_empty() => {
            (host.to_string(), port.trim().parse().unwrap_or(0))
        }
        _ => (spec.to_string(), 0),
    }
}

/// Run a mirror-mode external-peer **listen** session: bind a loopback listener
/// for a (future) debugger peer to connect back to, expose the env-var contract
/// the peer reads to find us, and serve DAP to the editor while queueing
/// breakpoints until the peer handshakes.
///
/// The peer process itself is out of scope for this wiring; this establishes the
/// host side of the mirror session, proven end-to-end against a fake peer in the
/// crate tests. The editor speaks DAP over stdio by default (add
/// `--socket`/`--port` for a socket editor connection).
fn run_external_peer_listen(spec: &str, editor_port: Option<u16>) -> anyhow::Result<()> {
    let (host, port) = parse_listen_bind(spec);
    let config = ExternalPeerLaunchConfig {
        mode: PeerRendezvousMode::Listen,
        control: ControlMode::Mirror,
        host,
        port,
        ..ExternalPeerLaunchConfig::default()
    };
    let (peer_listener, endpoint, bridge) = prepare_mirror_listen_session(&config)
        .map_err(|e| anyhow::anyhow!("failed to bind peer listener: {e}"))?;

    // Surface the env-var contract the (future) peer process reads to find and
    // authenticate to this host session. The token is a per-session bearer
    // secret, so it is masked in the log — logging it in cleartext would hand
    // any log reader the credential the handshake now enforces. Non-sensitive
    // keys (address, mode) are logged verbatim.
    for (key, value) in endpoint.env_vars() {
        let logged = if key == ENV_PEER_TOKEN { "<redacted>" } else { value.as_str() };
        tracing::info!(%key, value = %logged, "external-peer listen: peer env contract");
    }
    tracing::info!(
        peer_addr = %endpoint.addr,
        "external-peer listen: waiting for a debugger peer to connect back"
    );

    // The peer must present this token in its `peer/hello` to be accepted; the
    // acceptor rejects any handshake without a match, so the loopback bind is
    // not the sole access control.
    let expected_token = Some(endpoint.token.clone());

    match editor_port {
        Some(port) => {
            use std::net::TcpListener;
            let editor_listener = TcpListener::bind(("127.0.0.1", port))?;
            let (editor, _) = editor_listener.accept()?;
            run_mirror_listen_session_socket(
                editor,
                peer_listener,
                bridge,
                DEFAULT_LISTEN_HANDSHAKE_TIMEOUT,
                EXTERNAL_PEER_POLL,
                expected_token,
            )?;
        }
        None => {
            run_mirror_listen_session_stdio(
                peer_listener,
                bridge,
                DEFAULT_LISTEN_HANDSHAKE_TIMEOUT,
                EXTERNAL_PEER_POLL,
                expected_token,
            )?;
        }
    }
    Ok(())
}

/// Build a debug-session packet for `program`, deriving source facts from the
/// program text when it is readable.
fn build_session_packet(program: &Path) -> DebugSessionPacket {
    let mut builder = DebugSessionPlanBuilder::new(program);
    if let Ok(text) = std::fs::read_to_string(program) {
        let source = DebugSource::from_path(program);
        builder = builder.source_facts_from_text(&source, &text);
    }
    builder.build()
}

fn resolve_socket_port(args: &perl_lsp_rs_core::runtime::launcher::TransportArgs) -> Option<u16> {
    if args.socket || args.port.is_some() {
        Some(args.port.unwrap_or(DEFAULT_DAP_PORT))
    } else {
        None
    }
}

/// Perl Debug Adapter Protocol server
#[derive(Parser, Debug)]
#[command(name = "perl-dap", version, about, long_about = None)]
struct Args {
    #[command(flatten)]
    transport: perl_lsp_rs_core::runtime::launcher::TransportArgs,

    /// Logging level (error, warn, info, debug, trace)
    #[arg(long, default_value = "info")]
    log_level: String,

    /// Emit a Devel::ptkdb `.ptkdbrc` bootstrap for PROGRAM to stdout and exit.
    #[arg(long, value_name = "PROGRAM")]
    ptkdb_bootstrap_rc: Option<PathBuf>,

    /// Emit a `perl-lsp-debug-session-v1` JSON session plan for PROGRAM to
    /// stdout and exit.
    #[arg(long, value_name = "PROGRAM")]
    debug_session_plan: Option<PathBuf>,

    /// Connect to an external debugger peer at HOST:PORT (e.g. Devel::ptkdb) and
    /// relay DAP <-> the Perl Debugger Peer Protocol: the editor drives DAP, we
    /// drive the peer. Uses stdio by default (the editor spawns us); add
    /// `--socket`/`--port` for a socket editor connection instead.
    #[arg(long, value_name = "HOST:PORT")]
    external_peer: Option<String>,

    /// Listen for a mirror-mode external debugger peer to connect back (the
    /// `mode: "listen"` external-peer launch). Binds a loopback listener (a bare
    /// HOST or empty value allocates an ephemeral port), exposes the
    /// PERL_DAP_PEER* env contract, and serves DAP to the editor — queueing
    /// breakpoints until the peer handshakes. Editor control is mirror-rejected.
    /// Uses stdio by default; add `--socket`/`--port` for a socket editor link.
    #[arg(long, value_name = "HOST[:PORT]")]
    external_peer_listen: Option<String>,
}

fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // One-shot emit surfaces: these do not start a server. Write directly to
    // the stdout handle (rather than the print!/println! macros) so the shipped
    // binary stays clear of the `clippy::print_stdout` restriction lint.
    if let Some(program) = args.ptkdb_bootstrap_rc.as_deref() {
        let packet = build_session_packet(program);
        write!(std::io::stdout(), "{}", render_ptkdbrc(&packet, true))?;
        return Ok(());
    }
    if let Some(program) = args.debug_session_plan.as_deref() {
        let packet = build_session_packet(program);
        writeln!(std::io::stdout(), "{}", serde_json::to_string_pretty(&packet)?)?;
        return Ok(());
    }

    init_logging(&args.log_level);
    log_server_startup("perl-dap", env!("CARGO_PKG_VERSION"), args.transport.mode(), None, None);

    // External-peer bridge mode: drive an external debugger engine (ptkdb) over
    // the peer protocol while the editor speaks DAP. Additive path — the native
    // adapter is unchanged.
    if let Some(peer_addr) = args.external_peer.as_deref() {
        return match resolve_socket_port(&args.transport) {
            Some(port) => run_external_peer_bridge(port, peer_addr),
            None => run_external_peer_bridge_stdio(peer_addr),
        };
    }

    // External-peer LISTEN mode (mirror): we bind and wait for the peer to
    // connect back. Additive path — the native adapter is unchanged.
    if let Some(spec) = args.external_peer_listen.as_deref() {
        return run_external_peer_listen(spec, resolve_socket_port(&args.transport));
    }

    // The shipped `perl-dap` binary always runs the native adapter. The legacy
    // `BridgeAdapter` (proxy to Perl::LanguageServer) remains available as a
    // library-only compatibility/conformance reference, but is not a shipped
    // product path — see docs/reference legacy bridge notes.
    let config =
        DapConfig { log_level: args.log_level, mode: DapMode::Native, workspace_root: None };

    let mut server = DapServer::new(config)?;

    if let Some(port) = resolve_socket_port(&args.transport) {
        tracing::info!("Starting DAP server on port {}", port);
        server.run_socket(port)?;
        return Ok(());
    }

    tracing::info!("Starting DAP server on stdio");
    server.run()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::{Args, DEFAULT_DAP_PORT, resolve_socket_port};
    use clap::{CommandFactory, Parser};

    /// The shipped `perl-dap` CLI must not advertise legacy bridge mode or
    /// Perl::LanguageServer on its product surface. Bridge is a library-only
    /// compatibility reference, never a shipped command path.
    #[test]
    fn cli_help_has_no_bridge_product_surface() {
        let help = Args::command().render_long_help().to_string();
        assert!(
            !help.contains("--bridge"),
            "perl-dap --help must not expose the legacy --bridge flag: {help}"
        );
        assert!(
            !help.to_lowercase().contains("bridge"),
            "perl-dap --help must not mention bridge mode: {help}"
        );
        assert!(
            !help.contains("Perl::LanguageServer"),
            "perl-dap --help must not mention Perl::LanguageServer: {help}"
        );
    }

    /// Parsing must reject `--bridge`: the flag is removed from the shipped
    /// binary, so an unknown-argument error is the correct, guarded behavior.
    #[test]
    fn cli_rejects_removed_bridge_flag() {
        let result = Args::try_parse_from(["perl-dap", "--bridge"]);
        assert!(result.is_err(), "--bridge must be rejected by the shipped CLI");
    }

    #[test]
    fn socket_mode_uses_dap_default_port() {
        let args = perl_lsp_rs_core::runtime::launcher::TransportArgs {
            stdio: false,
            socket: true,
            port: None,
        };

        assert_eq!(resolve_socket_port(&args), Some(DEFAULT_DAP_PORT));
    }

    #[test]
    fn explicit_socket_port_is_preserved() {
        let args = perl_lsp_rs_core::runtime::launcher::TransportArgs {
            stdio: false,
            socket: true,
            port: Some(9_999),
        };

        assert_eq!(resolve_socket_port(&args), Some(9_999));
    }

    #[test]
    fn stdio_mode_does_not_resolve_a_socket_port() {
        let args = perl_lsp_rs_core::runtime::launcher::TransportArgs {
            stdio: true,
            socket: false,
            port: None,
        };

        assert_eq!(resolve_socket_port(&args), None);
    }
}
