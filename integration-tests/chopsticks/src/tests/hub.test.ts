import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { ChainTestManager } from '@/chainManager';
import { INITIAL_BALANCES } from '@/constants';
import { ChainSetup } from '@/setup';
import { type TransferDirection, TransferTest } from '@/transfers';
import { Accounts, Assets, Chains } from '@/types';

describe('Polkadot Hub -> Polimec Asset Transfer Tests', () => {
  const chainManager = new ChainTestManager();
  const chainSetup = new ChainSetup();
  const transferTest = new TransferTest(chainManager);
  const direction: TransferDirection = {
    source: Chains.PolkadotHub,
    destination: Chains.Polimec,
  };

  beforeAll(async () => await chainSetup.initialize());

  beforeEach(() => chainManager.connect());

  afterAll(async () => await chainSetup.cleanup());

  test('Send DOT to Polimec', () => transferTest.testNativeTransfer(INITIAL_BALANCES.DOT));

  test('Send USDt to Polimec', () =>
    transferTest.testAssetTransfer(Assets.USDT, direction, Accounts.ALICE, INITIAL_BALANCES.USDT));

  test('Send USDC to Polimec', () =>
    transferTest.testAssetTransfer(Assets.USDC, direction, Accounts.ALICE, INITIAL_BALANCES.USDC));

  test('Send Unknown Asset to Polimec', () =>
    expect(() =>
      transferTest.testAssetTransfer(Assets.UNKNOWN, direction, Accounts.ALICE),
    ).toThrow());
});
