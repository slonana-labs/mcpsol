//! Discriminator calculation using SHA256 sighash
//!
//! Compatible with Anchor's discriminator format for interoperability.

use sha2::{Sha256, Digest};

/// Calculate instruction discriminator (Anchor-compatible)
/// Format: sha256("global:<name>")[0..8]
pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    hash_to_discriminator(&preimage)
}

/// Calculate account discriminator (Anchor-compatible)
/// Format: sha256("account:<Name>")[0..8]
pub fn account_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("account:{}", name);
    hash_to_discriminator(&preimage)
}

/// Hash a preimage to an 8-byte discriminator
fn hash_to_discriminator(preimage: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(preimage.as_bytes());
    let hash = hasher.finalize();
    // Safety: SHA256 always produces 32 bytes, taking first 8 always succeeds
    #[allow(clippy::expect_used)]
    hash[..8].try_into().expect("SHA256 always produces 32 bytes")
}

/// Convert discriminator to hex string
pub fn discriminator_to_hex(disc: &[u8; 8]) -> [u8; 16] {
    const HEX_CHARS: &[u8; 16] = b"0123456789abcdef";
    let mut hex = [0u8; 16];
    for (i, byte) in disc.iter().enumerate() {
        hex[i * 2] = HEX_CHARS[(byte >> 4) as usize];
        hex[i * 2 + 1] = HEX_CHARS[(byte & 0x0f) as usize];
    }
    hex
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_instruction_discriminator() {
        // list_tools should match our constant
        let disc = instruction_discriminator("list_tools");
        assert_eq!(disc, [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0]);
    }

    #[test]
    fn test_account_discriminator() {
        let disc = account_discriminator("Counter");
        assert_eq!(disc.len(), 8);
    }

    #[test]
    fn test_discriminator_to_hex() {
        let disc = [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0];
        let hex = discriminator_to_hex(&disc);
        assert_eq!(&hex, b"42195e6a55fd41c0");
    }
}
