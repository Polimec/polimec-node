import { DERIVE_PATHS } from '@/constants';
import type { Accounts, ChainClient, ChainToDefinition, Chains } from '@/types';
import { sr25519CreateDerive } from '@polkadot-labs/hdkd';
import { DEV_PHRASE, entropyToMiniSecret, mnemonicToEntropy } from '@polkadot-labs/hdkd-helpers';
import type { PolkadotSigner, TypedApi } from 'polkadot-api';
import { getPolkadotSigner } from 'polkadot-api/signer';
import { filter, firstValueFrom, take } from 'rxjs';

export abstract class BaseChainManager {
  protected clients: Map<Chains, ChainClient<Chains>> = new Map();
  protected signers: Map<Accounts, PolkadotSigner> = new Map();

  constructor() {
    this.initializeSigners();
  }

  private initializeSigners() {
    const entropy = mnemonicToEntropy(DEV_PHRASE);
    const miniSecret = entropyToMiniSecret(entropy);
    const derive = sr25519CreateDerive(miniSecret);

    for (const [account, path] of Object.entries(DERIVE_PATHS)) {
      const keyPair = derive(path);
      this.signers.set(
        account as Accounts,
        getPolkadotSigner(keyPair.publicKey, 'Sr25519', keyPair.sign),
      );
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

  async waitForNextBlock(currentBlock: number) {
    const api = this.getApi(this.getChainType());
    return firstValueFrom(
      api.query.System.Number.watchValue().pipe(
        filter((newBlock) => newBlock > currentBlock),
        take(1),
      ),
    );
  }

  async getBlockNumber() {
    const chain = this.getChainType();
    const api = this.getApi(chain);
    return api.query.System.Number.getValue();
  }

  async getMessageQueueEvents() {
    const api = this.getApi(this.getChainType());
    return api.event.MessageQueue.Processed.pull();
  }

  async getExtrinsicFee() {
    const api = this.getApi(this.getChainType());
    const events = await api.event.TransactionPayment.TransactionFeePaid.pull();
    return events[0]?.payload.actual_fee || 0n;
  }

  async getNativeBalanceOf(account: Accounts) {
    const api = this.getApi(this.getChainType());
    const balance = await api.query.System.Account.getValue(account);
    return balance.data.free;
  }

  abstract getTreasuryAccount(): Accounts;

  // @ts-expect-error - TODO: Not sure which is the correct return type for this
  abstract getXcmPallet();

  abstract getChainType(): Chains;

  abstract getAssetBalanceOf(account: Accounts, asset: number): Promise<bigint>;

  abstract connect(): void;

  abstract disconnect(): void;
}
