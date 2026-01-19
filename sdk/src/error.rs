//! Error types for mcpsol programs.

use pinocchio::program_error::ProgramError;

/// Errors that can be returned by mcpsol programs.
///
/// These are converted to Solana's `ProgramError::Custom(code)` where
/// the code is the discriminant value.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
#[non_exhaustive]
pub enum McpSolError {
    /// Invalid instruction discriminator
    InvalidInstruction = 0,
    /// Account validation failed (generic)
    InvalidAccount = 1,
    /// Missing required account in instruction
    MissingAccount = 2,
    /// Account not signer when required
    MissingSigner = 3,
    /// Account not writable when required
    NotWritable = 4,
    /// Account owner mismatch (wrong program owns account)
    InvalidOwner = 5,
    /// Constraint violation (e.g., PDA mismatch, has_one check)
    ConstraintViolation = 6,
    /// Serialization/deserialization error
    SerializationError = 7,
    /// Arithmetic overflow/underflow
    Overflow = 8,
}

impl From<McpSolError> for ProgramError {
    fn from(e: McpSolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub type Result<T> = core::result::Result<T, ProgramError>;
