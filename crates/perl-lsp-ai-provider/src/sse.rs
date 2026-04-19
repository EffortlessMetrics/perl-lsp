//! Server-Sent Events frame parser for streaming AI responses.

/// A parsed SSE event.
#[derive(Debug, Clone)]
pub struct SseEvent {
    /// Event type (from `event:` field). Defaults to "message".
    pub event: String,
    /// Event data (from `data:` field(s), joined by newlines).
    pub data: String,
}

/// Parse SSE frames from a byte stream reader.
///
/// Yields events one at a time. Handles:
/// - Multi-line `data:` fields (joined by newline)
/// - Comment lines (`:` prefix, ignored)
/// - Keepalive empty lines
/// - `[DONE]` sentinel
pub struct SseParser<R: std::io::BufRead> {
    reader: R,
    done: bool,
}

impl<R: std::io::BufRead> SseParser<R> {
    pub fn new(reader: R) -> Self {
        Self {
            reader,
            done: false,
        }
    }

    /// Read the next SSE event. Returns None when stream is done.
    pub fn next_event(&mut self) -> Result<Option<SseEvent>, std::io::Error> {
        if self.done {
            return Ok(None);
        }

        let mut event_type = String::from("message");
        let mut data_lines: Vec<String> = Vec::new();
        let mut has_data = false;

        loop {
            let mut line = String::new();
            let bytes_read = self.reader.read_line(&mut line)?;
            if bytes_read == 0 {
                self.done = true;
                if has_data {
                    break;
                }
                return Ok(None);
            }

            let line = line.trim_end_matches(['\r', '\n']);

            // Empty line = event boundary
            if line.is_empty() {
                if has_data {
                    break;
                }
                continue;
            }

            // Comment line
            if line.starts_with(':') {
                continue;
            }

            // Parse field
            if let Some(value) = line.strip_prefix("event:") {
                event_type = value.trim().to_string();
            } else if let Some(value) = line.strip_prefix("data:") {
                let value = value.trim();
                if value == "[DONE]" {
                    self.done = true;
                    if has_data {
                        break;
                    }
                    return Ok(None);
                }
                data_lines.push(value.to_string());
                has_data = true;
            }
            // Ignore unknown fields (id:, retry:, etc.)
        }

        Ok(Some(SseEvent {
            event: event_type,
            data: data_lines.join("\n"),
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_simple_event() {
        let input = "data: {\"test\": true}\n\n";
        let mut parser = SseParser::new(Cursor::new(input));
        let event = parser.next_event().ok().flatten();
        assert!(event.is_some());
        let event = event.as_ref();
        assert_eq!(event.map(|e| e.event.as_str()), Some("message"));
        assert_eq!(event.map(|e| e.data.as_str()), Some("{\"test\": true}"));
    }

    #[test]
    fn parse_done_sentinel() {
        let input = "data: [DONE]\n\n";
        let mut parser = SseParser::new(Cursor::new(input));
        let result = parser.next_event();
        assert!(result.is_ok());
        assert!(result.ok().flatten().is_none());
    }

    #[test]
    fn skip_comments() {
        let input = ": keepalive\ndata: hello\n\n";
        let mut parser = SseParser::new(Cursor::new(input));
        let event = parser.next_event().ok().flatten();
        assert_eq!(event.map(|e| e.data), Some("hello".to_string()));
    }

    #[test]
    fn multi_data_lines_joined() {
        let input = "data: line1\ndata: line2\n\n";
        let mut parser = SseParser::new(Cursor::new(input));
        let event = parser.next_event().ok().flatten();
        assert_eq!(event.map(|e| e.data), Some("line1\nline2".to_string()));
    }

    #[test]
    fn custom_event_type() {
        let input = "event: custom\ndata: payload\n\n";
        let mut parser = SseParser::new(Cursor::new(input));
        let event = parser.next_event().ok().flatten();
        assert!(event.is_some());
        let event = event.as_ref();
        assert_eq!(event.map(|e| e.event.as_str()), Some("custom"));
        assert_eq!(event.map(|e| e.data.as_str()), Some("payload"));
    }

    #[test]
    fn multiple_events() {
        let input = "data: first\n\ndata: second\n\n";
        let mut parser = SseParser::new(Cursor::new(input));
        let e1 = parser.next_event().ok().flatten();
        assert_eq!(e1.map(|e| e.data), Some("first".to_string()));
        let e2 = parser.next_event().ok().flatten();
        assert_eq!(e2.map(|e| e.data), Some("second".to_string()));
        let e3 = parser.next_event();
        assert!(e3.is_ok());
        assert!(e3.ok().flatten().is_none());
    }

    #[test]
    fn empty_stream() {
        let input = "";
        let mut parser = SseParser::new(Cursor::new(input));
        let result = parser.next_event();
        assert!(result.is_ok());
        assert!(result.ok().flatten().is_none());
    }
}
