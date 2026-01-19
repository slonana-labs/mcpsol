//! Discriminator generation utilities
//!
//! Generates 8-byte discriminators for accounts and instructions using SHA256.
//! Compatible with Anchor's discriminator format for interoperability.

use sha2::{Digest, Sha256};

/// Generate discriminator for an account type.
/// Format: sha256("account:<AccountName>")[0..8]
pub fn account_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("account:{}", name);
    hash_to_discriminator(&preimage)
}

/// Generate discriminator for an instruction.
/// Format: sha256("global:<instruction_name>")[0..8]
pub fn instruction_discriminator(name: &str) -> [u8; 8] {
    let preimage = format!("global:{}", name);
    hash_to_discriminator(&preimage)
}

/// Hash a string to an 8-byte discriminator using SHA256
fn hash_to_discriminator(preimage: &str) -> [u8; 8] {
    let mut hasher = Sha256::new();
    hasher.update(preimage.as_bytes());
    let result = hasher.finalize();

    let mut discriminator = [0u8; 8];
    discriminator.copy_from_slice(&result[..8]);
    discriminator
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_discriminator() {
        // Test that same name produces same discriminator
        let disc1 = account_discriminator("Counter");
        let disc2 = account_discriminator("Counter");
        assert_eq!(disc1, disc2);

        // Test that different names produce different discriminators
        let disc3 = account_discriminator("Token");
        assert_ne!(disc1, disc3);
    }

    #[test]
    fn test_instruction_discriminator() {
        let disc1 = instruction_discriminator("initialize");
        let disc2 = instruction_discriminator("initialize");
        assert_eq!(disc1, disc2);

        let disc3 = instruction_discriminator("transfer");
        assert_ne!(disc1, disc3);
    }

    #[test]
    fn test_discriminator_length() {
        let disc = account_discriminator("Test");
        assert_eq!(disc.len(), 8);
    }
}
