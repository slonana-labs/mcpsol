# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What This Is

mcpsol embeds MCP (Model Context Protocol) schemas into Solana programs. Programs expose a `list_tools` instruction that returns JSON schema via `set_return_data`. No external IDL hosting - schema lives in the binary.

## Commands

```bash
cargo test --workspace              # Run all tests
cargo test -p mcpsol-core test_name # Run single test
cargo clippy --workspace            # Lint (strict - see below)
cargo fmt --all                     # Format
cargo build-sbf                     # Build for Solana
```

## Workspace Structure

| Crate | Purpose |
|-------|---------|
| `macros/` | Proc macros (`#[mcp_program]`, `#[mcp_instruction]`) |
| `sdk/` | Runtime SDK (Context, Accounts, read helpers) - re-exports macros |
| `core/` | Schema types, JSON generation, discriminators |
| `anchor/` | Anchor framework integration |
| `native/` | Native solana-program integration |
| `client/` | Off-chain client for schema discovery |
| `idl2mcp/` | Convert Anchor IDL to MCP schema |

## Key Constraints

**1024-byte limit**: Schema must fit in `return_data`. Compact JSON format used.

**no_std**: SDK code runs on-chain. No std library in `sdk/`, `macros/`, `core/`.

**Strict clippy**: `unwrap_used`, `expect_used`, `panic` are warnings. Use `ok_or()` or handle errors.

## Macro Code Generation

The `#[mcp_program]` macro in `macros/src/program.rs` generates:
- Program entrypoint
- Instruction dispatcher (discriminator matching)
- `list_tools` handler returning embedded schema

To test macro changes, run examples:
```bash
cargo test -p minimal-counter
cargo test -p counter
cargo test -p mcp_vault
```

## Discriminators

8-byte discriminators: `sha256("global:<instruction_name>")[..8]`

Same algorithm as Anchor. See `core/src/discriminator.rs`.
