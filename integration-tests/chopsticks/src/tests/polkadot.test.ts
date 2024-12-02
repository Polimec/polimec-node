import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
import { ChainTestManager } from '@/chainManager';
import { INITIAL_BALANCES } from '@/constants';
import { ChainSetup } from '@/setup';
import { type TransferDirection, TransferTest } from '@/transfers';
import { Accounts, Chains } from '@/types';

describe('Polkadot -> Polimec Transfer Tests', () => {
  const chainManager = new ChainTestManager();
  const chainSetup = new ChainSetup();
  const transferTest = new TransferTest(chainManager);

  const direction: TransferDirection = {
    source: Chains.Polkadot,
    destination: Chains.Polimec,
  };

  beforeAll(async () => await chainSetup.initialize());

  beforeEach(() => chainManager.connect());

  afterAll(async () => await chainSetup.cleanup());

  test('Send DOT to Polimec', () =>
    transferTest.testNativeTransfer(INITIAL_BALANCES.DOT, Accounts.ALICE, direction));
});
