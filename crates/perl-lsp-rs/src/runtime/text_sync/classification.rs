const TEMPLATE_EXTENSIONS: [&str; 4] = ["ep", "tt", "tt2", "mason"];

pub(super) fn is_embedded_template_uri(uri: &str) -> bool {
    perl_uri::uri_extension(uri)
        .is_some_and(|ext| TEMPLATE_EXTENSIONS.iter().any(|template| template.eq_ignore_ascii_case(ext)))
}

pub(super) fn is_perl_language_id(language_id: &str) -> bool {
    matches!(
        language_id.to_ascii_lowercase().as_str(),
        "perl" | "perl5" | "perl-cpanfile" | "embedded-perl" | "mojolicious"
    )
}
