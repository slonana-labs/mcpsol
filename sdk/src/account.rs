use pinocchio::account_info::AccountInfo;
use pinocchio::pubkey::Pubkey;

use crate::error::{McpSolError, Result};

/// Account wrapper with validation
///
/// # Security
/// This wrapper verifies:
/// - Account data matches expected type (via discriminator)
/// - Account is owned by the expected program (must pass program_id)
pub struct Account<'a, T: AccountDeserialize> {
    pub info: &'a AccountInfo,
    pub data: T,
}

impl<'a, T: AccountDeserialize> Account<'a, T> {
    /// Create Account with owner verification
    ///
    /// # Security
    /// Verifies the account is owned by `expected_owner` before deserializing.
    /// This prevents cross-program account substitution attacks.
    pub fn try_from_with_owner(info: &'a AccountInfo, expected_owner: &Pubkey) -> Result<Self> {
        // SECURITY: Verify account owner before trusting data
        // Safety: owner() returns a valid pointer to the account's owner pubkey
        if unsafe { info.owner() } != expected_owner {
            return Err(McpSolError::InvalidOwner.into());
        }
        let data = T::try_deserialize(&info.try_borrow_data()?)?;
        Ok(Self { info, data })
    }

    /// Create Account without owner verification
    ///
    /// # Safety
    /// This is unsafe in most contexts. Only use when you have verified
    /// the account owner through other means (e.g., PDA derivation).
    /// Prefer `try_from_with_owner` for security.
    pub fn try_from(info: &'a AccountInfo) -> Result<Self> {
        let data = T::try_deserialize(&info.try_borrow_data()?)?;
        Ok(Self { info, data })
    }
}

/// Signer account wrapper
pub struct Signer<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> Signer<'a> {
    pub fn try_from(info: &'a AccountInfo) -> Result<Self> {
        if !info.is_signer() {
            return Err(McpSolError::MissingSigner.into());
        }
        Ok(Self { info })
    }

    pub fn key(&self) -> &Pubkey {
        self.info.key()
    }
}

/// System account (SOL holder, no data)
///
/// # Security
/// Verifies the account is owned by the System Program.
pub struct SystemAccount<'a> {
    pub info: &'a AccountInfo,
}

/// System program ID (all zeros)
const SYSTEM_PROGRAM_ID: [u8; 32] = [0u8; 32];

impl<'a> SystemAccount<'a> {
    pub fn try_from(info: &'a AccountInfo) -> Result<Self> {
        // SECURITY: Verify account is owned by system program
        // Safety: owner() returns a valid pointer to the account's owner pubkey
        let owner = unsafe { info.owner() };
        if owner.as_ref() != &SYSTEM_PROGRAM_ID {
            return Err(McpSolError::InvalidOwner.into());
        }
        Ok(Self { info })
    }

    pub fn key(&self) -> &Pubkey {
        self.info.key()
    }

    pub fn lamports(&self) -> u64 {
        self.info.lamports()
    }
}

/// Program account (for CPI)
pub struct Program<'a> {
    pub info: &'a AccountInfo,
    pub program_id: &'a Pubkey,
}

impl<'a> Program<'a> {
    pub fn try_from(info: &'a AccountInfo, expected_id: &'a Pubkey) -> Result<Self> {
        if info.key() != expected_id {
            return Err(McpSolError::InvalidAccount.into());
        }
        Ok(Self {
            info,
            program_id: expected_id,
        })
    }
}

/// Unchecked account - no validation performed
///
/// # Security Warning
/// This type performs NO validation on the account. The account:
/// - May be owned by any program
/// - May contain arbitrary data
/// - May not be a signer even if expected
/// - May not be writable even if expected
///
/// Only use this when you:
/// 1. Need to accept accounts of unknown type
/// 2. Will perform all necessary validation manually
/// 3. Understand the security implications
///
/// For most cases, prefer `Account<T>`, `Signer`, or `SystemAccount`.
pub struct UncheckedAccount<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> UncheckedAccount<'a> {
    /// Create an unchecked account reference
    ///
    /// # Security
    /// No validation is performed. Caller must verify owner, signer status,
    /// writability, and data validity as needed.
    pub fn try_from(info: &'a AccountInfo) -> Result<Self> {
        Ok(Self { info })
    }
}

/// Trait for deserializing account data
pub trait AccountDeserialize: Sized {
    fn try_deserialize(data: &[u8]) -> Result<Self>;
}

/// Trait for serializing account data
pub trait AccountSerialize {
    fn try_serialize(&self, data: &mut [u8]) -> Result<()>;
}

/// Combined trait for account data
pub trait AccountData: AccountDeserialize + AccountSerialize {
    /// 8-byte discriminator for account type identification
    const DISCRIMINATOR: [u8; 8];

    /// Space required for this account (excluding discriminator)
    const SPACE: usize;
}
