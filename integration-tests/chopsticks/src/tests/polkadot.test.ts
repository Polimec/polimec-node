import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
import { TRANSFER_AMOUNTS } from '@/constants';
import { createChainManager } from '@/managers/Factory';
import { ChainSetup } from '@/setup';
import { PolkadotToPolimecTransfer } from '@/transfers/PolkadotToPolimec';
import { Accounts, Assets, Chains } from '@/types';

describe('Polkadot -> Polimec Transfer Tests', () => {
  const chainSetup = new ChainSetup();

  const sourceManager = createChainManager(Chains.Polkadot);
  const destManager = createChainManager(Chains.Polimec);
  const transferTest = new PolkadotToPolimecTransfer(sourceManager, destManager);

  beforeAll(async () => await chainSetup.initialize());
  beforeEach(() => {
    sourceManager.connect();
    destManager.connect();
  });
  afterAll(async () => await chainSetup.cleanup());

  test('Send DOT to Polimec', () =>
    transferTest.testTransfer({
      amount: TRANSFER_AMOUNTS.NATIVE,
      account: Accounts.ALICE,
      asset: Assets.DOT,
    }));
});
