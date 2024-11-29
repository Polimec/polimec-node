import { afterAll, afterEach, beforeAll, beforeEach, describe, expect, test } from 'bun:test';
import { ChainTestManager } from '@/chainManager';
import { INITIAL_BALANCES } from '@/constants';
import { ChainSetup } from '@/setup';
import { TransferTest } from '@/transfers';
import { Assets } from '@/types';

describe('Polkadot -> Polimec Transfer Tests', () => {
  const chainManager = new ChainTestManager();
  const chainSetup = new ChainSetup();
  const transferTest = new TransferTest(chainManager);

  beforeAll(async () => await chainSetup.initialize());

  beforeEach(() => chainManager.connect());

  afterAll(async () => await chainSetup.cleanup());

  // TODO: Add:
  // [] Polkadot: Send DOT to Polimec, via Polkadot Hub
  // Double check the current XCM Emulator tests
});
