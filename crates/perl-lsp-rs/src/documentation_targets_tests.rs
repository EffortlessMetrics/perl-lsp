use super::*;
use perl_tdd_support::must_some;

#[test]
fn from_perldoc_uri_parses_valid_perldoc_uri() {
    let target = must_some(PerlDocumentationTarget::from_perldoc_uri("perldoc://Local::Doc"));

    assert_eq!(target.name(), "Local::Doc");
    assert_eq!(&target.perldoc_uri(), "perldoc://Local::Doc");
}

#[test]
fn from_perldoc_uri_accepts_already_trimmed_name_boundary() {
    let name = "Local::Trimmed";
    assert!(name == name.trim(), "name == name.trim() must take the accepted URI branch",);
    let uri = format!("perldoc://{name}");

    let target = must_some(PerlDocumentationTarget::from_perldoc_uri(&uri));

    assert_eq!(target.name(), name);
    assert_eq!(&target.perldoc_uri(), "perldoc://Local::Trimmed");
}

#[test]
fn from_perldoc_uri_rejects_non_perldoc_scheme() {
    assert!(
        PerlDocumentationTarget::from_perldoc_uri("https://metacpan.org/pod/Local::Doc").is_none(),
        "expected non-perldoc scheme to be rejected",
    );
}

#[test]
fn from_perldoc_uri_rejects_whitespace_instead_of_normalizing() {
    for uri in ["perldoc:// Local::Doc", "perldoc://Local::Doc "] {
        assert!(
            PerlDocumentationTarget::from_perldoc_uri(uri).is_none(),
            "expected whitespace-bearing URI {uri} to be rejected",
        );
    }
}

#[test]
fn from_perldoc_uri_rejects_malformed_target_names() {
    for uri in ["perldoc://", "perldoc://Local/Doc", "perldoc://Local::>"] {
        assert!(
            PerlDocumentationTarget::from_perldoc_uri(uri).is_none(),
            "expected malformed URI {uri} to be rejected",
        );
    }
}

#[test]
fn from_simple_pod_link_target_extracts_bare_module_target() {
    let bare = must_some(PerlDocumentationTarget::from_simple_pod_link_target("  Local::Doc  "));

    assert_eq!(&bare.perldoc_uri(), "perldoc://Local::Doc");
}

#[test]
fn from_simple_pod_link_target_extracts_labeled_module_target() {
    let labeled =
        must_some(PerlDocumentationTarget::from_simple_pod_link_target("docs|  Local::Labeled  "));

    assert_eq!(&labeled.perldoc_uri(), "perldoc://Local::Labeled");
}

#[test]
fn from_simple_pod_link_target_accepts_core_pragma_targets() {
    let bare = must_some(PerlDocumentationTarget::from_simple_pod_link_target("strict"));
    let labeled =
        must_some(PerlDocumentationTarget::from_simple_pod_link_target("warnings docs|warnings"));

    assert_eq!(&bare.perldoc_uri(), "perldoc://strict");
    assert_eq!(&labeled.perldoc_uri(), "perldoc://warnings");
}

#[test]
fn perl_doc_name_segment_accepts_alphanumeric_and_underscore_tail() {
    let mut chars = "Doc_2".chars();
    assert_eq!(chars.next(), Some('D'));
    assert!(
        chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_'),
        "chars.all(|ch| ch.is_ascii_alphanumeric() || ch == '_') must accept tail characters",
    );
    assert!(is_perl_doc_name_segment("Doc_2"));
    assert!(!is_perl_doc_name_segment("Doc-2"));
}

#[test]
fn from_simple_pod_link_target_rejects_empty_labels() {
    for target in ["|Local::Doc", "   |Local::Doc"] {
        assert!(
            PerlDocumentationTarget::from_simple_pod_link_target(target).is_none(),
            "expected empty-label target {target} to be rejected",
        );
    }
}

#[test]
fn from_simple_pod_link_target_rejects_non_module_targets() {
    for target in
        ["/section", "docs|/section", "NotAModule", "docs|Broken::", "docs|https://example.invalid"]
    {
        assert!(
            PerlDocumentationTarget::from_simple_pod_link_target(target).is_none(),
            "expected non-module target {target} to be rejected",
        );
    }
}
