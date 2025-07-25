/// Unicode character classification for Perl identifiers
/// 
/// Perl allows a wide range of Unicode characters in identifiers,
/// including emoji and other symbols.

use unicode_ident::{is_xid_start, is_xid_continue};

/// Check if a character can start a Perl identifier
pub fn is_perl_identifier_start(ch: char) -> bool {
    // Use unicode-ident for standard Unicode identifier characters
    // This covers most scripts and languages automatically
    if ch == '_' || is_xid_start(ch) {
        return true;
    }
    
    // Check additional Unicode blocks that Perl allows
    // but aren't included in XID_Start (primarily emoji)
    matches!(ch as u32,
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
    )
}

/// Check if a character can continue a Perl identifier
pub fn is_perl_identifier_continue(ch: char) -> bool {
    // For continuation, we accept identifier start chars, XID_Continue chars,
    // and the single quote (for old-style package separators like Foo'Bar)
    is_perl_identifier_start(ch) || is_xid_continue(ch) || ch == '\''
}