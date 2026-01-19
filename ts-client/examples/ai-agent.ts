/**
 * AI Agent Example - Discovering and calling MCP-enabled Solana programs
 *
 * This example shows how an AI agent can:
 * 1. Discover what a program can do (list_tools)
 * 2. Understand the parameters needed
 * 3. Build and send transactions
 *
 * Run: npx ts-node examples/ai-agent.ts
 */

import { McpClient, PublicKey, Keypair } from '../src';

// Counter program deployed on devnet
const COUNTER_PROGRAM = new PublicKey('7QniyJzHpS7uFdYogBE5oUPxj6TXyNKFgkR4Dztbnbct');

async function main() {
  console.log('🤖 AI Agent: Discovering MCP-enabled Solana program...\n');

  const client = new McpClient('https://api.devnet.solana.com');

  // Step 1: Discover available tools
  console.log('📋 Calling list_tools to discover capabilities...');
  const schema = await client.listTools(COUNTER_PROGRAM);

  console.log(`\nProgram: ${schema.name}`);
  console.log(`Protocol: ${schema.v}`);
  console.log(`\nAvailable tools:`);

  for (const tool of schema.tools) {
    console.log(`\n  📌 ${tool.name}`);
    if (tool.description) {
      console.log(`     ${tool.description}`);
    }

    // Show parameters
    const accounts = Object.entries(tool.params)
      .filter(([_, type]) => type === 'pubkey')
      .map(([name]) => name);

    const args = Object.entries(tool.params)
      .filter(([_, type]) => type !== 'pubkey')
      .map(([name, type]) => `${name}: ${type}`);

    if (accounts.length > 0) {
      console.log(`     Accounts: ${accounts.join(', ')}`);
    }
    if (args.length > 0) {
      console.log(`     Args: ${args.join(', ')}`);
    }

    // Check for PDA info in description
    if (tool.description) {
      const pdaSeeds = client.parsePdaSeeds(tool.description);
      if (pdaSeeds) {
        console.log(`     PDA seeds: [${pdaSeeds.seeds.map(s =>
          s.type === 'literal' ? `"${s.value}"` : s.value
        ).join(', ')}]`);
      }
    }
  }

  // Step 2: AI decides to call increment
  console.log('\n\n🤖 AI Agent: I can see this program has increment/decrement tools.');
  console.log('   Let me build an increment instruction...\n');

  const incrementTool = client.findTool(schema, 'increment');
  if (!incrementTool) {
    console.log('❌ increment tool not found');
    return;
  }

  // In a real scenario, AI would have the actual account addresses
  const dummyCounter = Keypair.generate().publicKey;
  const dummyAuthority = Keypair.generate().publicKey;

  console.log('📝 Building increment instruction:');
  console.log(`   counter: ${dummyCounter.toBase58().slice(0, 20)}...`);
  console.log(`   authority: ${dummyAuthority.toBase58().slice(0, 20)}...`);
  console.log(`   amount: 10`);

  const ix = client.buildInstruction(
    COUNTER_PROGRAM,
    incrementTool,
    {
      counter: dummyCounter,
      authority: dummyAuthority,
    },
    {
      amount: 10,
    }
  );

  console.log('\n✅ Instruction built successfully!');
  console.log(`   Program: ${ix.programId.toBase58()}`);
  console.log(`   Accounts: ${ix.keys.length}`);
  console.log(`   Data: ${ix.data.toString('hex')}`);

  // Step 3: Show how AI would handle PDA derivation
  console.log('\n\n🤖 AI Agent: For the vault program, I need to derive PDAs...\n');

  const vaultDescription = 'Create vault PDA. seeds=["vault",owner,mint]';
  const pdaSeeds = client.parsePdaSeeds(vaultDescription);

  if (pdaSeeds) {
    console.log('📝 Parsed PDA seeds from description:');
    console.log(`   Literals: ${pdaSeeds.literals.join(', ')}`);
    console.log(`   References: ${pdaSeeds.refs.join(', ')}`);

    // Derive the PDA
    const owner = Keypair.generate().publicKey;
    const mint = Keypair.generate().publicKey;
    const vaultProgram = Keypair.generate().publicKey;

    const [vaultPda, bump] = await client.derivePda(
      vaultProgram,
      pdaSeeds,
      { owner, mint }
    );

    console.log(`\n   Derived vault PDA: ${vaultPda.toBase58().slice(0, 30)}...`);
    console.log(`   Bump: ${bump}`);
  }

  console.log('\n\n✨ AI Agent demo complete!');
  console.log('   The agent discovered tools, understood parameters,');
  console.log('   and can now build transactions autonomously.');
}

main().catch(console.error);
