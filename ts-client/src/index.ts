/**
 * @mcpsol/client - TypeScript client for MCP-enabled Solana programs
 *
 * Discover and interact with any MCP-enabled Solana program.
 *
 * @example
 * ```typescript
 * import { McpClient } from '@mcpsol/client';
 *
 * const client = new McpClient('https://api.devnet.solana.com');
 * const schema = await client.listTools(programId);
 *
 * console.log(`Program: ${schema.name}`);
 * for (const tool of schema.tools) {
 *   console.log(`  ${tool.name}: ${tool.description}`);
 * }
 * ```
 */

import {
  Connection,
  PublicKey,
  Transaction,
  TransactionInstruction,
  Keypair,
  AccountMeta,
} from '@solana/web3.js';

/** list_tools discriminator: sha256("global:list_tools")[0..8] */
export const LIST_TOOLS_DISCRIMINATOR = Buffer.from([
  0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0,
]);

/** Parsed MCP schema from on-chain program */
export interface McpSchema {
  /** Protocol version */
  v: string;
  /** Program name */
  name: string;
  /** Available tools */
  tools: McpTool[];
}

/** MCP tool definition */
export interface McpTool {
  /** Tool name (n in compact format) */
  name: string;
  /** Description (i in compact format) */
  description?: string;
  /** Discriminator hex (d in compact format) */
  discriminator: string;
  /** Parameters: name -> type */
  params: Record<string, string>;
  /** Required parameters in order */
  required: string[];
}

/** Parsed PDA seeds from description */
export interface PdaSeeds {
  /** Literal seed strings */
  literals: string[];
  /** Account/arg references */
  refs: string[];
  /** Raw seeds array */
  seeds: Array<{ type: 'literal' | 'ref'; value: string }>;
}

/**
 * MCP Client for discovering and calling Solana programs
 */
export class McpClient {
  private connection: Connection;

  constructor(rpcUrl: string | Connection) {
    this.connection =
      typeof rpcUrl === 'string' ? new Connection(rpcUrl, 'confirmed') : rpcUrl;
  }

  /**
   * Discover available tools by simulating list_tools instruction
   */
  async listTools(programId: PublicKey): Promise<McpSchema> {
    const instruction = new TransactionInstruction({
      programId,
      keys: [],
      data: LIST_TOOLS_DISCRIMINATOR,
    });

    // Create dummy transaction for simulation
    const payer = Keypair.generate();
    const { blockhash } = await this.connection.getLatestBlockhash();

    const tx = new Transaction();
    tx.recentBlockhash = blockhash;
    tx.feePayer = payer.publicKey;
    tx.add(instruction);

    // Simulate
    const result = await this.connection.simulateTransaction(tx);

    if (result.value.err) {
      throw new Error(`Simulation failed: ${JSON.stringify(result.value.err)}`);
    }

    if (!result.value.returnData) {
      throw new Error('No return data from program');
    }

    // Decode base64 return data
    const schemaBytes = Buffer.from(result.value.returnData.data[0], 'base64');
    const schemaJson = schemaBytes.toString('utf8');

    // Parse compact schema format
    const compact = JSON.parse(schemaJson);

    return this.parseCompactSchema(compact);
  }

  /**
   * Parse compact schema format to full McpSchema
   */
  private parseCompactSchema(compact: any): McpSchema {
    return {
      v: compact.v,
      name: compact.name,
      tools: compact.tools.map((t: any) => ({
        name: t.n,
        description: t.i,
        discriminator: t.d,
        params: t.p || {},
        required: t.r || [],
      })),
    };
  }

  /**
   * Build instruction from tool name and parameters
   */
  buildInstruction(
    programId: PublicKey,
    tool: McpTool,
    accounts: Record<string, PublicKey>,
    args: Record<string, any>
  ): TransactionInstruction {
    const keys: AccountMeta[] = [];
    const data: number[] = [];

    // Add discriminator
    const discBytes = Buffer.from(tool.discriminator, 'hex');
    data.push(...discBytes);

    // Process required params in order
    for (const param of tool.required) {
      const type = tool.params[param];

      if (type === 'pubkey') {
        // It's an account
        const baseName = this.getBaseName(param);
        const pubkey = accounts[baseName] || accounts[param];

        if (!pubkey) {
          throw new Error(`Missing account: ${param}`);
        }

        keys.push({
          pubkey,
          isSigner: this.isSigner(param),
          isWritable: this.isWritable(param),
        });
      } else {
        // It's an argument
        const value = args[param];
        if (value === undefined) {
          throw new Error(`Missing argument: ${param}`);
        }

        const encoded = this.encodeArg(type, value);
        data.push(...encoded);
      }
    }

    return new TransactionInstruction({
      programId,
      keys,
      data: Buffer.from(data),
    });
  }

  /**
   * Parse PDA seeds from tool description
   *
   * @example
   * "Create vault. seeds=[\"vault\",owner,mint]" -> { seeds: [...] }
   */
  parsePdaSeeds(description: string): PdaSeeds | null {
    const match = description.match(/seeds=\[(.*?)\]/);
    if (!match) return null;

    const seedsStr = match[1];
    const seeds: PdaSeeds['seeds'] = [];
    const literals: string[] = [];
    const refs: string[] = [];

    // Parse seeds: "literal" or reference
    const parts = seedsStr.split(',').map((s) => s.trim());

    for (const part of parts) {
      if (part.startsWith('"') && part.endsWith('"')) {
        const literal = part.slice(1, -1);
        seeds.push({ type: 'literal', value: literal });
        literals.push(literal);
      } else if (part.startsWith("'") && part.endsWith("'")) {
        const literal = part.slice(1, -1);
        seeds.push({ type: 'literal', value: literal });
        literals.push(literal);
      } else {
        seeds.push({ type: 'ref', value: part });
        refs.push(part);
      }
    }

    return { literals, refs, seeds };
  }

  /**
   * Derive PDA from parsed seeds
   */
  async derivePda(
    programId: PublicKey,
    pdaSeeds: PdaSeeds,
    values: Record<string, PublicKey | Buffer | string>
  ): Promise<[PublicKey, number]> {
    const seedBuffers: Buffer[] = [];

    for (const seed of pdaSeeds.seeds) {
      if (seed.type === 'literal') {
        seedBuffers.push(Buffer.from(seed.value));
      } else {
        const value = values[seed.value];
        if (!value) {
          throw new Error(`Missing seed value: ${seed.value}`);
        }

        if (value instanceof PublicKey) {
          seedBuffers.push(value.toBuffer());
        } else if (Buffer.isBuffer(value)) {
          seedBuffers.push(value);
        } else {
          seedBuffers.push(Buffer.from(value));
        }
      }
    }

    return PublicKey.findProgramAddressSync(seedBuffers, programId);
  }

  /**
   * Find a tool by name
   */
  findTool(schema: McpSchema, name: string): McpTool | undefined {
    return schema.tools.find((t) => t.name === name);
  }

  /**
   * Get base account name without suffix
   */
  private getBaseName(name: string): string {
    return name.replace(/_s$/, '').replace(/_w$/, '').replace(/_sw$/, '');
  }

  /**
   * Check if account is signer
   */
  private isSigner(name: string): boolean {
    return name.endsWith('_s') || name.endsWith('_sw');
  }

  /**
   * Check if account is writable
   */
  private isWritable(name: string): boolean {
    return name.endsWith('_w') || name.endsWith('_sw');
  }

  /**
   * Encode argument based on type
   */
  private encodeArg(type: string, value: any): number[] {
    const buf: number[] = [];

    switch (type) {
      case 'u8':
        buf.push(Number(value) & 0xff);
        break;
      case 'u16': {
        const v = Number(value);
        buf.push(v & 0xff, (v >> 8) & 0xff);
        break;
      }
      case 'u32': {
        const v = Number(value);
        buf.push(v & 0xff, (v >> 8) & 0xff, (v >> 16) & 0xff, (v >> 24) & 0xff);
        break;
      }
      case 'u64': {
        const v = BigInt(value);
        for (let i = 0; i < 8; i++) {
          buf.push(Number((v >> BigInt(i * 8)) & BigInt(0xff)));
        }
        break;
      }
      case 'i8':
        buf.push(Number(value) & 0xff);
        break;
      case 'i16': {
        const v = Number(value);
        buf.push(v & 0xff, (v >> 8) & 0xff);
        break;
      }
      case 'i32': {
        const v = Number(value);
        buf.push(v & 0xff, (v >> 8) & 0xff, (v >> 16) & 0xff, (v >> 24) & 0xff);
        break;
      }
      case 'i64': {
        const v = BigInt(value);
        for (let i = 0; i < 8; i++) {
          buf.push(Number((v >> BigInt(i * 8)) & BigInt(0xff)));
        }
        break;
      }
      case 'bool':
        buf.push(value ? 1 : 0);
        break;
      case 'pubkey': {
        const pk = new PublicKey(value);
        buf.push(...pk.toBuffer());
        break;
      }
      case 'str': {
        const strBytes = Buffer.from(String(value));
        // Length prefix (4 bytes)
        const len = strBytes.length;
        buf.push(len & 0xff, (len >> 8) & 0xff, (len >> 16) & 0xff, (len >> 24) & 0xff);
        buf.push(...strBytes);
        break;
      }
      case 'bytes': {
        const bytes = Buffer.isBuffer(value) ? value : Buffer.from(value, 'base64');
        const len = bytes.length;
        buf.push(len & 0xff, (len >> 8) & 0xff, (len >> 16) & 0xff, (len >> 24) & 0xff);
        buf.push(...bytes);
        break;
      }
      default:
        throw new Error(`Unknown type: ${type}`);
    }

    return buf;
  }

  /** Get the connection */
  get conn(): Connection {
    return this.connection;
  }
}

// Re-export for convenience
export { PublicKey, Connection, Keypair } from '@solana/web3.js';
