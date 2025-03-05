import { type Accounts, Asset, AssetLocation, AssetSourceRelation, Chains } from '@/types';
import { flatObject } from '@/utils.ts';
import { polimec } from '@polkadot-api/descriptors';
import { createClient } from 'polkadot-api';
import { withPolkadotSdkCompat } from 'polkadot-api/polkadot-sdk-compat';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { BaseChainManager } from './BaseManager';

export class PolimecManager extends BaseChainManager {
  private chain = Chains.Polimec;

  connect() {
    const provider = withPolkadotSdkCompat(getWsProvider(this.chain));
    const client = createClient(provider);

    // Verify connection
    if (!client) {
      throw new Error(`Failed to connect to ${this.chain}`);
    }

    const api = client.getTypedApi(polimec);
    this.clients.set(this.chain, { client, api });
  }

  disconnect() {
    this.clients.get(this.chain)?.client.destroy();
  }

  getChainType() {
    return this.chain;
  }

  getXcmPallet() {
    const api = this.getApi(this.chain);
    return api.tx.PolkadotXcm;
  }

  getTreasuryAccount() {
    return '58kXueYKLr5b8yCeY3Gd1nLQX2zSJLXjfMzTAuksNq25CFEL' as Accounts;
  }

  getAssetSourceRelation(asset: Asset): AssetSourceRelation {
    switch (asset) {
      case Asset.DOT:
        return AssetSourceRelation.Parent;
      case Asset.USDT:
        return AssetSourceRelation.Sibling;
      case Asset.USDC:
        return AssetSourceRelation.Sibling;
      case Asset.WETH:
        // Placeholder
        return AssetSourceRelation.Self;
      case Asset.PLMC:
        return AssetSourceRelation.Self;
    }
  }

  async getAssetBalanceOf(account: Accounts, asset: Asset): Promise<bigint> {
    const api = this.getApi(Chains.Polimec);
    const asset_source_relation = this.getAssetSourceRelation(asset);
    const asset_location = AssetLocation(asset, asset_source_relation).value;
    const account_balances_result = await api.apis.FungiblesApi.query_account_balances(account);
    if (account_balances_result.success === true && account_balances_result.value.type === 'V4') {
      const assets = account_balances_result.value.value;
      for (const asset of assets) {
        if (Bun.deepEquals(flatObject(asset.id), flatObject(asset_location))) {
          return asset.fun.value as bigint;
        }
      }
    }
    return 0n;
  }

  async getXcmFee() {
    const api = this.getApi(Chains.Polimec);
    const events = await api.event.PolkadotXcm.FeesPaid.pull();
    console.dir(events, { depth: null });

    return events[0]?.payload.fees?.[0]?.fun?.value ?? 0n;
  }

  async getTransactionFee() {
    const api = this.getApi(Chains.Polimec);
    const events = await api.event.TransactionPayment.TransactionFeePaid.pull();
    return (events[0]?.payload.actual_fee as bigint) || 0n;
  }
}
