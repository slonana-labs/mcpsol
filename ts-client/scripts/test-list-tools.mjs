// Test script to call list_tools on the deployed counter program
import { Connection, PublicKey, TransactionInstruction, VersionedTransaction, TransactionMessage } from '@solana/web3.js';

const PROGRAM_ID = new PublicKey('7QniyJzHpS7uFdYogBE5oUPxj6TXyNKFgkR4Dztbnbct');
const RPC_URL = 'https://api.devnet.solana.com';

// Use actual wallet that exists on devnet
const FEE_PAYER = new PublicKey('55KLP138Pp4MAaxFnoUR585kVRzYNrpMCSs9Hseh91Pk');

// list_tools discriminator: sha256("global:list_tools")[0..8]
// 42195e6a55fd41c0 in hex
const LIST_TOOLS_DISCRIMINATOR = Buffer.from([0x42, 0x19, 0x5e, 0x6a, 0x55, 0xfd, 0x41, 0xc0]);

async function main() {
    const connection = new Connection(RPC_URL, 'confirmed');

    // Create the list_tools instruction
    const instruction = new TransactionInstruction({
        keys: [], // list_tools needs no accounts
        programId: PROGRAM_ID,
        data: LIST_TOOLS_DISCRIMINATOR,
    });

    console.log('Simulating list_tools instruction...');
    console.log('Program ID:', PROGRAM_ID.toString());
    console.log('Discriminator:', LIST_TOOLS_DISCRIMINATOR.toString('hex'));
    console.log('');

    // Get recent blockhash
    const { blockhash } = await connection.getLatestBlockhash();

    // Create versioned transaction for simulation
    const messageV0 = new TransactionMessage({
        payerKey: FEE_PAYER,
        recentBlockhash: blockhash,
        instructions: [instruction],
    }).compileToV0Message();

    const transaction = new VersionedTransaction(messageV0);

    try {
        // Simulate with sigVerify disabled
        const result = await connection.simulateTransaction(transaction, {
            sigVerify: false,
            replaceRecentBlockhash: true,
        });

        if (result.value.err) {
            console.error('Simulation failed:', JSON.stringify(result.value.err));
            console.log('Logs:', result.value.logs);
            return;
        }

        console.log('Simulation succeeded!');
        console.log('Logs:', result.value.logs);
        console.log('Units consumed:', result.value.unitsConsumed);

        // Check for return data
        if (result.value.returnData) {
            const returnData = result.value.returnData;
            console.log('\nReturn data program:', returnData.programId);

            // Decode base64 return data
            const data = Buffer.from(returnData.data[0], 'base64');
            const schemaJson = data.toString('utf8');

            console.log('\n=== MCP SCHEMA (raw) ===');
            console.log(schemaJson);

            // Pretty print if valid JSON
            try {
                const schema = JSON.parse(schemaJson);
                console.log('\n=== MCP SCHEMA (formatted) ===');
                console.log(JSON.stringify(schema, null, 2));
            } catch (e) {
                console.log('(Could not parse as JSON:', e.message, ')');
            }
        } else {
            console.log('\nNo return data in response');
        }
    } catch (err) {
        console.error('Error:', err.message);
        if (err.logs) console.log('Logs:', err.logs);
    }
}

main();
