//! MCP Vault - Demonstrates PDAs and complex schemas
//!
//! This example shows:
//! - How to document PDA seeds in MCP schema descriptions
//! - Complex account structures
//! - Multi-instruction programs
//!
//! For AI agents, the PDA seeds in descriptions allow them to derive
//! the correct addresses before calling instructions.

use bytemuck::{Pod, Zeroable};
use mcpsol_core::{
    ArgType, McpSchema, McpSchemaBuilder, McpToolBuilder,
    LIST_TOOLS_DISCRIMINATOR, CachedSchemaPages,
};
use pinocchio::{
    account_info::AccountInfo,
    entrypoint,
    program_error::ProgramError,
    pubkey::Pubkey,
    ProgramResult,
};
use pinocchio_log::log;

// Seeds for PDAs (documented in schema for AI agents)
pub const VAULT_SEED: &[u8] = b"vault";
pub const VAULT_AUTH_SEED: &[u8] = b"vault_auth";

/// Vault account - stores owner and balance info
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Vault {
    pub discriminator: [u8; 8],
    pub owner: [u8; 32],
    pub mint: [u8; 32],
    pub bump: u8,
    pub auth_bump: u8,
    pub _padding: [u8; 6],
    pub balance: u64,
}

pub const VAULT_DISCRIMINATOR: [u8; 8] = [0x3b, 0x7a, 0x3e, 0x2c, 0x8f, 0x1d, 0x4a, 0x5b];

// Instruction discriminators
const INITIALIZE: [u8; 8] = [0xaf, 0xaf, 0x6d, 0x1f, 0x0d, 0x98, 0x9b, 0xed];
const DEPOSIT: [u8; 8] = [0xf2, 0x23, 0xc6, 0x89, 0x52, 0xe1, 0xf2, 0xb6];
const WITHDRAW: [u8; 8] = [0xb7, 0x12, 0x46, 0x9c, 0x94, 0x6d, 0xa1, 0x22];
const GET_INFO: [u8; 8] = [0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde, 0xf0];

/// Build MCP schema with PDA documentation
///
/// Note: PDA seeds are documented in the description field so AI agents
/// can derive the correct addresses. Format: seeds=["seed1", arg1, arg2]
fn build_schema() -> McpSchema {
    McpSchemaBuilder::new("mcp_vault")
        .add_tool(
            McpToolBuilder::new("list_tools")
                .description("List available MCP tools. Pass cursor byte to paginate.")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("initialize")
                .description("Create a new vault PDA. Derive address with seeds=[\"vault\", owner, mint]")
                .writable_desc("vault", "Vault PDA to create. seeds=[\"vault\", owner, mint, bump]")
                .signer_desc("owner", "Vault owner who can withdraw funds")
                .account_with_desc("mint", "Token mint for this vault", false, false)
                .account_with_desc("system_program", "System program for account creation", false, false)
                .arg_desc("vault_bump", "PDA bump seed for vault", ArgType::U8)
                .arg_desc("auth_bump", "PDA bump seed for vault authority", ArgType::U8)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("deposit")
                .description("Deposit SOL into the vault. Anyone can deposit.")
                .writable_desc("vault", "Vault to deposit into")
                .signer_desc("depositor", "Account depositing funds")
                .account_with_desc("system_program", "System program for transfer", false, false)
                .arg_desc("amount", "Amount of lamports to deposit", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("withdraw")
                .description("Withdraw SOL from vault. Only owner can withdraw.")
                .writable_desc("vault", "Vault to withdraw from")
                .writable_desc("recipient", "Account to receive withdrawn funds")
                .signer_desc("owner", "Must match vault owner")
                .arg_desc("amount", "Amount of lamports to withdraw", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("get_info")
                .description("Get vault balance and metadata via return_data")
                .account_with_desc("vault", "Vault to query", false, false)
                .build()
        )
        .build()
}

/// Cached schema pages for CU-efficient list_tools responses.
/// Pre-computes serialized JSON for each pagination page at first access.
static CACHED_PAGES: std::sync::OnceLock<CachedSchemaPages> = std::sync::OnceLock::new();

fn get_cached_pages() -> &'static CachedSchemaPages {
    CACHED_PAGES.get_or_init(|| CachedSchemaPages::from_schema(&build_schema()))
}

#[cfg(not(feature = "no-entrypoint"))]
entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }

    // Safe: Length >= 8 verified above
    let discriminator: [u8; 8] = data[..8]
        .try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    match discriminator {
        LIST_TOOLS_DISCRIMINATOR => {
            log!("list_tools");
            let cursor = data.get(8).copied().unwrap_or(0);
            let page_bytes = get_cached_pages().get_page(cursor);
            pinocchio::program::set_return_data(page_bytes);
            Ok(())
        }
        INITIALIZE => {
            log!("initialize");
            process_initialize(program_id, accounts, &data[8..])
        }
        DEPOSIT => {
            log!("deposit");
            if data.len() < 16 {
                return Err(ProgramError::InvalidInstructionData);
            }
            // Safe: Length >= 16 verified above
            let amount_bytes: [u8; 8] = data[8..16]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            let amount = u64::from_le_bytes(amount_bytes);
            process_deposit(program_id, accounts, amount)
        }
        WITHDRAW => {
            log!("withdraw");
            if data.len() < 16 {
                return Err(ProgramError::InvalidInstructionData);
            }
            // Safe: Length >= 16 verified above
            let amount_bytes: [u8; 8] = data[8..16]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            let amount = u64::from_le_bytes(amount_bytes);
            process_withdraw(program_id, accounts, amount)
        }
        GET_INFO => {
            log!("get_info");
            process_get_info(program_id, accounts)
        }
        _ => Err(ProgramError::InvalidInstructionData),
    }
}

fn process_initialize(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: &[u8],
) -> ProgramResult {
    let [vault, owner, mint, _system] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    if !owner.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    if args.len() < 2 {
        return Err(ProgramError::InvalidInstructionData);
    }

    let vault_bump = args[0];
    let auth_bump = args[1];

    // Verify vault PDA
    let seeds: &[&[u8]] = &[VAULT_SEED, owner.key().as_ref(), mint.key().as_ref(), &[vault_bump]];
    let expected = pinocchio::pubkey::create_program_address(seeds, program_id)
        .map_err(|_| ProgramError::InvalidSeeds)?;

    if vault.key() != &expected {
        return Err(ProgramError::InvalidSeeds);
    }

    // Initialize vault data
    let mut data = vault.try_borrow_mut_data()?;
    if data.len() < core::mem::size_of::<Vault>() {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let v: &mut Vault = bytemuck::from_bytes_mut(&mut data[..core::mem::size_of::<Vault>()]);
    v.discriminator = VAULT_DISCRIMINATOR;
    v.owner = *owner.key();
    v.mint = *mint.key();
    v.bump = vault_bump;
    v.auth_bump = auth_bump;
    v._padding = [0; 6];
    v.balance = 0;

    log!("Vault initialized");
    Ok(())
}

fn process_deposit(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [vault, depositor, _system] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // SECURITY: Verify vault is owned by this program
    // Safety: owner() returns valid pointer to account owner
    if unsafe { vault.owner() } != program_id {
        log!("Invalid vault owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    // SECURITY: Verify vault is writable
    if !vault.is_writable() {
        log!("Vault not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    if !depositor.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // SECURITY: Verify data size before bytemuck cast
    let data_len = vault.data_len();
    if data_len < core::mem::size_of::<Vault>() {
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Update vault balance (in real impl, transfer SOL via CPI)
    let mut data = vault.try_borrow_mut_data()?;
    let v: &mut Vault = bytemuck::from_bytes_mut(&mut data[..core::mem::size_of::<Vault>()]);

    if v.discriminator != VAULT_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }

    v.balance = v.balance.checked_add(amount).ok_or(ProgramError::ArithmeticOverflow)?;
    log!("Deposited {}. Balance: {}", amount, v.balance);
    Ok(())
}

fn process_withdraw(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let [vault, _recipient, owner] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // SECURITY: Verify vault is owned by this program
    // Safety: owner() returns valid pointer to account owner
    if unsafe { vault.owner() } != program_id {
        log!("Invalid vault owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    // SECURITY: Verify vault is writable
    if !vault.is_writable() {
        log!("Vault not writable");
        return Err(ProgramError::InvalidAccountData);
    }

    if !owner.is_signer() {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // SECURITY: Verify data size before bytemuck cast
    let data_len = vault.data_len();
    if data_len < core::mem::size_of::<Vault>() {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let mut data = vault.try_borrow_mut_data()?;
    let v: &mut Vault = bytemuck::from_bytes_mut(&mut data[..core::mem::size_of::<Vault>()]);

    if v.discriminator != VAULT_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }

    if &v.owner != owner.key() {
        log!("Unauthorized");
        return Err(ProgramError::InvalidAccountData);
    }

    v.balance = v.balance.checked_sub(amount).ok_or(ProgramError::InsufficientFunds)?;
    log!("Withdrew {}. Balance: {}", amount, v.balance);
    Ok(())
}

fn process_get_info(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let [vault] = accounts else {
        return Err(ProgramError::NotEnoughAccountKeys);
    };

    // SECURITY: Verify vault is owned by this program
    // Safety: owner() returns valid pointer to account owner
    if unsafe { vault.owner() } != program_id {
        log!("Invalid vault owner");
        return Err(ProgramError::IncorrectProgramId);
    }

    // SECURITY: Verify data size before bytemuck cast
    let data_len = vault.data_len();
    if data_len < core::mem::size_of::<Vault>() {
        return Err(ProgramError::AccountDataTooSmall);
    }

    let data = vault.try_borrow_data()?;
    let v: &Vault = bytemuck::from_bytes(&data[..core::mem::size_of::<Vault>()]);

    if v.discriminator != VAULT_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }

    let info = format!(
        "{{\"balance\":{},\"bump\":{},\"auth_bump\":{}}}",
        v.balance, v.bump, v.auth_bump
    );
    pinocchio::program::set_return_data(info.as_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use mcpsol_core::MAX_RETURN_DATA_SIZE;

    #[test]
    fn test_paginated_schema_with_cache() {
        // Demonstrates CU-optimized pagination pattern
        let cached = CachedSchemaPages::from_schema(&build_schema());

        for cursor in 0..cached.num_pages() {
            let page_bytes = cached.get_page(cursor as u8);
            let json = String::from_utf8(page_bytes.to_vec()).unwrap();

            println!("Page {} ({} bytes):\n{}\n", cursor, page_bytes.len(), json);

            assert!(
                page_bytes.len() <= MAX_RETURN_DATA_SIZE,
                "Page {} ({} bytes) exceeds 1024 limit",
                cursor,
                page_bytes.len()
            );
        }
    }

    #[test]
    fn test_schema_has_pda_info() {
        let cached = CachedSchemaPages::from_schema(&build_schema());
        let page_bytes = cached.get_page(1); // initialize
        let json = String::from_utf8(page_bytes.to_vec()).unwrap();

        println!("Initialize tool:\n{}", json);

        assert!(json.contains("seeds="));
        assert!(json.contains("\"vault_bump\""));
        assert!(json.contains("\"auth_bump\""));
    }

    #[test]
    fn test_vault_size() {
        assert_eq!(core::mem::size_of::<Vault>(), 88);
    }

    #[test]
    fn test_cached_pages_zero_alloc_lookup() {
        // Verify that get_page returns references (no allocation)
        let cached = CachedSchemaPages::from_schema(&build_schema());

        // Multiple calls should return identical slices (no regeneration)
        let page0_first = cached.get_page(0);
        let page0_second = cached.get_page(0);

        assert!(core::ptr::eq(page0_first.as_ptr(), page0_second.as_ptr()),
            "get_page should return same slice (no reallocation)");
    }
}
