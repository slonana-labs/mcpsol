import { describe, it, expect } from 'vitest';
import { McpClient, PublicKey, Keypair, LIST_TOOLS_DISCRIMINATOR } from './index';

describe('McpClient', () => {
  const client = new McpClient('https://api.devnet.solana.com');

  describe('parsePdaSeeds', () => {
    it('parses simple seeds', () => {
      const desc = 'Create vault. seeds=["vault",owner,mint]';
      const seeds = client.parsePdaSeeds(desc);

      expect(seeds).not.toBeNull();
      expect(seeds!.literals).toEqual(['vault']);
      expect(seeds!.refs).toEqual(['owner', 'mint']);
      expect(seeds!.seeds).toEqual([
        { type: 'literal', value: 'vault' },
        { type: 'ref', value: 'owner' },
        { type: 'ref', value: 'mint' },
      ]);
    });

    it('parses multiple literals', () => {
      const desc = 'PDA seeds=["prefix","suffix",account]';
      const seeds = client.parsePdaSeeds(desc);

      expect(seeds!.literals).toEqual(['prefix', 'suffix']);
      expect(seeds!.refs).toEqual(['account']);
    });

    it('returns null for no seeds', () => {
      const desc = 'Just a description without PDA';
      const seeds = client.parsePdaSeeds(desc);
      expect(seeds).toBeNull();
    });

    it('handles single quotes', () => {
      const desc = "seeds=['vault',owner]";
      const seeds = client.parsePdaSeeds(desc);

      expect(seeds!.literals).toEqual(['vault']);
      expect(seeds!.refs).toEqual(['owner']);
    });
  });

  describe('buildInstruction', () => {
    it('builds instruction with accounts and args', () => {
      const tool = {
        name: 'increment',
        discriminator: '0b12680968ae3b21',
        params: {
          counter_w: 'pubkey',
          authority_s: 'pubkey',
          amount: 'u64',
        },
        required: ['counter_w', 'authority_s', 'amount'],
      };

      const counter = Keypair.generate().publicKey;
      const authority = Keypair.generate().publicKey;
      const programId = Keypair.generate().publicKey;

      const ix = client.buildInstruction(
        programId,
        tool,
        { counter, authority },
        { amount: 100n }
      );

      expect(ix.programId.equals(programId)).toBe(true);
      expect(ix.keys.length).toBe(2);
      expect(ix.keys[0].pubkey.equals(counter)).toBe(true);
      expect(ix.keys[0].isWritable).toBe(true);
      expect(ix.keys[0].isSigner).toBe(false);
      expect(ix.keys[1].pubkey.equals(authority)).toBe(true);
      expect(ix.keys[1].isWritable).toBe(false);
      expect(ix.keys[1].isSigner).toBe(true);

      // Check discriminator
      expect(ix.data.slice(0, 8).toString('hex')).toBe('0b12680968ae3b21');

      // Check amount (100 as u64 little-endian)
      expect(ix.data.slice(8, 16)).toEqual(
        Buffer.from([100, 0, 0, 0, 0, 0, 0, 0])
      );
    });
  });

  describe('findTool', () => {
    it('finds tool by name', () => {
      const schema = {
        v: '2024-11-05',
        name: 'test',
        tools: [
          { name: 'foo', discriminator: '00', params: {}, required: [] },
          { name: 'bar', discriminator: '01', params: {}, required: [] },
        ],
      };

      const tool = client.findTool(schema, 'bar');
      expect(tool?.name).toBe('bar');
    });

    it('returns undefined for missing tool', () => {
      const schema = {
        v: '2024-11-05',
        name: 'test',
        tools: [],
      };

      const tool = client.findTool(schema, 'missing');
      expect(tool).toBeUndefined();
    });
  });

  describe('LIST_TOOLS_DISCRIMINATOR', () => {
    it('has correct value', () => {
      expect(LIST_TOOLS_DISCRIMINATOR.toString('hex')).toBe('42195e6a55fd41c0');
    });
  });
});
