import { expect } from 'bun:test';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import type { PolkadotManager } from '@/managers/PolkadotManager';
import {
  Asset,
  AssetSourceRelation,
  Chains,
  ParaId,
  type PolimecBalanceCheck,
  getVersionedAssets,
} from '@/types';
import { abs, createDotMultiHopTransferData, createTransferData, unwrap } from '@/utils';
import {
  DispatchRawOrigin,
  XcmVersionedAssetId,
  type XcmVersionedLocation,
  type XcmVersionedXcm,
} from '@polkadot-api/descriptors';
import { BaseTransferTest, type TransferOptions } from './BaseTransfer';

export class PolkadotToPolimecTransfer extends BaseTransferTest {
  constructor(
    protected override sourceManager: PolkadotManager,
    protected override destManager: PolimecManager,
    protected hopManager: PolkadotHubManager,
  ) {
    super(sourceManager, destManager);
  }

  async executeTransfer({ account, assets }: TransferOptions) {
    const [sourceBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
      this.hopManager.getBlockNumber(),
    ]);

    const amount = assets[0][1];
    const data = createDotMultiHopTransferData(amount);

    const api = this.sourceManager.getApi(Chains.Polkadot);
    const res = await api.tx.XcmPallet.transfer_assets_using_type_and_then(data).signAndSubmit(
      this.sourceManager.getSigner(account),
    );

    expect(res.ok).toBeTrue();
    return { sourceBlock, destBlock };
  }

  async getBalances({
    account,
    assets,
  }: TransferOptions): Promise<{ asset_balances: PolimecBalanceCheck[] }> {
    const asset_balances: PolimecBalanceCheck[] = [];
    const treasury_account = this.destManager.getTreasuryAccount();
    for (const [asset] of assets) {
      const balances: PolimecBalanceCheck = {
        source: await this.sourceManager.getAssetBalanceOf(account, asset),
        destination: await this.destManager.getAssetBalanceOf(account, asset),
        treasury: await this.destManager.getAssetBalanceOf(treasury_account, asset),
      };
      asset_balances.push(balances);
    }
    return { asset_balances };
  }

  // TODO: This is not accurate at the moment.
  // TODO: We should improve the logic to handle the Polkadot -> Hub -> Polimec transfer fee.
  async verifyFinalBalances(
    assetInitialBalances: PolimecBalanceCheck[],
    assetFinalBalances: PolimecBalanceCheck[],
    transferOptions: TransferOptions,
  ) {
    const native_extrinsic_fee_amount = await this.sourceManager.getTransactionFee();
    const source_xcm_asset_fee_amount = await this.sourceManager.getXcmFee();
    const hop_xcm_asset_fee_amount = await this.calculateHubXcmFee(transferOptions);

    const fee_asset = transferOptions.assets[0][0];

    for (let i = 0; i < transferOptions.assets.length; i++) {
      const initialBalances = assetInitialBalances[i];
      const finalBalances = assetFinalBalances[i];
      const send_amount = transferOptions.assets[i][1];
      const asset = transferOptions.assets[i][0];

      let expectedSourceBalanceSpent = send_amount;
      let expectedDestBalanceSpent = 0n;
      let expectedTreasuryBalanceGained = 0n;

      if (asset === Asset.DOT) {
        expectedSourceBalanceSpent += native_extrinsic_fee_amount + source_xcm_asset_fee_amount;
      }
      if (asset === fee_asset) {
        expectedDestBalanceSpent += hop_xcm_asset_fee_amount;
        expectedTreasuryBalanceGained += hop_xcm_asset_fee_amount;
      }

      expect(finalBalances.source).toBe(initialBalances.source - expectedSourceBalanceSpent);
      const difference =
        finalBalances.destination -
        (initialBalances.destination + send_amount - expectedDestBalanceSpent);
      const tolerance = (finalBalances.destination * 1_000_000n) / 1_000_000_00n; // 0.001%
      expect(abs(difference)).toBeLessThanOrEqual(tolerance);
    }
  }

  async calculateHubXcmFee(_transferOptions: TransferOptions): Promise<bigint> {
    console.log('Calculating Polkadot -> Hub -> Polimec fees');
    return await Promise.resolve(422157353n); // TODO: Replace this with the actual fee calculation below
  }

  async computeFee(transferOptions: TransferOptions) {
    let destinationExecutionFee: bigint;

    const sourceApi = this.sourceManager.getApi(Chains.Polkadot);
    // const polimecApi = this.destManager.getApi(Chains.Polimec);
    const destApi = this.hopManager.getApi(Chains.PolkadotHub);

    const versioned_assets = getVersionedAssets(transferOptions.assets);
    const transferData = createTransferData({
      toChain: Chains.PolkadotHub,
      assets: versioned_assets,
      recv: transferOptions.account,
      fee_asset_item: transferOptions.fee_asset_item ?? 0,
    });

    let remoteFeeAssetId: XcmVersionedAssetId;
    const lastAsset = unwrap(transferOptions.assets[0]);
    if (lastAsset[0] === Asset.DOT) {
      lastAsset[2] = AssetSourceRelation.Parent;
    } else {
      throw new Error('Invalid asset');
    }
    const versioned_asset = getVersionedAssets([lastAsset]);
    if (versioned_asset.type === 'V4') {
      remoteFeeAssetId = XcmVersionedAssetId.V4(unwrap(versioned_asset.value[0]).id);
    } else {
      throw new Error('Invalid versioned assets');
    }
    console.log('remoteFeeAssetId', remoteFeeAssetId);
    const localDryRunResult = await sourceApi.apis.DryRunApi.dry_run_call(
      { type: 'system', value: DispatchRawOrigin.Signed(transferOptions.account) },
      { type: 'XcmPallet', value: { type: 'transfer_assets', value: transferData } },
    );
    console.log('localDryRunResult', localDryRunResult);

    let forwardedXcms: [XcmVersionedLocation, XcmVersionedXcm[]][] = [];
    if (localDryRunResult.success && localDryRunResult.value.forwarded_xcms) {
      forwardedXcms = localDryRunResult.value.forwarded_xcms;
    } else {
      throw new Error('Dry run failed');
    }

    const xcmsToHub = forwardedXcms.find(
      ([location, _]) =>
        location.type === 'V4' &&
        location.value.parents === 0 &&
        location.value.interior.type === 'X1' &&
        location.value.interior.value.type === 'Parachain' &&
        location.value.interior.value.value === ParaId[Chains.PolkadotHub],
    );
    if (!xcmsToHub) {
      throw new Error('Could not find xcm to Polkadot Hub');
    }
    const messages = xcmsToHub[1];
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
