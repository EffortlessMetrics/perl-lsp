//! Helpers for decoding on-disk source files.
//!
//! Perl codebases still contain legacy Latin-1 encoded sources.
//! We decode UTF-8 first and then fall back to byte-preserving Latin-1
//! so indexing/features remain available for those files.

use std::path::Path;

pub(crate) fn decode_source_bytes(bytes: Vec<u8>) -> String {
    match String::from_utf8(bytes) {
        Ok(source) => source,
        Err(err) => {
            let raw = err.into_bytes();
            let mut decoded = String::with_capacity(raw.len());
            for byte in raw {
                decoded.push(char::from(byte));
            }
            decoded
        }
    }
}

pub(crate) fn read_source_file(path: &Path) -> std::io::Result<String> {
    std::fs::read(path).map(decode_source_bytes)
}

#[cfg(test)]
mod tests {
    use super::decode_source_bytes;

    #[test]
    fn decode_source_bytes_preserves_utf8() {
        let decoded = decode_source_bytes(b"use strict;\n".to_vec());
        assert_eq!(decoded, "use strict;\n");
    }

    #[test]
    fn decode_source_bytes_decodes_latin1_losslessly() {
        let decoded = decode_source_bytes(vec![0x53, 0xE5, 0x72, 0x0A]);
        assert_eq!(decoded, "Sår\n");
    }
}
