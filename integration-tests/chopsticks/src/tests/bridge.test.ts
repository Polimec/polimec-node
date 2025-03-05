import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
import { TRANSFER_AMOUNTS } from '@/constants';
import { createChainManager } from '@/managers/Factory';
import { polimec_storage } from '@/polimec';
import { ChainSetup } from '@/setup';
import { BridgeToPolimecTransfer } from '@/transfers/BridgeToPolimec';
import { Accounts, Asset, AssetSourceRelation, Chains } from '@/types';

describe('Bridge Hub -> Polimec Transfer Tests', () => {
  const sourceManager = createChainManager(Chains.BridgeHub);
  const hopManager = createChainManager(Chains.PolkadotHub);
  const destManager = createChainManager(Chains.Polimec);
  const transferTest = new BridgeToPolimecTransfer(sourceManager, hopManager, destManager);
  const chainSetup = new ChainSetup();

  beforeAll(async () => await chainSetup.initialize(polimec_storage));
  beforeEach(() => {
    sourceManager.connect();
    hopManager.connect();
    destManager.connect();
  });
  afterAll(async () => await chainSetup.cleanup());

  test(
    'Send WETH to Polimec',
    () =>
      transferTest.testTransfer({
        account: Accounts.ALICE,
        assets: [[Asset.WETH, TRANSFER_AMOUNTS.BRIDGED, AssetSourceRelation.Self]],
      }),
    { timeout: 25000 },
  );
});
