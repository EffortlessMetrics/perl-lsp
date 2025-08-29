use perl_parser::debug_adapter::{DapMessage, DebugAdapter};
use serde_json::json;
use std::io::Write;


#[test]
fn launch_and_disconnect() {
    let mut script = tempfile::NamedTempFile::new().unwrap();
    writeln!(script, "print \"hi\\n\";").unwrap();
    let path = script.path().to_str().unwrap().to_string();

    let mut adapter = DebugAdapter::new();

    // initialize
    let resp = adapter.handle_request(1, "initialize", None);
    matches!(resp, DapMessage::Response { success: true, .. });

    // launch debugger
    let launch_args = json!({"program": path, "stopOnEntry": true});
    let resp = adapter.handle_request(2, "launch", Some(launch_args));
    match resp {
        DapMessage::Response { success, .. } => assert!(success),
        _ => panic!("expected response"),
    }

    // disconnect
    let resp = adapter.handle_request(3, "disconnect", None);
    match resp {
        DapMessage::Response { success, .. } => assert!(success),
        _ => panic!("expected response"),
    }
}

