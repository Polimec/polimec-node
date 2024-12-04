import { TRANSFER_AMOUNTS } from '@/constants.ts';
import { type Accounts, Asset, AssetSourceRelation, Chains } from '@/types';
import { polkadot } from '@polkadot-api/descriptors';
import { createClient } from 'polkadot-api';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { BaseChainManager } from './BaseManager';

export class PolkadotManager extends BaseChainManager {
  connect() {
    const client = createClient(getWsProvider(this.getChainType()));
    const api = client.getTypedApi(polkadot);

    // Verify connection
    if (!client || !api) {
      throw new Error(`Failed to connect to ${this.getChainType()}`);
    }

    this.clients.set(this.getChainType(), { client, api });
  }

  disconnect() {
    this.clients.get(Chains.Polkadot)?.client.destroy();
  }

  getChainType() {
    return Chains.Polkadot;
  }

  getXcmPallet() {
    const api = this.getApi(Chains.Polkadot);
    return api.tx.XcmPallet;
  }

  getAssetSourceRelation(asset: Asset): AssetSourceRelation {
    switch (asset) {
      case Asset.DOT:
        return AssetSourceRelation.Self;
      case Asset.USDT:
        // Placeholder
        return AssetSourceRelation.Self;
      case Asset.USDC:
        // Placeholder
        return AssetSourceRelation.Self;
      case Asset.WETH:
        // Placeholder
        return AssetSourceRelation.Self;
    }
  }

  async getAssetBalanceOf(account: Accounts, asset: Asset): Promise<bigint> {
    const api = this.getApi(this.getChainType());
    if (asset === Asset.DOT) {
      const balance = await api.query.System.Account.getValue(account);
      return balance.data.free;
    }
    return 0n;
  }

  async getXcmFee() {
    const api = this.getApi(Chains.Polkadot);
    const events = await api.event.XcmPallet.FeesPaid.pull();
    return (events[0]?.payload.fees[0].fun.value as bigint) || 0n;
  }
}
