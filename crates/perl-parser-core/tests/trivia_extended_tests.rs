use perl_parser_core::trivia::Trivia;

#[test]
fn pod_comment_variant() -> Result<(), Box<dyn std::error::Error>> {
    let trivia = Trivia::PodComment("=head1 NAME\n\nMy Module\n\n=cut".to_string());
    assert_eq!(trivia.as_str(), "=head1 NAME\n\nMy Module\n\n=cut");
    assert_eq!(trivia.kind_name(), "pod");
    Ok(())
}

#[test]
fn kind_name_for_all_variants() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(
        Trivia::Whitespace("  ".to_string()).kind_name(),
        "whitespace"
    );
    assert_eq!(
        Trivia::LineComment("# comment".to_string()).kind_name(),
        "comment"
    );
    assert_eq!(Trivia::PodComment("=pod".to_string()).kind_name(), "pod");
    assert_eq!(Trivia::Newline.kind_name(), "newline");
    Ok(())
}

#[test]
fn trivia_as_str_newline() -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(Trivia::Newline.as_str(), "\n");
    Ok(())
}

#[test]
fn trivia_clone_equality() -> Result<(), Box<dyn std::error::Error>> {
    let t = Trivia::Whitespace("\t".to_string());
    let cloned = t.clone();
    assert_eq!(t, cloned);
    Ok(())
}

#[test]
fn trivia_different_variants_not_equal() -> Result<(), Box<dyn std::error::Error>> {
    let ws = Trivia::Whitespace(" ".to_string());
    let nl = Trivia::Newline;
    assert_ne!(ws, nl);
    Ok(())
}
