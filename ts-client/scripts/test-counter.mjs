// Comprehensive test for the counter program
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

const PROGRAM_ID = new PublicKey('7QniyJzHpS7uFdYogBE5oUPxj6TXyNKFgkR4Dztbnbct');
const RPC_URL = 'https://api.devnet.solana.com';

// Discriminators from the MCP schema
const DISCRIMINATORS = {
    list_tools: Buffer.from('42195e6a55fd41c0', 'hex'),
    initialize: Buffer.from('afaf6d1f0d989bed', 'hex'),
    increment: Buffer.from('0b12680968ae3b21', 'hex'),
    decrement: Buffer.from('6ae3a83bf81b9665', 'hex'),
};

// Load keypair from default Solana config
function loadKeypair() {
    const keypairPath = path.join(process.env.HOME, '.config/solana/id.json');
    const keypairData = JSON.parse(fs.readFileSync(keypairPath, 'utf8'));
    return Keypair.fromSecretKey(Uint8Array.from(keypairData));
}

// Encode u64 as little-endian bytes
function encodeU64(value) {
    const buffer = Buffer.alloc(8);
    buffer.writeBigUInt64LE(BigInt(value));
    return buffer;
}

async function main() {
    const connection = new Connection(RPC_URL, 'confirmed');
    const payer = loadKeypair();

    console.log('=== Counter Program Test ===');
    console.log('Program ID:', PROGRAM_ID.toString());
    console.log('Payer:', payer.publicKey.toString());
    console.log('');

    // 1. Test list_tools
    console.log('1. Testing list_tools...');
    await testListTools(connection, payer);

    // 2. Create a new counter account (just a regular account for now)
    console.log('\n2. Creating counter account...');
    const counterKeypair = Keypair.generate();
    console.log('Counter address:', counterKeypair.publicKey.toString());

    // 3. Test initialize
    console.log('\n3. Testing initialize...');
    await testInitialize(connection, payer, counterKeypair);

    // 4. Test increment
    console.log('\n4. Testing increment (amount=5)...');
    await testIncrement(connection, payer, counterKeypair.publicKey, 5);

    // 5. Test increment again
    console.log('\n5. Testing increment (amount=10)...');
    await testIncrement(connection, payer, counterKeypair.publicKey, 10);

    // 6. Test decrement
    console.log('\n6. Testing decrement (amount=3)...');
    await testDecrement(connection, payer, counterKeypair.publicKey, 3);

    console.log('\n=== All tests completed! ===');
}

async function testListTools(connection, payer) {
    const instruction = new TransactionInstruction({
        keys: [],
        programId: PROGRAM_ID,
        data: DISCRIMINATORS.list_tools,
    });

    const transaction = new Transaction().add(instruction);

    // Simulate to get return data
    const { blockhash } = await connection.getLatestBlockhash();
    transaction.recentBlockhash = blockhash;
    transaction.feePayer = payer.publicKey;
    transaction.sign(payer);

    const result = await connection.simulateTransaction(transaction);

    if (result.value.err) {
        console.log('  FAILED:', result.value.err);
        console.log('  Logs:', result.value.logs);
        return;
    }

    if (result.value.returnData) {
        const data = Buffer.from(result.value.returnData.data[0], 'base64');
        const schema = JSON.parse(data.toString('utf8'));
        console.log('  SUCCESS! Found', schema.tools.length, 'tools:');
        schema.tools.forEach(t => {
            console.log('    -', t.n || t.name);
        });
    }
}

async function testInitialize(connection, payer, counterKeypair) {
    // First, create the account with enough space
    // Counter struct: 8 (discriminator) + 8 (count) + 32 (authority) + 1 (bump) + 7 (padding) = 56 bytes
    const space = 56;
    const rentExemption = await connection.getMinimumBalanceForRentExemption(space);

    const createAccountIx = SystemProgram.createAccount({
        fromPubkey: payer.publicKey,
        newAccountPubkey: counterKeypair.publicKey,
        lamports: rentExemption,
        space: space,
        programId: PROGRAM_ID,
    });

    // Initialize instruction
    const initializeIx = new TransactionInstruction({
        keys: [
            { pubkey: counterKeypair.publicKey, isSigner: false, isWritable: true },
            { pubkey: payer.publicKey, isSigner: true, isWritable: false },
            { pubkey: SystemProgram.programId, isSigner: false, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: DISCRIMINATORS.initialize,
    });

    const transaction = new Transaction().add(createAccountIx, initializeIx);

    try {
        const sig = await sendAndConfirmTransaction(connection, transaction, [payer, counterKeypair]);
        console.log('  SUCCESS! Signature:', sig);
    } catch (err) {
        console.log('  Transaction result:', err.message);
        if (err.logs) {
            console.log('  Logs:', err.logs.slice(-5));
        }
    }
}

async function testIncrement(connection, payer, counterPubkey, amount) {
    const instruction = new TransactionInstruction({
        keys: [
            { pubkey: counterPubkey, isSigner: false, isWritable: true },
            { pubkey: payer.publicKey, isSigner: true, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: Buffer.concat([DISCRIMINATORS.increment, encodeU64(amount)]),
    });

    const transaction = new Transaction().add(instruction);

    try {
        const sig = await sendAndConfirmTransaction(connection, transaction, [payer]);
        console.log('  SUCCESS! Signature:', sig);
    } catch (err) {
        console.log('  Transaction result:', err.message);
        if (err.logs) {
            console.log('  Logs:', err.logs.slice(-5));
        }
    }
}

async function testDecrement(connection, payer, counterPubkey, amount) {
    const instruction = new TransactionInstruction({
        keys: [
            { pubkey: counterPubkey, isSigner: false, isWritable: true },
            { pubkey: payer.publicKey, isSigner: true, isWritable: false },
        ],
        programId: PROGRAM_ID,
        data: Buffer.concat([DISCRIMINATORS.decrement, encodeU64(amount)]),
    });

    const transaction = new Transaction().add(instruction);

    try {
        const sig = await sendAndConfirmTransaction(connection, transaction, [payer]);
        console.log('  SUCCESS! Signature:', sig);
    } catch (err) {
        console.log('  Transaction result:', err.message);
        if (err.logs) {
            console.log('  Logs:', err.logs.slice(-5));
        }
    }
}

main().catch(console.error);
