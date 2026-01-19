//! Example: Simple Counter Program using mcpsol SDK
//!
//! Demonstrates how MCP replaces IDL for program interfaces.
//! The program exposes a `list_tools` instruction that returns the MCP schema.

use mcpsol::prelude::*;
use mcpsol::account::AccountData;
use mcpsol_core::{
    McpSchema, McpSchemaBuilder,
    McpToolBuilder as CoreToolBuilder,
    ArgType, generate_paginated_schema_bytes,
};

// Program ID - the actual deployed address
pub const PROGRAM_ID: Pubkey = five8_const::decode_32_const(
    "7QniyJzHpS7uFdYogBE5oUPxj6TXyNKFgkR4Dztbnbct"
);

/// Counter account data
/// Must be repr(C) and Pod-compatible for zero-copy serialization
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, McpAccount)]
#[repr(C)]
#[mcp_account(
    name = "counter",
    description = "A simple counter that can be incremented or decremented"
)]
pub struct Counter {
    /// Current count value
    pub count: i64,
    /// Authority who can modify this counter (32 bytes)
    pub authority: [u8; 32],
    /// Bump seed for PDA derivation
    pub bump: u8,
    /// Padding to align struct
    pub _padding: [u8; 7],
}

/// Accounts for initialize instruction
#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(mut)]
    pub counter: &'info AccountInfo,
    #[account(signer)]
    pub authority: Signer<'info>,
    pub system_program: &'info AccountInfo,
}

/// Accounts for increment/decrement
#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    pub counter: &'info AccountInfo,
    #[account(signer)]
    pub authority: Signer<'info>,
}

/// Build paginated MCP schema with full descriptions
fn build_schema() -> McpSchema {
    McpSchemaBuilder::new("counter")
        .add_tool(
            CoreToolBuilder::new("list_tools")
                .description("List available MCP tools. Pass cursor byte to paginate.")
                .build()
        )
        .add_tool(
            CoreToolBuilder::new("initialize")
                .description("Create a new counter account with initial value of 0")
                .writable_desc("counter", "The counter account to initialize")
                .signer_desc("authority", "The authority who will control this counter")
                .account_with_desc("system_program", "System program", false, false)
                .build()
        )
        .add_tool(
            CoreToolBuilder::new("increment")
                .description("Add amount to the counter value")
                .writable_desc("counter", "The counter account to modify")
                .signer_desc("authority", "Must match the counter's authority")
                .arg_desc("amount", "Value to add to the counter", ArgType::U64)
                .build()
        )
        .add_tool(
            CoreToolBuilder::new("decrement")
                .description("Subtract amount from the counter value")
                .writable_desc("counter", "The counter account to modify")
                .signer_desc("authority", "Must match the counter's authority")
                .arg_desc("amount", "Value to subtract from the counter", ArgType::U64)
                .build()
        )
        .build()
}

/// Lazy static schema
static SCHEMA: std::sync::OnceLock<McpSchema> = std::sync::OnceLock::new();

fn get_schema() -> &'static McpSchema {
    SCHEMA.get_or_init(build_schema)
}

// Discriminator constants
const LIST_TOOLS: [u8; 8] = [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0];
const INITIALIZE: [u8; 8] = [0xaf, 0xaf, 0x6d, 0x1f, 0x0d, 0x98, 0x9b, 0xed];
const INCREMENT: [u8; 8] = [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21];
const DECREMENT: [u8; 8] = [0x6a, 0xe3, 0xa8, 0x3b, 0xf8, 0x1b, 0x96, 0x65];

// Entrypoint
pinocchio::entrypoint!(process_instruction);

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> pinocchio::ProgramResult {
    if data.len() < 8 {
        return Err(pinocchio::program_error::ProgramError::InvalidInstructionData);
    }

    let discriminator: [u8; 8] = data[..8].try_into()
        .map_err(|_| pinocchio::program_error::ProgramError::InvalidInstructionData)?;
    let ix_data = &data[8..];

    match discriminator {
        LIST_TOOLS => {
            let cursor = data.get(8).copied().unwrap_or(0);
            let schema_bytes = generate_paginated_schema_bytes(get_schema(), cursor);
            pinocchio::program::set_return_data(&schema_bytes);
            Ok(())
        }
        INITIALIZE => {
            process_initialize(program_id, accounts)
        }
        INCREMENT => {
            let amount = parse_u64(ix_data)?;
            process_increment(program_id, accounts, amount)
        }
        DECREMENT => {
            let amount = parse_u64(ix_data)?;
            process_decrement(program_id, accounts, amount)
        }
        _ => {
            pinocchio_log::log!("Unknown discriminator");
            Err(pinocchio::program_error::ProgramError::InvalidInstructionData)
        }
    }
}

fn parse_u64(data: &[u8]) -> core::result::Result<u64, pinocchio::program_error::ProgramError> {
    if data.len() < 8 {
        return Err(pinocchio::program_error::ProgramError::InvalidInstructionData);
    }
    Ok(u64::from_le_bytes(data[..8].try_into().unwrap()))
}

fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> pinocchio::ProgramResult {
    let counter_account = accounts.get(0)
        .ok_or(pinocchio::program_error::ProgramError::NotEnoughAccountKeys)?;
    let authority = accounts.get(1)
        .ok_or(pinocchio::program_error::ProgramError::NotEnoughAccountKeys)?;

    // SECURITY: Verify counter is owned by this program
    // Safety: owner() returns valid pointer to account owner
    if unsafe { counter_account.owner() } != program_id {
        pinocchio_log::log!("Invalid counter owner");
        return Err(pinocchio::program_error::ProgramError::IncorrectProgramId);
    }

    // SECURITY: Verify counter is writable
    if !counter_account.is_writable() {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    if !authority.is_signer() {
        return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature);
    }

    let mut data = counter_account.try_borrow_mut_data()?;

    // Write discriminator
    data[..8].copy_from_slice(&Counter::DISCRIMINATOR);
    // Write count = 0
    data[8..16].copy_from_slice(&0i64.to_le_bytes());
    // Write authority
    data[16..48].copy_from_slice(authority.key().as_ref());
    // Zero bump and padding
    data[48..56].fill(0);

    pinocchio_log::log!("Counter initialized!");
    Ok(())
}

fn process_increment(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> pinocchio::ProgramResult {
    let counter_account = accounts.get(0)
        .ok_or(pinocchio::program_error::ProgramError::NotEnoughAccountKeys)?;
    let authority = accounts.get(1)
        .ok_or(pinocchio::program_error::ProgramError::NotEnoughAccountKeys)?;

    // SECURITY: Verify counter is owned by this program
    // Safety: owner() returns valid pointer to account owner
    if unsafe { counter_account.owner() } != program_id {
        pinocchio_log::log!("Invalid counter owner");
        return Err(pinocchio::program_error::ProgramError::IncorrectProgramId);
    }

    // SECURITY: Verify counter is writable
    if !counter_account.is_writable() {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    if !authority.is_signer() {
        return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature);
    }

    let mut data = counter_account.try_borrow_mut_data()?;

    if data[..8] != Counter::DISCRIMINATOR {
        pinocchio_log::log!("Invalid discriminator");
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    if data[16..48] != *authority.key().as_ref() {
        pinocchio_log::log!("Authority mismatch");
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    let current = i64::from_le_bytes(data[8..16].try_into().unwrap());
    let new_count = current.saturating_add(amount as i64);
    data[8..16].copy_from_slice(&new_count.to_le_bytes());

    pinocchio_log::log!("Incremented to {}", new_count);
    Ok(())
}

fn process_decrement(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> pinocchio::ProgramResult {
    let counter_account = accounts.get(0)
        .ok_or(pinocchio::program_error::ProgramError::NotEnoughAccountKeys)?;
    let authority = accounts.get(1)
        .ok_or(pinocchio::program_error::ProgramError::NotEnoughAccountKeys)?;

    // SECURITY: Verify counter is owned by this program
    // Safety: owner() returns valid pointer to account owner
    if unsafe { counter_account.owner() } != program_id {
        pinocchio_log::log!("Invalid counter owner");
        return Err(pinocchio::program_error::ProgramError::IncorrectProgramId);
    }

    // SECURITY: Verify counter is writable
    if !counter_account.is_writable() {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    if !authority.is_signer() {
        return Err(pinocchio::program_error::ProgramError::MissingRequiredSignature);
    }

    let mut data = counter_account.try_borrow_mut_data()?;

    if data[..8] != Counter::DISCRIMINATOR {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    if data[16..48] != *authority.key().as_ref() {
        return Err(pinocchio::program_error::ProgramError::InvalidAccountData);
    }

    let current = i64::from_le_bytes(data[8..16].try_into().unwrap());
    let new_count = current.saturating_sub(amount as i64);
    data[8..16].copy_from_slice(&new_count.to_le_bytes());

    pinocchio_log::log!("Decremented to {}", new_count);
    Ok(())
}
