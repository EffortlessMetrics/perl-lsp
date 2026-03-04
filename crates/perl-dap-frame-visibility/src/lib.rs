//! Frame visibility classification for Perl DAP stack traces.

/// Returns true when a frame belongs to debugger/shim internals.
#[must_use]
pub fn is_internal_frame_name_and_path(name: &str, path: Option<&str>) -> bool {
    name.starts_with("Devel::TSPerlDAP::")
        || name.starts_with("DB::")
        || path.is_some_and(|value| value.contains("perl5db.pl"))
}

#[cfg(test)]
mod tests {
    use super::is_internal_frame_name_and_path;

    #[test]
    fn internal_frame_by_name_prefix() {
        assert!(is_internal_frame_name_and_path("DB::sub", Some("/app/main.pl")));
        assert!(is_internal_frame_name_and_path("Devel::TSPerlDAP::shim", Some("/app/main.pl")));
    }

    #[test]
    fn internal_frame_by_perl5db_path() {
        assert!(is_internal_frame_name_and_path("helper", Some("/usr/lib/perl5/perl5db.pl")));
    }

    #[test]
    fn non_internal_user_frame() {
        assert!(!is_internal_frame_name_and_path("main::run", Some("/app/main.pl")));
    }
}
