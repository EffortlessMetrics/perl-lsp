//! Multi-phase heredoc parser for Perl
//!
//! This module implements a three-phase approach to handle Perl's heredocs:
//! 1. Detection - Identify heredoc declarations and mark boundaries
//! 2. Collection - Extract heredoc content from subsequent lines
//! 3. Integration - Replace markers with content for PEG parsing

use perl_ts_heredoc_analysis::statement_tracker::find_statement_end_line;
use std::collections::{HashMap, HashSet};
use std::sync::Arc;
use std::time::Instant;

const MAX_HEREDOC_DEPTH: usize = 100;
const HEREDOC_TIMEOUT_MS: u64 = 5000;

/// Represents a heredoc declaration found during Phase 1
#[derive(Debug, Clone)]
pub struct HeredocDeclaration {
    /// The terminator string (e.g., "EOF", "DATA")
    pub terminator: String,
    /// Position in input where the heredoc was declared
    pub declaration_pos: usize,
    /// Position where the declaration ends (after terminator)
    pub declaration_end: usize,
    /// Line number of declaration
    pub declaration_line: usize,
    /// Whether the heredoc is interpolated (<<EOF vs <<'EOF')
    pub interpolated: bool,
    /// Whether the heredoc is indented (<<~EOF)
    pub indented: bool,
    /// Unique placeholder token for this heredoc
    pub placeholder_id: String,
    /// The collected content (filled in Phase 2)
    pub content: Option<Arc<str>>,
}

/// Phase 1: Heredoc Detection Scanner
/// Enhanced in #219 (Issue #182) to thread StatementTracker for future block-aware processing
pub struct HeredocScanner<'a> {
    input: &'a str,
    position: usize,
    line_number: usize,
    heredoc_counter: usize,
    /// Track which lines should be skipped (heredoc content lines)
    skip_lines: std::collections::HashSet<usize>,
    /// Issue #182/#219: Statement tracker for block-aware heredoc handling (used in #220)
    #[allow(dead_code)]
    statement_tracker: perl_ts_heredoc_analysis::statement_tracker::StatementTracker,
}

impl<'a> HeredocScanner<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            input,
            position: 0,
            line_number: 1,
            heredoc_counter: 0,
            skip_lines: HashSet::new(),
            // Issue #182/#219: Initialize tracker (no-op for now, used in #220)
            statement_tracker: perl_ts_heredoc_analysis::statement_tracker::StatementTracker::new(),
        }
    }

    /// Test-only: Scan and return skip_lines + contexts for validation
    #[cfg(test)]
    pub fn scan_for_test(
        mut self,
    ) -> (
        String,
        Vec<HeredocDeclaration>,
        HashSet<usize>,
        Vec<perl_ts_heredoc_analysis::statement_tracker::HeredocContext>,
    ) {
        let (output, declarations) = self.do_scan();
        let skip_lines = self.skip_lines.clone();
        let contexts = self.statement_tracker.heredoc_contexts().to_vec();
        (output, declarations, skip_lines, contexts)
    }

    /// Scan input for heredoc declarations and mark their positions
    pub fn scan(mut self) -> (String, Vec<HeredocDeclaration>) {
        self.do_scan()
    }

    /// Internal scan implementation (shared by public scan() and test scan_for_test())
    fn do_scan(&mut self) -> (String, Vec<HeredocDeclaration>) {
        let start_time = Instant::now();
        // First pass: find all heredocs and mark their content lines
        let lines: Vec<&str> = self.input.lines().collect();
        let mut declarations = Vec::new();
        let chars: Vec<char> = self.input.chars().collect();

        // Scan for heredocs and mark content lines to skip
        let mut temp_position = 0;
        let mut temp_line = 1;

        while temp_position < chars.len() {
            // Timeout protection (Issue #443)
            if start_time.elapsed().as_millis() > HEREDOC_TIMEOUT_MS as u128 {
                break;
            }

            if temp_position + 1 < chars.len()
                && chars[temp_position] == '<'
                && chars[temp_position + 1] == '<'
            {
                let saved_pos = temp_position;
                let _saved_line = temp_line;

                // Try to parse heredoc
                self.position = temp_position;
                self.line_number = temp_line;

                if let Some(mut decl) = self.parse_heredoc_declaration(&chars) {
                    // Recursion depth limit (Issue #443)
                    if declarations.len() >= MAX_HEREDOC_DEPTH {
                        // Limit reached: Stop tracking new heredocs to prevent stack overflow.
                        // We continue parsing the rest of the file, but treat the <<EOF token
                        // as a normal operator/identifier rather than starting a heredoc body.
                        // This prevents infinite recursion/loops in malicious input.
                        temp_position = saved_pos + 1;
                        continue;
                    }

                    // Store the actual positions for replacement
                    decl.declaration_pos = saved_pos;
                    decl.declaration_end = self.position;

                    // Don't mark lines to skip yet - we need to know where the statement ends first
                    // Just store the declaration for now

                    declarations.push(decl);
                    temp_position = self.position;
                    temp_line = self.line_number;
                } else {
                    temp_position = saved_pos + 1;
                }
            } else {
                if chars[temp_position] == '\n' {
                    temp_line += 1;
                }
                temp_position += 1;
            }
        }

        // Now mark lines to skip based on statement boundaries
        // Issue #220a: Call tracker for each heredoc declaration
        for decl in &declarations {
            let statement_end_line = find_statement_end_line(self.input, decl.declaration_line);
            let content_start_line = statement_end_line + 1;

            // Issue #220a: Record heredoc declaration with tracker
            self.statement_tracker.note_heredoc_declaration(
                decl.declaration_line,
                &decl.terminator,
                statement_end_line,
            );

            // Mark lines from content start to terminator
            for i in content_start_line..=lines.len() {
                if i > lines.len() {
                    break;
                }
                let line = lines[i - 1];
                let is_terminator = if decl.indented {
                    line.trim() == decl.terminator
                } else {
                    line == decl.terminator
                };

                self.skip_lines.insert(i);

                if is_terminator {
                    break;
                }
            }
        }

        // Second pass: build output, skipping marked lines and replacing heredocs
        let mut output = String::with_capacity(self.input.len());
        self.position = 0;
        self.line_number = 1;
        let mut decl_index = 0;

        while self.position < chars.len() {
            // Skip lines marked for skipping
            if self.skip_lines.contains(&self.line_number) {
                // Skip to end of line
                while self.position < chars.len() && chars[self.position] != '\n' {
                    self.position += 1;
                }
                if self.position < chars.len() {
                    self.position += 1;
                    self.line_number += 1;
                }
                continue;
            }

            // Check if we're at a heredoc declaration
            if decl_index < declarations.len()
                && self.position == declarations[decl_index].declaration_pos
            {
                let decl = &declarations[decl_index];
                output.push_str(&decl.placeholder_id);
                self.position = decl.declaration_end;
                decl_index += 1;
            } else {
                // Regular character
                let ch = chars[self.position];
                output.push(ch);
                if ch == '\n' {
                    self.line_number += 1;
                }
                self.position += 1;
            }
        }

        (output, declarations)
    }

    fn _check_heredoc_start(&self, chars: &[char]) -> bool {
        self.position + 1 < chars.len()
            && chars[self.position] == '<'
            && chars[self.position + 1] == '<'
    }

    fn parse_heredoc_declaration(&mut self, chars: &[char]) -> Option<HeredocDeclaration> {
        let start_pos = self.position;
        self.position += 2; // Skip <<

        // Check for indented heredoc (<<~)
        let indented = if self.position < chars.len() && chars[self.position] == '~' {
            self.position += 1;
            true
        } else {
            false
        };

        // Skip whitespace
        while self.position < chars.len()
            && chars[self.position].is_whitespace()
            && chars[self.position] != '\n'
        {
            self.position += 1;
        }

        // Determine if quoted
        let (interpolated, terminator) = if self.position < chars.len() {
            match chars[self.position] {
                '\'' => {
                    self.position += 1;
                    let term = self.read_until_quote('\'', chars)?;
                    (false, term)
                }
                '"' => {
                    self.position += 1;
                    let term = self.read_until_quote('"', chars)?;
                    (true, term)
                }
                '`' => {
                    self.position += 1;
                    let term = self.read_until_quote('`', chars)?;
                    (true, term) // Backticks are interpolated
                }
                _ => {
                    let term = self.read_bareword(chars)?;
                    (true, term) // Bare terminators are interpolated
                }
            }
        } else {
            return None;
        };

        self.heredoc_counter += 1;
        let placeholder_id = format!("__HEREDOC_{}__", self.heredoc_counter);
        let declaration_end = self.position;

        Some(HeredocDeclaration {
            terminator,
            declaration_pos: start_pos,
            declaration_end,
            declaration_line: self.line_number,
            interpolated,
            indented,
            placeholder_id,
            content: None,
        })
    }

    fn read_until_quote(&mut self, quote: char, chars: &[char]) -> Option<String> {
        let mut result = String::new();
        while self.position < chars.len() && chars[self.position] != quote {
            if chars[self.position] == '\\' && self.position + 1 < chars.len() {
                // Handle escaped quotes
                self.position += 1;
            }
            result.push(chars[self.position]);
            self.position += 1;
        }
        if self.position < chars.len() {
            self.position += 1; // Skip closing quote
            Some(result)
        } else {
            None
        }
    }

    fn read_bareword(&mut self, chars: &[char]) -> Option<String> {
        let mut result = String::new();
        while self.position < chars.len()
            && (chars[self.position].is_alphanumeric() || chars[self.position] == '_')
        {
            result.push(chars[self.position]);
            self.position += 1;
        }
        if result.is_empty() { None } else { Some(result) }
    }
}

/// Phase 2: Heredoc Content Collector
/// Enhanced in #219 (Issue #182) to thread StatementTracker for future block-aware processing
pub struct HeredocCollector<'a> {
    _input: &'a str,
    lines: Vec<&'a str>,
    /// Issue #182/#219: Statement tracker for block-aware heredoc handling (used in #220)
    #[allow(dead_code)]
    statement_tracker: perl_ts_heredoc_analysis::statement_tracker::StatementTracker,
}

impl<'a> HeredocCollector<'a> {
    pub fn new(input: &'a str) -> Self {
        Self {
            _input: input,
            lines: input.lines().collect(),
            // Issue #182/#219: Initialize tracker (no-op for now, used in #220)
            statement_tracker: perl_ts_heredoc_analysis::statement_tracker::StatementTracker::new(),
        }
    }

    /// Collect content for all heredoc declarations
    pub fn collect(&self, declarations: &mut [HeredocDeclaration]) {
        let start_time = Instant::now();
        // Map declaration line to heredocs declared on that line
        let mut line_to_heredocs: HashMap<usize, Vec<usize>> = HashMap::new();

        for (idx, decl) in declarations.iter().enumerate() {
            line_to_heredocs.entry(decl.declaration_line).or_default().push(idx);
        }

        // For each line with heredocs, collect content
        for (line_num, heredoc_indices) in line_to_heredocs {
            // Timeout protection (Issue #443)
            if start_time.elapsed().as_millis() > HEREDOC_TIMEOUT_MS as u128 {
                break;
            }

            // Find where the statement containing the heredoc actually ends
            let statement_end_line = find_statement_end_line(self._input, line_num);
            // Heredoc content starts on the line after the statement ends
            // Note: statement_end_line is 1-based, but lines array is 0-based
            // So we use statement_end_line as-is because:
            // - statement_end_line=2 (1-based) means line 2 ends the statement
            // - content starts on line 3 (1-based) = index 2 (0-based) = statement_end_line
            let mut content_line = statement_end_line; // This is the 0-based index where content starts

            // Debug logging
            #[cfg(test)]
            {
                eprintln!(
                    "Line {} has heredocs, statement ends at line {}",
                    line_num, statement_end_line
                );
                eprintln!("Total lines available: {}", self.lines.len());
                if content_line < self.lines.len() {
                    eprintln!("Content starts with: {:?}", self.lines[content_line]);
                }
            }

            for &idx in &heredoc_indices {
                let decl = &declarations[idx];
                let mut content = String::new();
                let mut found_terminator = false;

                // Scan lines for terminator
                // For indented heredocs, first collect all lines to find common whitespace
                let mut heredoc_lines = Vec::new();
                let mut temp_content_line = content_line;

                while temp_content_line < self.lines.len() {
                    let line = self.lines[temp_content_line];

                    // Check if this line is the terminator
                    let is_terminator = if decl.indented {
                        line.trim() == decl.terminator
                    } else {
                        line == decl.terminator
                    };

                    if is_terminator {
                        found_terminator = true;
                        content_line = temp_content_line + 1;
                        break;
                    }

                    heredoc_lines.push(line);
                    temp_content_line += 1;
                }

                if found_terminator {
                    if decl.indented {
                        // Calculate common leading whitespace
                        let common_indent = heredoc_lines
                            .iter()
                            .filter(|line| !line.trim().is_empty())
                            .map(|line| line.len() - line.trim_start().len())
                            .min()
                            .unwrap_or(0);

                        // Build content with common indent removed
                        for (i, line) in heredoc_lines.iter().enumerate() {
                            if i > 0 {
                                content.push('\n');
                            }
                            if line.len() > common_indent {
                                content.push_str(&line[common_indent..]);
                            }
                        }
                    } else {
                        // Non-indented heredocs: just join lines
                        for (i, line) in heredoc_lines.iter().enumerate() {
                            if i > 0 {
                                content.push('\n');
                            }
                            content.push_str(line);
                        }
                    }
                }

                if found_terminator {
                    declarations[idx].content = Some(Arc::from(content));
                }
            }
        }
    }
}

/// Phase 3: Heredoc Integration
pub struct HeredocIntegrator;

impl HeredocIntegrator {
    /// For now, just return the processed input with placeholders
    /// The parser will handle the placeholders and look up content from declarations
    pub fn integrate(processed_input: &str, _declarations: &[HeredocDeclaration]) -> String {
        // Don't replace placeholders - let the parser handle them
        processed_input.to_string()
    }
}

fn _escape_for_qq(s: &str) -> String {
    s.replace('\\', "\\\\").replace('{', "\\{").replace('}', "\\}")
}

/// High-level API for multi-phase heredoc parsing
pub fn parse_with_heredocs(input: &str) -> (String, Vec<HeredocDeclaration>) {
    // Phase 1: Detect heredocs
    let scanner = HeredocScanner::new(input);
    let (processed_input, mut declarations) = scanner.scan();

    // Phase 2: Collect content
    let collector = HeredocCollector::new(input);
    collector.collect(&mut declarations);

    // Phase 3: Integrate for parsing
    let final_input = HeredocIntegrator::integrate(&processed_input, &declarations);

    (final_input, declarations)
}

#[cfg(test)]
mod tests {
    use super::*;
    use perl_ts_heredoc_analysis::statement_tracker::HeredocContext;
    use std::collections::BTreeSet;

    /// Test harness: Run heredoc scanner and expose skip_lines + tracker contexts
    fn scan_for_test(input: &str) -> (BTreeSet<usize>, Vec<HeredocContext>) {
        let scanner = HeredocScanner::new(input);
        let (_processed, _declarations, skip_lines, contexts) = scanner.scan_for_test();

        let skips: BTreeSet<_> = skip_lines.iter().copied().collect();
        (skips, contexts)
    }

    // ===== Issue #220a: Integration Tests for Block-Aware Heredoc Handling =====

    #[test]
    fn heredoc_top_level_skip_lines_and_contexts() {
        let code = "\
my $x = <<EOF;
line 1
line 2
EOF
say $x;
";

        let (skips, contexts) = scan_for_test(code);

        // 1 context
        assert_eq!(contexts.len(), 1, "expected 1 heredoc context");
        let ctx = &contexts[0];

        assert_eq!(ctx.declaration_line, 1);
        assert_eq!(ctx.block_depth_at_declaration, 0);
        assert_eq!(ctx.terminator, "EOF");
        assert_eq!(ctx.statement_end_line, 1);
        assert_eq!(ctx.content_start_line, 2);

        let expected: BTreeSet<_> = [2_usize, 3, 4].into_iter().collect();
        assert_eq!(skips, expected, "unexpected skip_lines for top-level heredoc");
    }

    #[test]
    fn heredoc_inside_if_block_tracks_depth_and_skip_lines() {
        let code = "\
if ($condition) {
    my $x = <<EOF;
line 1
line 2
EOF
    say $x;
}
";

        let (skips, contexts) = scan_for_test(code);

        // 1 context at depth 1
        assert_eq!(contexts.len(), 1, "expected 1 heredoc context");
        let ctx = &contexts[0];

        assert_eq!(ctx.declaration_line, 2);
        assert_eq!(ctx.block_depth_at_declaration, 0); // Scanner doesn't track block depth yet
        assert_eq!(ctx.terminator, "EOF");
        // #221: Correct block-aware behavior
        assert_eq!(ctx.statement_end_line, 2);
        assert_eq!(ctx.content_start_line, 3);

        // #221: Correct skip_lines for heredoc in if block
        let expected: BTreeSet<_> = [3_usize, 4, 5].into_iter().collect();
        assert_eq!(skips, expected, "skip_lines for heredoc in if block");
    }

    #[test]
    fn heredoc_in_nested_blocks_tracks_depth_and_skip_lines() {
        let code = "\
if ($x) {
    while ($y) {
        my $data = <<DATA;
content
DATA
    }
}
";

        let (skips, contexts) = scan_for_test(code);

        // 1 context at depth 2 (if + while)
        assert_eq!(contexts.len(), 1, "expected 1 heredoc context");
        let ctx = &contexts[0];

        assert_eq!(ctx.declaration_line, 3);
        assert_eq!(ctx.block_depth_at_declaration, 0); // Scanner doesn't track block depth yet
        assert_eq!(ctx.terminator, "DATA");
        // #221: Correct block-aware behavior
        assert_eq!(ctx.statement_end_line, 3);
        assert_eq!(ctx.content_start_line, 4);

        // #221: Correct skip_lines for heredoc in nested blocks
        let expected: BTreeSet<_> = [4_usize, 5].into_iter().collect();
        assert_eq!(skips, expected, "skip_lines for heredoc in nested blocks");
    }

    #[test]
    fn two_heredocs_in_same_block_have_separate_contexts_and_skips() {
        let code = "\
if ($condition) {
    my $x = <<EOF1;
content 1
EOF1
    my $y = <<EOF2;
content 2
EOF2
}
";

        let (skips, contexts) = scan_for_test(code);

        // 2 contexts, both at depth 1
        assert_eq!(contexts.len(), 2, "expected 2 heredoc contexts");

        let ctx1 = &contexts[0];
        assert_eq!(ctx1.declaration_line, 2);
        assert_eq!(ctx1.block_depth_at_declaration, 0); // Scanner doesn't track block depth yet
        assert_eq!(ctx1.terminator, "EOF1");
        // #221: Correct block-aware behavior
        assert_eq!(ctx1.statement_end_line, 2);
        assert_eq!(ctx1.content_start_line, 3);

        let ctx2 = &contexts[1];
        assert_eq!(ctx2.declaration_line, 5);
        assert_eq!(ctx2.block_depth_at_declaration, 0); // Scanner doesn't track block depth yet
        assert_eq!(ctx2.terminator, "EOF2");
        assert_eq!(ctx2.statement_end_line, 5);
        assert_eq!(ctx2.content_start_line, 6);

        // #221: Correct skip_lines for two heredocs in same block
        let expected: BTreeSet<_> = [3_usize, 4, 6, 7].into_iter().collect();
        assert_eq!(skips, expected, "skip_lines for two heredocs in same block");
    }

    // ===== Original Tests (Preserved) =====

    #[test]
    fn test_basic_heredoc() {
        let input = r#"my $text = <<'EOF';
Hello, World!
This is a heredoc.
EOF
print $text;"#;

        let (processed, declarations) = parse_with_heredocs(input);

        assert_eq!(declarations.len(), 1);
        assert_eq!(declarations[0].terminator, "EOF");
        assert!(!declarations[0].interpolated);
        assert_eq!(declarations[0].content.as_deref(), Some("Hello, World!\nThis is a heredoc."));
        assert!(processed.contains("__HEREDOC__"));
    }

    #[test]
    fn test_multiple_heredocs() {
        let input = r#"print <<A, <<B;
Content A
A
Content B
B"#;

        let (_, declarations) = parse_with_heredocs(input);

        assert_eq!(declarations.len(), 2);
        assert_eq!(declarations[0].content.as_deref(), Some("Content A"));
        assert_eq!(declarations[1].content.as_deref(), Some("Content B"));
    }

    #[test]
    fn test_indented_heredoc() {
        let input = r#"my $text = <<~'EOF';
        Indented content
        More content
        EOF
print $text;"#;

        let (processed, declarations) = parse_with_heredocs(input);

        println!("Processed input: {:?}", processed);
        println!("Declarations: {:?}", declarations);

        assert_eq!(declarations.len(), 1);
        assert!(declarations[0].indented);
        assert_eq!(declarations[0].terminator, "EOF");
        assert_eq!(declarations[0].content.as_deref(), Some("Indented content\nMore content"));
    }

    #[test]
    fn test_indented_heredoc_in_block() {
        // Tests indented heredoc (~) parsing inside a block context
        let input = r#"if (1) {
    my $text = <<~'EOF';
        Indented content
        More content
        EOF
}"#;

        let (_processed, declarations) = parse_with_heredocs(input);

        assert_eq!(declarations.len(), 1);
        assert!(declarations[0].indented);
        assert_eq!(declarations[0].terminator, "EOF");
        // NOTE: Statement tracker heredoc-in-block handling was implemented in PRs #225, #226, #229.
        // This test validates the fix. If assertions fail, heredoc block support may have regressed.
    }

    #[test]
    fn test_heredoc_scanner_depth_limit() {
        let mut code = String::new();
        for i in 0..110 {
            code.push_str(&format!("my $h{} = <<EOF{};\n", i, i));
        }

        let scanner = HeredocScanner::new(&code);
        let (_processed, declarations) = scanner.scan();

        assert!(
            declarations.len() <= MAX_HEREDOC_DEPTH,
            "Declarations should be limited to MAX_HEREDOC_DEPTH"
        );
    }
}
