import { expect } from 'bun:test';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import {
  Asset,
  AssetSourceRelation,
  type BalanceCheck,
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

export class PolimecToHubTransfer extends BaseTransferTest {
  constructor(
    protected override sourceManager: PolimecManager,
    protected override destManager: PolkadotHubManager,
  ) {
    super(sourceManager, destManager);
  }

  async executeTransfer({ account, assets, fee_asset_item }: TransferOptions) {
    const [sourceBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
    ]);

    const versioned_assets = getVersionedAssets(assets);
    const data = createTransferData({
      toChain: Chains.PolkadotHub,
      assets: versioned_assets,
      recv: account,
      fee_asset_item: fee_asset_item ?? 0,
    });

    const res = await this.sourceManager
      .getXcmPallet()
      .transfer_assets(data)
      .signAndSubmit(this.sourceManager.getSigner(account));

    expect(res.ok).toBeTrue();
    return { sourceBlock, destBlock };
  }

  async getBalances(options: TransferOptions): Promise<{ asset_balances: BalanceCheck[] }> {
    const source = await this.sourceManager.getAssetBalanceOf(
      options.account,
      options.assets[0][0],
    );
    const destination = await this.destManager.getAssetBalanceOf(
      options.account,
      options.assets[0][0],
    );
    return { asset_balances: [{ source, destination }] };
  }

  async verifyFinalBalances(
    assetInitialBalances: PolimecBalanceCheck[],
    assetFinalBalances: PolimecBalanceCheck[],
    transferOptions: TransferOptions,
  ) {
    const native_extrinsic_fee_amount = await this.sourceManager.getTransactionFee();
    const source_xcm_asset_fee_amount = await this.sourceManager.getXcmFee();
    const dest_xcm_asset_fee_amount = await this.calculatePolkadotHubXcmFee(transferOptions);

    const fee_asset = transferOptions.assets[0][0];

    for (let i = 0; i < transferOptions.assets.length; i++) {
      const initialBalances = assetInitialBalances[i];
      const finalBalances = assetFinalBalances[i];
      const send_amount = transferOptions.assets[i][1];
      const asset = transferOptions.assets[i][0];

      let expectedSourceBalanceSpent = send_amount;
      let expectedDestBalanceSpent = 0n;
      let expectedTreasuryBalanceGained = 0n;

      if (asset === Asset.PLMC) {
        expectedSourceBalanceSpent += native_extrinsic_fee_amount + source_xcm_asset_fee_amount;
      }
      if (asset === fee_asset) {
        expectedDestBalanceSpent += dest_xcm_asset_fee_amount;
        expectedTreasuryBalanceGained += dest_xcm_asset_fee_amount;
      }

      expect(finalBalances.source).toBe(initialBalances.source - expectedSourceBalanceSpent);
      expect(finalBalances.destination).toBe(
        initialBalances.destination + send_amount - expectedDestBalanceSpent,
      );
      expect(finalBalances.treasury).toBe(initialBalances.treasury + expectedTreasuryBalanceGained);
    }
  }

  async calculatePolkadotHubXcmFee(transferOptions: TransferOptions): Promise<bigint> {
    let destinationExecutionFee: bigint;

    const sourceApi = this.sourceManager.getApi(Chains.Polimec);
    const destApi = this.destManager.getApi(Chains.PolkadotHub);

    const versioned_assets = getVersionedAssets(transferOptions.assets);
    const transferData = createTransferData({
      toChain: Chains.Polimec,
      assets: versioned_assets,
      recv: transferOptions.account,
      fee_asset_item: transferOptions.fee_asset_item ?? 0,
    });

    let remoteFeeAssetId: XcmVersionedAssetId;
    const feeAsset = unwrap(transferOptions.assets.at(transferData.fee_asset_item));
    if (feeAsset[2] === AssetSourceRelation.Self) {
      feeAsset[2] = AssetSourceRelation.Sibling;
    }
    const versioned_asset = getVersionedAssets([feeAsset]);
    if (versioned_asset.type === 'V4') {
      remoteFeeAssetId = XcmVersionedAssetId.V4(unwrap(versioned_asset.value.at(0)).id);
    } else {
      throw new Error('Invalid versioned assets');
    }

    const localDryRunResult = await sourceApi.apis.DryRunApi.dry_run_call(
      { type: 'system', value: DispatchRawOrigin.Signed(transferOptions.account) },
      { type: 'PolkadotXcm', value: { type: 'transfer_assets', value: transferData } },
    );

    let forwardedXcms: [XcmVersionedLocation, XcmVersionedXcm[]][] = [];
    if (localDryRunResult.success && localDryRunResult.value.forwarded_xcms) {
      forwardedXcms = localDryRunResult.value.forwarded_xcms;
    } else {
      throw new Error('Dry run failed');
    }

    const xcmsToPHub = forwardedXcms.find(
      ([location, _]) =>
        location.type === 'V4' &&
        location.value.parents === 1 &&
        location.value.interior.type === 'X1' &&
        location.value.interior.value.type === 'Parachain' &&
        location.value.interior.value.value === ParaId[Chains.PolkadotHub],
    );

    if (!xcmsToPHub) {
      throw new Error('Could not find xcm to polimec');
    }
    const messages = xcmsToPHub[1];
    const remoteXcm = messages[0];
    const remoteXcmWeightResult = await destApi.apis.XcmPaymentApi.query_xcm_weight(remoteXcm);
    if (remoteXcmWeightResult.success) {
      const remoteExecutionFeesResult = await destApi.apis.XcmPaymentApi.query_weight_to_asset_fee(
        remoteXcmWeightResult.value,
        remoteFeeAssetId,
      );
      if (remoteExecutionFeesResult.success) {
        destinationExecutionFee = remoteExecutionFeesResult.value;
      } else {
        throw new Error('Could not calculate destination xcm fee');
      }
    } else {
      throw new Error('Could not calculate xcm weight');
    }

    return destinationExecutionFee;
  }
}
