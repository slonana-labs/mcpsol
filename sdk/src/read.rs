//! Zero-cost unsafe read helpers for instruction data parsing.
//!
//! This module provides unchecked read functions for extracting typed values
//! from raw byte slices. These are used by the generated dispatcher code to
//! minimize CU overhead.
//!
//! # Safety Contract
//!
//! All functions in this module are `unsafe` and require the caller to ensure:
//! 1. The data slice has sufficient length for the read
//! 2. The offset is within bounds: `offset + size_of::<T>() <= data.len()`
//!
//! The generated dispatcher performs a single bounds check before calling
//! these functions:
//!
//! ```ignore
//! const EXPECTED_LEN: usize = 8 + 8; // discriminator + args
//! if instruction_data.len() < EXPECTED_LEN {
//!     return Err(InvalidInstructionData);
//! }
//! // SAFETY: Length validated above
//! let arg = unsafe { read_u64_unchecked(instruction_data, 8) };
//! ```
//!
//! # Performance
//!
//! These functions compile to single load instructions, reducing argument
//! parsing overhead significantly. On-chain estimates: ~60-70 CU (with Option
//! chains) down to ~5-10 CU per argument.

use core::ptr;

/// Read a u8 at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset < data.len()`.
#[inline(always)]
pub unsafe fn read_u8_unchecked(data: &[u8], offset: usize) -> u8 {
    debug_assert!(offset < data.len(), "read_u8_unchecked: offset out of bounds");
    *data.as_ptr().add(offset)
}

/// Read a u16 (little-endian) at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset + 2 <= data.len()`.
#[inline(always)]
pub unsafe fn read_u16_unchecked(data: &[u8], offset: usize) -> u16 {
    debug_assert!(offset + 2 <= data.len(), "read_u16_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const u16)
}

/// Read a u32 (little-endian) at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset + 4 <= data.len()`.
#[inline(always)]
pub unsafe fn read_u32_unchecked(data: &[u8], offset: usize) -> u32 {
    debug_assert!(offset + 4 <= data.len(), "read_u32_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const u32)
}

/// Read a u64 (little-endian) at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset + 8 <= data.len()`.
#[inline(always)]
pub unsafe fn read_u64_unchecked(data: &[u8], offset: usize) -> u64 {
    debug_assert!(offset + 8 <= data.len(), "read_u64_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const u64)
}

/// Read an i8 at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset < data.len()`.
#[inline(always)]
pub unsafe fn read_i8_unchecked(data: &[u8], offset: usize) -> i8 {
    debug_assert!(offset < data.len(), "read_i8_unchecked: offset out of bounds");
    *data.as_ptr().add(offset) as i8
}

/// Read an i16 (little-endian) at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset + 2 <= data.len()`.
#[inline(always)]
pub unsafe fn read_i16_unchecked(data: &[u8], offset: usize) -> i16 {
    debug_assert!(offset + 2 <= data.len(), "read_i16_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const i16)
}

/// Read an i32 (little-endian) at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset + 4 <= data.len()`.
#[inline(always)]
pub unsafe fn read_i32_unchecked(data: &[u8], offset: usize) -> i32 {
    debug_assert!(offset + 4 <= data.len(), "read_i32_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const i32)
}

/// Read an i64 (little-endian) at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset + 8 <= data.len()`.
#[inline(always)]
pub unsafe fn read_i64_unchecked(data: &[u8], offset: usize) -> i64 {
    debug_assert!(offset + 8 <= data.len(), "read_i64_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const i64)
}

/// Read a bool at the given offset without bounds checking.
///
/// Non-zero values are interpreted as `true`.
///
/// # Safety
///
/// Caller must ensure `offset < data.len()`.
#[inline(always)]
pub unsafe fn read_bool_unchecked(data: &[u8], offset: usize) -> bool {
    debug_assert!(offset < data.len(), "read_bool_unchecked: offset out of bounds");
    *data.as_ptr().add(offset) != 0
}

/// Read a 32-byte Pubkey at the given offset without bounds checking.
///
/// Returns the raw 32 bytes. Caller is responsible for constructing the
/// appropriate Pubkey type.
///
/// # Safety
///
/// Caller must ensure `offset + 32 <= data.len()`.
#[inline(always)]
pub unsafe fn read_pubkey_bytes_unchecked(data: &[u8], offset: usize) -> [u8; 32] {
    debug_assert!(offset + 32 <= data.len(), "read_pubkey_bytes_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const [u8; 32])
}

/// Read a fixed-size byte array at the given offset without bounds checking.
///
/// # Safety
///
/// Caller must ensure `offset + N <= data.len()`.
#[inline(always)]
pub unsafe fn read_bytes_unchecked<const N: usize>(data: &[u8], offset: usize) -> [u8; N] {
    debug_assert!(offset + N <= data.len(), "read_bytes_unchecked: offset out of bounds");
    ptr::read_unaligned(data.as_ptr().add(offset) as *const [u8; N])
}

/// Read an 8-byte discriminator at the start of instruction data.
///
/// This is a specialized function for the common case of reading the
/// discriminator from instruction data.
///
/// # Safety
///
/// Caller must ensure `data.len() >= 8`.
#[inline(always)]
pub unsafe fn read_discriminator_unchecked(data: &[u8]) -> [u8; 8] {
    debug_assert!(data.len() >= 8, "read_discriminator_unchecked: data too short");
    *(data.as_ptr() as *const [u8; 8])
}

// ============================================================================
// Type Size Constants
// ============================================================================

/// Size of each primitive type in bytes.
/// Used by the macro for compile-time offset calculation.
pub mod sizes {
    pub const U8: usize = 1;
    pub const U16: usize = 2;
    pub const U32: usize = 4;
    pub const U64: usize = 8;
    pub const I8: usize = 1;
    pub const I16: usize = 2;
    pub const I32: usize = 4;
    pub const I64: usize = 8;
    pub const BOOL: usize = 1;
    pub const PUBKEY: usize = 32;
    pub const DISCRIMINATOR: usize = 8;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_read_u8() {
        let data = [0x42u8, 0x00, 0x00, 0x00];
        assert_eq!(unsafe { read_u8_unchecked(&data, 0) }, 0x42);
    }

    #[test]
    fn test_read_u16() {
        let data = [0x34, 0x12, 0x00, 0x00]; // 0x1234 little-endian
        assert_eq!(unsafe { read_u16_unchecked(&data, 0) }, 0x1234);
    }

    #[test]
    fn test_read_u32() {
        let data = [0x78, 0x56, 0x34, 0x12]; // 0x12345678 little-endian
        assert_eq!(unsafe { read_u32_unchecked(&data, 0) }, 0x12345678);
    }

    #[test]
    fn test_read_u64() {
        let data = 0x123456789ABCDEFFu64.to_le_bytes();
        assert_eq!(unsafe { read_u64_unchecked(&data, 0) }, 0x123456789ABCDEFF);
    }

    #[test]
    fn test_read_i8() {
        let data = [0xFF]; // -1 as i8
        assert_eq!(unsafe { read_i8_unchecked(&data, 0) }, -1);
    }

    #[test]
    fn test_read_i16() {
        let data = (-1234i16).to_le_bytes();
        assert_eq!(unsafe { read_i16_unchecked(&data, 0) }, -1234);
    }

    #[test]
    fn test_read_i32() {
        let data = (-123456i32).to_le_bytes();
        assert_eq!(unsafe { read_i32_unchecked(&data, 0) }, -123456);
    }

    #[test]
    fn test_read_i64() {
        let data = (-123456789i64).to_le_bytes();
        assert_eq!(unsafe { read_i64_unchecked(&data, 0) }, -123456789);
    }

    #[test]
    fn test_read_bool() {
        let data = [0x00, 0x01, 0xFF];
        assert_eq!(unsafe { read_bool_unchecked(&data, 0) }, false);
        assert_eq!(unsafe { read_bool_unchecked(&data, 1) }, true);
        assert_eq!(unsafe { read_bool_unchecked(&data, 2) }, true); // Any non-zero is true
    }

    #[test]
    fn test_read_pubkey_bytes() {
        let mut data = [0u8; 32];
        data[0] = 0x11;
        data[31] = 0xFF;
        let pubkey = unsafe { read_pubkey_bytes_unchecked(&data, 0) };
        assert_eq!(pubkey[0], 0x11);
        assert_eq!(pubkey[31], 0xFF);
    }

    #[test]
    fn test_read_discriminator() {
        let data = [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21];
        let disc = unsafe { read_discriminator_unchecked(&data) };
        assert_eq!(disc, [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21]);
    }

    #[test]
    fn test_read_with_offset() {
        // Simulated instruction data: discriminator + u64 + u32
        let mut data = vec![0u8; 20];
        data[8..16].copy_from_slice(&1000u64.to_le_bytes());
        data[16..20].copy_from_slice(&42u32.to_le_bytes());

        assert_eq!(unsafe { read_u64_unchecked(&data, 8) }, 1000);
        assert_eq!(unsafe { read_u32_unchecked(&data, 16) }, 42);
    }

    #[test]
    fn test_read_bytes_generic() {
        let data = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
        let bytes: [u8; 4] = unsafe { read_bytes_unchecked(&data, 2) };
        assert_eq!(bytes, [0x03, 0x04, 0x05, 0x06]);
    }
}
