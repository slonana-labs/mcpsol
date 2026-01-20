# Integration Guide

## Choosing an Approach

| Approach | Use When |
|----------|----------|
| Macro-based | New programs, minimal boilerplate |
| Builder pattern | Need custom schema, pagination |
| Anchor integration | Existing Anchor programs |
| Native integration | Using raw solana-program |

## Macro-based Integration

Recommended for new programs. Generates everything automatically.

### Setup

```toml
[dependencies]
mcpsol = "0.1"
bytemuck = { version = "1.14", features = ["derive"] }
```

### Define Account

```rust
use mcpsol::prelude::*;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, McpAccount)]
#[repr(C)]
#[mcp_account(name = "counter", description = "Stores count value")]
pub struct Counter {
    pub count: i64,
    pub authority: [u8; 32],
}
```

Requirements:
- `#[repr(C)]` for deterministic layout
- `Pod` + `Zeroable` for zero-copy
- `McpAccount` generates discriminator

### Define Accounts Struct

```rust
#[derive(Accounts)]
pub struct Modify<'info> {
    #[account(mut)]
    pub counter: &'info AccountInfo,
    #[account(signer)]
    pub authority: Signer<'info>,
}
```

### Define Program

```rust
#[mcp_program(name = "my_counter", description = "A counter program")]
pub mod my_counter {
    use super::*;

    #[mcp_instruction(
        name = "increment",
        description = "Add to counter",
        accounts = "counter:mut, authority:signer"
    )]
    pub fn increment(ctx: Context<Modify>, amount: u64) -> Result<()> {
        // implementation
        Ok(())
    }
}
```

Account string format: `name:flags` where flags are:
- `mut` = writable
- `signer` = signer
- `mut,signer` = both

### Generated Code

The macro generates:
- `pinocchio::entrypoint!(process_instruction)`
- Instruction dispatcher
- `list_tools` handler
- `MCP_SCHEMA_JSON` constant
- `LIST_TOOLS_DISCRIMINATOR` constant
- Discriminator constants for each instruction

## Builder Pattern Integration

For programs needing custom schema construction.

### Setup

```toml
[dependencies]
mcpsol-core = "0.1"
pinocchio = "0.8"
```

### Build Schema

```rust
use mcpsol_core::{
    McpSchema, McpSchemaBuilder, McpToolBuilder, ArgType,
    LIST_TOOLS_DISCRIMINATOR, generate_paginated_schema_bytes,
};

fn build_schema() -> McpSchema {
    McpSchemaBuilder::new("my_program")
        .add_tool(
            McpToolBuilder::new("list_tools")
                .description("List MCP tools")
                .build()
        )
        .add_tool(
            McpToolBuilder::new("transfer")
                .description("Transfer tokens")
                .signer_writable_desc("from", "Source account")
                .writable_desc("to", "Destination account")
                .arg_desc("amount", "Amount to transfer", ArgType::U64)
                .build()
        )
        .build()
}
```

### Handle list_tools

```rust
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    let discriminator: [u8; 8] = data[..8].try_into()
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    if discriminator == LIST_TOOLS_DISCRIMINATOR {
        let cursor = data.get(8).copied().unwrap_or(0);
        let schema = build_schema();
        let bytes = generate_paginated_schema_bytes(&schema, cursor);
        pinocchio::program::set_return_data(&bytes);
        return Ok(());
    }

    // dispatch other instructions...
}
```

## Anchor Integration

### Setup

```toml
[dependencies]
anchor-lang = "0.30"
mcpsol-anchor = "0.1"
```

### Implement McpProgram

```rust
use anchor_lang::prelude::*;
use mcpsol_anchor::prelude::*;

pub struct MyProgram;

impl McpProgram for MyProgram {
    fn mcp_schema() -> McpSchema {
        McpSchemaBuilder::new("my_program")
            .add_tool(
                tool("initialize")
                    .description("Initialize account")
                    .signer_writable("account")
                    .signer("authority")
                    .build()
            )
            .build()
    }
}
```

### Add list_tools Instruction

```rust
#[program]
pub mod my_program {
    pub fn list_tools(_ctx: Context<ListTools>) -> Result<()> {
        let bytes = <super::MyProgram as McpProgram>::schema_bytes();
        anchor_lang::solana_program::program::set_return_data(&bytes);
        Ok(())
    }
}

#[derive(Accounts)]
pub struct ListTools {}
```

## Native Integration

### Setup

```toml
[dependencies]
mcpsol-native = "0.1"
solana-program = "2.0"
```

### Implementation

```rust
use mcpsol_native::prelude::*;
use solana_program::{
    entrypoint, entrypoint::ProgramResult,
    program::set_return_data, pubkey::Pubkey,
    account_info::AccountInfo,
};

fn build_schema() -> McpSchema {
    McpSchemaBuilder::new("native_program")
        .add_tool(/* ... */)
        .build()
}

entrypoint!(process_instruction);

pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if data.len() < 8 {
        return Err(ProgramError::InvalidInstructionData);
    }
    if data[..8] == LIST_TOOLS_DISCRIMINATOR {
        let cursor = data.get(8).copied().unwrap_or(0);
        let schema = build_schema();
        let bytes = generate_paginated_schema_bytes(&schema, cursor);
        set_return_data(&bytes);
        return Ok(());
    }
    // ...
}
```

## Testing

### Verify Schema Generation

```rust
#[test]
fn test_schema_size() {
    let schema = build_schema();
    for i in 0..schema.tools.len() {
        let bytes = generate_paginated_schema_bytes(&schema, i as u8);
        assert!(bytes.len() <= 1024, "Page {} exceeds limit", i);
    }
}
```

### Verify Discriminators

```rust
#[test]
fn test_discriminators() {
    assert_eq!(
        LIST_TOOLS_DISCRIMINATOR,
        [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0]
    );
}
```

## Deployment Checklist

1. Build: `cargo build-sbf`
2. Test: `cargo test`
3. Verify schema fits: check test output for byte counts
4. Deploy: `solana program deploy target/deploy/program.so`
5. Verify discovery: simulate `list_tools` against deployed program
