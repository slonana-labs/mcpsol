//! Anchor Counter with MCP Schema
//!
//! This example shows how to add MCP tool discovery to an Anchor program.

use anchor_lang::prelude::*;
use mcpsol_anchor::prelude::*;

declare_id!("AnchorCntr111111111111111111111111111111111");

/// Define the MCP schema for this program
/// This will be returned by the list_tools instruction
impl McpProgram for AnchorCounter {
    fn mcp_schema() -> McpSchema {
        McpSchemaBuilder::new("anchor_counter")
            .add_tool(
                tool("list_tools")
                    .description("List available MCP tools")
                    .build()
            )
            .add_tool(
                tool("initialize")
                    .description("Create a new counter account")
                    .signer_writable("counter")
                    .signer("authority")
                    .account("system_program", false, false)
                    .build()
            )
            .add_tool(
                tool("increment")
                    .description("Add amount to counter value")
                    .writable("counter")
                    .signer("authority")
                    .arg("amount", ArgType::U64)
                    .build()
            )
            .add_tool(
                tool("decrement")
                    .description("Subtract amount from counter value")
                    .writable("counter")
                    .signer("authority")
                    .arg("amount", ArgType::U64)
                    .build()
            )
            .build()
    }
}

// Dummy struct for McpProgram impl
pub struct AnchorCounter;

#[program]
pub mod anchor_counter {
    use super::*;

    /// List available MCP tools
    /// Call via simulation, read return_data to get schema
    pub fn list_tools(_ctx: Context<ListToolsCtx>) -> Result<()> {
        let schema_bytes = <super::AnchorCounter as McpProgram>::schema_bytes();
        anchor_lang::solana_program::program::set_return_data(&schema_bytes);
        msg!("Returned MCP schema ({} bytes)", schema_bytes.len());
        Ok(())
    }

    /// Initialize a new counter
    pub fn initialize(ctx: Context<Initialize>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        counter.authority = ctx.accounts.authority.key();
        counter.count = 0;
        msg!("Counter initialized");
        Ok(())
    }

    /// Increment counter by amount
    pub fn increment(ctx: Context<Modify>, amount: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        require!(
            counter.authority == ctx.accounts.authority.key(),
            ErrorCode::Unauthorized
        );
        counter.count = counter.count.checked_add(amount)
            .ok_or(ErrorCode::Overflow)?;
        msg!("Incremented to {}", counter.count);
        Ok(())
    }

    /// Decrement counter by amount
    pub fn decrement(ctx: Context<Modify>, amount: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        require!(
            counter.authority == ctx.accounts.authority.key(),
            ErrorCode::Unauthorized
        );
        counter.count = counter.count.checked_sub(amount)
            .ok_or(ErrorCode::Underflow)?;
        msg!("Decremented to {}", counter.count);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ListToolsCtx {}

#[derive(Accounts)]
pub struct Initialize<'info> {
    #[account(
        init,
        payer = authority,
        space = 8 + Counter::INIT_SPACE
    )]
    pub counter: Account<'info, Counter>,
    #[account(mut)]
    pub authority: Signer<'info>,
    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    pub counter: Account<'info, Counter>,
    pub authority: Signer<'info>,
}

#[account]
#[derive(InitSpace)]
pub struct Counter {
    pub authority: Pubkey,
    pub count: u64,
}

#[error_code]
pub enum ErrorCode {
    #[msg("Unauthorized")]
    Unauthorized,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Arithmetic underflow")]
    Underflow,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_schema_generation() {
        let schema = <AnchorCounter as McpProgram>::mcp_schema();
        assert_eq!(schema.name, "anchor_counter");
        assert_eq!(schema.tools.len(), 4);
    }

    #[test]
    fn test_schema_size() {
        let bytes = <AnchorCounter as McpProgram>::schema_bytes();
        println!("Schema: {}", String::from_utf8_lossy(&bytes));
        println!("Size: {} bytes", bytes.len());
        assert!(bytes.len() <= 1024);
    }
}
