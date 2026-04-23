//! Focused go-to-definition tests for Dancer and Dancer2 route targets.

mod common;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(test)]
mod dancer_navigation_tests {
    use super::TestResult;
    use crate::common::test_utils::{TestServerBuilder, semantic};

    fn goto_def(
        code: &str,
        uri: &str,
        needle: &str,
        target_line: usize,
    ) -> Result<serde_json::Value, Box<dyn std::error::Error>> {
        let server = TestServerBuilder::new().build();
        server.open_document(uri, code);
        let (line, character) = semantic::find_pos(code, needle, target_line);
        Ok(server.get_definition(uri, line, character))
    }

    #[test]
    fn dancer_route_target_definitions_to_named_sub() -> TestResult {
        let code =
            "use Dancer;\nget '/about' => 'show_about';\nsub show_about { return 'About'; }\n";
        let uri = "file:///dancer_route_target.pl";

        let resp = goto_def(code, uri, "show_about", 1)?;
        let (def_uri, def_line, _) = semantic::first_location(&resp)
            .ok_or("Expected goto-definition to resolve the Dancer route target")?;

        assert_eq!(def_uri, uri, "Definition should stay in the same file");
        assert_eq!(def_line, 2, "Definition should point to the named sub handler");
        Ok(())
    }

    #[test]
    fn dancer2_route_target_definitions_to_named_sub() -> TestResult {
        let code =
            "use Dancer2;\nget '/status' => 'show_status';\nsub show_status { return 'ok'; }\n";
        let uri = "file:///dancer2_route_target.pl";

        let resp = goto_def(code, uri, "show_status", 1)?;
        let (def_uri, def_line, _) = semantic::first_location(&resp)
            .ok_or("Expected goto-definition to resolve the Dancer2 route target")?;

        assert_eq!(def_uri, uri, "Definition should stay in the same file");
        assert_eq!(def_line, 2, "Definition should point to the named sub handler");
        Ok(())
    }
}
