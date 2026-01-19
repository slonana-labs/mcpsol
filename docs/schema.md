# Schema Specification

## Wire Format

Schemas are JSON objects optimized for size. Field names are abbreviated to fit within Solana's 1024-byte `return_data` limit.

### Root Object

```json
{
  "v": "2024-11-05",
  "name": "program_name",
  "tools": [...]
}
```

| Field | Type | Description |
|-------|------|-------------|
| `v` | string | Protocol version (date format) |
| `name` | string | Program identifier |
| `tools` | array | Available instructions |

### Tool Object

```json
{
  "n": "increment",
  "i": "Add amount to counter",
  "d": "0b12680968ae3b21",
  "p": {...},
  "r": [...]
}
```

| Field | Type | Required | Description |
|-------|------|----------|-------------|
| `n` | string | yes | Instruction name |
| `d` | string | yes | 8-byte discriminator (hex) |
| `i` | string | no | Human-readable description |
| `p` | object | no | Parameter definitions |
| `r` | array | no | Required parameter order |

### Parameter Object

Parameters are key-value pairs where the key encodes account flags:

```json
{
  "p": {
    "counter_w": "pubkey",
    "authority_s": "pubkey",
    "amount": "int"
  }
}
```

#### Account Suffixes

| Suffix | Meaning | Flags |
|--------|---------|-------|
| `_s` | signer | `is_signer: true` |
| `_w` | writable | `is_writable: true` |
| `_sw` | signer + writable | both true |
| (none) | read-only | both false |

#### Types

| Schema Type | Rust Type | Size | Notes |
|-------------|-----------|------|-------|
| `int` | `u64` | 8 bytes | Default integer |
| `u8` | `u8` | 1 byte | |
| `u16` | `u16` | 2 bytes | |
| `u32` | `u32` | 4 bytes | |
| `u64` | `u64` | 8 bytes | |
| `u128` | `u128` | 16 bytes | |
| `i8` | `i8` | 1 byte | |
| `i16` | `i16` | 2 bytes | |
| `i32` | `i32` | 4 bytes | |
| `i64` | `i64` | 8 bytes | |
| `i128` | `i128` | 16 bytes | |
| `bool` | `bool` | 1 byte | |
| `pubkey` | `Pubkey` | 32 bytes | |
| `str` | `String` | variable | 4-byte length prefix |
| `bytes` | `Vec<u8>` | variable | 4-byte length prefix |

### Required Array

The `r` array specifies parameter order for instruction data serialization:

```json
{
  "r": ["counter_w", "authority_s", "amount"]
}
```

Accounts come first (in order), then arguments.

## Extended Format

For detailed tool descriptions, use the extended format:

```json
{
  "n": "increment",
  "d": "0b12680968ae3b21",
  "description": "Add amount to the counter value",
  "p": {
    "counter": {
      "type": "pubkey",
      "writable": true,
      "description": "The counter account to modify"
    },
    "authority": {
      "type": "pubkey",
      "signer": true,
      "description": "Must match counter authority"
    },
    "amount": {
      "type": "u64",
      "description": "Value to add"
    }
  }
}
```

Extended format is used in paginated responses where each tool gets a full page.

## Pagination

When schemas exceed 1024 bytes, use cursor-based pagination:

### Request

```
Instruction data: [discriminator (8 bytes)][cursor (1 byte)]
```

Cursor values:
- `0` = first tool
- `1` = second tool
- `n` = nth tool

### Response

```json
{
  "v": "2024-11-05",
  "name": "program_name",
  "tools": [{...single tool...}],
  "nextCursor": "1"
}
```

`nextCursor` is absent on the last page.

### Client Algorithm

```python
cursor = 0
tools = []
while True:
    response = simulate(list_tools, cursor)
    tools.extend(response.tools)
    if "nextCursor" not in response:
        break
    cursor = int(response.nextCursor)
```

## Discriminator Calculation

### Instructions

```
SHA256("global:" + instruction_name)[0..8]
```

Example:
```
SHA256("global:increment") = 0b12680968ae3b21...
discriminator = [0x0b, 0x12, 0x68, 0x09, 0x68, 0xae, 0x3b, 0x21]
```

### Accounts

```
SHA256("account:" + AccountName)[0..8]
```

Example:
```
SHA256("account:Counter") = ff176b7a138ac63e...
discriminator = [0xff, 0x17, 0x6b, 0x7a, 0x13, 0x8a, 0xc6, 0x3e]
```

### Reserved

`list_tools` uses the fixed discriminator:
```
[0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0]
```

## Version History

| Version | Date | Changes |
|---------|------|---------|
| `2024-11-05` | 2024-11-05 | Initial specification |

## Size Budget

Target: < 1024 bytes per response

Typical sizes:
- Minimal tool (name + discriminator): ~50 bytes
- Tool with description + 2 accounts + 1 arg: ~200 bytes
- Tool with extended descriptions: ~400 bytes

Programs with 4+ tools should use pagination.
