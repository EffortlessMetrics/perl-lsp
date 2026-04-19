//! Shared DAP session model types for Perl debugging.

use serde::{Deserialize, Serialize};

/// Stack frame information used by the debug adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct StackFrame {
    pub id: i32,
    pub name: String,
    pub source: Source,
    pub line: i32,
    pub column: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_line: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_column: Option<i32>,
}

impl StackFrame {
    #[must_use]
    pub fn new(id: i32, name: impl Into<String>, source: Source, line: i32) -> Self {
        Self {
            id,
            name: name.into(),
            source,
            line,
            column: 1,
            end_line: None,
            end_column: None,
        }
    }

    #[must_use]
    pub fn with_column(mut self, column: i32) -> Self {
        self.column = column;
        self
    }

    #[must_use]
    pub fn with_end(mut self, end_line: i32, end_column: i32) -> Self {
        self.end_line = Some(end_line);
        self.end_column = Some(end_column);
        self
    }
}

/// Source file information for stack frames.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Source {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    pub path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_reference: Option<i32>,
}

impl Source {
    #[must_use]
    pub fn new(path: impl Into<String>) -> Self {
        let path = path.into();
        let path_name = std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);
        let name = if path_name.as_deref() == Some(path.as_str()) && path.contains('\\') {
            path.rsplit('\\')
                .find(|segment| !segment.is_empty())
                .map(ToOwned::to_owned)
        } else {
            path_name
        };

        Self {
            name,
            path,
            source_reference: None,
        }
    }
}

/// Variable information returned by the debug adapter.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Variable {
    pub name: String,
    pub value: String,
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_: Option<String>,
    pub variables_reference: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<i32>,
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
        let frame = StackFrame::new(2, "foo", src, 10)
            .with_column(5)
            .with_end(10, 20);
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
        assert!(
            !json.contains("endLine"),
            "endLine should be absent: {json}"
        );
        assert!(
            !json.contains("endColumn"),
            "endColumn should be absent: {json}"
        );
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
        assert!(
            json.contains("\"type\":"),
            "must serialize as 'type' not 'type_': {json}"
        );
        assert!(
            !json.contains("type_"),
            "must not leak Rust field name: {json}"
        );
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
