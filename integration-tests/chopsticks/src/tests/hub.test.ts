import { afterAll, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { TRANSFER_AMOUNTS } from '@/constants';
import { createChainManager } from '@/managers/Factory';
import { ChainSetup } from '@/setup';
import { HubToPolimecTransfer } from '@/transfers/HubToPolimec';
import { Accounts, Assets, Chains } from '@/types';

describe('Polkadot Hub -> Polimec Transfer Tests', () => {
  const sourceManager = createChainManager(Chains.PolkadotHub);
  const destManager = createChainManager(Chains.Polimec);
  const transferTest = new HubToPolimecTransfer(sourceManager, destManager);
  const chainSetup = new ChainSetup();

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

  test('Send USDt to Polimec', () =>
    transferTest.testTransfer({
      amount: TRANSFER_AMOUNTS.TOKENS,
      account: Accounts.ALICE,
      asset: Assets.USDT,
    }));

  test('Send USDC to Polimec', () =>
    transferTest.testTransfer({
      amount: TRANSFER_AMOUNTS.TOKENS,
      account: Accounts.ALICE,
      asset: Assets.USDC,
    }));

  test('Send Unknown Asset to Polimec', () =>
    expect(() =>
      transferTest.testTransfer({
        amount: TRANSFER_AMOUNTS.TOKENS,
        account: Accounts.ALICE,
        asset: Assets.UNKNOWN,
      }),
    ).toThrow());
});
