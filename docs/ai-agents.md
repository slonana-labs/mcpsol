# AI Agents

How to build AI agents that interact with MCP-enabled Solana programs.

## Discovery Flow

```
1. Agent receives program ID
2. Simulate list_tools instruction
3. Parse schema from return_data
4. Understand available operations
5. Build and submit transactions
```

## Discovering a Program

### Using TypeScript

```typescript
import { Connection, PublicKey, Transaction, TransactionInstruction } from '@solana/web3.js';

const LIST_TOOLS_DISCRIMINATOR = Buffer.from([0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0]);

async function discoverProgram(connection: Connection, programId: PublicKey) {
  const tools = [];
  let cursor = 0;

  while (true) {
    const data = Buffer.concat([
      LIST_TOOLS_DISCRIMINATOR,
      Buffer.from([cursor])
    ]);

    const ix = new TransactionInstruction({
      keys: [],
      programId,
      data
    });

    const tx = new Transaction().add(ix);
    const result = await connection.simulateTransaction(tx);

    if (!result.value.returnData) break;

    const schemaJson = Buffer.from(result.value.returnData.data[0], 'base64').toString();
    const schema = JSON.parse(schemaJson);

    tools.push(...schema.tools);

    if (!schema.nextCursor) break;
    cursor = parseInt(schema.nextCursor);
  }

  return tools;
}
```

### Using Rust

```rust
use mcpsol_client::McpClient;
use solana_sdk::pubkey::Pubkey;

fn discover(program_id: &Pubkey) -> Result<Vec<McpTool>, Error> {
    let client = McpClient::new("https://api.mainnet-beta.solana.com");
    let schema = client.list_tools(program_id)?;
    Ok(schema.tools)
}
```

## Understanding the Schema

### Parsing Tool Definitions

```typescript
interface Tool {
  n: string;           // name
  d: string;           // discriminator (hex)
  i?: string;          // description
  p?: Record<string, string | ParamDef>;  // parameters
  r?: string[];        // required order
}

interface ParamDef {
  type: string;
  signer?: boolean;
  writable?: boolean;
  description?: string;
}
```

### Extracting Account Requirements

```typescript
function parseAccounts(tool: Tool): Account[] {
  const accounts = [];

  for (const [key, value] of Object.entries(tool.p || {})) {
    if (typeof value === 'string' && value === 'pubkey') {
      // Simple format: name_suffix
      const isSigner = key.endsWith('_s') || key.endsWith('_sw');
      const isWritable = key.endsWith('_w') || key.endsWith('_sw');
      const name = key.replace(/_s$|_w$|_sw$/, '');

      accounts.push({ name, isSigner, isWritable });
    } else if (typeof value === 'object' && value.type === 'pubkey') {
      // Extended format
      accounts.push({
        name: key,
        isSigner: value.signer || false,
        isWritable: value.writable || false,
        description: value.description
      });
    }
  }

  return accounts;
}
```

## Building Transactions

### From Schema to Instruction

```typescript
function buildInstruction(
  programId: PublicKey,
  tool: Tool,
  accounts: Record<string, PublicKey>,
  args: Record<string, any>
): TransactionInstruction {
  // 1. Build discriminator
  const discriminator = Buffer.from(tool.d, 'hex');

  // 2. Build argument data
  const argBuffers = [];
  for (const paramName of tool.r || []) {
    if (paramName in args) {
      const value = args[paramName];
      const paramType = tool.p[paramName];
      argBuffers.push(serializeArg(value, paramType));
    }
  }

  // 3. Build account metas
  const keys = [];
  for (const paramName of tool.r || []) {
    if (paramName in accounts) {
      const { isSigner, isWritable } = parseAccountFlags(paramName);
      keys.push({
        pubkey: accounts[paramName],
        isSigner,
        isWritable
      });
    }
  }

  return new TransactionInstruction({
    keys,
    programId,
    data: Buffer.concat([discriminator, ...argBuffers])
  });
}
```

### Serializing Arguments

```typescript
function serializeArg(value: any, type: string): Buffer {
  switch (type) {
    case 'u8':
      return Buffer.from([value]);
    case 'u16':
      const b16 = Buffer.alloc(2);
      b16.writeUInt16LE(value);
      return b16;
    case 'u32':
      const b32 = Buffer.alloc(4);
      b32.writeUInt32LE(value);
      return b32;
    case 'u64':
    case 'int':
      const b64 = Buffer.alloc(8);
      b64.writeBigUInt64LE(BigInt(value));
      return b64;
    case 'bool':
      return Buffer.from([value ? 1 : 0]);
    case 'pubkey':
      return new PublicKey(value).toBuffer();
    default:
      throw new Error(`Unknown type: ${type}`);
  }
}
```

## Handling PDAs

Some programs document PDA seeds in descriptions:

```json
{
  "n": "initialize",
  "i": "Create vault. seeds=[\"vault\", owner, mint]",
  "p": {
    "vault_sw": "pubkey"
  }
}
```

### Parsing Seeds

```typescript
function parsePdaSeeds(description: string): string[] | null {
  const match = description.match(/seeds=\[(.*?)\]/);
  if (!match) return null;

  return match[1]
    .split(',')
    .map(s => s.trim().replace(/"/g, ''));
}

// Usage
const seeds = parsePdaSeeds(tool.i);
// ["vault", "owner", "mint"]
```

### Deriving PDAs

```typescript
async function derivePda(
  programId: PublicKey,
  seeds: string[],
  values: Record<string, PublicKey | string>
): Promise<[PublicKey, number]> {
  const seedBuffers = seeds.map(seed => {
    if (seed.startsWith('"')) {
      // Literal string
      return Buffer.from(seed.replace(/"/g, ''));
    } else if (seed in values) {
      // Variable reference
      const val = values[seed];
      return val instanceof PublicKey ? val.toBuffer() : Buffer.from(val);
    }
    throw new Error(`Unknown seed: ${seed}`);
  });

  return PublicKey.findProgramAddressSync(seedBuffers, programId);
}
```

## Agent Architecture

### Recommended Pattern

```typescript
class SolanaAgent {
  private connection: Connection;
  private schemaCache: Map<string, Tool[]> = new Map();

  async discover(programId: PublicKey): Promise<Tool[]> {
    const key = programId.toBase58();
    if (!this.schemaCache.has(key)) {
      const tools = await discoverProgram(this.connection, programId);
      this.schemaCache.set(key, tools);
    }
    return this.schemaCache.get(key)!;
  }

  async execute(
    programId: PublicKey,
    toolName: string,
    accounts: Record<string, PublicKey>,
    args: Record<string, any>,
    signer: Keypair
  ): Promise<string> {
    const tools = await this.discover(programId);
    const tool = tools.find(t => t.n === toolName);
    if (!tool) throw new Error(`Tool not found: ${toolName}`);

    const ix = buildInstruction(programId, tool, accounts, args);
    const tx = new Transaction().add(ix);

    return await sendAndConfirmTransaction(this.connection, tx, [signer]);
  }
}
```

### LLM Integration

When integrating with an LLM:

1. Discover program schema
2. Format tools as function definitions for the LLM
3. LLM selects tool and provides arguments
4. Agent builds and executes transaction
5. Return result to LLM

```typescript
function toolsToFunctions(tools: Tool[]): FunctionDef[] {
  return tools
    .filter(t => t.n !== 'list_tools')
    .map(tool => ({
      name: tool.n,
      description: tool.i || `Execute ${tool.n} instruction`,
      parameters: {
        type: 'object',
        properties: buildProperties(tool),
        required: tool.r || []
      }
    }));
}
```

## Error Handling

Common errors when interacting with MCP programs:

| Error | Cause | Solution |
|-------|-------|----------|
| `InvalidInstructionData` | Wrong discriminator or args | Verify schema, check serialization |
| `MissingRequiredSignature` | Account not signed | Check `_s` suffix accounts |
| `InvalidAccountData` | Wrong account or bad state | Verify account address and ownership |
| `AccountDataTooSmall` | Account not initialized | Initialize account first |

## Best Practices

1. **Cache schemas** - Discovery is expensive, cache results
2. **Validate before send** - Check account flags match schema
3. **Handle pagination** - Always iterate until no `nextCursor`
4. **Parse PDA seeds** - Look for `seeds=[...]` in descriptions
5. **Retry on simulation failure** - Network issues are common
