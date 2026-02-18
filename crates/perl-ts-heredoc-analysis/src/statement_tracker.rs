//! Statement boundary tracker for proper heredoc content collection
//!
//! This module provides a simple statement boundary detector that helps
//! the heredoc scanner know when a statement containing heredocs actually ends.
//!
//! Enhanced in Issue #182 to support block depth tracking and heredoc context management.

#[allow(dead_code)]
#[derive(Debug, Clone, Copy, PartialEq)]
enum BracketType {
    Paren,  // ()
    Square, // []
    Curly,  // {}
}

/// Type of code block (Issue #182)
#[derive(Debug, Clone, Copy, PartialEq)]
#[allow(dead_code)]
pub enum BlockType {
    If,
    Unless,
    While,
    Until,
    For,
    Foreach,
    Sub,
    Begin,
    End,
    Package,
    Anonymous, // Anonymous sub or do block
}

/// Tracks where code blocks begin and end (Issue #182)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct BlockBoundary {
    /// Type of block (if, while, for, sub, BEGIN, END, etc.)
    pub block_type: BlockType,

    /// Line number where block opens
    pub start_line: usize,

    /// Line number where block closes (None if not yet closed)
    pub end_line: Option<usize>,

    /// Depth of this block (0 = top-level)
    pub depth: usize,

    /// Parent block depth (None if top-level)
    pub parent_depth: Option<usize>,
}

/// Context information for a heredoc declaration (Issue #182)
#[derive(Debug, Clone)]
#[allow(dead_code)]
pub struct HeredocContext {
    /// Line number where heredoc was declared
    pub declaration_line: usize,

    /// Block depth at time of declaration (0 = top-level)
    pub block_depth_at_declaration: usize,

    /// The terminator string (e.g., "EOF", "DATA")
    pub terminator: String,

    /// Line where the statement containing this heredoc ends
    pub statement_end_line: usize,

    /// Line where content collection should start
    pub content_start_line: usize,
}

/// Tracks statement boundaries by monitoring brackets and semicolons
/// Enhanced in Issue #182 to support block depth and heredoc context tracking
#[allow(dead_code)]
pub struct StatementTracker {
    // Existing fields (original functionality)
    bracket_stack: Vec<BracketType>,
    in_string: Option<char>, // None, Some('"'), Some('\''), Some('`')
    escape_next: bool,

    // NEW: Issue #182 - Block depth tracking
    // Used in #219 (plumbing) and #220 (semantics)
    #[allow(dead_code)]
    block_depth: usize,

    // NEW: Issue #182 - Heredoc context management
    // Used in #220 (semantics) to correctly handle heredocs in blocks
    #[allow(dead_code)]
    heredoc_contexts: Vec<HeredocContext>,

    // NEW: Issue #182 - Track where blocks start/end
    // Used in #219 (plumbing) and #220 (semantics)
    #[allow(dead_code)]
    block_boundaries: Vec<BlockBoundary>,
}

#[allow(dead_code)]
impl Default for StatementTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[allow(dead_code)]
impl StatementTracker {
    pub fn new() -> Self {
        Self {
            bracket_stack: Vec::new(),
            in_string: None,
            escape_next: false,
            // Issue #182: Initialize new fields (no-op for now, used in #219/#220)
            block_depth: 0,
            heredoc_contexts: Vec::new(),
            block_boundaries: Vec::new(),
        }
    }

    /// Process a character and return true if we're at a statement boundary
    pub fn process_char(&mut self, ch: char, prev_char: Option<char>) -> bool {
        // Handle escape sequences
        if self.escape_next {
            self.escape_next = false;
            return false;
        }

        if ch == '\\' {
            self.escape_next = true;
            return false;
        }

        // Handle string state
        if let Some(quote) = self.in_string {
            if ch == quote {
                self.in_string = None;
            }
            return false;
        }

        // Check for string starts
        match ch {
            '"' | '\'' | '`' => {
                self.in_string = Some(ch);
                return false;
            }
            _ => {}
        }

        // Track brackets
        match ch {
            '(' => self.bracket_stack.push(BracketType::Paren),
            '[' => self.bracket_stack.push(BracketType::Square),
            '{' => self.bracket_stack.push(BracketType::Curly),
            ')' => {
                if self.bracket_stack.last() == Some(&BracketType::Paren) {
                    self.bracket_stack.pop();
                }
            }
            ']' => {
                if self.bracket_stack.last() == Some(&BracketType::Square) {
                    self.bracket_stack.pop();
                }
            }
            '}' => {
                if self.bracket_stack.last() == Some(&BracketType::Curly) {
                    self.bracket_stack.pop();
                }
            }
            _ => {}
        }

        // Statement ends at semicolon or newline when brackets are balanced
        if self.bracket_stack.is_empty() {
            match ch {
                ';' => return true,
                '\n' => {
                    // Check if the line ended with a comma (continuation)
                    if prev_char != Some(',') {
                        return true;
                    }
                }
                _ => {}
            }
        }

        false
    }

    /// Check if we're currently inside a balanced construct
    pub fn is_balanced(&self) -> bool {
        self.bracket_stack.is_empty() && self.in_string.is_none()
    }

    /// Reset the tracker state
    pub fn reset(&mut self) {
        self.bracket_stack.clear();
        self.in_string = None;
        self.escape_next = false;
        // Issue #182: Reset new fields (no-op for now, used in #219/#220)
        self.block_depth = 0;
        self.heredoc_contexts.clear();
        self.block_boundaries.clear();
    }

    // ===== Issue #182/#220: Block Tracking Semantics =====
    // These methods provide the API for block-aware heredoc handling.
    // Implemented in #220 to enable correct heredoc placement in AST.

    /// Note that a code block is opening (Issue #182/#220)
    ///
    /// This method should be called when a code block opens (e.g., after `if {`, `while {`, `sub {`).
    /// Records the block boundary with 0-based depth, then increments block_depth.
    #[inline]
    #[allow(dead_code)]
    pub fn note_block_open(&mut self, line: usize, block_type: BlockType) {
        // Parent depth is current depth - 1 (None if we're at top level)
        let parent_depth = if self.block_depth > 0 { Some(self.block_depth - 1) } else { None };

        // This block's depth is the current block_depth (0-based: first block = 0)
        self.block_boundaries.push(BlockBoundary {
            block_type,
            start_line: line,
            end_line: None, // Will be filled in by note_block_close
            depth: self.block_depth,
            parent_depth,
        });

        // Increment AFTER recording so depth is 0-based
        self.block_depth += 1;
    }

    /// Note that a code block is closing (Issue #182/#220)
    ///
    /// This method should be called when a code block closes (e.g., at `}`).
    /// Decrements block_depth first, then finds the block at that depth to close.
    #[inline]
    #[allow(dead_code)]
    pub fn note_block_close(&mut self, line: usize) {
        if self.block_depth > 0 {
            // Decrement first (mirrors the increment-after in note_block_open)
            self.block_depth -= 1;

            // Find the most recent unclosed block at the now-current depth (rfind pattern)
            if let Some(block) = self
                .block_boundaries
                .iter_mut()
                .rev()
                .find(|b| b.depth == self.block_depth && b.end_line.is_none())
            {
                block.end_line = Some(line);
            }
        }
    }

    /// Record a heredoc declaration with its context (Issue #182/#220)
    ///
    /// This method should be called when a heredoc declaration is detected.
    /// It records the heredoc context including the block depth at declaration time.
    #[inline]
    #[allow(dead_code)]
    pub fn note_heredoc_declaration(
        &mut self,
        line: usize,
        terminator: &str,
        statement_end_line: usize,
    ) {
        self.heredoc_contexts.push(HeredocContext {
            declaration_line: line,
            block_depth_at_declaration: self.block_depth,
            terminator: terminator.to_string(),
            statement_end_line,
            content_start_line: statement_end_line + 1,
        });
    }

    /// Get the current block depth (Issue #182/#220)
    #[inline]
    #[allow(dead_code)]
    pub fn current_block_depth(&self) -> usize {
        self.block_depth
    }

    /// Get heredoc contexts for processing (Issue #182/#220)
    #[inline]
    #[allow(dead_code)]
    pub fn heredoc_contexts(&self) -> &[HeredocContext] {
        &self.heredoc_contexts
    }

    /// Get block boundaries for analysis (Issue #182/#220)
    #[inline]
    #[allow(dead_code)]
    pub fn block_boundaries(&self) -> &[BlockBoundary] {
        &self.block_boundaries
    }
}

/// Find the line where a statement containing a heredoc actually ends
pub fn find_statement_end_line(input: &str, heredoc_line: usize) -> usize {
    let lines: Vec<&str> = input.lines().collect();

    if heredoc_line == 0 || heredoc_line > lines.len() {
        return heredoc_line;
    }

    let mut tracker = StatementTracker::new();
    let mut prev_char: Option<char> = None;

    // #221: Block-aware statement detection
    // Check if heredoc line ends with semicolon (outside strings/comments)
    let heredoc_line_text = lines[heredoc_line - 1];
    let ends_with_semicolon = heredoc_line_text
        .trim_end()
        .trim_end_matches(|c: char| c.is_whitespace() || c == ';')
        .len()
        < heredoc_line_text.trim_end().len()
        && heredoc_line_text.trim_end().ends_with(';');

    if ends_with_semicolon {
        // Heredoc statement is complete on this line (e.g., `my $x = <<EOF;`)
        // Don't pre-scan - this prevents block delimiters from being tracked
        // Just return the heredoc line itself
        return heredoc_line;
    }

    // Heredoc is part of a larger expression (e.g., inside hash/array/function call)
    // Pre-scan to establish bracket state for data structures
    for line in lines.iter().take(heredoc_line - 1) {
        for ch in line.chars() {
            if tracker.process_char(ch, prev_char) {
                tracker.reset();
            }
            prev_char = Some(ch);
        }
        if tracker.process_char('\n', prev_char) {
            tracker.reset();
        }
        prev_char = None;
    }

    // Scan from the heredoc line forward until a statement boundary is found
    for (idx, line) in lines.iter().enumerate().skip(heredoc_line - 1) {
        for ch in line.chars() {
            if tracker.process_char(ch, prev_char) {
                return idx + 1;
            }
            prev_char = Some(ch);
        }
        if tracker.process_char('\n', prev_char) {
            return idx + 1;
        }
        prev_char = None;
    }

    lines.len()
}

#[cfg(test)]
mod tests {
    use super::*;

    // ===== Existing tests (unchanged) =====

    #[test]
    fn test_simple_statement() {
        let input = "my $x = 42;";
        let end = find_statement_end_line(input, 1);
        assert_eq!(end, 1);
    }

    #[test]
    fn test_multi_line_hash() {
        let input = r#"my %hash = (
    key => <<'EOF'
);
content
EOF"#;
        let end = find_statement_end_line(input, 2);
        assert_eq!(end, 3); // Statement ends at line 3 with );
    }

    #[test]
    fn test_nested_parens() {
        let input = r#"func(
    arg1,
    func2(
        <<'EOF'
    ),
    arg3
);
content
EOF"#;
        let end = find_statement_end_line(input, 4);
        assert_eq!(end, 7); // Statement ends at line 7 with );
    }

    #[test]
    fn test_array_ref() {
        let input = r#"my $ref = [
    "first",
    <<'EOF',
    "third"
];
content
EOF"#;
        let end = find_statement_end_line(input, 3);
        assert_eq!(end, 5); // Statement ends at line 5 with ];
    }

    // ===== NEW: Issue #220 - Block Tracking Semantics Tests =====

    #[test]
    fn test_block_depth_tracking_single_block() {
        let mut tracker = StatementTracker::new();
        assert_eq!(tracker.current_block_depth(), 0);

        tracker.note_block_open(1, BlockType::If);
        assert_eq!(tracker.current_block_depth(), 1);

        tracker.note_block_close(3);
        assert_eq!(tracker.current_block_depth(), 0);
    }

    #[test]
    fn test_block_depth_tracking_nested_blocks() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        assert_eq!(tracker.current_block_depth(), 1);

        tracker.note_block_open(2, BlockType::While);
        assert_eq!(tracker.current_block_depth(), 2);

        tracker.note_block_open(3, BlockType::For);
        assert_eq!(tracker.current_block_depth(), 3);

        tracker.note_block_close(4);
        assert_eq!(tracker.current_block_depth(), 2);

        tracker.note_block_close(5);
        assert_eq!(tracker.current_block_depth(), 1);

        tracker.note_block_close(6);
        assert_eq!(tracker.current_block_depth(), 0);
    }

    #[test]
    fn test_block_boundaries_recorded() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        tracker.note_block_open(2, BlockType::While);

        let boundaries = tracker.block_boundaries();
        assert_eq!(boundaries.len(), 2);

        assert_eq!(boundaries[0].block_type, BlockType::If);
        assert_eq!(boundaries[0].start_line, 1);
        assert_eq!(boundaries[0].depth, 0); // 0-based: first block is depth 0
        assert_eq!(boundaries[0].end_line, None);

        assert_eq!(boundaries[1].block_type, BlockType::While);
        assert_eq!(boundaries[1].start_line, 2);
        assert_eq!(boundaries[1].depth, 1); // 0-based: nested block is depth 1
        assert_eq!(boundaries[1].end_line, None);
    }

    #[test]
    fn test_block_boundaries_closed() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        tracker.note_block_close(5);

        let boundaries = tracker.block_boundaries();
        assert_eq!(boundaries.len(), 1);
        assert_eq!(boundaries[0].end_line, Some(5));
    }

    #[test]
    fn test_heredoc_declaration_at_top_level() {
        let mut tracker = StatementTracker::new();

        tracker.note_heredoc_declaration(1, "EOF", 1);

        let contexts = tracker.heredoc_contexts();
        assert_eq!(contexts.len(), 1);

        assert_eq!(contexts[0].declaration_line, 1);
        assert_eq!(contexts[0].block_depth_at_declaration, 0);
        assert_eq!(contexts[0].terminator, "EOF");
        assert_eq!(contexts[0].statement_end_line, 1);
        assert_eq!(contexts[0].content_start_line, 2);
    }

    #[test]
    fn test_heredoc_declaration_in_if_block() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        tracker.note_heredoc_declaration(2, "EOF", 2);

        let contexts = tracker.heredoc_contexts();
        assert_eq!(contexts.len(), 1);

        assert_eq!(contexts[0].declaration_line, 2);
        assert_eq!(contexts[0].block_depth_at_declaration, 1);
        assert_eq!(contexts[0].terminator, "EOF");
    }

    #[test]
    fn test_heredoc_declaration_in_nested_blocks() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        tracker.note_block_open(2, BlockType::While);
        tracker.note_heredoc_declaration(3, "DATA", 3);

        let contexts = tracker.heredoc_contexts();
        assert_eq!(contexts.len(), 1);

        assert_eq!(contexts[0].block_depth_at_declaration, 2);
        assert_eq!(contexts[0].terminator, "DATA");
    }

    #[test]
    fn test_multiple_heredocs_in_same_block() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        tracker.note_heredoc_declaration(2, "EOF1", 2);
        tracker.note_heredoc_declaration(4, "EOF2", 4);

        let contexts = tracker.heredoc_contexts();
        assert_eq!(contexts.len(), 2);

        assert_eq!(contexts[0].terminator, "EOF1");
        assert_eq!(contexts[1].terminator, "EOF2");
        assert_eq!(contexts[0].block_depth_at_declaration, 1);
        assert_eq!(contexts[1].block_depth_at_declaration, 1);
    }

    #[test]
    fn test_block_parent_depth_tracking() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        tracker.note_block_open(2, BlockType::While);

        let boundaries = tracker.block_boundaries();

        assert_eq!(boundaries[0].parent_depth, None); // Top-level if has no parent
        assert_eq!(boundaries[1].parent_depth, Some(0)); // While's parent is at depth 0
    }

    #[test]
    fn test_reset_clears_block_tracking() {
        let mut tracker = StatementTracker::new();

        tracker.note_block_open(1, BlockType::If);
        tracker.note_heredoc_declaration(2, "EOF", 2);

        assert_eq!(tracker.current_block_depth(), 1);
        assert_eq!(tracker.heredoc_contexts().len(), 1);
        assert_eq!(tracker.block_boundaries().len(), 1);

        tracker.reset();

        assert_eq!(tracker.current_block_depth(), 0);
        assert_eq!(tracker.heredoc_contexts().len(), 0);
        assert_eq!(tracker.block_boundaries().len(), 0);
    }
}
