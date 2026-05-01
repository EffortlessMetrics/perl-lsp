pub(super) const MAX_EDIT_SIZE: usize = 64 * 1024;

pub(super) fn should_force_full_reparse(touched_bytes: usize, new_text: &str) -> bool {
    touched_bytes > 1024 || new_text.matches('\n').count() > 10
}
