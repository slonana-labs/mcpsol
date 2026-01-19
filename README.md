# mcpsol

On-chain interface discovery for Solana programs.

## Overview

mcpsol implements the Model Context Protocol (MCP) as a native communication layer for SVM programs. Programs expose their capabilities through a standardized `list_tools` instruction that returns a compact JSON schema via Solana's `set_return_data` syscall.

This enables runtime interface discovery without external dependencies:

- Schema is compiled into the program binary (no version drift)
- Discovery via simulated transaction (no file hosting required)
- Standardized JSON format across all frameworks
- Sub-kilobyte schema size (fits within 1024-byte return_data limit)

## Installation

```toml
[dependencies]
mcpsol = "0.1"
bytemuck = { version = "1.14", features = ["derive"] }
```

For Anchor programs:
```toml
[dependencies]
mcpsol-anchor = "0.1"
```

For native `solana-program`:
```toml
[dependencies]
mcpsol-native = "0.1"
```

## Usage

### Macro-based Integration

```rust
use mcpsol::prelude::*;

#[derive(Clone, Copy, bytemuck::Pod, bytemuck::Zeroable, McpAccount)]
#[repr(C)]
#[mcp_account(name = "counter", description = "Stores count and authority")]
pub struct Counter {
    pub count: i64,
    pub authority: [u8; 32],
}

#[mcp_program(name = "counter", description = "A minimal counter program")]
pub mod counter {
    use super::*;

    #[mcp_instruction(
        name = "increment",
        description = "Add amount to counter value",
        accounts = "counter:mut, authority:signer"
    )]
    pub fn increment(ctx: Context<Modify>, amount: u64) -> Result<()> {
        // implementation
    }
}
```

The macro generates:
- Program entrypoint
- Instruction dispatcher with discriminator matching
- `list_tools` instruction returning embedded schema
- Compile-time schema constant

### Client Discovery

```rust
use mcpsol_client::McpClient;

let client = McpClient::new("https://api.devnet.solana.com");
let schema = client.list_tools(&program_id)?;

for tool in &schema.tools {
    println!("{} - {}", tool.name, tool.description.unwrap_or_default());
}
```

### IDL Migration

Convert existing Anchor IDL to MCP schema:

```bash
idl2mcp --input target/idl/program.json --output schema.json
```

## Schema Format

```json
{
  "v": "2024-11-05",
  "name": "counter",
  "tools": [
    {
      "n": "increment",
      "d": "0b12680968ae3b21",
      "p": {
        "counter_w": "pubkey",
        "authority_s": "pubkey",
        "amount": "int"
      },
      "r": ["counter_w", "authority_s", "amount"]
    }
  ]
}
```

### Account Suffixes

| Suffix | Meaning |
|--------|---------|
| `_s` | Signer required |
| `_w` | Writable |
| `_sw` | Signer and writable |
| (none) | Read-only |

### Types

| Schema | Rust | Size |
|--------|------|------|
| `int` | `u64` | 8 bytes |
| `u8`-`u128` | unsigned int | varies |
| `i8`-`i128` | signed int | varies |
| `bool` | `bool` | 1 byte |
| `pubkey` | `Pubkey` | 32 bytes |
| `str` | `String` | length-prefixed |
| `bytes` | `Vec<u8>` | length-prefixed |

## Crate Structure

| Crate | Purpose |
|-------|---------|
| `mcpsol-core` | Schema types, JSON generation, discriminator utilities |
| `mcpsol` | Pinocchio SDK with proc-macros |
| `mcpsol-macros` | Procedural macros |
| `mcpsol-anchor` | Anchor integration |
| `mcpsol-native` | Native solana-program integration |
| `mcpsol-client` | Client library for discovery |
| `idl2mcp` | Anchor IDL converter |

## Examples

| Example | Description |
|---------|-------------|
| `examples/minimal-counter` | Macro-based implementation |
| `examples/counter` | Manual schema building |
| `examples/vault` | PDA derivation |
| `examples/native-counter` | Native solana-program |
| `examples/anchor-counter` | Anchor integration |

## Limitations

- **Schema size**: Complex programs may exceed the 1024-byte limit
- **Static schemas**: Fixed at compile time
- **Type brevity**: Compact format omits detailed type information

## Contributing

```bash
cargo test --workspace
cargo clippy --workspace
cargo fmt --all
cargo build-sbf
```

## License

MIT OR Apache-2.0
