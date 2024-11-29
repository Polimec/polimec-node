import { afterAll, afterEach, beforeAll, beforeEach, describe, test } from 'bun:test';
import { ChainTestManager } from '@/chainManager';
import { INITIAL_BALANCES } from '@/constants';
import { polimec_storage } from '@/polimec';
import { ChainSetup } from '@/setup';
import { type TransferDirection, TransferTest } from '@/transfers';
import { Accounts, Assets, Chains } from '@/types';

describe('Polimec -> Polkadot Hub Transfer Tests', () => {
  const chainManager = new ChainTestManager();
  const chainSetup = new ChainSetup();
  const transferTest = new TransferTest(chainManager);
  const direction: TransferDirection = {
    source: Chains.Polimec,
    destination: Chains.PolkadotHub,
  };

  beforeAll(async () => await chainSetup.initialize(polimec_storage));

  beforeEach(() => chainManager.connect());

  afterAll(async () => await chainSetup.cleanup());

  test('Send USDC to Polkadot Hub', () =>
    transferTest.testAssetTransfer(Assets.USDC, direction, Accounts.BOB, INITIAL_BALANCES.USDC));

  test('Send USDt to Polkadot Hub', () =>
    transferTest.testAssetTransfer(Assets.USDT, direction, Accounts.BOB, INITIAL_BALANCES.USDT));

  test('Send DOT to Polkadot Hub', () =>
    transferTest.testAssetTransfer(Assets.DOT, direction, Accounts.BOB, INITIAL_BALANCES.DOT));
});
