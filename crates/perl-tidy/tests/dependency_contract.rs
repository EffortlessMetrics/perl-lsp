#[test]
fn native_manifest_excludes_protocol_and_subprocess_dependencies() {
    let manifest = include_str!("../Cargo.toml");
    for forbidden in ["lsp-types", "perl-lsp-", "perl-subprocess-runtime"] {
        assert!(!manifest.contains(forbidden), "perl-tidy must not depend on {forbidden}");
    }
}
