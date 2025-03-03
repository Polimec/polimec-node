import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
import { TRANSFER_AMOUNTS } from '@/constants';
import { createChainManager } from '@/managers/Factory';
import { polimec_storage } from '@/polimec';
import { ChainSetup } from '@/setup';
import { PolimecToHubTransfer } from '@/transfers/PolimecToHub';
import { Accounts, Asset, AssetSourceRelation, Chains } from '@/types';

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

  test(
    'Send USDC to Hub',
    () =>
      transferTest.testTransfer({
        account: Accounts.BOB,
        assets: [[Asset.USDC, TRANSFER_AMOUNTS.TOKENS, AssetSourceRelation.Sibling]],
      }),
    { timeout: 25000 },
  );

  test(
    'Send USDT to Hub',
    () =>
      transferTest.testTransfer({
        account: Accounts.BOB,
        assets: [[Asset.USDT, TRANSFER_AMOUNTS.TOKENS, AssetSourceRelation.Sibling]],
      }),
    { timeout: 25000 },
  );

  test(
    'Send DOT to Hub',
    () =>
      transferTest.testTransfer({
        account: Accounts.BOB,
        assets: [[Asset.DOT, TRANSFER_AMOUNTS.NATIVE, AssetSourceRelation.Parent]],
      }),
    { timeout: 25000 },
  );

  test(
    'Send PLMC to Hub',
    () =>
      transferTest.testTransfer({
        account: Accounts.BOB,
        assets: [
          [Asset.PLMC, TRANSFER_AMOUNTS.NATIVE, AssetSourceRelation.Self],
          [Asset.DOT, TRANSFER_AMOUNTS.NATIVE, AssetSourceRelation.Parent],
        ],
        fee_asset_item: 1,
      }),
    { timeout: 25000000 },
  );
});
