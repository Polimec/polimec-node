import { afterAll, afterEach, beforeAll, beforeEach, describe, test } from 'bun:test';
import { ChainTestManager } from './chainManager';
import { INITIAL_BALANCES } from './constants';
import { ChainSetup } from './setup';
import { TransferTest } from './transfers';
import { Assets } from './types';

describe('Asset Management Tests', () => {
  const chainManager = new ChainTestManager();
  const chainSetup = new ChainSetup();
  const transferTest = new TransferTest(chainManager);

  beforeAll(() => chainSetup.initialize());
  afterAll(() => chainSetup.cleanup());

  beforeEach(() => chainManager.connect());
  afterEach(() => chainManager.disconnect());

  test('Polkadot Hub: Send USDt to Polimec', () =>
    transferTest.testAssetTransfer(Assets.USDT, INITIAL_BALANCES.USDT));

  test('Polkadot Hub: Send USDC to Polimec', () =>
    transferTest.testAssetTransfer(Assets.USDC, INITIAL_BALANCES.USDC));

  test('Polkadot Hub: Send DOT to Polimec', () =>
    transferTest.testNativeTransfer(INITIAL_BALANCES.DOT));

  // TODO: Add:
  // [] Polimec: Send USDt to Polkadot Hub
  // [] Polimec: Send USDC to Polkadot Hub
  // [] Polimec: Send DOT to Polkadot Hub
  // [] Polkadot: Send DOT to Polimec, via Polkadot Hub
  // [] Polkadot Hub: Send Random Asset to Polimec
  // Double check the current XCM Emulator tests 
});
