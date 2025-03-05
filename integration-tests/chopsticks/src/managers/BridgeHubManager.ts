import { type Accounts, Asset, AssetSourceRelation, Chains } from '@/types';
import { bridge } from '@polkadot-api/descriptors';
import { createClient } from 'polkadot-api';
import { withPolkadotSdkCompat } from 'polkadot-api/polkadot-sdk-compat';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { BaseChainManager } from './BaseManager';

export class BridgerHubManagaer extends BaseChainManager {
  private chain = Chains.BridgeHub;

  connect() {
    const provider = withPolkadotSdkCompat(getWsProvider(this.chain));
    const client = createClient(provider);

    // Verify connection
    if (!client) {
      throw new Error(`Failed to connect to ${this.chain}`);
    }

    const api = client.getTypedApi(bridge);
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
        // TODO: Check it Placeholder
        return AssetSourceRelation.Self;
    }
  }

  // Note: On BridgeHub, there should be no balance for any asset.
  // There is DOT, but we are not tracking it.
  async getAssetBalanceOf(account: Accounts, asset: Asset): Promise<bigint> {
    return 0n;
  }

  async getLocalXcmFee() {
    const api = this.getApi(Chains.BridgeHub);
    const events = await api.event.PolkadotXcm.FeesPaid.pull();
    if (!events.length) return 0n;
    const fees = events[0]?.payload?.fees;
    if (!fees?.length) return 0n;
    return (fees[0]?.fun?.value as bigint) || 0n;
  }
}
