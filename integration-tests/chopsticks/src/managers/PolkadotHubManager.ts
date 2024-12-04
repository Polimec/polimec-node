import { type Accounts, type Assets, Chains } from '@/types';
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

  async getAssetBalanceOf(account: Accounts, asset: Assets) {
    const api = this.getApi(Chains.PolkadotHub);
    const balance = await api.query.Assets.Account.getValue(asset, account);
    return balance?.balance || 0n;
  }

  async getSwapCredit() {
    const api = this.getApi(Chains.PolkadotHub);
    const events = await api.event.AssetConversion.SwapCreditExecuted.pull();
    return events[0]?.payload.amount_in || 0n;
  }

  async getXcmFee() {
    const api = this.getApi(Chains.PolkadotHub);
    const events = await api.event.PolkadotXcm.FeesPaid.pull();
    return (events[0]?.payload.fees[0].fun.value as bigint) || 0n;
  }
}
