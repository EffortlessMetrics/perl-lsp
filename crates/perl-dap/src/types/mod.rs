//! Shared DAP session model types for Perl debugging.

use serde::{Deserialize, Serialize};

/// Stack frame information used by the debug adapter.
///
/// Corresponds to the DAP `StackFrame` type in the `stackTrace` response.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    /// Unique numeric identifier for this frame within the current stopped state.
    pub id: i32,
    /// Human-readable display name for the frame (e.g. `"main::foo"`).
    pub name: String,
    /// Source file that contains this frame's code.
    pub source: Source,
    /// 1-based line number of the current instruction in the frame.
    pub line: i32,
    /// 1-based column number of the current instruction in the frame.
    pub column: i32,
    /// 1-based end line of the range covered by this frame, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i32>,
    /// 1-based end column of the range covered by this frame, if known.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i32>,
}

impl StackFrame {
    /// Create a new stack frame at `line` with column defaulting to 1.
    #[must_use]
    pub fn new(id: i32, name: impl Into<String>, source: Source, line: i32) -> Self {
        Self { id, name: name.into(), source, line, column: 1, end_line: None, end_column: None }
    }

    /// Override the column for this frame.
    #[must_use]
    pub fn with_column(mut self, column: i32) -> Self {
        self.column = column;
        self
    }

    /// Set the end position (end line and end column) of this frame's source range.
    #[must_use]
    pub fn with_end(mut self, end_line: i32, end_column: i32) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }
}

/// Source file information for stack frames.
///
/// Corresponds to the DAP `Source` type used in `stackTrace` and breakpoint responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    /// Optional display name for the source file (typically the file's base name).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    /// Absolute or workspace-relative path to the source file.
    pub path: String,
    /// Optional DAP source reference for sources that have no file path.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i32>,
}

impl Source {
    /// Create a `Source` from a file path, deriving the display name from the final component.
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let path_name = std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);
        let name = if path_name.as_deref() == Some(path.as_str()) && path.contains('\\') {
            path.rsplit('\\').find(|segment| !segment.is_empty()).map(ToOwned::to_owned)
        } else {
            path_name
        };

        Self { name, path, source_reference: None }
    }
}

/// Variable information returned by the debug adapter.
///
/// Corresponds to the DAP `Variable` type in `variables` responses.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    /// The variable's display name (e.g. `"$x"`, `"@arr"`).
    pub name: String,
    /// The variable's value rendered as a string for display.
    pub value: String,
    /// Optional type hint for the variable (e.g. `"SCALAR"`, `"ARRAY"`, `"HASH"`).
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    /// Reference handle for fetching nested variables; 0 means the variable has no children.
    pub variables_reference: i32,
    /// Hint for how many named child variables this variable has, if structured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<i32>,
    /// Hint for how many indexed child variables this variable has, if it is an array.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<i32>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stack_frame_new_defaults() {
        let src = Source::new("/path/to/script.pl");
        let frame = StackFrame::new(1, "main::foo", src, 42);
        assert_eq!(frame.id, 1);
        assert_eq!(frame.name, "main::foo");
        assert_eq!(frame.line, 42);
        assert_eq!(frame.column, 1);
        assert!(frame.end_line.is_none());
        assert!(frame.end_column.is_none());
    }

    #[test]
    fn stack_frame_with_column_and_end() {
        let src = Source::new("/a.pl");
        let frame = StackFrame::new(2, "foo", src, 10).with_column(5).with_end(10, 20);
        assert_eq!(frame.column, 5);
        assert_eq!(frame.end_line, Some(10));
        assert_eq!(frame.end_column, Some(20));
    }

    #[test]
    fn source_new_extracts_filename() {
        let src = Source::new("/path/to/Module.pm");
        assert_eq!(src.path, "/path/to/Module.pm");
        assert_eq!(src.name, Some("Module.pm".to_string()));
        assert!(src.source_reference.is_none());
    }

    #[test]
    fn stack_frame_serde_round_trip() -> serde_json::Result<()> {
        let src = Source::new("/script.pl");
        let frame = StackFrame::new(1, "run", src, 5);
        let json = serde_json::to_string(&frame)?;
        let back: StackFrame = serde_json::from_str(&json)?;
        assert_eq!(back.id, 1);
        assert_eq!(back.line, 5);
        Ok(())
    }

    #[test]
    fn stack_frame_optional_fields_omitted_in_json() -> serde_json::Result<()> {
        let src = Source::new("/a.pl");
        let frame = StackFrame::new(1, "foo", src, 1);
        let json = serde_json::to_string(&frame)?;
        assert!(!json.contains("endLine"), "endLine should be absent: {json}");
        assert!(!json.contains("endColumn"), "endColumn should be absent: {json}");
        Ok(())
    }

    #[test]
    fn variable_type_field_serializes_as_type_not_type_underscore() -> serde_json::Result<()> {
        let var = Variable {
            name: "$x".to_string(),
            value: "42".to_string(),
            type_: Some("SCALAR".to_string()),
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
        };
        let json = serde_json::to_string(&var)?;
        assert!(json.contains("\"type\":"), "must serialize as 'type' not 'type_': {json}");
        assert!(!json.contains("type_"), "must not leak Rust field name: {json}");
        Ok(())
    }

    #[test]
    fn variable_optional_fields_omitted_when_none() -> serde_json::Result<()> {
        let var = Variable {
            name: "$x".to_string(),
            value: "1".to_string(),
            type_: None,
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
        };
        let json = serde_json::to_string(&var)?;
        assert!(!json.contains("namedVariables"), "absent: {json}");
        assert!(!json.contains("indexedVariables"), "absent: {json}");
        Ok(())
    }

    #[test]
    fn variable_serde_round_trip() -> serde_json::Result<()> {
        let var = Variable {
            name: "@arr".to_string(),
            value: "(3 elements)".to_string(),
            type_: Some("ARRAY".to_string()),
            variables_reference: 7,
            named_variables: None,
            indexed_variables: Some(3),
        };
        let json = serde_json::to_string(&var)?;
        let back: Variable = serde_json::from_str(&json)?;
        assert_eq!(back.variables_reference, 7);
        assert_eq!(back.indexed_variables, Some(3));
        Ok(())
    }
}
