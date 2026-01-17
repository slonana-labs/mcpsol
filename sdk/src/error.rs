use pinocchio::program_error::ProgramError;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u32)]
pub enum McpSolError {
    /// Invalid instruction discriminator
    InvalidInstruction = 0,
    /// Account validation failed
    InvalidAccount = 1,
    /// Missing required account
    MissingAccount = 2,
    /// Account not signer when required
    MissingSigner = 3,
    /// Account not writable when required
    NotWritable = 4,
    /// Account owner mismatch
    InvalidOwner = 5,
    /// Constraint violation
    ConstraintViolation = 6,
    /// Serialization error
    SerializationError = 7,
    /// Arithmetic overflow
    Overflow = 8,
}

impl From<McpSolError> for ProgramError {
    fn from(e: McpSolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub type Result<T> = core::result::Result<T, ProgramError>;
