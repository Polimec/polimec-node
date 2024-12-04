import { type Accounts, Chains } from '@/types';
import { polkadot } from '@polkadot-api/descriptors';
import { createClient } from 'polkadot-api';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { BaseChainManager } from './BaseManager';

export class PolkadotManager extends BaseChainManager {
  connect() {
    const client = createClient(getWsProvider(this.getChainType()));
    const api = client.getTypedApi(polkadot);

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

  async getAssetBalanceOf(_account: Accounts, _asset: number): Promise<bigint> {
    throw new Error('Polkadot does not support assets');
  }

  async getXcmFee() {
    const api = this.getApi(Chains.Polkadot);
    const events = await api.event.XcmPallet.FeesPaid.pull();
    return (events[0]?.payload.fees[0].fun.value as bigint) || 0n;
  }
}
