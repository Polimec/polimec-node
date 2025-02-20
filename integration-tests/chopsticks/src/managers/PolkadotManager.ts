import { type Accounts, Asset, AssetSourceRelation, Chains } from '@/types';
import { polkadot } from '@polkadot-api/descriptors';
import { createClient } from 'polkadot-api';
import { withPolkadotSdkCompat } from 'polkadot-api/polkadot-sdk-compat';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { BaseChainManager } from './BaseManager';

export class PolkadotManager extends BaseChainManager {
  private chain = Chains.Polkadot;

  connect() {
    const provider = withPolkadotSdkCompat(getWsProvider(this.chain));
    const client = createClient(provider);

    // Verify connection
    if (!client) {
      throw new Error(`Failed to connect to ${this.chain}`);
    }

    const api = client.getTypedApi(polkadot);
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
      case Asset.ETH:
        // Placeholder
        return AssetSourceRelation.Self;
    }

    return AssetSourceRelation.Self;
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

  async getTransactionFee() {
    const api = this.getApi(Chains.Polkadot);
    const events = await api.event.TransactionPayment.TransactionFeePaid.pull();
    return (events[0]?.payload.actual_fee as bigint) || 0n;
  }
}
