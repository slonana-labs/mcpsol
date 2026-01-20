//! Native Solana Counter with MCP Schema
//!
//! This example shows how to use mcpsol-native to add MCP tool discovery
//! to a native Solana program (using solana-program directly).

use bytemuck::{Pod, Zeroable};
use mcpsol_native::prelude::*;
use mcpsol_native::{McpSchema, generate_paginated_schema_bytes};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::set_return_data,
    program_error::ProgramError,
    pubkey::Pubkey,
};

// Program ID (replace with your deployed program ID)
solana_program::declare_id!("NativCntr1111111111111111111111111111111111");

/// Counter account data (16 bytes)
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct Counter {
    pub authority: [u8; 32], // Pubkey
    pub count: u64,
}

/// Account discriminator for Counter
pub const COUNTER_DISCRIMINATOR: [u8; 8] = [0xff, 0x17, 0x6b, 0x7a, 0x13, 0x8a, 0xc6, 0x3e];

/// Build the MCP schema with full descriptions for AI agents
fn build_schema() -> McpSchema {
    McpSchemaBuilder::new("native_counter")
        .add_tool(
            McpToolBuilder::new("list_tools")
                .description("List available MCP tools. Pass cursor byte after discriminator to paginate.")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("initialize")
                .description("Create a new counter account owned by the authority")
                .signer_writable_desc("counter", "The counter account to initialize (must be pre-allocated)")
                .signer_desc("authority", "The authority who will control this counter")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("increment")
                .description("Add amount to the counter value. Only the authority can call this.")
                .writable_desc("counter", "The counter account to modify")
                .signer_desc("authority", "Must match the counter's authority")
                .arg_desc("amount", "The value to add to the counter", ArgType::U64)
                .build()
        )
        .add_tool(
            McpToolBuilder::new("decrement")
                .description("Subtract amount from the counter value. Only the authority can call this.")
                .writable_desc("counter", "The counter account to modify")
                .signer_desc("authority", "Must match the counter's authority")
                .arg_desc("amount", "The value to subtract from the counter", ArgType::U64)
                .build()
        )
        .build()
}

/// Lazy static schema - built once on first access
static SCHEMA: std::sync::OnceLock<McpSchema> = std::sync::OnceLock::new();

fn get_schema() -> &'static McpSchema {
    SCHEMA.get_or_init(build_schema)
}

// Instruction discriminators
const LIST_TOOLS: [u8; 8] = LIST_TOOLS_DISCRIMINATOR;
const INITIALIZE: [u8; 8] = [0xaf, 0xaf, 0x6d, 0x1f, 0x0d, 0x98, 0x9b, 0xed];
const INCREMENT: [u8; 8] = [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21];
const DECREMENT: [u8; 8] = [0x6a, 0xe3, 0xa8, 0x3b, 0xf8, 0x1b, 0x96, 0x65];

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
        LIST_TOOLS => {
            msg!("Instruction: list_tools");
            // Get cursor from instruction data (byte 8), default to 0
            let cursor = data.get(8).copied().unwrap_or(0);
            let schema_bytes = generate_paginated_schema_bytes(get_schema(), cursor);
            set_return_data(&schema_bytes);
            Ok(())
        }
        INITIALIZE => {
            msg!("Instruction: initialize");
            process_initialize(program_id, accounts)
        }
        INCREMENT => {
            msg!("Instruction: increment");
            // SECURITY: Verify data length before slicing
            if data.len() < 16 {
                return Err(ProgramError::InvalidInstructionData);
            }
            // Safe: Length >= 16 verified above
            let amount_bytes: [u8; 8] = data[8..16]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            let amount = u64::from_le_bytes(amount_bytes);
            process_increment(program_id, accounts, amount)
        }
        DECREMENT => {
            msg!("Instruction: decrement");
            // SECURITY: Verify data length before slicing
            if data.len() < 16 {
                return Err(ProgramError::InvalidInstructionData);
            }
            // Safe: Length >= 16 verified above
            let amount_bytes: [u8; 8] = data[8..16]
                .try_into()
                .map_err(|_| ProgramError::InvalidInstructionData)?;
            let amount = u64::from_le_bytes(amount_bytes);
            process_decrement(program_id, accounts, amount)
        }
        _ => {
            msg!("Unknown instruction");
            Err(ProgramError::InvalidInstructionData)
        }
    }
}

fn process_initialize(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;
    let authority = next_account_info(accounts_iter)?;

    // Verify signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // SECURITY: Verify ownership
    if counter_account.owner != program_id {
        return Err(ProgramError::IllegalOwner);
    }

    // SECURITY: Verify counter is writable
    if !counter_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // SECURITY: Verify data size
    let data_len = counter_account.data_len();
    if data_len < 48 {
        msg!("Account data too small: {} < 48", data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }

    // Initialize account data
    let mut data = counter_account.try_borrow_mut_data()?;

    // Write discriminator
    data[..8].copy_from_slice(&COUNTER_DISCRIMINATOR);

    // Write authority pubkey
    data[8..40].copy_from_slice(authority.key.as_ref());

    // Write initial count (0)
    data[40..48].copy_from_slice(&0u64.to_le_bytes());

    msg!("Counter initialized");
    Ok(())
}

fn process_increment(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;
    let authority = next_account_info(accounts_iter)?;

    // SECURITY: Verify counter is owned by this program
    if counter_account.owner != program_id {
        msg!("Invalid counter owner");
        return Err(ProgramError::IllegalOwner);
    }

    // SECURITY: Verify counter is writable
    if !counter_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // SECURITY: Verify data size before accessing
    let data_len = counter_account.data_len();
    if data_len < 48 {
        msg!("Account data too small: {} < 48", data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }

    let mut data = counter_account.try_borrow_mut_data()?;

    // Verify discriminator
    if data[..8] != COUNTER_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify authority
    if &data[8..40] != authority.key.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Read current count - safe: data_len >= 48 verified above
    let count_bytes: [u8; 8] = data[40..48]
        .try_into()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let count = u64::from_le_bytes(count_bytes);

    // Increment
    let new_count = count.checked_add(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Write new count
    data[40..48].copy_from_slice(&new_count.to_le_bytes());

    msg!("Incremented {} -> {}", count, new_count);
    Ok(())
}

fn process_decrement(program_id: &Pubkey, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
    let accounts_iter = &mut accounts.iter();
    let counter_account = next_account_info(accounts_iter)?;
    let authority = next_account_info(accounts_iter)?;

    // SECURITY: Verify counter is owned by this program
    if counter_account.owner != program_id {
        msg!("Invalid counter owner");
        return Err(ProgramError::IllegalOwner);
    }

    // SECURITY: Verify counter is writable
    if !counter_account.is_writable {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify signer
    if !authority.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // SECURITY: Verify data size before accessing
    let data_len = counter_account.data_len();
    if data_len < 48 {
        msg!("Account data too small: {} < 48", data_len);
        return Err(ProgramError::AccountDataTooSmall);
    }

    let mut data = counter_account.try_borrow_mut_data()?;

    // Verify discriminator
    if data[..8] != COUNTER_DISCRIMINATOR {
        return Err(ProgramError::InvalidAccountData);
    }

    // Verify authority
    if &data[8..40] != authority.key.as_ref() {
        return Err(ProgramError::InvalidAccountData);
    }

    // Read current count - safe: data_len >= 48 verified above
    let count_bytes: [u8; 8] = data[40..48]
        .try_into()
        .map_err(|_| ProgramError::InvalidAccountData)?;
    let count = u64::from_le_bytes(count_bytes);

    // Decrement
    let new_count = count.checked_sub(amount)
        .ok_or(ProgramError::ArithmeticOverflow)?;

    // Write new count
    data[40..48].copy_from_slice(&new_count.to_le_bytes());

    msg!("Decremented {} -> {}", count, new_count);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_paginated_schema_page0() {
        let schema = build_schema();
        let json_bytes = generate_paginated_schema_bytes(&schema, 0);
        let json = String::from_utf8(json_bytes).unwrap();

        println!("Page 0 ({} bytes):\n{}", json.len(), json);

        // First page should have list_tools
        assert!(json.contains("\"name\":\"list_tools\""));
        assert!(json.contains("\"nextCursor\":\"1\""));
        assert!(json.len() <= MAX_RETURN_DATA_SIZE);
    }

    #[test]
    fn test_paginated_schema_page1() {
        let schema = build_schema();
        let json_bytes = generate_paginated_schema_bytes(&schema, 1);
        let json = String::from_utf8(json_bytes).unwrap();

        println!("Page 1 ({} bytes):\n{}", json.len(), json);

        // Second page should have initialize with full descriptions
        assert!(json.contains("\"name\":\"initialize\""));
        assert!(json.contains("\"description\":\"Create a new counter account"));
        assert!(json.contains("\"counter\":{\"type\":\"pubkey\",\"signer\":true,\"writable\":true"));
        assert!(json.contains("\"description\":\"The counter account to initialize"));
        assert!(json.contains("\"nextCursor\":\"2\""));
        assert!(json.len() <= MAX_RETURN_DATA_SIZE);
    }

    #[test]
    fn test_paginated_schema_last_page() {
        let schema = build_schema();
        let json_bytes = generate_paginated_schema_bytes(&schema, 3);
        let json = String::from_utf8(json_bytes).unwrap();

        println!("Page 3 (last) ({} bytes):\n{}", json.len(), json);

        // Last page (decrement) should NOT have nextCursor
        assert!(json.contains("\"name\":\"decrement\""));
        assert!(!json.contains("nextCursor"));
        assert!(json.len() <= MAX_RETURN_DATA_SIZE);
    }

    #[test]
    fn test_all_pages_fit_in_return_data() {
        let schema = build_schema();

        for cursor in 0..schema.tools.len() {
            let json_bytes = generate_paginated_schema_bytes(&schema, cursor as u8);
            println!("Page {} size: {} bytes", cursor, json_bytes.len());
            assert!(
                json_bytes.len() <= MAX_RETURN_DATA_SIZE,
                "Page {} ({} bytes) exceeds limit",
                cursor,
                json_bytes.len()
            );
        }
    }
}
