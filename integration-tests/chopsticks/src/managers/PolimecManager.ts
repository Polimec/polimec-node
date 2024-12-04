import { type Accounts, type Assets, Chains } from '@/types';
import { polimec } from '@polkadot-api/descriptors';
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

  async getAssetBalanceOf(account: Accounts, asset: Assets) {
    const api = this.getApi(Chains.Polimec);
    const balance = await api.query.ForeignAssets.Account.getValue(asset, account);
    return balance?.balance || 0n;
  }

  async getXcmFee() {
    const api = this.getApi(Chains.Polimec);
    const events = await api.event.PolkadotXcm.FeesPaid.pull();
    if (!events.length) return 0n;
    const fees = events[0]?.payload?.fees;
    if (!fees?.length) return 0n;
    return (fees[0]?.fun?.value as bigint) || 0n;
  }
}
