/// Advance byte offset to just after the next line break (handles \n and \r\n)
fn after_line_break(src: &[u8], mut off: usize) -> usize {
    // Skip to newline if in middle of line
    while off < src.len() && src[off] != b'\n' && src[off] != b'\r' {
        off += 1;
    }
    if off < src.len() {
        if src[off] == b'\r' {
            off += 1;
            if off < src.len() && src[off] == b'\n' {
                off += 1;
            }
        } else if src[off] == b'\n' {
            off += 1;
        }
    }
    off
}

/// Unescape a string literal (e.g., convert \n to newline)
fn unescape_label(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\\' {
            if let Some(next) = chars.next() {
                match next {
                    'n' => out.push('\n'),
                    'r' => out.push('\r'),
                    't' => out.push('\t'),
                    '\\' => out.push('\\'),
                    '"' => out.push('"'),
                    '\'' => out.push('\''),
                    '$' => out.push('$'),
                    '@' => out.push('@'),
                    _ => {
                        out.push('\\');
                        out.push(next);
                    }
                }
            } else {
                out.push('\\');
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse heredoc delimiter from a string like "<<EOF", "<<'EOF'", "<<~EOF", "<<`EOF`"
fn parse_heredoc_delimiter(s: &str) -> (String, bool, bool, bool) {
    let mut chars = s.chars();

    // Skip <<
    chars.next();
    chars.next();

    // Check for indented heredoc
    let indented = if chars.as_str().starts_with('~') {
        chars.next();
        true
    } else {
        false
    };

    let rest = chars.as_str().trim();

    // Check for empty label (<<; or <<\n)
    if rest.is_empty() || rest.starts_with(';') {
        return (String::new(), true, indented, false);
    }

    // Check quoting to determine interpolation and command execution
    let (delimiter, interpolated, command) =
        if rest.starts_with('"') && rest.ends_with('"') && rest.len() >= 2 {
            // Double-quoted: interpolated, unescape label
            (unescape_label(&rest[1..rest.len() - 1]), true, false)
        } else if rest.starts_with('\'') && rest.ends_with('\'') && rest.len() >= 2 {
            // Single-quoted: not interpolated, no unescape
            (rest[1..rest.len() - 1].to_string(), false, false)
        } else if rest.starts_with('`') && rest.ends_with('`') && rest.len() >= 2 {
            // Backtick: interpolated, command execution, unescape label
            (unescape_label(&rest[1..rest.len() - 1]), true, true)
        } else {
            // Bare word: interpolated, no unescape (except maybe explicit escapes?)
            // Bare identifiers don't usually have escapes, but can have weird chars?
            // "EOF" -> EOF.
            (rest.to_string(), true, false)
        };

    (delimiter, interpolated, indented, command)
}

/// Map heredoc delimiter text to collector QuoteKind (Sprint A Day 4)
fn map_heredoc_quote_kind(text: &str, _interpolated: bool) -> heredoc_collector::QuoteKind {
    // Skip << and optional ~
    let rest = text.trim_start_matches('<').trim_start_matches('~').trim();

    if rest.starts_with('\'') && rest.ends_with('\'') {
        heredoc_collector::QuoteKind::Single
    } else if rest.starts_with('"') && rest.ends_with('"') {
        heredoc_collector::QuoteKind::Double
    } else if rest.starts_with('`') && rest.ends_with('`') {
        heredoc_collector::QuoteKind::Backtick
    } else {
        // Bare word (unquoted)
        heredoc_collector::QuoteKind::Unquoted
    }
}

const MAX_HEREDOC_DEPTH: usize = 50;
const HEREDOC_TIMEOUT_MS: u64 = 5000;

impl<'a> Parser<'a> {
    /// Enqueue a heredoc declaration for later content collection
    fn push_heredoc_decl(
        &mut self,
        label: String,
        allow_indent: bool,
        quote: heredoc_collector::QuoteKind,
        decl_start: usize,
        decl_end: usize,
    ) {
        if self.pending_heredocs.len() >= MAX_HEREDOC_DEPTH {
            self.errors.push(ParseError::syntax(
                format!("Heredoc depth limit exceeded (max {})", MAX_HEREDOC_DEPTH),
                decl_start,
            ));
            return;
        }

        if self.pending_heredocs.is_empty() {
            self.heredoc_start_time = Some(Instant::now());
        }

        self.pending_heredocs.push_back(PendingHeredoc {
            label: Arc::from(label.as_str()),
            allow_indent,
            quote,
            decl_span: heredoc_collector::Span { start: decl_start, end: decl_end },
        });
    }

    /// Drain all pending heredocs after statement completion (FIFO order)
    fn drain_pending_heredocs(&mut self, root: &mut Node) {
        if self.pending_heredocs.is_empty() {
            self.heredoc_start_time = None;
            return;
        }

        // Check for timeout
        if let Some(start) = self.heredoc_start_time {
            if start.elapsed().as_millis() > HEREDOC_TIMEOUT_MS as u128 {
                self.errors.push(ParseError::syntax(
                    format!("Heredoc parsing timed out (> {}ms)", HEREDOC_TIMEOUT_MS),
                    self.byte_cursor,
                ));
                // Clear pending to prevent further processing/hanging
                self.pending_heredocs.clear();
                self.heredoc_start_time = None;
                return;
            }
        }

        // Advance to first content line (handle newline after statement terminator)
        self.byte_cursor = after_line_break(self.src_bytes, self.byte_cursor);

        // Keep a copy of the declarations so we can match outputs back to inputs
        let pending: Vec<_> = self.pending_heredocs.iter().cloned().collect();

        let out = collect_all(
            self.src_bytes,
            self.byte_cursor,
            std::mem::take(&mut self.pending_heredocs),
        );

        // Zip 1:1 in order (collector preserves input order)
        for (decl, body) in pending.into_iter().zip(out.contents.into_iter()) {
            if !body.terminated {
                self.errors.push(ParseError::syntax(
                    format!("Unterminated heredoc: expected terminator '{}'", decl.label),
                    decl.decl_span.start,
                ));
            }

            let _attached = self.try_attach_heredoc_at_node(root, decl.decl_span, &body);

            // Defensive guardrail: warn if heredoc node wasn't found at expected span
            #[cfg(debug_assertions)]
            if !_attached {
                eprintln!(
                    "[WARNING] drain_pending_heredocs: Failed to attach heredoc content at span {}..{} - no matching Heredoc node found in AST",
                    decl.decl_span.start, decl.decl_span.end
                );
            }
        }
        self.byte_cursor = out.next_offset;
    }

    /// Attach collected heredoc content to its declaration node by matching declaration span
    /// Returns true if a matching Heredoc node was found and updated, false otherwise
    fn try_attach_heredoc_at_node(
        &self,
        root: &mut Node,
        decl_span: heredoc_collector::Span,
        body: &HeredocContent,
    ) -> bool {
        // Depth-first search for the Heredoc node with matching declaration span
        self.try_attach_at_node(root, decl_span, body)
    }

    /// Try to attach heredoc content at this node or its children
    fn try_attach_at_node(
        &self,
        node: &mut Node,
        decl_span: heredoc_collector::Span,
        body: &HeredocContent,
    ) -> bool {
        // Check if this node's span matches the declaration span
        let node_matches =
            node.location.start == decl_span.start && node.location.end == decl_span.end;

        if node_matches {
            // Try to attach at this node
            if let NodeKind::Heredoc { content, body_span, .. } = &mut node.kind {
                // Reify the body bytes from src_bytes using the collector's segments
                let mut s = String::new();
                for (i, seg) in body.segments.iter().enumerate() {
                    if seg.end > seg.start {
                        let bytes = &self.src_bytes[seg.start..seg.end];
                        // Source is valid UTF-8 (enforced by lexer)
                        s.push_str(std::str::from_utf8(bytes).unwrap_or_default());
                    }
                    if i + 1 < body.segments.len() {
                        // Normalize line breaks for AST convenience
                        s.push('\n');
                    }
                }
                *content = s;

                // Store body span for breakpoint detection
                *body_span = if body.full_span.start < body.full_span.end {
                    Some(SourceLocation {
                        start: body.full_span.start,
                        end: body.full_span.end,
                    })
                } else {
                    None // Empty heredoc
                };

                return true;
            }
        }

        // Recursively search children (DFS) using for_each_child_mut
        let mut found = false;
        node.for_each_child_mut(|child| {
            if !found && self.try_attach_at_node(child, decl_span, body) {
                found = true;
            }
        });

        #[cfg(debug_assertions)]
        if !found && node_matches {
            eprintln!(
                "warn: no Heredoc node found for decl span {}..{} (matched span but not Heredoc kind)",
                decl_span.start, decl_span.end
            );
        }

        found
    }

}