import { pah, polimec, polkadot } from '@polkadot-api/descriptors';
import { sr25519CreateDerive } from '@polkadot-labs/hdkd';
import { DEV_PHRASE, entropyToMiniSecret, mnemonicToEntropy } from '@polkadot-labs/hdkd-helpers';
import { type PolkadotSigner, type TypedApi, createClient } from 'polkadot-api';
import { getPolkadotSigner } from 'polkadot-api/signer';
import { getWsProvider } from 'polkadot-api/ws-provider/web';
import { filter, firstValueFrom, take } from 'rxjs';
import {
  Accounts,
  type Assets,
  type ChainClient,
  type ChainToDefinition,
  Chains,
  type Parachain,
} from './types';

export class ChainTestManager {
  private clients: Map<Chains, ChainClient<Chains>> = new Map();
  private signers: Map<Accounts, PolkadotSigner> = new Map();

  constructor() {
    // Initialize HDKD
    const entropy = mnemonicToEntropy(DEV_PHRASE);
    const miniSecret = entropyToMiniSecret(entropy);
    const derive = sr25519CreateDerive(miniSecret);

    // Setup signers
    const hdkdKeyPairAlice = derive('//Alice');
    const hdkdKeyPairBob = derive('//Bob');

    this.signers.set(
      Accounts.ALICE,
      getPolkadotSigner(hdkdKeyPairAlice.publicKey, 'Sr25519', hdkdKeyPairAlice.sign),
    );

    this.signers.set(
      Accounts.BOB,
      getPolkadotSigner(hdkdKeyPairBob.publicKey, 'Sr25519', hdkdKeyPairBob.sign),
    );
  }

  connect() {
    // Initialize chain connections
    const polimecClient = createClient(getWsProvider(Chains.Polimec));
    const polkadotHubClient = createClient(getWsProvider(Chains.PolkadotHub));
    const polkadotClient = createClient(getWsProvider(Chains.Polkadot));

    this.clients.set(Chains.Polimec, {
      client: polimecClient,
      api: polimecClient.getTypedApi(polimec),
    });

    this.clients.set(Chains.PolkadotHub, {
      client: polkadotHubClient,
      api: polkadotHubClient.getTypedApi(pah),
    });

    this.clients.set(Chains.Polkadot, {
      client: polkadotClient,
      api: polkadotClient.getTypedApi(polkadot),
    });
  }

  disconnect() {
    for (const client of this.clients.values()) {
      client.client.destroy();
    }
  }

  getApi<T extends Chains>(chain: T): TypedApi<ChainToDefinition[T]> {
    const client = this.clients.get(chain);
    if (!client) throw new Error(`Chain ${chain} not initialized`);
    return client.api as TypedApi<ChainToDefinition[T]>;
  }

  getSigner(account: Accounts) {
    const signer = this.signers.get(account);
    if (!signer) throw new Error(`Signer for ${account} not found`);
    return signer;
  }

  waitForNextBlock = (chain: Chains, currentBlock: number) => {
    const api = this.getApi(chain);
    return firstValueFrom(
      api.query.System.Number.watchValue().pipe(
        filter((newBlock) => newBlock > currentBlock),
        take(1),
      ),
    );
  };

  getMessageQueueEvents = async (chain: Parachain) => {
    const api = this.getApi(chain);
    return api.event.MessageQueue.Processed.pull();
  };

  getExtrinsicFee = async (chain: Parachain) => {
    const api = this.getApi(chain);
    const event = await api.event.TransactionPayment.TransactionFeePaid.pull();
    return event[0].payload.actual_fee;
  };

  getXcmFee = async (chain: Parachain) => {
    const api = this.getApi(chain);
    const event = await api.event.PolkadotXcm.FeesPaid.pull();
    return event[0].payload.fees[0].fun.value as bigint;
  };

  getSwapCreditExecuted = async (chain: Chains.PolkadotHub) => {
    const api = this.getApi(chain);
    const event = await api.event.AssetConversion.SwapCreditExecuted.pull();
    return event[0]?.payload.amount_in || 0n;
  };

  async getFreeBalance(chain: Chains, account: Accounts) {
    const api = this.getApi(chain);
    const balance = await api.query.System.Account.getValue(account);
    return balance.data.free;
  }

  async getBlockNumber(chain: Chains) {
    const api = this.getApi(chain);
    const blockNumber = await api.query.System.Number.getValue();
    return blockNumber;
  }

  async getAssetsBalance(chain: Parachain, account: Accounts, asset: Assets) {
    if (chain === Chains.Polimec) {
      const api = this.getApi(chain);
      const assetBalance = await api.query.ForeignAssets.Account.getValue(asset, account);
      return assetBalance?.balance || 0n;
    }
    const api = this.getApi(chain);
    const assetBalance = await api.query.Assets.Account.getValue(asset, account);
    return assetBalance?.balance || 0n;
  }
}
