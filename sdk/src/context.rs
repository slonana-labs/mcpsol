use pinocchio::account_info::AccountInfo;
use pinocchio::pubkey::Pubkey;

use crate::error::Result;

/// Instruction context holding accounts and program id
pub struct Context<'info, T: Accounts<'info>> {
    pub program_id: &'info Pubkey,
    pub accounts: T,
    pub remaining_accounts: &'info [AccountInfo],
}

impl<'info, T: Accounts<'info>> Context<'info, T> {
    pub fn new(
        program_id: &'info Pubkey,
        accounts: T,
        remaining_accounts: &'info [AccountInfo],
    ) -> Self {
        Self {
            program_id,
            accounts,
            remaining_accounts,
        }
    }
}

/// Trait for account structs that can be validated and loaded
pub trait Accounts<'info>: Sized {
    /// Try to load accounts from the provided account infos
    fn try_accounts(
        program_id: &Pubkey,
        accounts: &'info [AccountInfo],
    ) -> Result<Self>;
}

/// Builder for creating context from raw entrypoint data
pub struct ContextBuilder<'info> {
    program_id: &'info Pubkey,
    accounts: &'info [AccountInfo],
}

impl<'info> ContextBuilder<'info> {
    pub fn new(program_id: &'info Pubkey, accounts: &'info [AccountInfo]) -> Self {
        Self {
            program_id,
            accounts,
        }
    }

    pub fn build<T: Accounts<'info>>(self) -> Result<Context<'info, T>> {
        let accounts = T::try_accounts(self.program_id, self.accounts)?;
        Ok(Context::new(self.program_id, accounts, &[]))
    }
}
