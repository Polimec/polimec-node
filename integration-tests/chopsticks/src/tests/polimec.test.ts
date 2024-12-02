import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
import { TRANSFER_AMOUNTS } from '@/constants';
import { createChainManager } from '@/managers/Factory';
import { polimec_storage } from '@/polimec';
import { ChainSetup } from '@/setup';
import { PolimecToHubTransfer } from '@/transfers/PolimecToHub';
import { Accounts, Assets, Chains } from '@/types';

describe('Polimec -> Hub Transfer Tests', () => {
  const sourceManager = createChainManager(Chains.Polimec);
  const destManager = createChainManager(Chains.PolkadotHub);
  const transferTest = new PolimecToHubTransfer(sourceManager, destManager);
  const chainSetup = new ChainSetup();

  beforeAll(async () => await chainSetup.initialize(polimec_storage));
  beforeEach(() => {
    sourceManager.connect();
    destManager.connect();
  });
  afterAll(async () => await chainSetup.cleanup());

  test('Send USDC to Hub', () =>
    transferTest.testTransfer({
      amount: TRANSFER_AMOUNTS.TOKENS,
      account: Accounts.BOB,
      asset: Assets.USDC,
    }));

  test('Send USDt to Hub', () =>
    transferTest.testTransfer({
      amount: TRANSFER_AMOUNTS.TOKENS,
      account: Accounts.BOB,
      asset: Assets.USDT,
    }));

  test('Send DOT to Hub', () =>
    transferTest.testTransfer({
      amount: TRANSFER_AMOUNTS.NATIVE,
      account: Accounts.BOB,
      asset: Assets.DOT,
    }));
});
