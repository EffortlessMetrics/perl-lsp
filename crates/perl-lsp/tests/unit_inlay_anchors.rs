//! Unit tests for inlay hint anchor logic (public LSP surface only).
//! We assert that specific labels (e.g., `FILEHANDLE:`/`ARRAY:`/`hash`)
//! are placed exactly at the token we expect.

#[cfg(test)]
mod tests {
    use parking_lot::Mutex;
    use perl_lsp::LspServer;
    use serde_json::json;
    use std::io::{Cursor, Write};
    use std::sync::Arc;

    /// Start a server with a writable buffer so we can reuse the harness pattern if needed.
    fn start_server() -> (LspServer, Arc<Mutex<Cursor<Vec<u8>>>>) {
        let buf = Arc::new(Mutex::new(Cursor::<Vec<u8>>::new(Vec::new())));
        let srv = LspServer::with_output(Arc::new(Mutex::new(Box::new(Cursor::<Vec<u8>>::new(
            Vec::new(),
        )) as Box<dyn Write + Send>)));
        (srv, buf)
    }

    /// Drive initialize + didOpen + inlayHint(range) and return the result array (or empty array).
    fn get_hints(server: &mut LspServer, uri: &str, text: &str) -> Vec<serde_json::Value> {
        // initialize (min caps; advertise pull diags so server won't publish)
        let _ = server.handle_request(
            serde_json::from_value(json!({
                "jsonrpc":"2.0","id":1,"method":"initialize","params":{
                    "capabilities":{"textDocument":{"diagnostic":{}}}
                }
            }))
            .unwrap(),
        );
        let _ = server.handle_request(
            serde_json::from_value(json!({"jsonrpc":"2.0","method":"initialized","params":{}}))
                .unwrap(),
        );

        // didOpen
        let _ = server.handle_request(
            serde_json::from_value(json!({
              "jsonrpc":"2.0","method":"textDocument/didOpen","params":{
                "textDocument":{"uri":uri,"languageId":"perl","version":1,"text":text}
              }
            }))
            .unwrap(),
        );

        // full-file range (0..big)
        let res = server.handle_request(
            serde_json::from_value(json!({
              "jsonrpc":"2.0","id":2,"method":"textDocument/inlayHint","params":{
                "textDocument":{"uri":uri},
                "range":{"start":{"line":0,"character":0},"end":{"line":999,"character":0}}
              }
            }))
            .unwrap(),
        );

        // Extract result array
        res.and_then(|r| r.result).and_then(|r| r.as_array().cloned()).unwrap_or_default()
    }

    /// Assert that a hint with `label` is anchored at (line, char) where `needle`
    /// first occurs in `text`. We search on the specific `expected_line`.
    /// Also ensures exactly one hint matches (no duplicates).
    fn assert_unique_label_at(
        text: &str,
        hints: &[serde_json::Value],
        label: &str,
        expected_line: usize,
        needle: &str,
    ) {
        // find column of `needle` in the given line
        let line_str = text.lines().nth(expected_line).expect("line exists");
        let col = line_str.find(needle).expect("needle present on expected line");
        let want_line = expected_line as u32;
        let want_char = col as u32;

        // count matching hints to ensure uniqueness
        let matches = hints
            .iter()
            .filter(|h| {
                h.get("label").and_then(|l| l.as_str()) == Some(label)
                    && h.pointer("/position/line").and_then(|v| v.as_u64())
                        == Some(want_line as u64)
                    && h.pointer("/position/character").and_then(|v| v.as_u64())
                        == Some(want_char as u64)
            })
            .count();

        assert_eq!(
            matches, 1,
            "Expected exactly one `{label}` at {want_line}:{want_char}, got {matches}.\nHints: {hints:#?}"
        );
    }

    #[test]
    fn anchor_filehandle_nonparen() {
        // Tests anchoring behavior for non-parenthesized function calls.
        // For `open my $fh, ...` we anchor at "my" to precede the variable declaration.
        // For array/hash operations, we anchor at the sigil position.
        let (mut server, _out) = start_server();
        let uri = "file:///tmp/anchors.pl";
        let text = r#"
open my $fh, "<", $file;
push @arr, "x";
my %h = ();
my $r = {};
"#;
        let hints = get_hints(&mut server, uri, text);
        // Lines are 0-based; first non-empty is line 1.
        // For "open my $fh", the FILEHANDLE hint anchors at "my" (column 5)
        assert_unique_label_at(text, &hints, "FILEHANDLE:", 1, "my");
        // For "push @arr", the ARRAY hint anchors at "@arr" (column 5)
        assert_unique_label_at(text, &hints, "ARRAY:", 2, "@arr");
    }

    #[test]
    fn anchor_parenthesized_calls() {
        // Tests anchoring behavior for parenthesized function calls.
        // For `open(FH, ...)` we anchor at '(' to maintain visual alignment.
        // For other args, we anchor at the variable/token position.
        let (mut server, _out) = start_server();
        let uri = "file:///tmp/paren.pl";
        let text = r#"
push(@arr, "x");
substr($s, 0, 5);
open(FH, "<", "file.txt");
"#;
        let hints = get_hints(&mut server, uri, text);
        // For "push(@arr", the ARRAY hint anchors at "@arr" (column 5)
        assert_unique_label_at(text, &hints, "ARRAY:", 1, "@arr");
        // For "substr($s", the str hint anchors at "$s" (column 7)
        assert_unique_label_at(text, &hints, "str:", 2, "$s");
        // For "open(FH", the FILEHANDLE hint anchors at "(" (column 4)
        // This keeps the label visually aligned with parenthesized calls
        assert_unique_label_at(text, &hints, "FILEHANDLE:", 3, "(");
    }
}
