# Heredoc Declaration Domain Schema

**Version**: 1.0.0
**Date**: 2025-11-05
**Related**: [SPEC-183](SPEC_183_HEREDOC_DECLARATION_PARSER.md), [ADR-005](adr/ADR_005_HEREDOC_MANUAL_PARSING.md)
**Status**: Draft

---

## Overview

This document defines the comprehensive domain model for heredoc declaration parsing in the perl-parser crate. It describes the data structures, their relationships, invariants, and how they flow through the LSP workflow pipeline (Parse → Index → Navigate → Complete → Analyze).

**Target Audience**: Implementers, reviewers, and future maintainers of the heredoc parsing infrastructure.

---

## 1. Core Domain Entities

### 1.1 HeredocDeclaration

**Purpose**: Represents a parsed heredoc declaration with all metadata required for content matching and LSP integration.

**Definition**:
```rust
/// Enhanced heredoc declaration with manual parsing metadata
///
/// This structure captures all information about a heredoc declaration encountered
/// during parsing, including the terminator, quote style, position information,
/// and processing flags. It serves as the canonical representation throughout
/// the LSP workflow pipeline.
///
/// # LSP Workflow Role
///
/// - **Parse**: Generated from `<<` operator during tokenization
/// - **Index**: Terminator stored in symbol table for reference resolution
/// - **Navigate**: Position information enables go-to-definition for heredoc delimiters
/// - **Complete**: Future support for terminator autocomplete
/// - **Analyze**: Validates terminator matching and reports mismatches
///
/// # Invariants
///
/// - `terminator` is never empty (validated during parsing)
/// - `declaration_end >= declaration_pos` (position range is valid)
/// - `declaration_length = declaration_end - declaration_pos` (derived field consistency)
/// - `interpolated = quote_style.is_interpolated()` (derived from quote style)
/// - If `indented`, then terminator matching uses `trim()` (semantic constraint)
///
/// # Example
///
/// ```rust
/// use perl_parser::heredoc_declaration_parser::{HeredocDeclaration, HeredocQuoteStyle};
///
/// let declaration = HeredocDeclaration {
///     terminator: "EOF\n".to_string(),
///     raw_terminator: r#""EOF\n""#.to_string(),
///     quote_style: HeredocQuoteStyle::DoubleQuote,
///     declaration_pos: 0,
///     declaration_end: 8,
///     declaration_length: 8,
///     declaration_line: 1,
///     interpolated: true,
///     indented: false,
///     placeholder_id: "__HEREDOC_1__".to_string(),
///     content: None,
/// };
/// ```
#[non_exhaustive]
#[derive(Debug, Clone, PartialEq)]
pub struct HeredocDeclaration {
    /// The terminator string after quote processing and escape handling
    ///
    /// **Processing Rules**:
    /// - Bare: Identifier as-is (e.g., `<<EOF` → `"EOF"`)
    /// - Double-quote: Escape sequences processed (e.g., `<<"EOF\n"` → `"EOF\n"`)
    /// - Single-quote: Literal except `\'` and `\\` (e.g., `<<'EOF\n'` → `"EOF\\n"`)
    /// - Backtick: Same as double-quote (e.g., `<<`EOF\n`` → `"EOF\n"`)
    ///
    /// **Examples**:
    /// - `<<EOF` → `"EOF"`
    /// - `<<"END_DATA"` → `"END_DATA"`
    /// - `<<"MARKER\n"` → `"MARKER\n"` (newline character)
    /// - `<<'MARKER\n'` → `"MARKER\\n"` (literal backslash-n)
    ///
    /// **Invariant**: Must not be empty (validated in parser)
    pub terminator: String,

    /// Raw terminator as it appeared in source code (for diagnostics)
    ///
    /// Preserves the exact source representation including quotes and escape
    /// sequences. Used for error messages and source reconstruction.
    ///
    /// **Examples**:
    /// - `<<EOF` → `"EOF"`
    /// - `<<"END_DATA"` → `"\"END_DATA\""`
    /// - `<<'MARKER\n'` → `"'MARKER\\n'"` (literal backslash-n in quotes)
    pub raw_terminator: String,

    /// Quote style used for the terminator
    ///
    /// Determines interpolation behavior and escape sequence processing rules.
    /// See [`HeredocQuoteStyle`] for detailed semantics.
    pub quote_style: HeredocQuoteStyle,

    /// Position in input where the heredoc was declared (start of `<<`)
    ///
    /// **Byte Offset**: Zero-indexed byte position in UTF-8 source string
    /// **Usage**: Error reporting, AST node location, LSP position mapping
    pub declaration_pos: usize,

    /// Position where the declaration ends (after terminator)
    ///
    /// **Byte Offset**: Zero-indexed byte position immediately after terminator
    /// **Usage**: Token stream advancement, AST node location
    pub declaration_end: usize,

    /// Length of the declaration in bytes (for token stream advancement)
    ///
    /// **Derived Field**: `declaration_length = declaration_end - declaration_pos`
    /// **Invariant**: Must equal `raw_terminator.len() + overhead`
    /// **Usage**: Advancing parser position after successful parse
    pub declaration_length: usize,

    /// Line number of declaration (1-indexed)
    ///
    /// **Line Counting**: Uses newline characters (`\n`) as delimiters
    /// **Usage**: Error reporting, AST node location, LSP position mapping
    pub declaration_line: usize,

    /// Whether the heredoc is interpolated (based on quote style)
    ///
    /// **Derived Field**: `interpolated = quote_style.is_interpolated()`
    ///
    /// **Interpolation Rules**:
    /// - `true`: Variables (`$var`, `@array`, `%hash`) expanded during content parsing
    /// - `false`: Content treated literally, no variable expansion
    ///
    /// **Quote Style Mapping**:
    /// - `Bare`: `true` (interpolated)
    /// - `DoubleQuote`: `true` (interpolated)
    /// - `SingleQuote`: `false` (literal)
    /// - `Backtick`: `true` (interpolated, command substitution)
    pub interpolated: bool,

    /// Whether the heredoc is indented (`<<~` operator)
    ///
    /// **Indentation Stripping Rules** (Perl 5.26+):
    /// - Find minimum leading whitespace across all content lines
    /// - Remove that amount of whitespace from each line
    /// - Terminator line also matched with `trim()` instead of exact match
    ///
    /// **Example**:
    /// ```perl
    /// my $text = <<~EOF;
    ///     This is indented
    ///     by 4 spaces
    ///     EOF
    /// # Result: "This is indented\nby 4 spaces\n"
    /// ```
    pub indented: bool,

    /// Unique placeholder token for this heredoc (e.g., `"__HEREDOC_1__"`)
    ///
    /// **Generation**: Monotonically increasing counter per parse session
    /// **Usage**: Phase 1 scanner replaces declaration with placeholder
    /// **Format**: `"__HEREDOC_{counter}__"` where counter is unique per file
    /// **Lifetime**: Valid only during single parse session, not persisted
    pub placeholder_id: String,

    /// The collected content (filled in Phase 2 of heredoc processing)
    ///
    /// **Phase 1 (Declaration)**: `None` (content not yet collected)
    /// **Phase 2 (Collection)**: `Some(Arc<str>)` with heredoc body
    /// **Phase 3 (Integration)**: Shared reference for efficient AST storage
    ///
    /// **Arc Rationale**: Heredoc content can be large (>1MB), sharing avoids clones
    pub content: Option<Arc<str>>,
}
```

**Relationships**:
- Contains: `HeredocQuoteStyle` (enum)
- Used by: `HeredocDeclarationParser` (produces)
- Used by: `HeredocScanner` (Phase 1)
- Used by: `HeredocCollector` (Phase 2, populates `content`)
- Used by: `Node::HeredocDeclaration` (AST integration)

**Lifecycle**:
```
[Parse] HeredocDeclarationParser::parse()
    ↓
[Phase 1] HeredocDeclaration { content: None }
    ↓
[Phase 2] HeredocCollector::collect()
    ↓
[Phase 2] HeredocDeclaration { content: Some(...) }
    ↓
[Phase 3] Node::HeredocDeclaration(declaration)
    ↓
[Index] Symbol table entry for terminator
```

---

### 1.2 HeredocQuoteStyle

**Purpose**: Enumeration of quote styles for heredoc terminators, determining interpolation and escape processing behavior.

**Definition**:
```rust
/// Quote style for heredoc terminators
///
/// Determines how the heredoc content is processed:
/// - Interpolation: Whether variables like `$var` are expanded
/// - Escape processing: Whether sequences like `\n` are converted to actual characters
///
/// # Perl Reference Behavior
///
/// ```perl
/// # Bare (interpolated)
/// <<EOF
/// Hello, $world!
/// EOF
///
/// # Double-quoted (interpolated, escapes processed)
/// <<"EOF"
/// Line 1\nLine 2
/// EOF
///
/// # Single-quoted (literal, no interpolation)
/// <<'EOF'
/// Literal $variable and \n
/// EOF
///
/// # Backtick-quoted (command substitution, interpolated)
/// <<`EOF`
/// echo "Hello, $world!"
/// EOF
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HeredocQuoteStyle {
    /// Bare terminator: `<<EOF`
    ///
    /// **Interpolation**: Yes (variables expanded)
    /// **Escape Processing**: No (terminator treated as identifier)
    /// **Content Processing**: Same as double-quoted heredoc
    ///
    /// **Example**:
    /// ```perl
    /// my $name = "World";
    /// my $text = <<EOF;
    /// Hello, $name!
    /// EOF
    /// # Result: "Hello, World!\n"
    /// ```
    Bare,

    /// Double-quoted terminator: `<<"EOF"`
    ///
    /// **Interpolation**: Yes (variables expanded)
    /// **Escape Processing**: Yes in terminator (`\n`, `\t`, `\"`, `\\`, `\$`, `\@`)
    /// **Content Processing**: Full interpolation and escape processing
    ///
    /// **Escape Sequences**:
    /// - `\n` → newline (U+000A)
    /// - `\t` → tab (U+0009)
    /// - `\r` → carriage return (U+000D)
    /// - `\"` → double-quote (U+0022)
    /// - `\\` → backslash (U+005C)
    /// - `\$` → dollar sign (U+0024, prevents interpolation)
    /// - `\@` → at sign (U+0040, prevents interpolation)
    ///
    /// **Example**:
    /// ```perl
    /// my $marker = <<"END\nMARK";  # Terminator is "END\nMARK" (two lines)
    /// content
    /// END
    /// MARK
    /// ```
    DoubleQuote,

    /// Single-quoted terminator: `<<'EOF'`
    ///
    /// **Interpolation**: No (literal content)
    /// **Escape Processing**: Limited (`\'` → `'`, `\\` → `\`, others literal)
    /// **Content Processing**: No interpolation, most escapes literal
    ///
    /// **Escape Sequences** (limited):
    /// - `\'` → single-quote (U+0027)
    /// - `\\` → backslash (U+005C)
    /// - `\n` → literal backslash + n (not newline)
    /// - `\t` → literal backslash + t (not tab)
    ///
    /// **Example**:
    /// ```perl
    /// my $text = <<'EOF';
    /// Literal $variable and \n
    /// EOF
    /// # Result: "Literal $variable and \\n\n" (backslash-n literal)
    /// ```
    SingleQuote,

    /// Backtick-quoted terminator: `<<`EOF``
    ///
    /// **Interpolation**: Yes (variables expanded)
    /// **Escape Processing**: Yes (same as double-quoted)
    /// **Content Processing**: Executed as shell command after interpolation
    ///
    /// **Special Behavior**: Content is passed to shell for execution
    ///
    /// **Example**:
    /// ```perl
    /// my $dir = "/tmp";
    /// my $output = <<`CMD`;
    /// ls -la $dir
    /// CMD
    /// # Result: Shell output of `ls -la /tmp`
    /// ```
    ///
    /// **Security Note**: Command substitution heredocs require careful input validation
    Backtick,
}

impl HeredocQuoteStyle {
    /// Returns whether this quote style supports interpolation
    ///
    /// **Interpolation**: Variable expansion (`$var`, `@array`, `%hash`)
    ///
    /// # Returns
    ///
    /// - `true`: Bare, DoubleQuote, Backtick
    /// - `false`: SingleQuote
    pub fn is_interpolated(&self) -> bool {
        matches!(self, Self::Bare | Self::DoubleQuote | Self::Backtick)
    }

    /// Returns whether this quote style processes escape sequences in the terminator
    ///
    /// **Note**: This applies only to the terminator label, not the content.
    /// Content escape processing is determined separately during Phase 2.
    ///
    /// # Returns
    ///
    /// - `true`: DoubleQuote, Backtick
    /// - `false`: Bare, SingleQuote (limited escapes)
    pub fn processes_escapes(&self) -> bool {
        matches!(self, Self::DoubleQuote | Self::Backtick)
    }

    /// Returns the quote character for this style, if any
    ///
    /// # Returns
    ///
    /// - `None`: Bare (no quotes)
    /// - `Some('"')`: DoubleQuote
    /// - `Some('\'')`: SingleQuote
    /// - ``Some('`')``: Backtick
    pub fn quote_char(&self) -> Option<char> {
        match self {
            Self::Bare => None,
            Self::DoubleQuote => Some('"'),
            Self::SingleQuote => Some('\''),
            Self::Backtick => Some('`'),
        }
    }

    /// Returns a human-readable description for error messages
    pub fn description(&self) -> &'static str {
        match self {
            Self::Bare => "bare (unquoted) heredoc",
            Self::DoubleQuote => "double-quoted heredoc",
            Self::SingleQuote => "single-quoted heredoc",
            Self::Backtick => "backtick-quoted heredoc (command substitution)",
        }
    }
}
```

**Relationships**:
- Contained by: `HeredocDeclaration.quote_style`
- Determined by: `HeredocDeclarationParser` during state machine execution

**State Machine Mapping**:
```
DetectQuoteStyle state
    ↓
    ├─ peek('"') → Self::DoubleQuote
    ├─ peek('\'') → Self::SingleQuote
    ├─ peek('`') → Self::Backtick
    └─ peek(alphanumeric) → Self::Bare
```

---

### 1.3 HeredocParseState

**Purpose**: State machine states for heredoc declaration parsing.

**Definition**:
```rust
/// States for heredoc declaration parsing state machine
///
/// The parser operates as a deterministic finite automaton (DFA), transitioning
/// between states based on input characters. Each state has specific responsibilities
/// and valid transitions.
///
/// # State Machine Overview
///
/// ```text
/// Start
///   ↓ '<'
/// FirstAngle
///   ↓ '<'
/// CheckIndent
///   ↓ '~' or whitespace
/// PreTerminatorWhitespace
///   ↓ quote char or alphanumeric
/// DetectQuoteStyle
///   ↓ (split based on quote)
///   ├─ Bare → ReadingBareTerminator → Complete
///   └─ Quoted → ReadingQuotedTerminator ⇄ EscapeSequence → Complete
///                                          (on '\' in double/backtick)
/// ```
///
/// # Error Handling
///
/// Any invalid input transitions to `Error(HeredocParseError)` state.
/// Once in Error state, parser halts and returns the error.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HeredocParseState {
    /// Initial state: expecting '<<' token
    ///
    /// **Valid Transitions**:
    /// - '<' → `FirstAngle`
    /// - Other → `Error(ExpectedHeredocOperator)`
    Start,

    /// After first '<', waiting for second '<'
    ///
    /// **Valid Transitions**:
    /// - '<' → `CheckIndent`
    /// - Other → `Error(ExpectedHeredocOperator)`
    FirstAngle,

    /// After '<<', checking for indented heredoc '~'
    ///
    /// **Valid Transitions**:
    /// - '~' → `PreTerminatorWhitespace` (set `indented = true`)
    /// - Whitespace → `PreTerminatorWhitespace`
    /// - Other → `DetectQuoteStyle`
    CheckIndent,

    /// Reading optional whitespace before terminator
    ///
    /// **Valid Transitions**:
    /// - Whitespace → `PreTerminatorWhitespace` (loop)
    /// - Other → `DetectQuoteStyle`
    PreTerminatorWhitespace,

    /// Determining quote style (bare, ", ', `)
    ///
    /// **Valid Transitions**:
    /// - '"' → `ReadingQuotedTerminator` (DoubleQuote)
    /// - '\'' → `ReadingQuotedTerminator` (SingleQuote)
    /// - '`' → `ReadingQuotedTerminator` (Backtick)
    /// - Alphanumeric/`_` → `ReadingBareTerminator`
    /// - Other → `Error(InvalidBareTerminatorChar)`
    DetectQuoteStyle,

    /// Reading bare (unquoted) terminator
    ///
    /// **Valid Transitions**:
    /// - Alphanumeric/`_` → `ReadingBareTerminator` (append to buffer)
    /// - Whitespace/newline → `Complete`
    /// - Other → `Error(InvalidBareTerminatorChar)`
    ///
    /// **Termination**: First non-identifier character
    ReadingBareTerminator,

    /// Reading quoted terminator content
    ///
    /// **Valid Transitions**:
    /// - Matching quote → `Complete`
    /// - '\' (in DoubleQuote/Backtick) → `EscapeSequence`
    /// - '\' (in SingleQuote) → Limited escape handling
    /// - Other → `ReadingQuotedTerminator` (append to buffer)
    /// - EOF → `Error(UnterminatedQuotedLabel)`
    ///
    /// **Termination**: Matching closing quote
    ReadingQuotedTerminator,

    /// Inside escape sequence within quoted terminator
    ///
    /// **Valid Transitions** (DoubleQuote/Backtick):
    /// - 'n', 't', 'r', '"', '\\', '`', '$', '@' → `ReadingQuotedTerminator`
    /// - Other → `Error(InvalidEscapeSequence)`
    /// - EOF → `Error(UnterminatedQuotedLabel)`
    ///
    /// **Processing**: Convert escape to actual character, append to `terminator_buffer`
    EscapeSequence,

    /// Completed successfully
    ///
    /// **Terminal State**: No further transitions
    /// **Action**: Return `Ok(HeredocDeclaration)`
    Complete,

    /// Error state with diagnostic information
    ///
    /// **Terminal State**: No further transitions
    /// **Action**: Return `Err(HeredocParseError)`
    Error(HeredocParseError),
}
```

**Relationships**:
- Maintained by: `HeredocDeclarationParser.state`
- Transitions driven by: `HeredocDeclarationParser::step()`

**State Invariants**:
- Parser can only be in one state at a time
- Terminal states (`Complete`, `Error`) have no outgoing transitions
- Every state has defined transitions for all possible inputs
- State machine is deterministic (no ambiguous transitions)

---

### 1.4 HeredocDeclarationParser

**Purpose**: State machine parser for heredoc declarations.

**Definition**:
```rust
/// Context maintained during heredoc declaration parsing
///
/// This structure encapsulates the complete parsing state, including input buffer,
/// current position, accumulated terminator, and state machine state. It provides
/// a clean API for parsing heredoc declarations with comprehensive error handling.
///
/// # Usage Pattern
///
/// ```rust
/// use perl_parser::heredoc_declaration_parser::HeredocDeclarationParser;
///
/// let input = r#"<<"EOF\n""#;
/// let mut parser = HeredocDeclarationParser::new(input, 0);
///
/// match parser.parse() {
///     Ok(declaration) => {
///         println!("Parsed terminator: {}", declaration.terminator);
///         println!("Quote style: {:?}", declaration.quote_style);
///     }
///     Err(e) => {
///         eprintln!("Parse error: {}", e);
///     }
/// }
/// ```
///
/// # Memory Safety
///
/// - Uses `Vec<char>` for safe UTF-8 character iteration (no byte slicing panics)
/// - Bounds checking on every position access via `peek_char()` and `advance()`
/// - String buffers pre-allocated to minimize allocations during parsing
#[derive(Debug)]
pub struct HeredocDeclarationParser<'a> {
    /// Input string being parsed
    ///
    /// **Lifetime**: Borrowed for the duration of parsing (no ownership transfer)
    /// **Encoding**: UTF-8 (Rust default)
    input: &'a str,

    /// Character buffer for efficient random access
    ///
    /// **Rationale**: Converts UTF-8 bytes to Unicode code points for safe iteration
    /// **Memory**: 4 bytes per character (worst case for non-ASCII)
    /// **Trade-off**: Memory overhead vs. UTF-8 boundary safety
    chars: Vec<char>,

    /// Current position in character buffer
    ///
    /// **Indexing**: Zero-based index into `chars` array
    /// **Bounds**: Always `≤ chars.len()`
    /// **Advancement**: Via `advance()` method (checked bounds)
    position: usize,

    /// Starting position (for relative offset calculations)
    ///
    /// **Usage**: Calculate `declaration_length` and absolute positions
    /// **Typically**: 0 for standalone parsing, non-zero when parsing within larger file
    start_position: usize,

    /// Current state machine state
    ///
    /// **Transitions**: Via `step()` method based on input characters
    /// **Terminal States**: `Complete`, `Error`
    state: HeredocParseState,

    /// Accumulated terminator string (after escape processing)
    ///
    /// **Contents**: Processed terminator ready for content matching
    /// **Escapes**: Already converted (e.g., `\n` → newline character)
    /// **Growth**: Appended character-by-character during parsing
    terminator_buffer: String,

    /// Raw terminator for diagnostics (before escape processing)
    ///
    /// **Contents**: Exact source representation including quotes and escapes
    /// **Usage**: Error messages, source reconstruction, debugging
    /// **Example**: `"\"EOF\\n\""` for input `<<"EOF\n"`
    raw_terminator_buffer: String,

    /// Detected quote style
    ///
    /// **Initialization**: `None` until `DetectQuoteStyle` state
    /// **Finalization**: Set during `DetectQuoteStyle` → `Reading*` transition
    quote_style: Option<HeredocQuoteStyle>,

    /// Whether <<~ operator was used
    ///
    /// **Detection**: Set in `CheckIndent` state on '~' character
    /// **Usage**: Affects terminator matching (trim vs. exact) in Phase 2
    indented: bool,

    /// Error information if parsing fails
    ///
    /// **Initialization**: `None` for successful parsing
    /// **Finalization**: Set when transitioning to `Error` state
    /// **Contents**: Detailed error with position and context
    error: Option<HeredocParseError>,
}

impl<'a> HeredocDeclarationParser<'a> {
    /// Create new parser for heredoc declaration
    ///
    /// # Arguments
    ///
    /// * `input` - Source string containing heredoc declaration
    /// * `start_position` - Byte offset in larger file (0 for standalone parsing)
    ///
    /// # Examples
    ///
    /// ```rust
    /// // Standalone parsing
    /// let parser = HeredocDeclarationParser::new("<<EOF", 0);
    ///
    /// // Parsing within larger file at position 42
    /// let parser = HeredocDeclarationParser::new(&file_content[42..], 42);
    /// ```
    pub fn new(input: &'a str, start_position: usize) -> Self {
        Self {
            input,
            chars: input.chars().collect(),
            position: 0,
            start_position,
            state: HeredocParseState::Start,
            terminator_buffer: String::with_capacity(32),
            raw_terminator_buffer: String::with_capacity(32),
            quote_style: None,
            indented: false,
            error: None,
        }
    }

    /// Main parsing entry point
    ///
    /// Runs the state machine to completion, returning either a successful
    /// `HeredocDeclaration` or an error with diagnostic information.
    ///
    /// # Returns
    ///
    /// - `Ok(HeredocDeclaration)`: Successfully parsed heredoc declaration
    /// - `Err(HeredocParseError)`: Parse failure with position and context
    ///
    /// # State Machine Execution
    ///
    /// 1. Loop while `state != Complete`
    /// 2. Each iteration calls `step()` to transition states
    /// 3. Terminal states (`Complete`, `Error`) exit the loop
    /// 4. Build final `HeredocDeclaration` from accumulated state
    ///
    /// # Performance
    ///
    /// - **Best Case**: 5-7 state transitions for bare terminators (e.g., `<<EOF`)
    /// - **Typical**: 10-15 transitions for quoted terminators (e.g., `<<"EOF"`)
    /// - **Worst Case**: 50+ transitions for long terminators with escapes
    ///
    /// # Examples
    ///
    /// ```rust
    /// let mut parser = HeredocDeclarationParser::new(r#"<<"EOF\n""#, 0);
    /// let declaration = parser.parse().unwrap();
    ///
    /// assert_eq!(declaration.terminator, "EOF\n");
    /// assert_eq!(declaration.quote_style, HeredocQuoteStyle::DoubleQuote);
    /// assert!(declaration.interpolated);
    /// assert!(!declaration.indented);
    /// ```
    pub fn parse(&mut self) -> Result<HeredocDeclaration, HeredocParseError> {
        // State machine loop
        while self.state != HeredocParseState::Complete {
            if let HeredocParseState::Error(err) = self.state {
                return Err(err);
            }

            self.step()?;
        }

        // Construct final declaration
        Ok(self.build_declaration())
    }

    /// Execute one step of the state machine
    ///
    /// **Internal Method**: Called by `parse()` in a loop
    ///
    /// # State Handler Dispatch
    ///
    /// Matches current state and delegates to appropriate handler method.
    /// Each handler is responsible for:
    /// 1. Validating input at current position
    /// 2. Updating parser state (buffers, flags, position)
    /// 3. Transitioning to next state or returning error
    fn step(&mut self) -> Result<(), HeredocParseError> {
        match self.state {
            HeredocParseState::Start => self.handle_start(),
            HeredocParseState::FirstAngle => self.handle_first_angle(),
            HeredocParseState::CheckIndent => self.handle_check_indent(),
            HeredocParseState::PreTerminatorWhitespace => self.handle_pre_terminator_whitespace(),
            HeredocParseState::DetectQuoteStyle => self.handle_detect_quote_style(),
            HeredocParseState::ReadingBareTerminator => self.handle_reading_bare_terminator(),
            HeredocParseState::ReadingQuotedTerminator => self.handle_reading_quoted_terminator(),
            HeredocParseState::EscapeSequence => self.handle_escape_sequence(),
            HeredocParseState::Complete => Ok(()),
            HeredocParseState::Error(err) => Err(err),
        }
    }

    // ... State handler methods (see SPEC-183 Section 3.4)

    /// Build final HeredocDeclaration from parser state
    ///
    /// **Internal Method**: Called by `parse()` after reaching `Complete` state
    ///
    /// # Invariants Enforced
    ///
    /// - `terminator` not empty (validation in state handlers)
    /// - `declaration_end >= declaration_pos`
    /// - `declaration_length` correctly calculated
    /// - `interpolated` derived from `quote_style`
    ///
    /// # Returns
    ///
    /// Complete `HeredocDeclaration` ready for Phase 2 content collection
    fn build_declaration(&self) -> HeredocDeclaration {
        HeredocDeclaration {
            terminator: self.terminator_buffer.clone(),
            raw_terminator: self.raw_terminator_buffer.clone(),
            quote_style: self.quote_style.expect("Quote style must be set"),
            declaration_pos: self.start_position,
            declaration_end: self.start_position + self.position,
            declaration_length: self.position,
            declaration_line: 1, // Calculated by caller if needed
            interpolated: self.quote_style.expect("Quote style must be set").is_interpolated(),
            indented: self.indented,
            placeholder_id: String::new(), // Set by HeredocScanner
            content: None, // Filled in Phase 2
        }
    }

    /// Peek at current character without advancing
    ///
    /// **Bounds Safety**: Returns `None` if position out of bounds
    ///
    /// # Returns
    ///
    /// - `Some(char)`: Character at current position
    /// - `None`: End of input reached
    fn peek_char(&self) -> Option<char> {
        if self.position < self.chars.len() {
            Some(self.chars[self.position])
        } else {
            None
        }
    }

    /// Advance position by one character
    ///
    /// **Bounds Safety**: Saturates at `chars.len()` (no overflow)
    fn advance(&mut self) {
        if self.position < self.chars.len() {
            self.position += 1;
        }
    }
}
```

**Relationships**:
- Produces: `HeredocDeclaration`
- Contains: `HeredocParseState` (enum)
- Throws: `HeredocParseError` (enum)
- Used by: `HeredocScanner::parse_heredoc_declaration()`
- Used by: `Parser::parse_heredoc_declaration()`

**Lifecycle**:
```
new() → Start state
    ↓
parse() → State machine loop
    ↓
step() × N → State transitions
    ↓
Complete state → build_declaration()
    ↓
Return Ok(HeredocDeclaration)
```

---

### 1.5 HeredocParseError

**Purpose**: Error types for heredoc declaration parsing failures.

**Definition**:
```rust
/// Errors that can occur during heredoc declaration parsing
///
/// Each error variant includes contextual information (position, expected input)
/// to enable actionable error messages for users.
///
/// # Error Handling Strategy
///
/// Following the [Error Handling Strategy Guide](ERROR_HANDLING_STRATEGY.md):
/// - **Fail Fast**: Return error immediately on invalid input
/// - **Actionable Messages**: Include position, expected input, and suggestions
/// - **Defensive Parsing**: Validate bounds before every array access
/// - **Graceful Degradation**: Parser fallback to treating `<<` as left-shift
///
/// # Usage in LSP Context
///
/// Errors are converted to LSP diagnostics with:
/// - Position: Line/column from byte offset
/// - Severity: Error (invalid Perl syntax)
/// - Message: Error description with suggestion
/// - Quick fix: Suggested correction (when applicable)
#[derive(Debug, Clone, thiserror::Error, PartialEq, Eq)]
pub enum HeredocParseError {
    /// Expected '<<' at position, found different input
    ///
    /// **Context**: Parser initialized at non-heredoc position
    ///
    /// **Example**:
    /// ```text
    /// Input: "< foo"
    ///         ^
    /// Error: Expected '<<' at position 0, found '<'
    /// ```
    ///
    /// **Suggestion**: "Heredoc requires '<<' operator"
    #[error("Expected '<<' at position {position}, found '{found}'")]
    ExpectedHeredocOperator {
        position: usize,
        found: String,
    },

    /// Unterminated quoted heredoc label (missing closing quote)
    ///
    /// **Context**: Reached EOF while reading quoted terminator
    ///
    /// **Example**:
    /// ```text
    /// Input: <<"EOF
    ///            ^
    /// Error: Unterminated quoted heredoc label at position 3
    /// ```
    ///
    /// **Suggestion**: "Add closing quote (\") to complete the heredoc terminator"
    #[error("Unterminated quoted heredoc label at position {position}")]
    UnterminatedQuotedLabel {
        position: usize,
        quote_char: char,
    },

    /// Invalid escape sequence in heredoc label
    ///
    /// **Context**: Unsupported escape in double-quoted or backtick terminator
    ///
    /// **Example**:
    /// ```text
    /// Input: <<"EOF\q"
    ///              ^
    /// Error: Invalid escape sequence '\q' in heredoc label at position 6
    /// ```
    ///
    /// **Valid Escapes**: \n, \t, \r, \\, \", \`, \$, \@
    ///
    /// **Suggestion**: "Use \\q for literal backslash-q"
    #[error("Invalid escape sequence '\\{escape}' in heredoc label at position {position}")]
    InvalidEscapeSequence {
        position: usize,
        escape: char,
    },

    /// Empty heredoc terminator
    ///
    /// **Context**: Quoted terminator with no content (`<<""`)
    ///
    /// **Example**:
    /// ```text
    /// Input: <<""
    ///           ^
    /// Error: Empty heredoc terminator at position 3
    /// ```
    ///
    /// **Suggestion**: "Heredoc terminator must contain at least one character"
    #[error("Empty heredoc terminator at position {position}")]
    EmptyTerminator {
        position: usize,
    },

    /// Invalid character in bare heredoc terminator
    ///
    /// **Context**: Bare terminator contains non-identifier character
    ///
    /// **Example**:
    /// ```text
    /// Input: <<EO-F
    ///           ^
    /// Error: Invalid character '-' in bare heredoc terminator at position 4
    /// ```
    ///
    /// **Valid Characters**: [a-zA-Z0-9_]
    ///
    /// **Suggestion**: "Use quoted terminator for special characters: <<\"EO-F\""
    #[error("Invalid character '{ch}' in bare heredoc terminator at position {position}")]
    InvalidBareTerminatorChar {
        position: usize,
        ch: char,
    },

    /// Heredoc declaration exceeds maximum length
    ///
    /// **Context**: Terminator longer than `MAX_HEREDOC_TERMINATOR_LENGTH` (256)
    ///
    /// **Example**:
    /// ```text
    /// Input: <<A...256 characters...Z
    ///                               ^
    /// Error: Heredoc declaration exceeds maximum length 256 at position 258
    /// ```
    ///
    /// **Rationale**: Prevent DoS attacks with extremely long terminators
    ///
    /// **Suggestion**: "Shorten heredoc terminator to 256 characters or less"
    #[error("Heredoc declaration exceeds maximum length {max_length} at position {position}")]
    TerminatorTooLong {
        position: usize,
        max_length: usize,
    },
}

impl HeredocParseError {
    /// Convert error to actionable user-facing message with suggestions
    ///
    /// **Usage**: LSP diagnostic generation, error reporting
    ///
    /// # Returns
    ///
    /// Tuple of (error message, suggestion)
    pub fn to_diagnostic(&self) -> (String, String) {
        match self {
            Self::ExpectedHeredocOperator { position, found } => (
                format!("Expected '<<' at position {}, found '{}'", position, found),
                "Heredoc requires '<<' operator".to_string(),
            ),
            Self::UnterminatedQuotedLabel { position, quote_char } => (
                format!("Unterminated quoted heredoc label at position {}", position),
                format!("Add closing quote ({}) to complete the heredoc terminator", quote_char),
            ),
            Self::InvalidEscapeSequence { position, escape } => (
                format!("Invalid escape sequence '\\{}' at position {}", escape, position),
                format!("Valid escape sequences: \\n, \\t, \\r, \\\\, \\\", \\`, \\$, \\@. Use \\\\{} for literal backslash-{}", escape, escape),
            ),
            Self::EmptyTerminator { position } => (
                format!("Empty heredoc terminator at position {}", position),
                "Heredoc terminator must contain at least one character".to_string(),
            ),
            Self::InvalidBareTerminatorChar { position, ch } => (
                format!("Invalid character '{}' in bare heredoc terminator at position {}", ch, position),
                format!("Use quoted terminator for special characters: <<\"{}\"", ch),
            ),
            Self::TerminatorTooLong { position, max_length } => (
                format!("Heredoc declaration exceeds maximum length {} at position {}", max_length, position),
                format!("Shorten heredoc terminator to {} characters or less", max_length),
            ),
        }
    }
}
```

**Relationships**:
- Produced by: `HeredocDeclarationParser` (on error)
- Contained in: `HeredocParseState::Error`
- Converted to: LSP `Diagnostic` (in LSP integration layer)

**Error Severity Mapping**:
```
HeredocParseError → LSP DiagnosticSeverity::Error
```

---

## 2. Helper Functions

### 2.1 normalize_crlf

**Purpose**: Normalize line endings for cross-platform consistency.

**Definition**:
```rust
/// Normalize CRLF (`\r\n`) and CR (`\r`) to LF (`\n`) for cross-platform consistency
///
/// Perl heredoc terminators must match exactly after line ending normalization.
/// This function ensures consistent behavior regardless of repository line ending
/// configuration (Windows/Unix/Mac).
///
/// # Algorithm
///
/// Two-pass normalization:
/// 1. Replace all `\r\n` (CRLF) with `\n` (LF)
/// 2. Replace standalone `\r` (CR) with `\n` (LF)
///
/// # Examples
///
/// ```rust
/// assert_eq!(normalize_crlf("EOF\r\n"), "EOF\n");  // Windows
/// assert_eq!(normalize_crlf("EOF\n"), "EOF\n");    // Unix
/// assert_eq!(normalize_crlf("EOF\r"), "EOF\n");    // Mac Classic
/// ```
///
/// # Performance
///
/// - **Allocation**: Single `String` allocation
/// - **Latency**: ~50ns for typical input (<20 chars)
/// - **Memory**: Input length + small overhead
pub fn normalize_crlf(input: &str) -> String {
    input.replace("\r\n", "\n").replace('\r', "\n")
}
```

**Usage Context**:
- Called during: Terminator parsing (declaration phase)
- Called during: Content line matching (collection phase)
- Rationale: Ensure `<<"EOF\r\n"` and `<<"EOF\n"` produce identical terminators

### 2.2 matches_terminator

**Purpose**: Match heredoc content terminator line with declaration.

**Definition**:
```rust
/// Match heredoc content terminator line against declaration
///
/// Performs exact string comparison after CRLF normalization and optional
/// indentation trimming (for `<<~` heredocs).
///
/// # Arguments
///
/// * `line` - Content line to test as terminator
/// * `declaration` - Heredoc declaration with terminator string
///
/// # Returns
///
/// `true` if line matches terminator, `false` otherwise
///
/// # Algorithm
///
/// 1. Normalize line endings: `line` → `normalized_line`
/// 2. Apply indentation handling:
///    - If `declaration.indented`: Compare `normalized_line.trim()` (strip whitespace)
///    - Otherwise: Compare `normalized_line` (exact match)
/// 3. Exact string comparison: `comparison_line == declaration.terminator`
///
/// # Examples
///
/// ```rust
/// let declaration = HeredocDeclaration {
///     terminator: "EOF".to_string(),
///     indented: false,
///     // ... other fields
/// };
///
/// assert!(matches_terminator("EOF\n", &declaration));      // Exact match
/// assert!(matches_terminator("EOF\r\n", &declaration));    // CRLF normalized
/// assert!(!matches_terminator(" EOF\n", &declaration));    // Leading space (not indented)
///
/// let indented_declaration = HeredocDeclaration {
///     terminator: "EOF".to_string(),
///     indented: true,
///     // ... other fields
/// };
///
/// assert!(matches_terminator("  EOF\n", &indented_declaration));  // Whitespace stripped
/// ```
///
/// # Performance
///
/// - **Best Case**: 2-3 comparisons (no normalization needed)
/// - **Typical**: 5-10 comparisons (CRLF normalization + trim)
/// - **Latency**: <100ns for typical terminators (<20 chars)
pub fn matches_terminator(line: &str, declaration: &HeredocDeclaration) -> bool {
    let normalized_line = normalize_crlf(line);

    let comparison_line = if declaration.indented {
        // For <<~, compare against trimmed line
        normalized_line.trim()
    } else {
        // For <<, must match exactly (after CRLF normalization)
        normalized_line.as_str()
    };

    // Exact string comparison
    comparison_line == declaration.terminator
}
```

**Usage Context**:
- Called during: Phase 2 content collection
- Called by: `HeredocCollector::extract_heredoc_content()`
- Rationale: Separate function for testability and reuse

---

## 3. Domain Relationships

### 3.1 Entity Relationship Diagram

```text
┌─────────────────────────────────────────────────────────────────────┐
│                         HeredocDeclarationParser                    │
│                                                                     │
│  ┌─────────────────────────────────────────────────────────────┐   │
│  │ State Machine                                                │   │
│  │  ┌─────────────┐    ┌──────────────┐    ┌────────────────┐  │   │
│  │  │ Start       │───→│ FirstAngle   │───→│ CheckIndent    │  │   │
│  │  └─────────────┘    └──────────────┘    └────────────────┘  │   │
│  │         │                                        │            │   │
│  │         ↓                                        ↓            │   │
│  │  ┌─────────────────────────────┐    ┌───────────────────────┐│  │
│  │  │ DetectQuoteStyle            │←───│ PreTerminatorWS       ││  │
│  │  └─────────────────────────────┘    └───────────────────────┘│  │
│  │         │                                                      │  │
│  │         ↓                                                      │  │
│  │  ┌──────────────┬─────────────────────────┬─────────────────┐│  │
│  │  │ Bare         │ DoubleQuote             │ SingleQuote     ││  │
│  │  └──────────────┴─────────────────────────┴─────────────────┘│  │
│  │         │                   │                     │            │  │
│  │         ↓                   ↓                     ↓            │  │
│  │  ┌──────────────┐    ┌──────────────┐    ┌──────────────┐    │  │
│  │  │ReadingBare   │    │ReadingQuoted │⇄   │EscapeSequence│    │  │
│  │  └──────────────┘    └──────────────┘    └──────────────┘    │  │
│  │         │                   │                                 │  │
│  │         └───────────────────┴───────────→ Complete           │  │
│  └─────────────────────────────────────────────────────────────┘  │
│                                                                     │
│  Produces ↓                                                         │
└───────────┼─────────────────────────────────────────────────────────┘
            │
            ↓
    ┌───────────────────┐
    │ HeredocDeclaration│
    ├───────────────────┤
    │ terminator        │
    │ raw_terminator    │
    │ quote_style       │───→ ┌──────────────────┐
    │ declaration_pos   │     │ HeredocQuoteStyle│
    │ declaration_end   │     ├──────────────────┤
    │ declaration_length│     │ Bare             │
    │ declaration_line  │     │ DoubleQuote      │
    │ interpolated      │     │ SingleQuote      │
    │ indented          │     │ Backtick         │
    │ placeholder_id    │     └──────────────────┘
    │ content: Option   │
    └───────────────────┘
            │
            │ Used by (Phase 2)
            ↓
    ┌───────────────────┐
    │ HeredocCollector  │
    ├───────────────────┤
    │ extract_content() │─→ matches_terminator(line, declaration)
    └───────────────────┘
            │
            ↓
    ┌───────────────────┐
    │ Node::Heredoc     │  (AST Integration)
    └───────────────────┘
```

### 3.2 Data Flow Through LSP Workflow

```text
[Input: Perl Source Code]
         ↓
┌────────────────────────────────────────────────────────────────────┐
│ PARSE STAGE                                                        │
│                                                                    │
│  Parser::parse_quote_operator()                                   │
│         ↓                                                          │
│  HeredocDeclarationParser::parse()                                │
│         ↓                                                          │
│  HeredocDeclaration { content: None }                             │
│         ↓                                                          │
│  HeredocScanner::scan() (Phase 1)                                 │
│         ↓                                                          │
│  Processed source with placeholders (__HEREDOC_N__)               │
│         ↓                                                          │
│  HeredocCollector::collect() (Phase 2)                            │
│         ↓                                                          │
│  HeredocDeclaration { content: Some(Arc<str>) }                   │
│         ↓                                                          │
│  Node::HeredocDeclaration(declaration) (AST)                      │
└────────────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────────────┐
│ INDEX STAGE                                                        │
│                                                                    │
│  SymbolExtractor::extract_from_node(node)                         │
│         ↓                                                          │
│  Symbol {                                                          │
│    name: declaration.terminator,                                  │
│    kind: SymbolKind::String,                                      │
│    location: declaration.declaration_pos,                         │
│  }                                                                 │
│         ↓                                                          │
│  SymbolTable.insert(terminator, symbol)                           │
└────────────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────────────┐
│ NAVIGATE STAGE                                                     │
│                                                                    │
│  textDocument/definition request for terminator                   │
│         ↓                                                          │
│  SymbolTable.lookup(terminator)                                   │
│         ↓                                                          │
│  Location { uri, range: declaration_pos..declaration_end }        │
└────────────────────────────────────────────────────────────────────┘
         ↓
┌────────────────────────────────────────────────────────────────────┐
│ ANALYZE STAGE                                                      │
│                                                                    │
│  Validate terminator matching in content                          │
│         ↓                                                          │
│  matches_terminator(line, declaration)                            │
│         ↓                                                          │
│  Diagnostic if no matching terminator found                       │
└────────────────────────────────────────────────────────────────────┘
```

---

## 4. Domain Invariants

### 4.1 Structural Invariants

**HeredocDeclaration**:
- `terminator` must not be empty
- `declaration_end >= declaration_pos`
- `declaration_length = declaration_end - declaration_pos`
- `interpolated = quote_style.is_interpolated()`
- If `indented`, then terminator matching uses `trim()`

**HeredocDeclarationParser**:
- `position <= chars.len()` (bounds check)
- `terminator_buffer.len() <= MAX_HEREDOC_TERMINATOR_LENGTH` (DoS prevention)
- State machine determinism: One state at a time, no ambiguous transitions

**HeredocQuoteStyle**:
- Enum exhaustiveness: All Perl heredoc styles covered
- Correct interpolation mapping: `is_interpolated()` matches Perl semantics

### 4.2 Semantic Invariants

**CRLF Normalization**:
- `normalize_crlf("...\r\n...") == normalize_crlf("...\n...")`
- Idempotent: `normalize_crlf(normalize_crlf(s)) == normalize_crlf(s)`

**Terminator Matching**:
- Reflexive: `matches_terminator(declaration.terminator, declaration) == true`
- Commutative with normalization: `matches_terminator(normalize_crlf(line), decl) == matches_terminator(line, decl)`

**Escape Sequences**:
- `\"` in double-quote → literal `"` (not terminator)
- `\\` in double-quote → literal `\` (not escape prefix)
- `\n` in single-quote → literal `\` + `n` (not newline)

---

## 5. Testing Strategy

### 5.1 Property-Based Testing

**Properties to Validate**:

1. **Idempotent Parsing**: `parse(parse(input).to_string()) == parse(input)`
2. **CRLF Normalization**: `parse(input.replace("\n", "\r\n")) == parse(input)`
3. **Terminator Round-Trip**: `matches_terminator(declaration.terminator, declaration) == true`
4. **Escape Inverse**: `parse(escape(terminator)) == terminator` (for valid escapes)
5. **State Machine Coverage**: All states reachable with valid inputs

**Proptest Strategies**:
```rust
use proptest::prelude::*;

prop_compose! {
    fn arbitrary_bare_terminator()(term in "[a-zA-Z_][a-zA-Z0-9_]{0,50}") -> String {
        format!("<<{}", term)
    }
}

prop_compose! {
    fn arbitrary_quoted_terminator()(term in "[a-zA-Z0-9\\\\nt ]{1,30}") -> String {
        format!(r#"<<"{}""#, term)
    }
}

proptest! {
    #[test]
    fn test_bare_terminators_parse(input in arbitrary_bare_terminator()) {
        let mut parser = HeredocDeclarationParser::new(&input, 0);
        let result = parser.parse();
        prop_assert!(result.is_ok(), "Failed to parse: {}", input);
    }
}
```

### 5.2 Mutation Testing Targets

**Critical Mutations**:
- Position arithmetic: `position + 1` → `position` (off-by-one)
- Escape mapping: `'\n'` → `'\t'` (wrong character)
- CRLF normalization: Skip `replace("\r\n", "\n")` (missing normalization)
- State transitions: `Complete` → `Error` (wrong terminal state)
- Terminator comparison: `==` → `!=` (inverted logic)

**Mutation Score Target**: 87% (aligned with PR #153 standards)

---

## 6. Performance Characteristics

### 6.1 Complexity Analysis

| Operation | Time Complexity | Space Complexity | Notes |
|-----------|----------------|------------------|-------|
| `new()` | O(n) | O(n) | Convert UTF-8 to `Vec<char>` |
| `parse()` | O(m) | O(m) | m = terminator length (typically m << n) |
| `normalize_crlf()` | O(n) | O(n) | Two string replacements |
| `matches_terminator()` | O(m) | O(m) | String comparison after normalization |

**Variables**:
- n = input length (full heredoc declaration)
- m = terminator length (typically <20 characters)

### 6.2 Benchmark Targets

| Scenario | Target Latency (μs) | Memory (bytes) |
|----------|---------------------|----------------|
| Bare terminator (`<<EOF`) | <50 | ~200 |
| Quoted simple (`<<"EOF"`) | <80 | ~250 |
| Quoted with escapes (`<<"EO\nF"`) | <100 | ~300 |
| Error path (invalid input) | <20 | ~150 |

**Measurement**: Criterion benchmarks on AMD Ryzen 9 5950X, 1000 iterations

---

## 7. Security Considerations

### 7.1 UTF-8 Safety

**Risk**: Byte slicing on multi-byte UTF-8 characters causes panics

**Mitigation**:
```rust
// SAFE: Use chars().collect() for character iteration
let chars: Vec<char> = input.chars().collect();
let slice = &chars[start..end];  // Always valid

// UNSAFE: Byte slicing
let slice = &input[start..end];  // Can panic on UTF-8 boundaries
```

### 7.2 DoS Prevention

**Risk**: Extremely long terminators exhaust memory

**Mitigation**:
```rust
const MAX_HEREDOC_TERMINATOR_LENGTH: usize = 256;

if self.terminator_buffer.len() > MAX_HEREDOC_TERMINATOR_LENGTH {
    return Err(HeredocParseError::TerminatorTooLong {
        position: self.position,
        max_length: MAX_HEREDOC_TERMINATOR_LENGTH,
    });
}
```

### 7.3 Path Traversal (Backtick Heredocs)

**Risk**: Command substitution heredocs execute arbitrary shell commands

**Mitigation**:
- Document security implications in `HeredocQuoteStyle::Backtick`
- LSP server sandboxing (separate process, no shell access)
- Future: Optional security policy to disable backtick heredocs

---

## 8. Future Enhancements

### 8.1 SIMD Terminator Matching

**Optimization**: Use AVX2 for parallel terminator comparison

**Target**: 10x speedup for terminators >16 characters

**Implementation**:
```rust
#[cfg(target_arch = "x86_64")]
fn matches_terminator_simd(line: &str, terminator: &str) -> bool {
    use std::arch::x86_64::*;
    // AVX2 parallel byte comparison
}
```

### 8.2 String Interning for Common Terminators

**Optimization**: Cache frequently used terminators (EOF, END, DATA)

**Target**: ~20% latency reduction via cache hit

**Implementation**:
```rust
static COMMON_TERMINATORS: Lazy<HashMap<&'static str, Arc<str>>> = Lazy::new(|| {
    ["EOF", "END", "DATA", "SQL", "HTML"]
        .iter()
        .map(|&s| (s, Arc::from(s)))
        .collect()
});
```

### 8.3 LSP Quick Fixes for Malformed Heredocs

**Feature**: Actionable quick fixes in LSP diagnostics

**Example**:
```json
{
  "diagnostics": [{
    "message": "Unterminated quoted heredoc label",
    "codeActions": [{
      "title": "Add closing quote",
      "edit": {
        "changes": {
          "file:///path/to/file.pl": [{
            "range": { "start": { "line": 5, "character": 10 }, "end": { ... } },
            "newText": "\""
          }]
        }
      }
    }]
  }]
}
```

---

## Changelog

| Date | Version | Author | Changes |
|------|---------|--------|---------|
| 2025-11-05 | 1.0.0 | spec-creator | Initial domain schema |

---

**Document End**
