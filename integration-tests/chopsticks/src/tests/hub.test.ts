import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
import { TRANSFER_AMOUNTS } from '@/constants';
import { createChainManager } from '@/managers/Factory';
import { ChainSetup } from '@/setup';
import { HubToPolimecTransfer } from '@/transfers/HubToPolimec';
import { Accounts, Asset, AssetSourceRelation, Chains } from '@/types';

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

  test(
    'Send DOT to Polimec',
    () =>
      transferTest.testTransfer({
        account: Accounts.ALICE,
        assets: [[Asset.DOT, TRANSFER_AMOUNTS.NATIVE, AssetSourceRelation.Parent]],
      }),
    { timeout: 25000 },
  );

  test(
    'Send USDT to Polimec',
    () =>
      transferTest.testTransfer({
        account: Accounts.ALICE,
        assets: [[Asset.USDT, TRANSFER_AMOUNTS.TOKENS, AssetSourceRelation.Self]],
      }),
    { timeout: 25000 },
  );

  test(
    'Send USDC to Polimec',
    () =>
      transferTest.testTransfer({
        account: Accounts.ALICE,
        assets: [[Asset.USDC, TRANSFER_AMOUNTS.TOKENS, AssetSourceRelation.Self]],
      }),
    { timeout: 25000 },
  );

  // test(
  //   'Send WETH to Polimec',
  //   () =>
  //     transferTest.testTransfer({
  //       account: Accounts.ALICE,
  //       assets: [
  //         // [Asset.USDT, TRANSFER_AMOUNTS.TOKENS, AssetSourceRelation.Self],
  //         [Asset.WETH, TRANSFER_AMOUNTS.BRIDGED, AssetSourceRelation.Self],
  //       ],
  //     }),
  //   { timeout: 25000 },
  // );
});
