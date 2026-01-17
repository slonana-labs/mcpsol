use pinocchio::account_info::AccountInfo;
use pinocchio::pubkey::Pubkey;

use crate::Result;

/// Instruction context holding accounts and program id
pub struct Context<'a, 'info, T: Accounts<'info>> {
    pub program_id: &'a Pubkey,
    pub accounts: T,
    pub remaining_accounts: &'a [AccountInfo],
    _marker: core::marker::PhantomData<&'info ()>,
}

impl<'a, 'info, T: Accounts<'info>> Context<'a, 'info, T> {
    pub fn new(
        program_id: &'a Pubkey,
        accounts: T,
        remaining_accounts: &'a [AccountInfo],
    ) -> Self {
        Self {
            program_id,
            accounts,
            remaining_accounts,
            _marker: core::marker::PhantomData,
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
pub struct ContextBuilder<'a> {
    program_id: &'a Pubkey,
    accounts: &'a [AccountInfo],
}

impl<'a> ContextBuilder<'a> {
    pub fn new(program_id: &'a Pubkey, accounts: &'a [AccountInfo]) -> Self {
        Self {
            program_id,
            accounts,
        }
    }

    pub fn build<'info, T: Accounts<'info>>(self) -> Result<Context<'a, 'info, T>>
    where
        'a: 'info,
    {
        let accounts = T::try_accounts(self.program_id, self.accounts)?;
        Ok(Context::new(self.program_id, accounts, &[]))
    }
}
