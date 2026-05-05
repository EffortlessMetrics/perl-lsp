mod common;

use common::{read_response_matching_i64, start_lsp_server};
use perl_lsp_rs_core::transport::framing::frame;
use serde_json::json;
use std::io::Write;
use std::time::Duration;

#[test]
fn handles_back_to_back_frames_in_single_write() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    let first = json!({
        "jsonrpc": "2.0",
        "id": 901,
        "method": "test/one",
        "params": {}
    })
    .to_string();
    let second = json!({
        "jsonrpc": "2.0",
        "id": 902,
        "method": "test/two",
        "params": {}
    })
    .to_string();

    let mut bytes = frame(first.as_bytes());
    bytes.extend_from_slice(&frame(second.as_bytes()));

    server.stdin_writer().write_all(&bytes)?;
    server.stdin_writer().flush()?;

    let timeout = Duration::from_secs(2);
    let first_response =
        read_response_matching_i64(&server, 901, timeout).ok_or("missing response 901")?;
    let second_response =
        read_response_matching_i64(&server, 902, timeout).ok_or("missing response 902")?;

    assert_eq!(first_response.get("id"), Some(&json!(901)));
    assert_eq!(second_response.get("id"), Some(&json!(902)));
    Ok(())
}

#[test]
fn handles_split_frame_header_and_body_across_writes() -> Result<(), Box<dyn std::error::Error>> {
    let server = start_lsp_server();

    let request = json!({
        "jsonrpc": "2.0",
        "id": 903,
        "method": "test/split",
        "params": {}
    })
    .to_string();

    let bytes = frame(request.as_bytes());
    let split_at = bytes
        .windows(4)
        .position(|window| window == b"\r\n\r\n")
        .map(|index| index + 4)
        .ok_or("framed message missing header terminator")?;

    server.stdin_writer().write_all(&bytes[..split_at])?;
    server.stdin_writer().flush()?;
    std::thread::sleep(Duration::from_millis(10));
    server.stdin_writer().write_all(&bytes[split_at..])?;
    server.stdin_writer().flush()?;

    let timeout = Duration::from_secs(2);
    let response =
        read_response_matching_i64(&server, 903, timeout).ok_or("missing response 903")?;

    assert_eq!(response.get("id"), Some(&json!(903)));
    Ok(())
}
