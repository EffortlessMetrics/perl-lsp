//! Tests for go-to-definition support for indirect object syntax
//!
//! Validates AC3: LSP go-to-definition navigates to method definitions for indirect calls

mod common;

#[cfg(test)]
mod indirect_definition_tests {
    use crate::common::test_utils::TestServerBuilder;
    use serde_json::Value;

    /// Extract the first definition location from an LSP response.
    /// Returns (uri, line, character) for easier assertions.
    fn first_location(resp: &Value) -> Result<(String, u32, u32), Box<dyn std::error::Error>> {
        let arr = resp
            .get("result")
            .ok_or("missing result field")?
            .as_array()
            .ok_or("result is not an array")?;
        let first = arr.first().ok_or("result array is empty")?;
        let uri = first
            .get("uri")
            .ok_or("missing uri field")?
            .as_str()
            .ok_or("uri is not a string")?
            .to_string();
        let range = first.get("range").ok_or("missing range field")?;
        let start = &range["start"];
        let line =
            start.get("line").ok_or("missing line field")?.as_u64().ok_or("line is not a number")?
                as u32;
        let character = start
            .get("character")
            .ok_or("missing character field")?
            .as_u64()
            .ok_or("character is not a number")? as u32;
        Ok((uri, line, character))
    }

    /// Compute (line, character) for a given `needle` on a specific `target_line`.
    fn find_pos(
        code: &str,
        needle: &str,
        target_line: usize,
    ) -> Result<(u32, u32), Box<dyn std::error::Error>> {
        let line = code
            .lines()
            .nth(target_line)
            .ok_or_else(|| format!("no line {} in test code", target_line))?;
        let col = line
            .find(needle)
            .ok_or_else(|| format!("could not find `{needle}` on line {target_line}"))?;
        Ok((target_line as u32, col as u32))
    }

    /// Test go-to-definition for indirect method call within same file
    #[test]
    fn test_indirect_call_goto_definition_same_file() -> Result<(), Box<dyn std::error::Error>> {
        // Define a simple Perl file with an indirect method call
        let source = r#"package MyClass;

sub new {
    my $class = shift;
    return bless {}, $class;
}

sub move {
    my ($self, $x, $y) = @_;
    print "Moving to ($x, $y)\n";
}

package main;
my $obj = MyClass->new();
move $obj 10, 20;
"#;

        let uri = "file:///test_indirect.pl";
        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, source);

        // Find position of "move" in "move $obj 10, 20"
        let (line, character) = find_pos(source, "move $obj", 14)?;
        let response = server.get_definition(uri, line, character);
        eprintln!("Definition result: {response:#}");

        // Should navigate to the move sub definition
        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        // Line 7 (0-indexed) is where "sub move" is defined
        assert_eq!(def_line, 7, "Should point to line where 'sub move' is defined");

        Ok(())
    }

    /// Test go-to-definition for builtin indirect syntax (print $fh)
    #[test]
    fn test_builtin_indirect_goto_definition() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"package main;
open my $fh, '<', 'test.txt' or die;
print $fh "Hello\n";
close $fh;
"#;

        let uri = "file:///test_builtin_indirect.pl";
        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, source);

        // Try to find definition for "print" in "print $fh"
        let (line, character) = find_pos(source, "print $fh", 2)?;
        let response = server.get_definition(uri, line, character);
        eprintln!("Builtin definition result: {response:#}");

        // Builtins typically don't have user-defined definitions, so we expect empty result
        // This test just ensures we don't crash
        let result = response.get("result");
        assert!(result.is_some(), "should have result field even if empty");

        Ok(())
    }

    /// Test go-to-definition for indirect constructor call
    #[test]
    fn test_indirect_constructor_goto_definition() -> Result<(), Box<dyn std::error::Error>> {
        let source = r#"package Player;

sub new {
    my ($class, $name) = @_;
    return bless { name => $name }, $class;
}

package main;
my $player = new Player "Alice";
"#;

        let uri = "file:///test_constructor.pl";
        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, source);

        // Find position of "new" in "new Player"
        let (line, character) = find_pos(source, "new Player", 8)?;
        let response = server.get_definition(uri, line, character);
        eprintln!("Constructor definition result: {response:#}");

        let (def_uri, def_line, _def_char) = first_location(&response)?;

        assert_eq!(def_uri, uri, "definition should be in same file");
        // Line 2 (0-indexed) is where "sub new" is defined
        assert_eq!(def_line, 2, "Should point to line where 'sub new' is defined");

        Ok(())
    }
}
