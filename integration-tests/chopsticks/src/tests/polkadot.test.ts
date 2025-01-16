import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
import { TRANSFER_AMOUNTS } from '@/constants';
import { createChainManager } from '@/managers/Factory';
import { ChainSetup } from '@/setup';
import { PolkadotToPolimecTransfer } from '@/transfers/PolkadotToPolimec';
import { Accounts, Asset, AssetSourceRelation, Chains } from '@/types';

describe('Polkadot -> Polimec Transfer Tests', () => {
  const chainSetup = new ChainSetup();

  const sourceManager = createChainManager(Chains.Polkadot);
  const destManager = createChainManager(Chains.Polimec);
  const hopManager = createChainManager(Chains.PolkadotHub);
  const transferTest = new PolkadotToPolimecTransfer(sourceManager, destManager, hopManager);

  beforeAll(async () => await chainSetup.initialize());
  beforeEach(() => {
    sourceManager.connect();
    hopManager.connect();
    destManager.connect();
  });
  afterAll(async () => await chainSetup.cleanup());

  test(
    'Send DOT to Polimec, via AH',
    () =>
      transferTest.testTransfer({
        account: Accounts.ALICE,
        assets: [[Asset.DOT, TRANSFER_AMOUNTS.NATIVE, AssetSourceRelation.Self]],
      }),
    { timeout: 25000 },
  );
});
