//! UTF-16 position tests for workspace index
//!
//! These tests ensure that the workspace index correctly handles UTF-16
//! positioning as required by the LSP specification, especially for:
//! - Emoji characters (multiple UTF-16 code units)
//! - Non-BMP characters
//! - Mixed ASCII and non-ASCII text

#[cfg(test)]
mod tests {
    use crate::workspace_index::{SymbolKind, WorkspaceIndex};
    use anyhow::{anyhow, Result};

    #[test]
    fn test_utf16_positions_with_emoji() -> Result<()> {
        let index = WorkspaceIndex::new();
        let uri = "file:///emoji_test.pl";

        // Code with emoji and non-ASCII characters
        // Lines are 0-indexed, columns are UTF-16 code units
        let code = r#"#!/usr/bin/perl
# This file has emoji! 🎉🎊
use strict;

package Emoji📦;  # Package with emoji

sub celebrate🎉 {  # Function with emoji
    my $message = "Hello 世界 🌍";  # Mixed scripts
    print $message;
}

my $♥ = 'love';  # Heart variable
my $π = 3.14159;  # Greek letter pi
"#;

        index.index_file(url::Url::parse(uri)?, code.to_string()).map_err(|e| anyhow!(e))?;

        let symbols = index.file_symbols(uri);

        // Find specific symbols and check their positions
        let package = symbols
            .iter()
            .find(|s| s.name == "Emoji📦" && s.kind == SymbolKind::Package)
            .ok_or_else(|| anyhow!("Should find emoji package"))?;

        // Line 4 (0-indexed), "package " is 8 chars, starts at col 0
        assert_eq!(package.range.start.line, 4);
        assert_eq!(package.range.start.column, 0);

        let sub = symbols
            .iter()
            .find(|s| s.name == "celebrate🎉" && s.kind == SymbolKind::Subroutine)
            .ok_or_else(|| anyhow!("Should find celebrate function"))?;

        // Line 6, "sub " is 4 chars
        assert_eq!(sub.range.start.line, 6);
        assert_eq!(sub.range.start.column, 0);

        let heart_var = symbols
            .iter()
            .find(|s| s.name == "$♥" && s.kind.is_variable())
            .ok_or_else(|| anyhow!("Should find heart variable"))?;

        // Line 11, "my " is 3 chars
        assert_eq!(heart_var.range.start.line, 11);
        assert_eq!(heart_var.range.start.column, 3);

        let pi_var = symbols
            .iter()
            .find(|s| s.name == "$π" && s.kind.is_variable())
            .ok_or_else(|| anyhow!("Should find pi variable"))?;

        // Line 12
        assert_eq!(pi_var.range.start.line, 12);
        assert_eq!(pi_var.range.start.column, 3);

        Ok(())
    }

    #[test]
    fn test_utf16_column_calculation() -> Result<()> {
        let index = WorkspaceIndex::new();
        let uri = "file:///utf16_test.pl";

        // Test specific UTF-16 column calculations
        // 'a' = 1 UTF-16 unit
        // '😀' = 2 UTF-16 units (surrogate pair)
        // '世' = 1 UTF-16 unit (BMP character)
        let code = r#"my $a = 1;  # ASCII only, column 3
my $😀 = 2;  # Emoji at column 3, takes 2 UTF-16 units
my $世 = 3;  # CJK at column 3, takes 1 UTF-16 unit
"#;

        index.index_file(url::Url::parse(uri)?, code.to_string()).map_err(|e| anyhow!(e))?;

        let symbols = index.file_symbols(uri);

        // Check each variable's position
        let var_a = symbols.iter().find(|s| s.name == "$a").ok_or_else(|| anyhow!("Should find $a"))?;
        assert_eq!(var_a.range.start.column, 3); // "my " = 3 units

        let var_emoji = symbols
            .iter()
            .find(|s| s.name == "$😀")
            .ok_or_else(|| anyhow!("Should find emoji variable"))?;
        assert_eq!(var_emoji.range.start.column, 3); // "my " = 3 units

        let var_cjk =
            symbols.iter().find(|s| s.name == "$世").ok_or_else(|| anyhow!("Should find CJK variable"))?;
        assert_eq!(var_cjk.range.start.column, 3); // "my " = 3 units

        Ok(())
    }

    #[test]
    fn test_multi_package_in_file() -> Result<()> {
        let index = WorkspaceIndex::new();
        let uri = "file:///multi_package.pl";

        let code = r#"package First;

sub first_sub {
    return "first";
}

package Second;

sub second_sub {
    return "second";
}

package Third::Nested;

sub third_sub {
    return "third";
}
"#;

        index.index_file(url::Url::parse(uri)?, code.to_string()).map_err(|e| anyhow!(e))?;

        let symbols = index.file_symbols(uri);

        // Should have all three packages
        assert!(symbols.iter().any(|s| s.name == "First" && s.kind == SymbolKind::Package));
        assert!(symbols.iter().any(|s| s.name == "Second" && s.kind == SymbolKind::Package));
        assert!(symbols.iter().any(|s| s.name == "Third::Nested" && s.kind == SymbolKind::Package));

        // Subroutines should be qualified with their package
        let first_sub = symbols
            .iter()
            .find(|s| s.name == "first_sub" && s.kind == SymbolKind::Subroutine)
            .ok_or_else(|| anyhow!("Should find first_sub"))?;
        assert_eq!(first_sub.qualified_name, Some("First::first_sub".to_string()));

        let second_sub = symbols
            .iter()
            .find(|s| s.name == "second_sub" && s.kind == SymbolKind::Subroutine)
            .ok_or_else(|| anyhow!("Should find second_sub"))?;
        assert_eq!(second_sub.qualified_name, Some("Second::second_sub".to_string()));

        let third_sub = symbols
            .iter()
            .find(|s| s.name == "third_sub" && s.kind == SymbolKind::Subroutine)
            .ok_or_else(|| anyhow!("Should find third_sub"))?;
        assert_eq!(third_sub.qualified_name, Some("Third::Nested::third_sub".to_string()));

        Ok(())
    }

    #[test]
    fn test_read_write_tracking() -> Result<()> {
        let index = WorkspaceIndex::new();
        let uri = "file:///read_write.pl";

        let code = r#"my $x;           # Definition
$x = 5;          # Write
$x++;            # Read and Write
my $y = $x + 1;  # Read $x
print $x;        # Read
"#;

        index.index_file(url::Url::parse(uri)?, code.to_string()).map_err(|e| anyhow!(e))?;

        let refs = index.find_references("$x");

        // Should have multiple references for $x
        assert!(refs.len() >= 4, "Should have at least 4 references to $x");

        // Check that we found references at different positions
        let lines: Vec<_> = refs.iter().map(|r| r.range.start.line).collect();
        assert!(lines.contains(&0), "Should have reference on line 0 (definition)");
        assert!(lines.contains(&1), "Should have reference on line 1 (assignment)");
        assert!(lines.contains(&3), "Should have reference on line 3 (read in expression)");
        assert!(lines.contains(&4), "Should have reference on line 4 (print)");

        Ok(())
    }

    #[test]
    fn test_document_update_reindex() -> Result<()> {
        let index = WorkspaceIndex::new();
        let uri = "file:///update_test.pl";

        // Initial version
        let code_v1 = r#"sub old_name {
    return 42;
}"#;

        index.index_file(url::Url::parse(uri)?, code_v1.to_string()).map_err(|e| anyhow!(e))?;

        let symbols_v1 = index.file_symbols(uri);
        assert!(symbols_v1.iter().any(|s| s.name == "old_name"));
        assert!(!symbols_v1.iter().any(|s| s.name == "new_name"));

        // Updated version - renamed function
        let code_v2 = r#"sub new_name {
    return 42;
}"#;

        index.index_file(url::Url::parse(uri)?, code_v2.to_string()).map_err(|e| anyhow!(e))?;

        let symbols_v2 = index.file_symbols(uri);
        assert!(!symbols_v2.iter().any(|s| s.name == "old_name"), "old_name should be gone");
        assert!(symbols_v2.iter().any(|s| s.name == "new_name"), "new_name should exist");

        Ok(())
    }

    #[test]
    fn test_clear_file() -> Result<()> {
        let index = WorkspaceIndex::new();
        let uri = "file:///clear_test.pl";

        let code = r#"package Test;
sub test_sub { }"#;

        index.index_file(url::Url::parse(uri)?, code.to_string()).map_err(|e| anyhow!(e))?;

        // Verify symbols exist
        let symbols = index.file_symbols(uri);
        assert!(!symbols.is_empty());

        // Clear the file
        index.clear_file(uri);

        // Verify symbols are gone
        let symbols_after = index.file_symbols(uri);
        assert!(symbols_after.is_empty());

        // Verify definition lookup also fails
        assert!(index.find_definition("Test").is_none());
        assert!(index.find_definition("test_sub").is_none());

        Ok(())
    }
}
