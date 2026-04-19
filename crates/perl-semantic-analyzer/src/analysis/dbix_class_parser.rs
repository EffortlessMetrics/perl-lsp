//! DBIx::Class Result Class Parser
//!
//! Parses DBIx::Class result class files to extract table names and column definitions.
//! This enables type inference for `->search()`, `->first()`, and `->find()` calls.
//!
//! # Example
//!
//! ```perl
//! package MyApp::Schema::Result::User;
//! __PACKAGE__->table("users");
//! __PACKAGE__->add_columns(
//!     id => { data_type => "integer", is_nullable => 0 },
//!     name => { data_type => "varchar", size => 255 },
//! );
//! __PACKAGE__->set_primary_key("id");
//! ```

use regex::Regex;
use std::collections::HashMap;

/// Information about a DBIx::Class result class
#[derive(Debug, Clone)]
pub struct ResultClassInfo {
    /// The package name (e.g., "MyApp::Schema::Result::User")
    pub package_name: String,
    /// The database table name (e.g., "users")
    pub table_name: Option<String>,
    /// Column definitions
    pub columns: Vec<ColumnInfo>,
}

/// Information about a single column in a result class
#[derive(Debug, Clone)]
pub struct ColumnInfo {
    /// Column name (e.g., "id", "name", "email")
    pub name: String,
    /// Data type as declared (e.g., "integer", "varchar")
    pub data_type: String,
    /// Whether the column is nullable
    pub is_nullable: bool,
}

/// Parses a DBIx::Class result class file and extracts table and column information.
///
/// Returns `None` if the source doesn't appear to be a DBIx::Class result class.
pub fn parse_result_class(source: &str) -> Option<ResultClassInfo> {
    // Extract package name
    let package_name = extract_package_name(source)?;

    // Only process if it looks like a DBIx::Class result class
    if !source.contains("__PACKAGE__->table") && !source.contains("__PACKAGE__->add_columns") {
        return None;
    }

    // Extract table name
    let table_name = extract_table_name(source);

    // Extract columns
    let columns = extract_columns(source);

    Some(ResultClassInfo { package_name, table_name, columns })
}

/// Extracts the package name from source code
fn extract_package_name(source: &str) -> Option<String> {
    let re = Regex::new(r#"(?m)^\s*package\s+([\w:]+)\s*;"#).ok()?;
    for line in source.lines() {
        if let Some(caps) = re.captures(line) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}

/// Extracts the table name from `__PACKAGE__->table("...")` calls
fn extract_table_name(source: &str) -> Option<String> {
    let re = Regex::new(r##"__PACKAGE__\->table\s*\(\s*["']([^"']+)["']\s*\)"##).ok()?;
    for line in source.lines() {
        if let Some(caps) = re.captures(line) {
            if let Some(m) = caps.get(1) {
                return Some(m.as_str().to_string());
            }
        }
    }
    None
}

/// Extracts column definitions from `__PACKAGE__->add_columns(...)` calls
fn extract_columns(source: &str) -> Vec<ColumnInfo> {
    let col_re = Regex::new(r#"(\w+)\s*=>\s*\{\s*data_type\s*=>\s*["']([^"']+)["'].*?\}"#).ok();
    let nullable_re = Regex::new(r#"is_nullable\s*=>\s*(\d+|false|true)"#).ok();
    let add_cols_re = Regex::new(r#"__PACKAGE__\->add_columns\s*\("#).ok();

    let mut columns = Vec::new();
    let mut in_add_columns = false;
    let mut bracket_depth = 0;

    for line in source.lines() {
        if !in_add_columns {
            if let Some(ref re) = add_cols_re {
                if re.is_match(line) {
                    in_add_columns = true;
                    bracket_depth = 0;
                }
            }
        }

        if in_add_columns {
            for ch in line.chars() {
                match ch {
                    '(' | '{' => bracket_depth += 1,
                    ')' | '}' => bracket_depth -= 1,
                    _ => {}
                }
            }

            // Extract column definitions from this line
            if let Some(ref re) = col_re {
                for caps in re.captures_iter(line) {
                    let name = caps.get(1).map(|m| m.as_str().to_string());
                    let data_type = caps.get(2).map(|m| m.as_str().to_string());

                    if let (Some(n), Some(dt)) = (name, data_type) {
                        // Check if nullable
                        let is_nullable = nullable_re
                            .as_ref()
                            .and_then(|re| re.find(line))
                            .map(|m| m.as_str().contains("=> 1") || m.as_str().contains("=> true"))
                            .unwrap_or(true);

                        columns.push(ColumnInfo { name: n, data_type: dt, is_nullable });
                    }
                }
            }

            // End of add_columns block
            if bracket_depth == 0 && line.contains(')') {
                in_add_columns = false;
            }
        }
    }

    columns
}

/// Builds a map of result class names to their info
#[allow(dead_code)]
pub fn build_result_class_map(sources: &[(&str, &str)]) -> HashMap<String, ResultClassInfo> {
    let mut map = HashMap::new();
    for (package_name, source) in sources {
        if let Some(info) = parse_result_class(source) {
            // Use the parsed package name if it matches, otherwise use the provided one
            let key = if info.package_name == *package_name {
                info.package_name.clone()
            } else {
                (*package_name).to_string()
            };
            map.insert(key, info);
        }
    }
    map
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_basic_result_class() {
        let source = r#"
package MyApp::Schema::Result::User;
__PACKAGE__->table("users");
__PACKAGE__->add_columns(
    id => { data_type => "integer", is_nullable => 0 },
    name => { data_type => "varchar", size => 255 },
);
__PACKAGE__->set_primary_key("id");
"#;

        let result = parse_result_class(source);
        assert!(result.is_some());

        let info = result.unwrap();
        assert_eq!(info.package_name, "MyApp::Schema::Result::User");
        assert_eq!(info.table_name, Some("users".to_string()));
        assert_eq!(info.columns.len(), 2);

        assert_eq!(info.columns[0].name, "id");
        assert_eq!(info.columns[0].data_type, "integer");
        assert!(!info.columns[0].is_nullable);

        assert_eq!(info.columns[1].name, "name");
        assert_eq!(info.columns[1].data_type, "varchar");
    }

    #[test]
    fn test_parse_result_class_with_nullable() {
        let source = r#"
package MyApp::Schema::Result::User;
__PACKAGE__->table("users");
__PACKAGE__->add_columns(
    id => { data_type => "integer", is_nullable => 0 },
    email => { data_type => "varchar", is_nullable => 1 },
);
__PACKAGE__->set_primary_key("id");
"#;

        let result = parse_result_class(source).unwrap();
        assert_eq!(result.columns.len(), 2);
        assert!(!result.columns[0].is_nullable);
        assert!(result.columns[1].is_nullable);
    }

    #[test]
    fn test_parse_result_class_not_dbix_class() {
        let source = r#"
package MyApp::SomeClass;
sub new { bless {}, shift }
"#;

        let result = parse_result_class(source);
        assert!(result.is_none());
    }

    #[test]
    fn test_parse_result_class_empty_columns() {
        let source = r#"
package MyApp::Schema::Result::Empty;
__PACKAGE__->table("empty");
__PACKAGE__->add_columns();
"#;

        let result = parse_result_class(source).unwrap();
        assert_eq!(result.columns.len(), 0);
    }

    #[test]
    fn test_extract_package_name() {
        let source = r#"
package MyApp::Schema::Result::User;
"#;
        assert_eq!(extract_package_name(source), Some("MyApp::Schema::Result::User".to_string()));
    }

    #[test]
    fn test_extract_table_name() {
        let source = r#"
__PACKAGE__->table("users");
"#;
        assert_eq!(extract_table_name(source), Some("users".to_string()));
    }
}
