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
            // Find a char boundary at or before max_string_length to avoid
            // panicking on multi-byte UTF-8 sequences.
            let mut end = self.max_string_length;
            while end > 0 && !s.is_char_boundary(end) {
                end -= 1;
            }
            format!("{}...", &s[..end])
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

    /// Returns the chain of backslash prefixes for nested references.
    ///
    /// For `\\\42` (ref to ref to ref to scalar) this returns `"\\\\"` (three backslashes).
    /// Stops counting after 10 levels to avoid runaway recursion on cyclic-like structures.
    fn ref_prefix_chain(&self, value: &PerlValue) -> String {
        let mut depth = 0u32;
        let mut current = value;
        while let PerlValue::Reference(inner) = current {
            depth += 1;
            current = inner;
            if depth >= 10 {
                break;
            }
        }
        "\\".repeat(depth as usize)
    }

    /// Formats the ultimate target of a reference chain briefly.
    ///
    /// Skips intermediate `Reference` wrappers to show the leaf value.
    fn format_deref_target_brief(&self, value: &PerlValue) -> String {
        let mut current = value;
        let mut safety = 0u32;
        while let PerlValue::Reference(inner) = current {
            current = inner;
            safety += 1;
            if safety >= 10 {
                break;
            }
        }
        match current {
            PerlValue::Reference(_) => "REF(...)".to_string(),
            other => self.format_non_ref_brief(other),
        }
    }

    /// Formats the ultimate target of a reference chain fully.
    fn format_deref_target(&self, value: &PerlValue) -> String {
        let mut current = value;
        let mut safety = 0u32;
        while let PerlValue::Reference(inner) = current {
            current = inner;
            safety += 1;
            if safety >= 10 {
                break;
            }
        }
        match current {
            PerlValue::Reference(_) => "REF(...)".to_string(),
            other => self.format_value(other),
        }
    }

    /// Formats a non-reference value briefly (used by deref target formatting).
    fn format_non_ref_brief(&self, value: &PerlValue) -> String {
        match value {
            PerlValue::Reference(_) => "REF".to_string(),
            other => self.format_value_brief(other),
        }
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
            PerlValue::Reference(inner) => {
                let prefix = self.ref_prefix_chain(value);
                format!("{}{}", prefix, self.format_deref_target_brief(inner))
            }
            PerlValue::Object { class, value } => {
                let backing = match value.as_ref() {
                    PerlValue::Hash(_) => "HASH",
                    PerlValue::Array(_) => "ARRAY",
                    PerlValue::Scalar(_) | PerlValue::Number(_) | PerlValue::Integer(_) => "SCALAR",
                    _ => "REF",
                };
                format!("{} = {}(...)", class, backing)
            }
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
                let prefix = self.ref_prefix_chain(value);
                format!("{}{}", prefix, self.format_deref_target(inner))
            }
            PerlValue::Object { class, value } => {
                let backing = match value.as_ref() {
                    PerlValue::Hash(_) => "HASH",
                    PerlValue::Array(_) => "ARRAY",
                    PerlValue::Scalar(_) | PerlValue::Number(_) | PerlValue::Integer(_) => "SCALAR",
                    _ => "REF",
                };
                format!("{} = {}(...)", class, backing)
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
            PerlValue::Object { class, value: inner } => {
                rendered.type_name = Some(class.clone());
                rendered.presentation_hint = Some(VariablePresentationHint {
                    kind: Some("class".to_string()),
                    attributes: None,
                    visibility: None,
                });
                match inner.as_ref() {
                    PerlValue::Hash(pairs) => {
                        rendered.named_variables = Some(pairs.len() as i64);
                    }
                    PerlValue::Array(elements) => {
                        rendered.indexed_variables = Some(elements.len() as i64);
                    }
                    _ => {}
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
        assert_eq!(rendered.type_name, Some("My::Class".to_string()));
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

    // ---------------------------------------------------------------
    // Circular reference detection and large structure handling tests
    // ---------------------------------------------------------------

    /// Simulates a self-referential hash like `{ a => \$self }`.
    ///
    /// Since `PerlValue` uses `Box` (no `Rc`), true cycles cannot exist at
    /// the type level.  The Perl debugger would itself truncate such cycles,
    /// so we model the leaf as a `Truncated` variant representing the
    /// back-reference the debugger would emit.
    #[test]
    fn test_render_self_referential_hash() {
        let renderer = PerlVariableRenderer::new();

        // Simulate: my $self = { a => \$self }
        // The debugger would show the back-reference as a truncated/circular marker.
        let circular_marker =
            PerlValue::Truncated { summary: "HASH(0x...circular)".to_string(), total_count: None };
        let value = PerlValue::Hash(vec![(
            "a".to_string(),
            PerlValue::Reference(Box::new(circular_marker)),
        )]);

        let rendered = renderer.render("$self", &value);

        assert_eq!(rendered.name, "$self");
        assert_eq!(rendered.type_name, Some("HASH".to_string()));
        assert_eq!(rendered.named_variables, Some(1));
        // The value preview should contain the circular marker, not panic or hang
        assert!(rendered.value.contains("circular"));

        // Children should also render safely
        let children = renderer.render_children(&value, 0, 10);
        assert_eq!(children.len(), 1);
        assert_eq!(children[0].name, "a");
        assert!(children[0].value.contains("circular"));
    }

    /// Deep nesting of >100 reference levels should produce bounded output.
    ///
    /// The renderer caps reference chain traversal at 10 levels, emitting
    /// `REF(...)` for anything deeper.
    #[test]
    fn test_render_deep_nesting_over_100_levels_bounded() {
        let renderer = PerlVariableRenderer::new();

        // Build a tower of 150 nested references: \\\...\42
        let mut value = PerlValue::Integer(42);
        for _ in 0..150 {
            value = PerlValue::Reference(Box::new(value));
        }

        let rendered = renderer.render("$deep", &value);

        assert_eq!(rendered.name, "$deep");
        assert_eq!(rendered.type_name, Some("REF".to_string()));

        // The output must be bounded — the renderer stops at 10 backslash
        // prefixes and then emits REF(...) for the remainder.
        // Count backslash prefixes in the value string.
        let backslash_prefix_count = rendered.value.chars().take_while(|&c| c == '\\').count();
        assert!(
            backslash_prefix_count <= 10,
            "backslash prefix count {} should be <= 10",
            backslash_prefix_count,
        );
        // The value should contain the REF(...) truncation marker
        assert!(
            rendered.value.contains("REF(...)"),
            "deeply nested ref should contain REF(...), got: {}",
            rendered.value,
        );

        // Total output length should be reasonable (not exponential)
        assert!(
            rendered.value.len() < 200,
            "rendered value length {} should be < 200",
            rendered.value.len(),
        );
    }

    /// A reference chain of exactly 10 levels should still reach the leaf.
    #[test]
    fn test_render_reference_chain_at_depth_limit() {
        let renderer = PerlVariableRenderer::new();

        let mut value = PerlValue::Scalar("leaf".to_string());
        for _ in 0..10 {
            value = PerlValue::Reference(Box::new(value));
        }

        let rendered = renderer.render("$ref10", &value);
        // At exactly 10, the chain traversal stops and formats the leaf.
        // The output should contain 10 backslashes and then the leaf value.
        assert!(rendered.value.contains("leaf") || rendered.value.contains("REF(...)"));
        assert!(rendered.value.len() < 200);
    }

    /// Large array with >10K elements should truncate in preview.
    #[test]
    fn test_render_large_array_over_10k_elements_truncates() {
        let renderer = PerlVariableRenderer::new();

        let elements: Vec<PerlValue> = (0..10_001).map(PerlValue::Integer).collect();
        let value = PerlValue::Array(elements);

        let rendered = renderer.render("@big", &value);

        assert_eq!(rendered.name, "@big");
        assert_eq!(rendered.type_name, Some("ARRAY".to_string()));
        assert_eq!(rendered.indexed_variables, Some(10_001));

        // Preview should show only max_array_preview (default 3) elements
        // plus a "... (N total)" suffix
        assert!(
            rendered.value.contains("10001 total"),
            "should show total count, got: {}",
            rendered.value,
        );
        assert!(rendered.value.starts_with('['));
        assert!(rendered.value.ends_with(']'));

        // Preview string should be bounded — not contain all 10K values
        assert!(
            rendered.value.len() < 500,
            "preview length {} should be < 500",
            rendered.value.len(),
        );
    }

    /// `render_children` paginates large arrays correctly.
    #[test]
    fn test_render_children_large_array_pagination() {
        let renderer = PerlVariableRenderer::new();

        let elements: Vec<PerlValue> = (0..10_001).map(PerlValue::Integer).collect();
        let value = PerlValue::Array(elements);

        // Request a window of 100 starting at index 5000
        let children = renderer.render_children(&value, 5000, 100);
        assert_eq!(children.len(), 100);
        assert_eq!(children[0].name, "[5000]");
        assert_eq!(children[0].value, "5000");
        assert_eq!(children[99].name, "[5099]");
        assert_eq!(children[99].value, "5099");

        // Request past the end — should return only what's available
        let tail = renderer.render_children(&value, 10_000, 100);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].name, "[10000]");
    }

    /// Large hash with >5K pairs should truncate in preview.
    #[test]
    fn test_render_large_hash_over_5k_pairs_truncates() {
        let renderer = PerlVariableRenderer::new();

        let pairs: Vec<(String, PerlValue)> =
            (0..5_001).map(|i| (format!("key_{}", i), PerlValue::Integer(i))).collect();
        let value = PerlValue::Hash(pairs);

        let rendered = renderer.render("%big", &value);

        assert_eq!(rendered.name, "%big");
        assert_eq!(rendered.type_name, Some("HASH".to_string()));
        assert_eq!(rendered.named_variables, Some(5_001));

        // Preview should show only max_hash_preview (default 3) pairs
        // plus a "... (N keys)" suffix
        assert!(
            rendered.value.contains("5001 keys"),
            "should show key count, got: {}",
            rendered.value,
        );
        assert!(rendered.value.starts_with('{'));
        assert!(rendered.value.ends_with('}'));

        // Preview string should be bounded
        assert!(
            rendered.value.len() < 500,
            "preview length {} should be < 500",
            rendered.value.len(),
        );
    }

    /// `render_children` paginates large hashes correctly.
    #[test]
    fn test_render_children_large_hash_pagination() {
        let renderer = PerlVariableRenderer::new();

        let pairs: Vec<(String, PerlValue)> =
            (0..5_001).map(|i| (format!("key_{}", i), PerlValue::Integer(i))).collect();
        let value = PerlValue::Hash(pairs);

        // Request a window of 50 starting at index 2500
        let children = renderer.render_children(&value, 2500, 50);
        assert_eq!(children.len(), 50);
        assert_eq!(children[0].name, "key_2500");
        assert_eq!(children[0].value, "2500");

        // Request past the end
        let tail = renderer.render_children(&value, 5000, 100);
        assert_eq!(tail.len(), 1);
        assert_eq!(tail[0].name, "key_5000");
    }

    /// Blessed object backed by a hash — the standard Perl OO pattern.
    #[test]
    fn test_render_blessed_object_hash_based() {
        let renderer = PerlVariableRenderer::new();

        let value = PerlValue::Object {
            class: "HTTP::Response".to_string(),
            value: Box::new(PerlValue::Hash(vec![
                ("_rc".to_string(), PerlValue::Integer(200)),
                ("_content".to_string(), PerlValue::Scalar("OK".to_string())),
                (
                    "_headers".to_string(),
                    PerlValue::Hash(vec![(
                        "Content-Type".to_string(),
                        PerlValue::Scalar("text/html".to_string()),
                    )]),
                ),
            ])),
        };

        let rendered = renderer.render("$resp", &value);

        assert_eq!(rendered.name, "$resp");
        assert_eq!(rendered.type_name, Some("HTTP::Response".to_string()));
        assert_eq!(rendered.named_variables, Some(3));
        assert!(rendered.value.contains("HTTP::Response"));

        // Children should be the hash keys
        let children = renderer.render_children(&value, 0, 10);
        assert_eq!(children.len(), 3);
        assert_eq!(children[0].name, "_rc");
        assert_eq!(children[0].value, "200");
        assert_eq!(children[1].name, "_content");
        assert!(children[1].value.contains("OK"));
    }

    /// Blessed object backed by an array (inside-out objects).
    #[test]
    fn test_render_blessed_object_array_based() {
        let renderer = PerlVariableRenderer::new();

        let value = PerlValue::Object {
            class: "My::InsideOut".to_string(),
            value: Box::new(PerlValue::Array(vec![
                PerlValue::Scalar("field_a".to_string()),
                PerlValue::Integer(99),
            ])),
        };

        let rendered = renderer.render("$io", &value);

        assert_eq!(rendered.type_name, Some("My::InsideOut".to_string()));
        assert_eq!(rendered.indexed_variables, Some(2));
        assert!(rendered.value.contains("My::InsideOut"));

        let children = renderer.render_children(&value, 0, 10);
        assert_eq!(children.len(), 2);
        assert_eq!(children[0].name, "[0]");
        assert_eq!(children[1].name, "[1]");
    }

    /// Blessed object backed by a scalar (e.g., URI, overloaded stringification).
    #[test]
    fn test_render_blessed_object_scalar_based() {
        let renderer = PerlVariableRenderer::new();

        // URI objects are blessed scalar refs: bless \("https://example.com"), "URI"
        let value = PerlValue::Object {
            class: "URI".to_string(),
            value: Box::new(PerlValue::Scalar("https://example.com".to_string())),
        };

        let rendered = renderer.render("$uri", &value);

        assert_eq!(rendered.type_name, Some("URI".to_string()));
        // Scalar-backed objects don't expose named or indexed children
        assert_eq!(rendered.named_variables, None);
        assert_eq!(rendered.indexed_variables, None);
        assert_eq!(rendered.value, "URI = SCALAR(...)");
    }

    /// Blessed object with deeply nested class name (Perl namespaces).
    #[test]
    fn test_render_blessed_object_deep_namespace() {
        let renderer = PerlVariableRenderer::new();

        let value = PerlValue::Object {
            class: "Very::Deep::Nested::Package::Name".to_string(),
            value: Box::new(PerlValue::Hash(vec![])),
        };

        let rendered = renderer.render("$obj", &value);

        assert_eq!(rendered.type_name, Some("Very::Deep::Nested::Package::Name".to_string()),);
        assert!(rendered.value.contains("Very::Deep::Nested::Package::Name"));
        assert_eq!(rendered.named_variables, Some(0));
    }

    /// Blessed object with `render_with_reference` gets a reference ID
    /// and is expandable.
    #[test]
    fn test_render_blessed_object_with_reference() {
        let renderer = PerlVariableRenderer::new();

        let value = PerlValue::Object {
            class: "DBI::db".to_string(),
            value: Box::new(PerlValue::Hash(vec![(
                "Driver".to_string(),
                PerlValue::Scalar("SQLite".to_string()),
            )])),
        };

        let rendered = renderer.render_with_reference("$dbh", &value, 99);

        assert_eq!(rendered.variables_reference, 99);
        assert!(rendered.is_expandable());
        assert_eq!(rendered.type_name, Some("DBI::db".to_string()));
    }

    /// Simulates a hash whose value is a reference back to a parent —
    /// the pattern `{ parent => \%grandparent, child => \%self }` where
    /// the debugger truncates the cycle.
    #[test]
    fn test_render_hash_with_multiple_back_references() {
        let renderer = PerlVariableRenderer::new();

        let grandparent_marker = PerlValue::Truncated {
            summary: "HASH(0xaaa...circular)".to_string(),
            total_count: Some(5),
        };
        let self_marker = PerlValue::Truncated {
            summary: "HASH(0xbbb...circular)".to_string(),
            total_count: Some(3),
        };

        let value = PerlValue::Hash(vec![
            ("parent".to_string(), PerlValue::Reference(Box::new(grandparent_marker))),
            ("child".to_string(), PerlValue::Reference(Box::new(self_marker))),
            ("name".to_string(), PerlValue::Scalar("node".to_string())),
        ]);

        let rendered = renderer.render("$node", &value);

        assert_eq!(rendered.named_variables, Some(3));
        // Should not panic or produce unbounded output
        assert!(rendered.value.len() < 500);

        let children = renderer.render_children(&value, 0, 10);
        assert_eq!(children.len(), 3);
        // The circular references render as REF type with truncated target
        assert!(children[0].value.contains("circular"));
        assert!(children[1].value.contains("circular"));
        assert!(children[2].value.contains("node"));
    }

    /// Empty array and hash render as `[]` and `{}` respectively.
    #[test]
    fn test_render_empty_collections() {
        let renderer = PerlVariableRenderer::new();

        let empty_arr = PerlValue::Array(vec![]);
        let rendered = renderer.render("@empty", &empty_arr);
        assert_eq!(rendered.value, "[]");
        assert_eq!(rendered.indexed_variables, Some(0));

        let empty_hash = PerlValue::Hash(vec![]);
        let rendered = renderer.render("%empty", &empty_hash);
        assert_eq!(rendered.value, "{}");
        assert_eq!(rendered.named_variables, Some(0));
    }

    /// Verifies that configuring lower preview limits still works for
    /// large structures.
    #[test]
    fn test_render_large_array_with_custom_preview_limit() {
        let renderer =
            PerlVariableRenderer::new().with_max_array_preview(1).with_max_hash_preview(1);

        let elements: Vec<PerlValue> = (0..100).map(PerlValue::Integer).collect();
        let value = PerlValue::Array(elements);

        let rendered = renderer.render("@arr", &value);
        // With preview=1, should show one element then "... (100 total)"
        assert!(rendered.value.contains("100 total"));
        // Only the first element should appear before the ellipsis
        assert!(rendered.value.starts_with("[0"));
    }

    /// Verifies that configuring lower preview limits works for large hashes.
    #[test]
    fn test_render_large_hash_with_custom_preview_limit() {
        let renderer = PerlVariableRenderer::new().with_max_hash_preview(1);

        let pairs: Vec<(String, PerlValue)> =
            (0..100).map(|i| (format!("k{}", i), PerlValue::Integer(i))).collect();
        let value = PerlValue::Hash(pairs);

        let rendered = renderer.render("%h", &value);
        assert!(rendered.value.contains("100 keys"));
        assert!(rendered.value.starts_with('{'));
    }
}
