// Read counter account state
import { Connection, PublicKey } from '@solana/web3.js';

const PROGRAM_ID = new PublicKey('7QniyJzHpS7uFdYogBE5oUPxj6TXyNKFgkR4Dztbnbct');
const RPC_URL = 'https://api.devnet.solana.com';

// Counter address from the test
const COUNTER_ADDRESS = process.argv[2];

if (!COUNTER_ADDRESS) {
    console.log('Usage: node read-counter.mjs <counter-address>');
    console.log('Example: node read-counter.mjs DVg3PeYMVKMpNcYZ3BG7aTMhFUqZVLeEptGqZaA1mB1J');
    process.exit(1);
}

async function main() {
    const connection = new Connection(RPC_URL, 'confirmed');
    const counterPubkey = new PublicKey(COUNTER_ADDRESS);

    console.log('Reading counter account:', counterPubkey.toString());
    console.log('');

    const accountInfo = await connection.getAccountInfo(counterPubkey);

    if (!accountInfo) {
        console.log('Account not found!');
        return;
    }

    console.log('Owner:', accountInfo.owner.toString());
    console.log('Lamports:', accountInfo.lamports);
    console.log('Data length:', accountInfo.data.length, 'bytes');
    console.log('');

    // Parse counter data
    // Layout: 8 bytes discriminator + 8 bytes count (i64) + 32 bytes authority + 1 byte bump + 7 padding
    const data = accountInfo.data;

    const discriminator = data.slice(0, 8).toString('hex');
    const count = data.readBigInt64LE(8);
    const authority = new PublicKey(data.slice(16, 48));
    const bump = data[48];

    console.log('=== Counter State ===');
    console.log('Discriminator:', discriminator);
    console.log('Count:', count.toString());
    console.log('Authority:', authority.toString());
    console.log('Bump:', bump);

    // Verify discriminator matches "account:Counter"
    // sha256("account:Counter")[0..8]
    console.log('\nExpected discriminator for Counter account: check macros/src/discriminator.rs');
}

main().catch(console.error);
