use perl_parser::{ErrorCategory, ErrorClass, ParseError};

#[test]
fn parser_facade_exposes_error_classification() {
    assert_eq!(ParseError::Cancelled.error_class(), ErrorCategory::Transient);
    assert_eq!(ParseError::UnexpectedEof.error_class(), ErrorCategory::UserError);
    assert_eq!(ParseError::RecursionLimit.error_class(), ErrorCategory::ResourceLimit);
    assert_eq!(
        ParseError::Advisory { message: "valid warning".to_string(), location: 0 }.error_class(),
        ErrorCategory::Advisory
    );
}
