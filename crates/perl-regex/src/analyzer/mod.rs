mod capture;
mod hover;
mod modifiers;
mod parser;

pub use capture::CaptureGroup;

pub struct RegexAnalyzer;

impl RegexAnalyzer {
    pub fn extract_named_captures(pattern: &str) -> Vec<CaptureGroup> {
        capture::extract_named_captures(pattern)
    }

    pub fn hover_text_for_regex(pattern: &str, modifiers: &str) -> String {
        hover::hover_text_for_regex(pattern, modifiers)
    }
}
