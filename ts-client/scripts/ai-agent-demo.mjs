/**
 * AI Agent Demo: Discovering and using MCP-enabled Solana programs
 *
 * This demonstrates how an AI agent would:
 * 1. Discover available tools via list_tools
 * 2. Parse the schema to understand parameters
 * 3. Build and execute transactions
 */

import {
    Connection,
    PublicKey,
    TransactionInstruction,
    Transaction,
    Keypair,
    SystemProgram,
    sendAndConfirmTransaction
} from '@solana/web3.js';
import fs from 'fs';
import path from 'path';

const RPC_URL = 'https://api.devnet.solana.com';

function loadKeypair() {
    const keypairPath = path.join(process.env.HOME, '.config/solana/id.json');
    const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf8'));
    return Keypair.fromSecretKey(Uint8Array.from(keypairData));
}

class McpSolanaAgent {
    constructor(connection, payer) {
        this.connection = connection;
        this.payer = payer;
        this.programSchemas = new Map();
    }

    /**
     * Discover tools from an MCP-enabled program
     */
    async discoverProgram(programId) {
        console.log(`🔍 Discovering program: ${programId}`);

        // list_tools discriminator (universal)
        const listToolsDisc = Buffer.from('42195e6a55fd41c0', 'hex');

        const instruction = new TransactionInstruction({
            keys: [],
            programId: new PublicKey(programId),
            data: listToolsDisc,
        });

        const transaction = new Transaction().add(instruction);
        const { blockhash } = await this.connection.getLatestBlockhash();
        transaction.recentBlockhash = blockhash;
        transaction.feePayer = this.payer.publicKey;
        transaction.sign(this.payer);

        const result = await this.connection.simulateTransaction(transaction);

        if (result.value.err) {
            throw new Error(`Discovery failed: ${JSON.stringify(result.value.err)}`);
        }

        if (!result.value.returnData) {
            throw new Error('No return data - program may not be MCP-enabled');
        }

        const data = Buffer.from(result.value.returnData.data[0], 'base64');
        const schema = JSON.parse(data.toString('utf8'));

        this.programSchemas.set(programId, schema);
        return schema;
    }

    /**
     * List available tools for a program
     */
    listTools(programId) {
        const schema = this.programSchemas.get(programId);
        if (!schema) {
            throw new Error('Program not discovered. Call discoverProgram first.');
        }

        return schema.tools.map(t => ({
            name: t.n || t.name,
            discriminator: t.d,
            params: t.p || {},
            required: t.r || [],
        }));
    }

    /**
     * Call a tool on the program
     */
    async callTool(programId, toolName, accounts, args = {}) {
        const schema = this.programSchemas.get(programId);
        if (!schema) {
            throw new Error('Program not discovered');
        }

        const tool = schema.tools.find(t => (t.n || t.name) === toolName);
        if (!tool) {
            throw new Error(`Tool not found: ${toolName}`);
        }

        console.log(`⚡ Calling ${toolName}...`);

        // Build instruction data: discriminator + serialized args
        const discriminator = Buffer.from(tool.d, 'hex');
        let data = discriminator;

        // Serialize arguments (simple u64 for now)
        if (args.amount !== undefined) {
            const amountBuf = Buffer.alloc(8);
            amountBuf.writeBigUInt64LE(BigInt(args.amount));
            data = Buffer.concat([data, amountBuf]);
        }

        // Build account keys from accounts object
        const keys = accounts.map(acc => ({
            pubkey: new PublicKey(acc.pubkey),
            isSigner: acc.isSigner,
            isWritable: acc.isWritable,
        }));

        const instruction = new TransactionInstruction({
            keys,
            programId: new PublicKey(programId),
            data,
        });

        const transaction = new Transaction().add(instruction);

        // Add signers
        const signers = [this.payer];
        if (accounts.some(a => a.keypair)) {
            signers.push(...accounts.filter(a => a.keypair).map(a => a.keypair));
        }

        const sig = await sendAndConfirmTransaction(this.connection, transaction, signers);
        console.log(`   ✓ Success: ${sig}`);
        return sig;
    }
}

async function main() {
    console.log('=== AI Agent MCP Demo ===\n');

    const connection = new Connection(RPC_URL, 'confirmed');
    const payer = loadKeypair();
    const agent = new McpSolanaAgent(connection, payer);

    const PROGRAM_ID = '7QniyJzHpS7uFdYogBE5oUPxj6TXyNKFgkR4Dztbnbct';

    // Step 1: Discover the program
    console.log('Step 1: Discover available tools\n');
    const schema = await agent.discoverProgram(PROGRAM_ID);
    console.log(`   Program: ${schema.name}`);
    console.log(`   Protocol: ${schema.v}`);

    // Step 2: List tools
    console.log('\nStep 2: Available tools:\n');
    const tools = agent.listTools(PROGRAM_ID);
    tools.forEach(tool => {
        console.log(`   📦 ${tool.name}`);
        console.log(`      Discriminator: ${tool.discriminator}`);
        if (Object.keys(tool.params).length > 0) {
            console.log(`      Params: ${JSON.stringify(tool.params)}`);
        }
    });

    // Step 3: Create a new counter
    console.log('\nStep 3: Initialize a new counter\n');
    const counterKeypair = Keypair.generate();
    console.log(`   Counter address: ${counterKeypair.publicKey.toString()}`);

    // Create the account first
    const space = 56;
    const rentExemption = await connection.getMinimumBalanceForRentExemption(space);
    const createAccountIx = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: counterKeypair.publicKey,
        lamports: rentExemption,
        space,
        programId: new PublicKey(PROGRAM_ID),
    });

    const createTx = new Transaction().add(createAccountIx);
    await sendAndConfirmTransaction(connection, createTx, [payer, counterKeypair]);
    console.log('   Account created');

    // Initialize
    await agent.callTool(PROGRAM_ID, 'initialize', [
        { pubkey: counterKeypair.publicKey.toString(), isSigner: false, isWritable: true },
        { pubkey: payer.publicKey.toString(), isSigner: true, isWritable: false },
        { pubkey: SystemProgram.programId.toString(), isSigner: false, isWritable: false },
    ]);

    // Step 4: Increment
    console.log('\nStep 4: Increment by 100\n');
    await agent.callTool(PROGRAM_ID, 'increment', [
        { pubkey: counterKeypair.publicKey.toString(), isSigner: false, isWritable: true },
        { pubkey: payer.publicKey.toString(), isSigner: true, isWritable: false },
    ], { amount: 100 });

    // Step 5: Decrement
    console.log('\nStep 5: Decrement by 42\n');
    await agent.callTool(PROGRAM_ID, 'decrement', [
        { pubkey: counterKeypair.publicKey.toString(), isSigner: false, isWritable: true },
        { pubkey: payer.publicKey.toString(), isSigner: true, isWritable: false },
    ], { amount: 42 });

    // Step 6: Read final state
    console.log('\nStep 6: Read final counter state\n');
    const accountInfo = await connection.getAccountInfo(counterKeypair.publicKey);
    const count = accountInfo.data.readBigInt64LE(8);
    console.log(`   Final count: ${count} (expected: 58 = 0 + 100 - 42)`);

    console.log('\n=== Demo Complete! ===');
    console.log(`\nView on Solana Explorer:`);
    console.log(`https://explorer.solana.com/address/${counterKeypair.publicKey.toString()}?cluster=devnet`);
}

main().catch(console.error);
