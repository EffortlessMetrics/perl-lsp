//! Hashing utilities shared by core crates and workspace tools.

/// Compute an FNV-1a 64-bit hash and return it as a tagged hex string.
#[must_use]
pub fn fnv1a64_hex(bytes: &[u8]) -> String {
    let mut hash: u64 = 0xcbf29ce484222325;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    format!("fnv1a64:{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::fnv1a64_hex;
    use std::error::Error;

    #[test]
    fn fnv1a64_hex_matches_known_vectors() -> Result<(), Box<dyn Error>> {
        assert_eq!(fnv1a64_hex(b""), "fnv1a64:cbf29ce484222325");
        assert_eq!(fnv1a64_hex(b"hello"), "fnv1a64:a430d84680aabd0b");
        Ok(())
    }

    #[test]
    fn fnv1a64_hex_is_deterministic() -> Result<(), Box<dyn Error>> {
        let h1 = fnv1a64_hex(b"hello");
        let h2 = fnv1a64_hex(b"hello");
        assert_eq!(h1, h2);
        Ok(())
    }

    #[test]
    fn fnv1a64_hex_differs_on_different_input() -> Result<(), Box<dyn Error>> {
        let h1 = fnv1a64_hex(b"hello");
        let h2 = fnv1a64_hex(b"world");
        assert_ne!(h1, h2);
        Ok(())
    }
}
