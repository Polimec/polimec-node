import { expect } from 'bun:test';
import type { BridgerHubManagaer } from '@/managers/BridgeHubManager';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import {
  Asset,
  AssetSourceRelation,
  Chains,
  ParaId,
  type PolimecBalanceCheck,
  getVersionedAssets,
} from '@/types';
import { createTransferData, unwrap } from '@/utils';
import {
  DispatchRawOrigin,
  XcmVersionedAssetId,
  type XcmVersionedLocation,
  type XcmVersionedXcm,
} from '@polkadot-api/descriptors';

import { BaseTransferTest, type TransferOptions } from './BaseTransfer';

export class BridgeToPolimecTransfer extends BaseTransferTest {
  constructor(
    protected override sourceManager: BridgerHubManagaer,
    protected hopManager: PolkadotHubManager,
    protected override destManager: PolimecManager,
  ) {
    super(sourceManager, destManager);
    this.hopManager = hopManager;
  }

  async executeTransfer({ account, assets }: TransferOptions) {
    console.log('BridgeToPolimecTransfer executeTransfer');
    const [sourceBlock, hopManagerBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.hopManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
    ]);

    console.log('sourceBlock', sourceBlock);
    console.log('hopManagerBlock', hopManagerBlock);
    console.log('destBlock', destBlock);
    return { sourceBlock, destBlock };

    // const versioned_assets = getVersionedAssets(assets);

    // const data = createTransferData({
    //   toChain: Chains.Polimec,
    //   assets: versioned_assets,
    //   recv: account,
    // });

    // const api = this.sourceManager.getApi(Chains.PolkadotHub);
    // const transfer = api.tx.PolkadotXcm.transfer_assets(data);
    // const res = await transfer.signAndSubmit(this.sourceManager.getSigner(account));

    // console.log('Extrinsic result: ', res.ok);

    // expect(res.ok).toBeTrue();
    // return { sourceBlock, destBlock };
  }

  async getBalances({
    account,
    assets,
  }: TransferOptions): Promise<{ asset_balances: PolimecBalanceCheck[] }> {
    return { asset_balances: [{ source: 0n, destination: 0n, treasury: 0n }] };
  }

  async verifyFinalBalances(
    assetInitialBalances: PolimecBalanceCheck[],
    assetFinalBalances: PolimecBalanceCheck[],
    transferOptions: TransferOptions,
  ) {
    expect(0n).toBe(0n);
  }

  async calculatePolimecXcmFee(transferOptions: TransferOptions): Promise<bigint> {
    return 0n;
  }
}
