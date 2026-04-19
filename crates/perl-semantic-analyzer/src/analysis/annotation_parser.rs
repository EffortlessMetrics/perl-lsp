//! Annotation parser for `# type: DBI::Row[...]` comments.
//!
//! Parses type hints embedded in Perl comments that enable DBI row type inference.
//! Format: `# type: DBI::Row[col1=>Type1, col2=>Type2, ...]`

use regex::Regex;

use super::type_inference::{PerlType, ScalarType};

/// Parses a `# type: DBI::Row[...]` annotation and extracts column types.
///
/// Returns `None` if the comment doesn't match the DBI::Row annotation pattern.
///
/// # Examples
///
/// ```
/// use perl_semantic_analyzer::analysis::annotation_parser::parse_dbi_row_annotation;
///
/// let comment = "# type: DBI::Row[id=>Int, name=>Str]";
/// let result = parse_dbi_row_annotation(comment);
/// assert!(result.is_some());
/// let keys = result.unwrap();
/// assert_eq!(keys.len(), 2);
/// ```
pub fn parse_dbi_row_annotation(comment: &str) -> Option<Vec<(String, PerlType)>> {
    // Pattern: # type: DBI::Row[...]
    let pattern = Regex::new(r#"#\s*type:\s*DBI::Row\[(.*)\]"#).ok()?;

    // Extract the content inside DBI::Row[...]
    let captures = pattern.captures(comment)?;
    let content = captures.get(1)?.as_str();

    // Parse key=>Type pairs
    parse_column_pairs(content)
}

/// Parses comma-separated `key=>Type` pairs.
///
/// Handles whitespace around commas and arrows.
fn parse_column_pairs(content: &str) -> Option<Vec<(String, PerlType)>> {
    let mut result = Vec::new();

    for pair_str in content.split(',') {
        let pair_str = pair_str.trim();
        if pair_str.is_empty() {
            continue;
        }

        // Parse "key=>Type" format
        if let Some((key, type_str)) = pair_str.split_once("=>") {
            let key = key.trim().to_string();
            let type_str = type_str.trim();

            if let Some(perl_type) = parse_type_string(type_str) {
                result.push((key, perl_type));
            }
        }
    }

    if result.is_empty() && !content.trim().is_empty() {
        // If we have content but couldn't parse any pairs, return None
        return None;
    }

    Some(result)
}

/// Parses a type string like "Int", "Str", "Bool" into a PerlType.
fn parse_type_string(type_str: &str) -> Option<PerlType> {
    match type_str {
        "Int" | "Integer" => Some(PerlType::Scalar(ScalarType::Integer)),
        "Str" | "String" => Some(PerlType::Scalar(ScalarType::String)),
        "Float" | "Num" | "Num()>" => Some(PerlType::Scalar(ScalarType::Float)),
        "Bool" | "Boolean" => Some(PerlType::Scalar(ScalarType::Boolean)),
        "Any" => Some(PerlType::Any),
        "Undef" => Some(PerlType::Scalar(ScalarType::Undef)),
        _ => Some(PerlType::Any),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_dbi_row_single_column() {
        let result = parse_dbi_row_annotation("# type: DBI::Row[id=>Int]");
        assert!(result.is_some());
        let keys = result.unwrap();
        assert_eq!(keys.len(), 1);
        assert_eq!(keys[0].0, "id");
        assert_eq!(keys[0].1, PerlType::Scalar(ScalarType::Integer));
    }

    #[test]
    fn test_parse_dbi_row_multiple_columns() {
        let result = parse_dbi_row_annotation("# type: DBI::Row[id=>Int, name=>Str, email=>Str]");
        assert!(result.is_some());
        let keys = result.unwrap();
        assert_eq!(keys.len(), 3);
        assert_eq!(keys[0].0, "id");
        assert_eq!(keys[0].1, PerlType::Scalar(ScalarType::Integer));
        assert_eq!(keys[1].0, "name");
        assert_eq!(keys[1].1, PerlType::Scalar(ScalarType::String));
        assert_eq!(keys[2].0, "email");
        assert_eq!(keys[2].1, PerlType::Scalar(ScalarType::String));
    }

    #[test]
    fn test_parse_dbi_row_with_spaces() {
        let result = parse_dbi_row_annotation("# type: DBI::Row[ id => Int , name => Str ]");
        assert!(result.is_some());
        let keys = result.unwrap();
        assert_eq!(keys.len(), 2);
    }

    #[test]
    fn test_parse_dbi_row_empty() {
        let result = parse_dbi_row_annotation("# type: DBI::Row[]");
        assert!(result.is_some());
        let keys = result.unwrap();
        assert!(keys.is_empty());
    }

    #[test]
    fn test_parse_dbi_row_invalid_format() {
        // Not a DBI::Row annotation
        let result = parse_dbi_row_annotation("# type: NotDBIRow[id=>Int]");
        assert!(result.is_none());

        // Empty comment
        let result = parse_dbi_row_annotation("");
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_type_string_various_types() {
        assert_eq!(parse_type_string("Int"), Some(PerlType::Scalar(ScalarType::Integer)));
        assert_eq!(parse_type_string("Str"), Some(PerlType::Scalar(ScalarType::String)));
        assert_eq!(parse_type_string("Bool"), Some(PerlType::Scalar(ScalarType::Boolean)));
        assert_eq!(parse_type_string("Float"), Some(PerlType::Scalar(ScalarType::Float)));
        assert_eq!(parse_type_string("Any"), Some(PerlType::Any));
        assert_eq!(parse_type_string("Unknown"), Some(PerlType::Any));
    }
}
