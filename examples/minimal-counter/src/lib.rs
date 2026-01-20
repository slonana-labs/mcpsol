//! Minimal Counter - The simplest possible MCP-enabled Solana program
//!
//! This example shows the ideal developer experience:
//! - ~50 lines of actual code (vs 200+ in the verbose example)
//! - Zero boilerplate for discriminators, dispatching, or schema
//!
//! Compare with examples/counter which has the same functionality
//! but with manual wiring.

use mcpsol::prelude::*;

/// Counter account - just define your data
#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, McpAccount)]
#[repr(C)]
#[mcp_account(name = "counter", description = "A simple counter")]
pub struct Counter {
    pub count: i64,
    pub authority: [u8; 32],
}

/// Accounts for modifying the counter
#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    pub counter: &'info AccountInfo,
    #[account(signer)]
    pub authority: Signer<'info>,
}

// That's it! The #[mcp_program] macro generates:
// - Entrypoint
// - Instruction dispatcher
// - MCP schema JSON
// - list_tools instruction
// - All discriminator constants

#[mcp_program(name = "minimal_counter", description = "Minimal MCP counter example")]
pub mod minimal_counter {
    use super::*;

    #[mcp_instruction(
        name = "increment",
        description = "Increase counter value",
        accounts = "counter:mut, authority:signer"
    )]
    pub fn increment<'info>(ctx: Context<'info, Modify<'info>>, amount: u64) -> Result<()> {
        let counter = ctx.accounts.counter;
        let authority = ctx.accounts.authority.key();
        let mut data = counter.try_borrow_mut_data()?;

        // Verify discriminator
        if data[..8] != Counter::DISCRIMINATOR {
            return Err(McpSolError::InvalidAccount.into());
        }

        // Verify authority matches stored authority
        // Layout: [0..8] discriminator, [8..16] count, [16..48] authority
        if data[16..48] != *authority.as_ref() {
            return Err(McpSolError::ConstraintViolation.into());
        }

        // Update count - safe: slice [8..16] is 8 bytes after discriminator check
        let current_bytes: [u8; 8] = data[8..16]
            .try_into()
            .map_err(|_| McpSolError::InvalidAccount)?;
        let current = i64::from_le_bytes(current_bytes);
        let new_count = current.saturating_add(amount as i64);
        data[8..16].copy_from_slice(&new_count.to_le_bytes());

        Ok(())
    }

    #[mcp_instruction(
        name = "decrement",
        description = "Decrease counter value",
        accounts = "counter:mut, authority:signer"
    )]
    pub fn decrement<'info>(ctx: Context<'info, Modify<'info>>, amount: u64) -> Result<()> {
        let counter = ctx.accounts.counter;
        let authority = ctx.accounts.authority.key();
        let mut data = counter.try_borrow_mut_data()?;

        if data[..8] != Counter::DISCRIMINATOR {
            return Err(McpSolError::InvalidAccount.into());
        }

        // Verify authority matches stored authority
        if data[16..48] != *authority.as_ref() {
            return Err(McpSolError::ConstraintViolation.into());
        }

        // Update count - safe: slice [8..16] is 8 bytes after discriminator check
        let current_bytes: [u8; 8] = data[8..16]
            .try_into()
            .map_err(|_| McpSolError::InvalidAccount)?;
        let current = i64::from_le_bytes(current_bytes);
        let new_count = current.saturating_sub(amount as i64);
        data[8..16].copy_from_slice(&new_count.to_le_bytes());

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generated() {
        // Verify the schema constant was generated
        let schema = minimal_counter::MCP_SCHEMA_JSON;
        let schema_str = std::str::from_utf8(schema).unwrap();

        println!("Generated schema ({} bytes):\n{}", schema.len(), schema_str);

        // Parse and verify
        assert!(schema_str.contains("\"name\":\"minimal_counter\""));
        assert!(schema_str.contains("increment"));
        assert!(schema_str.contains("decrement"));
        assert!(schema_str.contains("list_tools"));
        assert!(schema.len() <= 1024, "Schema too large for return_data");
    }

    #[test]
    fn test_discriminators() {
        // Verify discriminator was generated
        let disc = minimal_counter::LIST_TOOLS_DISCRIMINATOR;
        assert_eq!(disc, [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0]);
    }

    #[test]
    fn test_counter_discriminator() {
        // Verify account discriminator
        println!("Counter discriminator: {:02x?}", Counter::DISCRIMINATOR);
    }
}
