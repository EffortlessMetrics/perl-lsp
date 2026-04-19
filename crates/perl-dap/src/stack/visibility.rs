use super::StackFrame;

/// Returns true when a frame belongs to debugger/shim internals.
#[must_use]
pub fn is_internal_frame_name_and_path(name: &str, path: Option<&str>) -> bool {
    name.starts_with("Devel::TSPerlDAP::")
        || name.starts_with("DB::")
        || path.is_some_and(|value| value.contains("perl5db.pl"))
}

/// Returns true when a frame belongs to debugger/shim internals.
#[must_use]
pub fn is_internal_frame(frame: &StackFrame) -> bool {
    is_internal_frame_name_and_path(
        &frame.name,
        frame
            .source
            .as_ref()
            .and_then(|source| source.path.as_deref()),
    )
}

/// Filter out internal debugger and shim frames from user-visible stack traces.
#[must_use]
pub fn filter_user_visible_frames(frames: Vec<StackFrame>) -> Vec<StackFrame> {
    frames
        .into_iter()
        .filter(|frame| !is_internal_frame(frame))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::super::{Source, StackFrame};
    use super::{filter_user_visible_frames, is_internal_frame, is_internal_frame_name_and_path};

    fn frame(id: i64, name: &str, path: &str) -> StackFrame {
        StackFrame::new(id, name.to_string(), Some(Source::new(path)), 1)
    }

    #[test]
    fn internal_frame_by_name_prefix() {
        assert!(is_internal_frame(&frame(1, "DB::sub", "/app/main.pl")));
        assert!(is_internal_frame(&frame(
            2,
            "Devel::TSPerlDAP::shim",
            "/app/main.pl"
        )));
    }

    #[test]
    fn internal_frame_by_perl5db_path() {
        assert!(is_internal_frame(&frame(
            3,
            "helper",
            "/usr/lib/perl5/perl5db.pl"
        )));
    }

    #[test]
    fn classification_from_name_and_path() {
        assert!(is_internal_frame_name_and_path(
            "DB::sub",
            Some("/app/main.pl")
        ));
        assert!(is_internal_frame_name_and_path(
            "helper",
            Some("/usr/lib/perl5/perl5db.pl")
        ));
        assert!(!is_internal_frame_name_and_path(
            "main::run",
            Some("/app/main.pl")
        ));
    }

    #[test]
    fn filter_keeps_only_user_frames_preserving_order() {
        let frames = vec![
            frame(1, "main::start", "/app/main.pl"),
            frame(2, "DB::sub", "/usr/lib/perl5/perl5db.pl"),
            frame(3, "Utils::run", "/app/lib/Utils.pm"),
        ];

        let filtered = filter_user_visible_frames(frames);
        assert_eq!(filtered.len(), 2);
        assert_eq!(filtered[0].name, "main::start");
        assert_eq!(filtered[1].name, "Utils::run");
    }
}
