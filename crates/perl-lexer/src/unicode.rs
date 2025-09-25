use std::sync::atomic::{AtomicU64, Ordering};
/// Unicode character classification for Perl identifiers
///
/// Perl allows a wide range of Unicode characters in identifiers,
/// including emoji and other symbols.
use unicode_ident::{is_xid_continue, is_xid_start};

// Performance tracking for Unicode operations
static UNICODE_CHAR_CHECKS: AtomicU64 = AtomicU64::new(0);
static UNICODE_EMOJI_HITS: AtomicU64 = AtomicU64::new(0);

/// Get Unicode processing statistics for debugging
#[allow(dead_code)]
pub fn get_unicode_stats() -> (u64, u64) {
    (UNICODE_CHAR_CHECKS.load(Ordering::Relaxed), UNICODE_EMOJI_HITS.load(Ordering::Relaxed))
}

/// Reset Unicode processing statistics
#[allow(dead_code)]
pub fn reset_unicode_stats() {
    UNICODE_CHAR_CHECKS.store(0, Ordering::Relaxed);
    UNICODE_EMOJI_HITS.store(0, Ordering::Relaxed);
}

/// Check if a character can start a Perl identifier
pub fn is_perl_identifier_start(ch: char) -> bool {
    UNICODE_CHAR_CHECKS.fetch_add(1, Ordering::Relaxed);

    // Use unicode-ident for standard Unicode identifier characters
    // This covers most scripts and languages automatically
    if ch == '_' || is_xid_start(ch) {
        return true;
    }

    // Check additional Unicode blocks that Perl allows
    // but aren't included in XID_Start (primarily emoji)
    let is_emoji = matches!(ch as u32,
        // Emoji and symbols
        0x1F300..=0x1F6FF |  // Miscellaneous Symbols and Pictographs (includes 🚀)
        0x1F900..=0x1F9FF |  // Supplemental Symbols and Pictographs
        0x2600..=0x26FF |    // Miscellaneous Symbols (includes ♥)
        0x2700..=0x27BF |    // Dingbats
        0x1F000..=0x1F02F |  // Mahjong Tiles
        0x1F0A0..=0x1F0FF |  // Playing Cards
        0x1F100..=0x1F1FF |  // Enclosed Alphanumeric Supplement
        0x1F200..=0x1F2FF |  // Enclosed Ideographic Supplement
        0x1F700..=0x1F77F |  // Alchemical Symbols
        0x1F780..=0x1F7FF |  // Geometric Shapes Extended
        0x1F800..=0x1F8FF |  // Supplemental Arrows-C
        0x1FA00..=0x1FA6F |  // Chess Symbols
        0x1FA70..=0x1FAFF    // Symbols and Pictographs Extended-A
    );

    if is_emoji {
        UNICODE_EMOJI_HITS.fetch_add(1, Ordering::Relaxed);
    }

    is_emoji
}

/// Check if a character can continue a Perl identifier
pub fn is_perl_identifier_continue(ch: char) -> bool {
    // For continuation, we accept identifier start chars, XID_Continue chars,
    // and the single quote (for old-style package separators like Foo'Bar)
    is_perl_identifier_start(ch) || is_xid_continue(ch) || ch == '\''
}

/// Validate Unicode string complexity for performance monitoring
/// Returns (`char_count`, `emoji_count`, `complex_char_count`)
#[allow(dead_code)]
pub fn analyze_unicode_complexity(text: &str) -> (usize, usize, usize) {
    let mut char_count = 0;
    let mut emoji_count = 0;
    let mut complex_char_count = 0;

    for ch in text.chars() {
        char_count += 1;

        // Count emojis and complex Unicode
        let ch_u32 = ch as u32;
        if matches!(ch_u32, 0x1F300..=0x1F9FF | 0x2600..=0x27BF) {
            emoji_count += 1;
        }

        // Count complex characters (surrogate pairs, combining marks, etc.)
        if ch_u32 > 0xFFFF || ch.len_utf8() > 2 {
            complex_char_count += 1;
        }
    }

    (char_count, emoji_count, complex_char_count)
}
