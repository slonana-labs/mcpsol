use pinocchio::account_info::AccountInfo;
use pinocchio::program_error::ProgramError;
use pinocchio::pubkey::Pubkey;

use crate::error::McpSolError;
use crate::Result;

/// Account wrapper with validation
pub struct Account<'a, T: AccountDeserialize> {
    pub info: &'a AccountInfo,
    pub data: T,
}

impl<'a, T: AccountDeserialize> Account<'a, T> {
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
pub struct SystemAccount<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> SystemAccount<'a> {
    pub fn try_from(info: &'a AccountInfo) -> Result<Self> {
        // System accounts are owned by system program
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

/// Unchecked account - no validation
pub struct UncheckedAccount<'a> {
    pub info: &'a AccountInfo,
}

impl<'a> UncheckedAccount<'a> {
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
