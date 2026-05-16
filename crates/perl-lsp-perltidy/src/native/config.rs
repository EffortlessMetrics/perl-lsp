use serde::{Deserialize, Serialize};

/// Native formatter operating mode.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FormatterMode {
    /// Run the Rust-native formatter.
    #[default]
    Native,
    /// Run native formatting with compatibility defaults for common legacy profiles.
    Compat,
    /// Explicitly use an external legacy formatter adapter.
    ExternalLegacy,
    /// Disable formatting.
    Off,
}

/// Final newline handling policy.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum FinalNewline {
    /// Preserve the input's final newline state.
    #[default]
    Preserve,
    /// Ensure exactly one final newline when formatting succeeds.
    Insert,
    /// Remove trailing final newlines when formatting succeeds.
    Trim,
}

/// Trailing comma handling for wrapped delimited expressions.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TrailingComma {
    /// Preserve the native formatter's current behavior and do not add commas.
    #[default]
    Preserve,
    /// Add a trailing comma when a call, list, or hash is rendered across lines.
    AddWhenWrapped,
}

/// Opening brace placement for supported native block layouts.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum BracePlacement {
    /// Keep the opening brace on the block header line.
    #[default]
    SameLine,
    /// Place the opening brace on its own line at the block indentation.
    NextLine,
}

/// Placement for supported native `else` and `elsif` block tails.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ElsePlacement {
    /// Keep `else` and `elsif` cuddled to the previous closing brace.
    #[default]
    Cuddled,
    /// Place `else` and `elsif` on a fresh line at the block indentation.
    SeparateLine,
}

/// Spacing between supported control keywords and their condition parentheses.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum KeywordSpacing {
    /// Insert a space between the keyword and condition parentheses.
    #[default]
    Space,
    /// Omit the space between the keyword and condition parentheses.
    Compact,
}

/// Configuration shared by native formatter implementations.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FormatConfig {
    /// Formatter engine mode.
    pub mode: FormatterMode,
    /// Preferred line width.
    pub line_width: u32,
    /// Indentation width when spaces are used.
    pub indent_width: u32,
    /// Whether indentation should use tabs instead of spaces.
    pub use_tabs: bool,
    /// Final newline handling.
    pub final_newline: FinalNewline,
    /// Trailing comma handling for wrapped delimited expressions.
    pub trailing_comma: TrailingComma,
    /// Opening brace placement for supported block layouts.
    pub brace_placement: BracePlacement,
    /// Else/elsif placement for supported block tails.
    pub else_placement: ElsePlacement,
    /// Keyword spacing for supported control-flow condition headers.
    pub keyword_spacing: KeywordSpacing,
}

impl Default for FormatConfig {
    fn default() -> Self {
        Self {
            mode: FormatterMode::Native,
            line_width: 100,
            indent_width: 4,
            use_tabs: false,
            final_newline: FinalNewline::Preserve,
            trailing_comma: TrailingComma::Preserve,
            brace_placement: BracePlacement::SameLine,
            else_placement: ElsePlacement::Cuddled,
            keyword_spacing: KeywordSpacing::Space,
        }
    }
}

impl FormatConfig {
    /// Build a compatibility-oriented native configuration.
    #[must_use]
    pub fn compat() -> Self {
        Self { mode: FormatterMode::Compat, ..Self::default() }
    }

    /// Build an explicit external legacy configuration.
    #[must_use]
    pub fn external_legacy() -> Self {
        Self { mode: FormatterMode::ExternalLegacy, ..Self::default() }
    }
}
