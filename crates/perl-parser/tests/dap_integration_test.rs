use perl_parser::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::fs::write;
use std::sync::mpsc::channel;
use std::time::Duration;
use tempfile::tempdir;

fn wait_for_event(rx: &std::sync::mpsc::Receiver<DapMessage>, name: &str) {
    loop {
        let msg = rx.recv_timeout(Duration::from_secs(10)).expect("event not received");
        if let DapMessage::Event { ref event, .. } = msg
            && event == name
        {
            return;
        }
    }
}

#[test]
#[ignore]
fn test_dap_basic_flow() {
    let dir = tempdir().unwrap();
    let script_path = dir.path().join("sample.pl");
    write(
        &script_path,
        r#"use strict;
use warnings;

my $x = 1;
$x++;
print "x=$x\n";
"#,
    )
    .unwrap();

    let mut adapter = DebugAdapter::new();
    let (tx, rx) = channel();
    adapter.set_event_sender(tx);

    let _ = adapter.handle_request(1, "initialize", None);
    wait_for_event(&rx, "initialized");

    let launch_args = json!({
        "program": script_path.to_str().unwrap(),
        "args": [],
        "stopOnEntry": true
    });
    let _ = adapter.handle_request(2, "launch", Some(launch_args));
    wait_for_event(&rx, "stopped");

    let _ = adapter.handle_request(3, "disconnect", None);
}
