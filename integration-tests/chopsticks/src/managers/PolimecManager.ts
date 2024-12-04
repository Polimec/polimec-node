import { type Accounts, Asset, AssetLocation, AssetSourceRelation, Chains } from '@/types';
import { polimec } from '@polkadot-api/descriptors';
import { isEqual } from 'lodash';
import { createClient } from 'polkadot-api';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { BaseChainManager } from './BaseManager';

export class PolimecManager extends BaseChainManager {
  connect() {
    const client = createClient(getWsProvider(this.getChainType()));
    const api = client.getTypedApi(polimec);

    // Verify connection
    if (!client || !api) {
      throw new Error(`Failed to connect to ${this.getChainType()}`);
    }

    this.clients.set(this.getChainType(), { client, api });
  }

  disconnect() {
    this.clients.get(Chains.Polimec)?.client.destroy();
  }

  getChainType() {
    return Chains.Polimec;
  }

  getXcmPallet() {
    const api = this.getApi(Chains.Polimec);
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
    }
  }

  async getAssetBalanceOf(account: Accounts, asset: Asset): Promise<bigint> {
    const api = this.getApi(Chains.Polimec);
    const asset_source_relation = this.getAssetSourceRelation(asset);
    const asset_location = AssetLocation(asset, asset_source_relation).value;
    const account_balances_result = await api.apis.FungiblesApi.query_account_balances(account);
    console.log('Requested asset location in PolimecManager');
    console.dir(asset_location, { depth: null, colors: true });
    console.log('\n\n');
    if (account_balances_result.success === true && account_balances_result.value.type === 'V4') {
      const assets = account_balances_result.value.value;
      for (const asset of assets) {
        if (Bun.deepEquals(asset.id, asset_location)) {
          console.log('Found asset. Balance is: ', asset.fun.value);
          console.dir(asset, { depth: null, colors: true });
          console.log('\n\n');
          return asset.fun.value;
        }
        console.log('Not it chief: \n');
        console.dir(asset, { depth: null, colors: true });
        console.log('\n\n');
      }
    }
    console.log('Asset not found');
    console.log('\n\n');
    return 0n;
  }

  async getLocalXcmFee() {
    const api = this.getApi(Chains.Polimec);
    const events = await api.event.PolkadotXcm.FeesPaid.pull();
    if (!events.length) return 0n;
    const fees = events[0]?.payload?.fees;
    if (!fees?.length) return 0n;
    return (fees[0]?.fun?.value as bigint) || 0n;
  }
}
