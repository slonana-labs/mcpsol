# Architecture

## Overview

```
┌─────────────────────────────────────────────────────────────────┐
│                         Build Time                               │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   Source Code          Proc Macros           Program Binary     │
│   ┌──────────┐        ┌──────────┐          ┌──────────────┐   │
│   │ #[mcp_   │        │ mcpsol-  │          │ Entrypoint   │   │
│   │ program] │ ────── │ macros   │ ──────── │ Dispatcher   │   │
│   │          │        │          │          │ Schema const │   │
│   │ #[mcp_   │        │ Parse    │          │ list_tools   │   │
│   │ instruc- │        │ Generate │          │ Business     │   │
│   │ tion]    │        │ Emit     │          │ logic        │   │
│   └──────────┘        └──────────┘          └──────────────┘   │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────────┐
│                         Runtime                                  │
├─────────────────────────────────────────────────────────────────┤
│                                                                  │
│   Client               SVM Runtime            Program            │
│   ┌──────────┐        ┌──────────┐          ┌──────────────┐   │
│   │ Simulate │        │ Execute  │          │ list_tools:  │   │
│   │ list_    │ ────── │ read-    │ ──────── │ set_return_  │   │
│   │ tools    │        │ only     │          │ data(schema) │   │
│   └────┬─────┘        └──────────┘          └──────────────┘   │
│        │                                                         │
│        v                                                         │
│   ┌──────────┐                                                  │
│   │ Parse    │                                                  │
│   │ JSON     │                                                  │
│   │ Build tx │                                                  │
│   └──────────┘                                                  │
│                                                                  │
└─────────────────────────────────────────────────────────────────┘
```

## Crate Structure

```
mcpsol/
├── core/           # Framework-agnostic types
├── macros/         # Proc macros
├── sdk/            # Pinocchio integration (main crate)
├── anchor/         # Anchor integration
├── native/         # solana-program integration
├── client/         # Rust client library
└── idl2mcp/        # IDL converter tool
```

### mcpsol-core

Zero-dependency crate containing:

- `McpSchema` - Root schema type
- `McpTool` - Tool/instruction definition
- `McpToolBuilder` - Builder pattern for tools
- `McpSchemaBuilder` - Builder pattern for schemas
- `generate_schema_bytes()` - JSON serialization
- `generate_paginated_schema_bytes()` - Paginated serialization
- Discriminator calculation utilities

No framework dependencies. Used by all other crates.

### mcpsol-macros

Procedural macros:

- `#[mcp_program]` - Generates entrypoint, dispatcher, schema constant
- `#[mcp_instruction]` - Registers instruction with schema
- `#[derive(McpAccount)]` - Generates account discriminator
- `#[mcp_account]` - Account metadata for schema

### mcpsol (sdk)

Main SDK for Pinocchio-based programs:

- Re-exports core types
- `prelude` module for convenient imports
- `Context` type for instruction handlers
- `Accounts` derive macro
- Error types

### mcpsol-anchor

Anchor integration:

- `McpProgram` trait
- Builder helpers (`tool()` function)
- Re-exports for Anchor compatibility

### mcpsol-native

Native `solana-program` integration:

- Same API as core
- Works without Pinocchio

### mcpsol-client

Client library for schema discovery:

- `McpClient` - RPC wrapper
- `list_tools()` - Schema discovery
- `build_instruction()` - Instruction construction from schema

### idl2mcp

CLI tool for converting Anchor IDL to MCP schema:

```bash
idl2mcp --input target/idl/program.json --output schema.json
```

## Discriminator Calculation

Discriminators use Anchor-compatible SHA256 sighash:

```rust
// Instructions: SHA256("global:<name>")[..8]
let disc = sha256(b"global:increment")[..8];

// Accounts: SHA256("account:<Name>")[..8]
let disc = sha256(b"account:Counter")[..8];

// Reserved: list_tools
const LIST_TOOLS: [u8; 8] = [0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0];
```

This ensures compatibility with existing Anchor programs.

## Schema Generation Flow

### Macro-based (compile time)

```
1. #[mcp_program] parses module
2. Collects #[mcp_instruction] attributes
3. Generates schema JSON as const byte array
4. Emits dispatcher matching discriminators
5. Emits list_tools handler returning schema
```

### Builder-based (runtime)

```
1. build_schema() called on first list_tools
2. McpSchemaBuilder constructs McpSchema
3. generate_paginated_schema_bytes() serializes
4. set_return_data() returns to caller
```

## Pagination

For schemas exceeding 1024 bytes, pagination splits tools across multiple calls:

```
Request:  list_tools(cursor=0)
Response: {"name":"prog","tools":[{tool0}],"nextCursor":"1"}

Request:  list_tools(cursor=1)
Response: {"name":"prog","tools":[{tool1}],"nextCursor":"2"}

Request:  list_tools(cursor=2)
Response: {"name":"prog","tools":[{tool2}]}  // no nextCursor = done
```

Each page contains one tool. Cursor is the tool index.

## Memory Layout

Account data follows Anchor conventions:

```
[0..8]     discriminator (8 bytes)
[8..]      account fields (Pod layout)
```

Instruction data:

```
[0..8]     discriminator (8 bytes)
[8..]      arguments (little-endian)
```
