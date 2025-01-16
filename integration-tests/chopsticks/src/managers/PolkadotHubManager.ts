import { type Accounts, Asset, AssetLocation, AssetSourceRelation, Chains } from '@/types';
import { flatObject } from '@/utils.ts';
import { pah } from '@polkadot-api/descriptors';
import { createClient } from 'polkadot-api';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { BaseChainManager } from './BaseManager';

export class PolkadotHubManager extends BaseChainManager {
  connect() {
    const client = createClient(getWsProvider(this.getChainType()));
    const api = client.getTypedApi(pah);

    // Verify connection
    if (!client || !api) {
      throw new Error(`Failed to connect to ${this.getChainType()}`);
    }

    this.clients.set(this.getChainType(), { client, api });
  }

  disconnect() {
    this.clients.get(Chains.PolkadotHub)?.client.destroy();
  }

  getChainType() {
    return Chains.PolkadotHub;
  }

  getXcmPallet() {
    const api = this.getApi(Chains.PolkadotHub);
    return api.tx.PolkadotXcm;
  }

  getAssetSourceRelation(asset: Asset): AssetSourceRelation {
    switch (asset) {
      case Asset.DOT:
        return AssetSourceRelation.Parent;
      case Asset.USDT:
        return AssetSourceRelation.Self;
      case Asset.USDC:
        return AssetSourceRelation.Self;
      case Asset.WETH:
        // This is not actually used, so we use Self as a placeholder
        return AssetSourceRelation.Self;
    }
  }

  async getAssetBalanceOf(account: Accounts, asset: Asset): Promise<bigint> {
    const api = this.getApi(Chains.PolkadotHub);
    const asset_source_relation = this.getAssetSourceRelation(asset);
    const asset_location = AssetLocation(asset, asset_source_relation).value;
    const account_balances_result = await api.apis.FungiblesApi.query_account_balances(account);
    console.log('Requested asset location in PolkadotHubManager');
    console.dir(asset_location, { depth: null, colors: true });

    console.log('\n\n');
    if (account_balances_result.success === true && account_balances_result.value.type === 'V4') {
      const assets = account_balances_result.value.value;
      for (const asset of assets) {
        if (Bun.deepEquals(flatObject(asset.id), flatObject(asset_location))) {
          return asset.fun.value as bigint;
        }
      }
    }
    console.log('Asset not found using query_account_balances Runtime API');
    return 0n;
  }

  async getSwapCredit() {
    const api = this.getApi(Chains.PolkadotHub);
    const events = await api.event.AssetConversion.SwapCreditExecuted.pull();
    return events[0]?.payload.amount_in || 0n;
  }

  async getXcmFee() {
    const api = this.getApi(Chains.PolkadotHub);
    const events = await api.event.PolkadotXcm.FeesPaid.pull();
    console.log('Fees paid event');
    console.dir(events, { depth: null, colors: true });
    return (events[0]?.payload.fees[0].fun.value as bigint) || 0n;
  }

  async getTransactionFee() {
    const api = this.getApi(Chains.PolkadotHub);
    const events = await api.event.TransactionPayment.TransactionFeePaid.pull();
    return (events[0]?.payload.actual_fee as bigint) || 0n;
  }
}
