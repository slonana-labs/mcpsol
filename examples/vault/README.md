# vault

PDA documentation in MCP schemas for AI agent consumption.

## What This Demonstrates

- Documenting PDA seeds in tool descriptions
- Complex multi-instruction programs
- Query instructions returning data via `set_return_data`
- Proper PDA verification

## Key Pattern: PDA Seeds in Descriptions

AI agents need to derive PDAs before calling instructions. Document seeds in the description:

```rust
McpToolBuilder::new("initialize")
    .description("Create vault PDA. seeds=[\"vault\", owner, mint]")
    .writable_desc("vault", "Vault PDA. seeds=[\"vault\", owner, mint, bump]")
```

This allows AI agents to:
1. Parse the seed format from description
2. Derive the PDA address
3. Call the instruction with correct accounts

## Code Structure

```
src/lib.rs    # Vault with deposit/withdraw/query
```

Instructions:
- `initialize` - Create vault PDA with owner and mint
- `deposit` - Anyone can deposit SOL
- `withdraw` - Only owner can withdraw
- `get_info` - Query balance via return_data

## Build

```bash
cargo build-sbf -p vault
```

## Test

```bash
cargo test -p vault -- --nocapture
```

Tests verify:
- Each schema page fits in 1024 bytes
- PDA seed documentation is present
- Vault struct size is correct (88 bytes)

## Account Structure

```rust
pub struct Vault {
    pub discriminator: [u8; 8],
    pub owner: [u8; 32],
    pub mint: [u8; 32],
    pub bump: u8,
    pub auth_bump: u8,
    pub _padding: [u8; 6],
    pub balance: u64,
}
```

## Deploy

```bash
solana program deploy target/deploy/vault.so --url devnet
```
