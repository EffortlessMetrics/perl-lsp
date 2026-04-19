//! Tests for method modifier go-to-definition navigation.
//!
//! Covers issue #3599: go-to-definition on the method name string inside
//! `before`/`after`/`around`/`override`/`augment` modifiers should navigate to
//! the `sub` being modified.

mod common;

type TestResult = Result<(), Box<dyn std::error::Error>>;

#[cfg(test)]
mod method_modifier_definition_tests {
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

    // ── before modifier ──────────────────────────────────────────────────

    /// Go-to-definition on `'save'` in `before 'save' => sub { }` must jump to
    /// the `sub save { }` definition.
    #[test]
    fn test_before_modifier_goto_def_jumps_to_sub() -> TestResult {
        // Line 0: package MyApp::User;
        // Line 1: use Moo;
        // Line 2: sub save { }
        // Line 3: before 'save' => sub { };
        let code = "package MyApp::User;\nuse Moo;\nsub save { }\nbefore 'save' => sub { };\n";
        let uri = "file:///myapp_user.pl";

        let resp = goto_def(code, uri, "save", 3)?;

        let (def_uri, def_line, _) = semantic::first_location(&resp)
            .ok_or("Expected goto-definition on 'save' in before modifier to find sub save")?;

        assert_eq!(def_uri, uri, "Definition should be in the same file");
        assert_eq!(
            def_line, 2,
            "Definition should point to line 2 (sub save), got line {def_line}"
        );
        Ok(())
    }

    /// Go-to-definition on `'save'` in `after 'save' => sub { }` must jump to
    /// the `sub save { }` definition.
    #[test]
    fn test_after_modifier_goto_def_jumps_to_sub() -> TestResult {
        // Line 0: package MyApp::User;
        // Line 1: use Moo;
        // Line 2: sub save { }
        // Line 3: after 'save' => sub { };
        let code = "package MyApp::User;\nuse Moo;\nsub save { }\nafter 'save' => sub { };\n";
        let uri = "file:///myapp_user_after.pl";

        let resp = goto_def(code, uri, "save", 3)?;

        let (def_uri, def_line, _) = semantic::first_location(&resp)
            .ok_or("Expected goto-definition on 'save' in after modifier to find sub save")?;

        assert_eq!(def_uri, uri, "Definition should be in the same file");
        assert_eq!(
            def_line, 2,
            "Definition should point to line 2 (sub save), got line {def_line}"
        );
        Ok(())
    }

    /// Go-to-definition on `'validate'` in `around 'validate' => sub { }` must jump
    /// to the `sub validate { }` definition.
    #[test]
    fn test_around_modifier_goto_def_jumps_to_sub() -> TestResult {
        // Line 0: package MyApp::User;
        // Line 1: use Moose;
        // Line 2: sub validate { }
        // Line 3: around 'validate' => sub { };
        let code =
            "package MyApp::User;\nuse Moose;\nsub validate { }\naround 'validate' => sub { };\n";
        let uri = "file:///myapp_user_around.pl";

        let resp = goto_def(code, uri, "validate", 3)?;

        let (def_uri, def_line, _) = semantic::first_location(&resp).ok_or(
            "Expected goto-definition on 'validate' in around modifier to find sub validate",
        )?;

        assert_eq!(def_uri, uri, "Definition should be in the same file");
        assert_eq!(
            def_line, 2,
            "Definition should point to line 2 (sub validate), got line {def_line}"
        );
        Ok(())
    }

    /// Go-to-definition on `'render'` in `override 'render' => sub { }` must jump
    /// to the `sub render { }` definition.
    #[test]
    fn test_override_modifier_goto_def_jumps_to_sub() -> TestResult {
        let code =
            "package MyApp::User;\nuse Moose;\nsub render { }\noverride 'render' => sub { };\n";
        let uri = "file:///myapp_user_override.pl";

        let resp = goto_def(code, uri, "render", 3)?;

        let (def_uri, def_line, _) = semantic::first_location(&resp).ok_or(
            "Expected goto-definition on 'render' in override modifier to find sub render",
        )?;

        assert_eq!(def_uri, uri, "Definition should be in the same file");
        assert_eq!(
            def_line, 2,
            "Definition should point to line 2 (sub render), got line {def_line}"
        );
        Ok(())
    }

    /// Go-to-definition on `'render'` in `augment 'render' => sub { }` must jump
    /// to the `sub render { }` definition.
    #[test]
    fn test_augment_modifier_goto_def_jumps_to_sub() -> TestResult {
        let code =
            "package MyApp::User;\nuse Moose;\nsub render { }\naugment 'render' => sub { };\n";
        let uri = "file:///myapp_user_augment.pl";

        let resp = goto_def(code, uri, "render", 3)?;

        let (def_uri, def_line, _) = semantic::first_location(&resp)
            .ok_or("Expected goto-definition on 'render' in augment modifier to find sub render")?;

        assert_eq!(def_uri, uri, "Definition should be in the same file");
        assert_eq!(
            def_line, 2,
            "Definition should point to line 2 (sub render), got line {def_line}"
        );
        Ok(())
    }

    /// Multiple modifiers on different methods: go-to-definition on each modifier's
    /// method name string must navigate to the correct `sub`.
    #[test]
    fn test_multiple_modifiers_each_goto_correct_sub() -> TestResult {
        // Line 0: package MyApp::Service;
        // Line 1: use Moo;
        // Line 2: sub create { }
        // Line 3: sub update { }
        // Line 4: before 'create' => sub { };
        // Line 5: after 'update' => sub { };
        let code = "package MyApp::Service;\nuse Moo;\nsub create { }\nsub update { }\nbefore 'create' => sub { };\nafter 'update' => sub { };\n";
        let uri = "file:///myapp_service.pl";

        // Test before 'create'
        let resp_create = goto_def(code, uri, "create", 4)?;
        let (_, create_line, _) = semantic::first_location(&resp_create)
            .ok_or("Expected goto-definition on 'create' in before modifier")?;
        assert_eq!(
            create_line, 2,
            "before 'create' should go to line 2, got {create_line}"
        );

        // Test after 'update'
        let resp_update = goto_def(code, uri, "update", 5)?;
        let (_, update_line, _) = semantic::first_location(&resp_update)
            .ok_or("Expected goto-definition on 'update' in after modifier")?;
        assert_eq!(
            update_line, 3,
            "after 'update' should go to line 3, got {update_line}"
        );

        Ok(())
    }
}
