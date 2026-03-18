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
        Self { id, name: name.into(), source, line, column: 1, end_line: None, end_column: None }
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
        let name = std::path::Path::new(&path)
            .file_name()
            .and_then(|name| name.to_str())
            .map(ToOwned::to_owned);

        Self { name, path, source_reference: None }
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
