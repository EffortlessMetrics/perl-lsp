//! Variable renderer for DAP protocol.
//!
//! This module provides the [`VariableRenderer`] trait and [`PerlVariableRenderer`]
//! implementation for converting Perl values into DAP-compatible variable representations.

use crate::PerlValue;
use serde::{Deserialize, Serialize};

/// A rendered variable for the DAP protocol.
///
/// This struct represents a variable in a format suitable for the Debug Adapter Protocol,
/// supporting lazy expansion of complex data structures.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderedVariable {
    /// The name of the variable (e.g., "$foo", "@bar", "%hash")
    pub name: String,

    /// The string representation of the value
    pub value: String,

    /// The type of the variable (e.g., "SCALAR", "ARRAY", "HASH")
    #[serde(rename = "type", skip_serializing_if = "Option::is_none")]
    pub type_name: Option<String>,

    /// Reference ID for lazy expansion (0 = not expandable)
    pub variables_reference: i64,

    /// Number of named children (for objects/hashes)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub named_variables: Option<i64>,

    /// Number of indexed children (for arrays)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub indexed_variables: Option<i64>,

    /// Presentation hint for the UI
    #[serde(skip_serializing_if = "Option::is_none")]
    pub presentation_hint: Option<VariablePresentationHint>,

    /// Memory address (if available)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub memory_reference: Option<String>,
}

/// Presentation hints for variable display in the DAP UI.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct VariablePresentationHint {
    /// The kind of variable (e.g., "property", "method", "class")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kind: Option<String>,

    /// Attributes (e.g., "static", "constant", "readOnly")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub attributes: Option<Vec<String>>,

    /// Visibility (e.g., "public", "private", "protected")
    #[serde(skip_serializing_if = "Option::is_none")]
    pub visibility: Option<String>,
}

impl RenderedVariable {
    /// Creates a new rendered variable with the given name and value.
    #[must_use]
    pub fn new(name: impl Into<String>, value: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            value: value.into(),
            type_name: None,
            variables_reference: 0,
            named_variables: None,
            indexed_variables: None,
            presentation_hint: None,
            memory_reference: None,
        }
    }

    /// Sets the type name for this variable.
    #[must_use]
    pub fn with_type(mut self, type_name: impl Into<String>) -> Self {
        self.type_name = Some(type_name.into());
        self
    }

    /// Sets the variables reference for lazy expansion.
    #[must_use]
    pub fn with_reference(mut self, reference: i64) -> Self {
        self.variables_reference = reference;
        self
    }

    /// Sets the indexed variables count (for arrays).
    #[must_use]
    pub fn with_indexed_variables(mut self, count: i64) -> Self {
        self.indexed_variables = Some(count);
        self
    }

    /// Sets the named variables count (for hashes/objects).
    #[must_use]
    pub fn with_named_variables(mut self, count: i64) -> Self {
        self.named_variables = Some(count);
        self
    }

    /// Returns true if this variable can be expanded.
    #[must_use]
    pub fn is_expandable(&self) -> bool {
        self.variables_reference != 0
    }
}

/// Trait for rendering Perl values into DAP variables.
///
/// Implementations of this trait convert [`PerlValue`] instances into
/// [`RenderedVariable`] structures suitable for the DAP protocol.
pub trait VariableRenderer {
    /// Render a Perl value into a DAP variable.
    ///
    /// # Arguments
    ///
    /// * `name` - The variable name (e.g., "$foo")
    /// * `value` - The Perl value to render
    ///
    /// # Returns
    ///
    /// A [`RenderedVariable`] suitable for the DAP protocol.
    fn render(&self, name: &str, value: &PerlValue) -> RenderedVariable;

    /// Render a Perl value with a specific variables reference ID.
    ///
    /// This is used when the value is expandable and needs a reference ID
    /// for lazy loading of children.
    ///
    /// # Arguments
    ///
    /// * `name` - The variable name
    /// * `value` - The Perl value to render
    /// * `reference_id` - The variables reference ID for expansion
    fn render_with_reference(
        &self,
        name: &str,
        value: &PerlValue,
        reference_id: i64,
    ) -> RenderedVariable;

    /// Render the children of an expandable value.
    ///
    /// # Arguments
    ///
    /// * `value` - The parent value to expand
    /// * `start` - The starting index for pagination (0-based)
    /// * `count` - The maximum number of children to return
    ///
    /// # Returns
    ///
    /// A vector of rendered child variables.
    fn render_children(
        &self,
        value: &PerlValue,
        start: usize,
        count: usize,
    ) -> Vec<RenderedVariable>;
}

/// Default Perl variable renderer implementation.
///
/// This renderer follows Perl conventions for variable display:
/// - Strings are quoted
/// - Arrays show element count
/// - Hashes show key count
/// - References show the referent type
/// - Objects show class name
#[derive(Debug, Default)]
pub struct PerlVariableRenderer {
    /// Maximum string length before truncation
    max_string_length: usize,
    /// Maximum array elements to show in preview
    max_array_preview: usize,
    /// Maximum hash pairs to show in preview
    max_hash_preview: usize,
}

impl PerlVariableRenderer {
    /// Creates a new Perl variable renderer with default settings.
    #[must_use]
    pub fn new() -> Self {
        Self { max_string_length: 100, max_array_preview: 3, max_hash_preview: 3 }
    }

    /// Sets the maximum string length before truncation.
    #[must_use]
    pub fn with_max_string_length(mut self, length: usize) -> Self {
        self.max_string_length = length;
        self
    }

    /// Sets the maximum array elements in preview.
    #[must_use]
    pub fn with_max_array_preview(mut self, count: usize) -> Self {
        self.max_array_preview = count;
        self
    }

    /// Sets the maximum hash pairs in preview.
    #[must_use]
    pub fn with_max_hash_preview(mut self, count: usize) -> Self {
        self.max_hash_preview = count;
        self
    }

    /// Formats a scalar string value with quoting and truncation.
    fn format_string(&self, s: &str) -> String {
        let truncated = if s.len() > self.max_string_length {
            format!("{}...", &s[..self.max_string_length])
        } else {
            s.to_string()
        };

        // Escape special characters and quote
        let escaped = truncated
            .replace('\\', "\\\\")
            .replace('"', "\\\"")
            .replace('\n', "\\n")
            .replace('\r', "\\r")
            .replace('\t', "\\t");

        format!("\"{}\"", escaped)
    }

    /// Formats an array value for preview.
    fn format_array_preview(&self, elements: &[PerlValue]) -> String {
        if elements.is_empty() {
            return "[]".to_string();
        }

        let preview: Vec<String> = elements
            .iter()
            .take(self.max_array_preview)
            .map(|v| self.format_value_brief(v))
            .collect();

        let suffix = if elements.len() > self.max_array_preview {
            format!(", ... ({} total)", elements.len())
        } else {
            String::new()
        };

        format!("[{}{}]", preview.join(", "), suffix)
    }

    /// Formats a hash value for preview.
    fn format_hash_preview(&self, pairs: &[(String, PerlValue)]) -> String {
        if pairs.is_empty() {
            return "{}".to_string();
        }

        let preview: Vec<String> = pairs
            .iter()
            .take(self.max_hash_preview)
            .map(|(k, v)| format!("{} => {}", k, self.format_value_brief(v)))
            .collect();

        let suffix = if pairs.len() > self.max_hash_preview {
            format!(", ... ({} keys)", pairs.len())
        } else {
            String::new()
        };

        format!("{{{}{}}}", preview.join(", "), suffix)
    }

    /// Formats a value briefly (for use in previews).
    fn format_value_brief(&self, value: &PerlValue) -> String {
        match value {
            PerlValue::Undef => "undef".to_string(),
            PerlValue::Scalar(s) => self.format_string(s),
            PerlValue::Number(n) => n.to_string(),
            PerlValue::Integer(i) => i.to_string(),
            PerlValue::Array(elements) => format!("ARRAY({})", elements.len()),
            PerlValue::Hash(pairs) => format!("HASH({})", pairs.len()),
            PerlValue::Reference(inner) => format!("\\{}", inner.type_name()),
            PerlValue::Object { class, .. } => format!("{}=...", class),
            PerlValue::Code { name } => {
                name.as_ref().map_or_else(|| "CODE(...)".to_string(), |n| format!("\\&{}", n))
            }
            PerlValue::Glob(name) => format!("*{}", name),
            PerlValue::Regex(pattern) => format!("qr/{}/", pattern),
            PerlValue::Tied { class, .. } => format!("TIED({})", class),
            PerlValue::Truncated { summary, .. } => summary.clone(),
            PerlValue::Error(msg) => format!("<error: {}>", msg),
        }
    }

    /// Formats a full value (for the value field).
    fn format_value(&self, value: &PerlValue) -> String {
        match value {
            PerlValue::Undef => "undef".to_string(),
            PerlValue::Scalar(s) => self.format_string(s),
            PerlValue::Number(n) => n.to_string(),
            PerlValue::Integer(i) => i.to_string(),
            PerlValue::Array(elements) => self.format_array_preview(elements),
            PerlValue::Hash(pairs) => self.format_hash_preview(pairs),
            PerlValue::Reference(inner) => {
                format!("\\{}", self.format_value_brief(inner))
            }
            PerlValue::Object { class, value } => {
                format!("{}={}", class, self.format_value_brief(value))
            }
            PerlValue::Code { name } => {
                name.as_ref().map_or_else(|| "sub { ... }".to_string(), |n| format!("\\&{}", n))
            }
            PerlValue::Glob(name) => format!("*{}", name),
            PerlValue::Regex(pattern) => format!("qr/{}/", pattern),
            PerlValue::Tied { class, value } => {
                if let Some(v) = value {
                    format!("TIED({}) = {}", class, self.format_value_brief(v))
                } else {
                    format!("TIED({})", class)
                }
            }
            PerlValue::Truncated { summary, total_count } => {
                if let Some(count) = total_count {
                    format!("{} ({} total)", summary, count)
                } else {
                    summary.clone()
                }
            }
            PerlValue::Error(msg) => format!("<error: {}>", msg),
        }
    }
}

impl VariableRenderer for PerlVariableRenderer {
    fn render(&self, name: &str, value: &PerlValue) -> RenderedVariable {
        let formatted_value = self.format_value(value);
        let type_name = value.type_name().to_string();

        let mut rendered = RenderedVariable::new(name, formatted_value).with_type(type_name);

        // Set child counts for expandable types
        match value {
            PerlValue::Array(elements) => {
                rendered.indexed_variables = Some(elements.len() as i64);
            }
            PerlValue::Hash(pairs) => {
                rendered.named_variables = Some(pairs.len() as i64);
            }
            PerlValue::Object { value: inner, .. } => {
                if let PerlValue::Hash(pairs) = inner.as_ref() {
                    rendered.named_variables = Some(pairs.len() as i64);
                }
            }
            _ => {}
        }

        rendered
    }

    fn render_with_reference(
        &self,
        name: &str,
        value: &PerlValue,
        reference_id: i64,
    ) -> RenderedVariable {
        let mut rendered = self.render(name, value);

        if value.is_expandable() {
            rendered.variables_reference = reference_id;
        }

        rendered
    }

    fn render_children(
        &self,
        value: &PerlValue,
        start: usize,
        count: usize,
    ) -> Vec<RenderedVariable> {
        match value {
            PerlValue::Array(elements) => elements
                .iter()
                .enumerate()
                .skip(start)
                .take(count)
                .map(|(i, v)| self.render(&format!("[{}]", i), v))
                .collect(),
            PerlValue::Hash(pairs) => {
                pairs.iter().skip(start).take(count).map(|(k, v)| self.render(k, v)).collect()
            }
            PerlValue::Reference(inner) => {
                vec![self.render("$_", inner)]
            }
            PerlValue::Object { value: inner, .. } => self.render_children(inner, start, count),
            PerlValue::Tied { value: Some(inner), .. } => self.render_children(inner, start, count),
            _ => vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_render_scalar() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Scalar("hello".to_string());
        let rendered = renderer.render("$x", &value);

        assert_eq!(rendered.name, "$x");
        assert_eq!(rendered.value, "\"hello\"");
        assert_eq!(rendered.type_name, Some("SCALAR".to_string()));
        assert_eq!(rendered.variables_reference, 0);
    }

    #[test]
    fn test_render_integer() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Integer(42);
        let rendered = renderer.render("$n", &value);

        assert_eq!(rendered.name, "$n");
        assert_eq!(rendered.value, "42");
        assert_eq!(rendered.type_name, Some("SCALAR".to_string()));
    }

    #[test]
    fn test_render_array() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Array(vec![
            PerlValue::Integer(1),
            PerlValue::Integer(2),
            PerlValue::Integer(3),
        ]);
        let rendered = renderer.render("@arr", &value);

        assert_eq!(rendered.name, "@arr");
        assert!(rendered.value.starts_with('['));
        assert_eq!(rendered.type_name, Some("ARRAY".to_string()));
        assert_eq!(rendered.indexed_variables, Some(3));
    }

    #[test]
    fn test_render_hash() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Hash(vec![
            ("key1".to_string(), PerlValue::Scalar("value1".to_string())),
            ("key2".to_string(), PerlValue::Integer(42)),
        ]);
        let rendered = renderer.render("%hash", &value);

        assert_eq!(rendered.name, "%hash");
        assert!(rendered.value.starts_with('{'));
        assert_eq!(rendered.type_name, Some("HASH".to_string()));
        assert_eq!(rendered.named_variables, Some(2));
    }

    #[test]
    fn test_render_with_reference() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Array(vec![PerlValue::Integer(1)]);
        let rendered = renderer.render_with_reference("@arr", &value, 42);

        assert_eq!(rendered.variables_reference, 42);
        assert!(rendered.is_expandable());
    }

    #[test]
    fn test_render_children_array() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Array(vec![
            PerlValue::Integer(10),
            PerlValue::Integer(20),
            PerlValue::Integer(30),
        ]);
        let children = renderer.render_children(&value, 0, 10);

        assert_eq!(children.len(), 3);
        assert_eq!(children[0].name, "[0]");
        assert_eq!(children[0].value, "10");
        assert_eq!(children[1].name, "[1]");
        assert_eq!(children[2].name, "[2]");
    }

    #[test]
    fn test_render_children_hash() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Hash(vec![
            ("foo".to_string(), PerlValue::Integer(1)),
            ("bar".to_string(), PerlValue::Integer(2)),
        ]);
        let children = renderer.render_children(&value, 0, 10);

        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "foo");
        assert_eq!(children[1].name, "bar");
    }

    #[test]
    fn test_render_object() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Object {
            class: "My::Class".to_string(),
            value: Box::new(PerlValue::Hash(vec![(
                "attr".to_string(),
                PerlValue::Scalar("value".to_string()),
            )])),
        };
        let rendered = renderer.render("$obj", &value);

        assert_eq!(rendered.name, "$obj");
        assert!(rendered.value.contains("My::Class"));
        assert_eq!(rendered.type_name, Some("OBJECT".to_string()));
        assert_eq!(rendered.named_variables, Some(1));
    }

    #[test]
    fn test_string_truncation() {
        let renderer = PerlVariableRenderer::new().with_max_string_length(10);
        let value = PerlValue::Scalar("this is a very long string".to_string());
        let rendered = renderer.render("$s", &value);

        assert!(rendered.value.contains("..."));
        assert!(rendered.value.len() < 30);
    }

    #[test]
    fn test_string_escaping() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Scalar("line1\nline2\ttab".to_string());
        let rendered = renderer.render("$s", &value);

        assert!(rendered.value.contains("\\n"));
        assert!(rendered.value.contains("\\t"));
    }

    #[test]
    fn test_render_undef() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Undef;
        let rendered = renderer.render("$x", &value);

        assert_eq!(rendered.value, "undef");
        assert_eq!(rendered.type_name, Some("undef".to_string()));
    }

    #[test]
    fn test_render_reference() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Reference(Box::new(PerlValue::Integer(42)));
        let rendered = renderer.render("$ref", &value);

        assert_eq!(rendered.name, "$ref");
        assert!(rendered.value.contains("42"));
        assert_eq!(rendered.type_name, Some("REF".to_string()));
    }

    #[test]
    fn test_render_code() {
        let renderer = PerlVariableRenderer::new();
        let value = PerlValue::Code { name: Some("my_sub".to_string()) };
        let rendered = renderer.render("$code", &value);

        assert!(rendered.value.contains("my_sub"));
        assert_eq!(rendered.type_name, Some("CODE".to_string()));
    }
}
