import { expect } from 'bun:test';
import { INITIAL_BALANCES } from '@/constants';
import type { BaseChainManager } from '@/managers/BaseManager';
import type { PolkadotManager } from '@/managers/PolkadotManager';
import { type Accounts, Assets, Chains } from '@/types';
import { createMultiHopTransferData } from '@/utils';
import { type BaseTransferOptions, BaseTransferTest } from './BaseTransfer';

interface PolkadotTransferOptions extends BaseTransferOptions {
  asset: Assets.DOT;
}

export class PolkadotToPolimecTransfer extends BaseTransferTest<PolkadotTransferOptions> {
  constructor(
    protected override sourceManager: PolkadotManager,
    protected override destManager: BaseChainManager,
  ) {
    super(sourceManager, destManager);
  }

  async executeTransfer({ amount, account }: PolkadotTransferOptions) {
    const [sourceBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
    ]);

    const data = createMultiHopTransferData({
      amount,
      toChain: Chains.Polimec,
      recv: account,
    });

    const api = this.sourceManager.getApi(Chains.Polkadot);
    const res = await api.tx.XcmPallet.transfer_assets_using_type_and_then(data).signAndSubmit(
      this.sourceManager.getSigner(account),
    );

    expect(res.ok).toBeTrue();
    return { sourceBlock, destBlock };
  }

  async checkBalances({ account }: Omit<PolkadotTransferOptions, 'amount'>) {
    return {
      balances: {
        source: await this.sourceManager.getNativeBalanceOf(account),
        destination: await this.destManager.getAssetBalanceOf(account, Assets.DOT),
      },
    };
  }

  async verifyFinalBalances(
    balances: { source: bigint; destination: bigint },
    { amount }: PolkadotTransferOptions,
  ) {
    const fee = await this.sourceManager.getExtrinsicFee();
    const xcmFee = await this.sourceManager.getXcmFee();
    const totalFee = fee + xcmFee;

    expect(balances.source).toBe(INITIAL_BALANCES.DOT - amount - totalFee);
    expect(balances.destination).toBeGreaterThan(0n);
  }
}
