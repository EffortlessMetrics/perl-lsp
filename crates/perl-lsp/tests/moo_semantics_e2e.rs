//! End-to-end semantic receipts for Moo/Moose and Class::Accessor idioms.

mod common;

#[cfg(test)]
mod moo_semantics_e2e_tests {
    use crate::common::test_utils::{TestServerBuilder, semantic};
    use serde_json::Value;

    fn completion_items(response: &Value) -> Option<&Vec<Value>> {
        response["result"]["items"].as_array().or_else(|| response["result"].as_array())
    }

    #[test]
    fn moo_has_accessors_complete_and_resolve_definition() -> Result<(), Box<dyn std::error::Error>>
    {
        let code = r#"package Demo::User;
use Moo;
has 'name' => (is => 'ro', isa => 'Str');

sub greet {
    my $self = shift;
    my $method = $self->name;
    return name();
}
"#;
        let uri = "file:///moo_semantics.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        let completion_line = code
            .lines()
            .position(|line| line.contains("$self->"))
            .ok_or("completion line not found")?;
        let completion_char = code
            .lines()
            .nth(completion_line)
            .and_then(|line| line.find("$self->name"))
            .map(|idx| idx + "$self->".len())
            .ok_or("completion position not found")?;

        let completion_response =
            server.get_completion(uri, completion_line as u32, completion_char as u32);
        let items = completion_items(&completion_response).ok_or("missing completion items")?;
        assert!(
            items.iter().any(|item| item["label"] == "name"),
            "expected `name` accessor completion, got: {completion_response:#}"
        );

        let call_line = code
            .lines()
            .position(|line| line.contains("name();"))
            .ok_or("definition call line not found")?;
        let call_char = code
            .lines()
            .nth(call_line)
            .and_then(|line| line.find("name()"))
            .ok_or("definition call position not found")?;
        let expected_def_line =
            code.lines().position(|line| line.contains("has 'name'")).ok_or("has line missing")?
                as u32;

        let definition_response = server.get_definition(uri, call_line as u32, call_char as u32);
        let (_, def_line, _) =
            semantic::first_location(&definition_response).ok_or("definition result missing")?;
        assert_eq!(
            def_line, expected_def_line,
            "definition should resolve to Moo `has` declaration line"
        );

        server.shutdown();
        Ok(())
    }

    #[test]
    fn class_accessor_methods_complete_and_resolve_definition()
    -> Result<(), Box<dyn std::error::Error>> {
        let code = r#"package Demo::Classy;
use parent 'Class::Accessor';
__PACKAGE__->mk_accessors(qw(foo bar));

sub run {
    my $self = shift;
    my $method = $self->foo;
    return foo();
}
"#;
        let uri = "file:///class_accessor_semantics.pl";

        let mut server = TestServerBuilder::new().build();
        server.open_document(uri, code);

        let completion_line = code
            .lines()
            .position(|line| line.contains("$self->"))
            .ok_or("completion line not found")?;
        let completion_char = code
            .lines()
            .nth(completion_line)
            .and_then(|line| line.find("$self->foo"))
            .map(|idx| idx + "$self->".len())
            .ok_or("completion position not found")?;

        let completion_response =
            server.get_completion(uri, completion_line as u32, completion_char as u32);
        let items = completion_items(&completion_response).ok_or("missing completion items")?;
        assert!(
            items.iter().any(|item| item["label"] == "foo"),
            "expected `foo` completion, got: {completion_response:#}"
        );
        assert!(
            items.iter().any(|item| item["label"] == "bar"),
            "expected `bar` completion, got: {completion_response:#}"
        );

        let call_line = code
            .lines()
            .position(|line| line.contains("foo();"))
            .ok_or("definition call line not found")?;
        let call_char = code
            .lines()
            .nth(call_line)
            .and_then(|line| line.find("foo()"))
            .ok_or("definition call position not found")?;
        let expected_def_line = code
            .lines()
            .position(|line| line.contains("mk_accessors"))
            .ok_or("mk_accessors line missing")? as u32;

        let definition_response = server.get_definition(uri, call_line as u32, call_char as u32);
        let (_, def_line, _) =
            semantic::first_location(&definition_response).ok_or("definition result missing")?;
        assert_eq!(
            def_line, expected_def_line,
            "definition should resolve to Class::Accessor generator line"
        );

        server.shutdown();
        Ok(())
    }
}
