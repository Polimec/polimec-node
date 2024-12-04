import { expect } from 'bun:test';
import { INITIAL_BALANCES } from '@/constants';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import { type Accounts, Assets, Chains } from '@/types';
import { createTransferData } from '@/utils';
import { BaseTransferTest } from './BaseTransfer';

interface HubTransferOptions {
  amount: bigint;
  account: Accounts;
  asset: Assets;
}

export class HubToPolimecTransfer extends BaseTransferTest<HubTransferOptions> {
  constructor(
    protected override sourceManager: PolkadotHubManager,
    protected override destManager: PolimecManager,
  ) {
    super(sourceManager, destManager);
  }

  async executeTransfer({ amount, account, asset }: HubTransferOptions) {
    const [sourceBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
    ]);

    const data = createTransferData({
      amount,
      toChain: Chains.Polimec,
      recv: account,
      assetIndex: asset === Assets.DOT ? undefined : BigInt(asset),
    });

    const api = this.sourceManager.getApi(Chains.PolkadotHub);
    const res = await api.tx.PolkadotXcm.transfer_assets(data).signAndSubmit(
      this.sourceManager.getSigner(account),
    );

    expect(res.ok).toBeTrue();
    return { sourceBlock, destBlock };
  }

  async checkBalances({ account, asset }: Omit<HubTransferOptions, 'amount'>) {
    const isNativeTransfer = asset === Assets.DOT;
    return {
      balances: {
        source: isNativeTransfer
          ? await this.sourceManager.getNativeBalanceOf(account)
          : await this.sourceManager.getAssetBalanceOf(account, asset),
        destination: await this.destManager.getAssetBalanceOf(account, asset),
      },
    };
  }

  async verifyFinalBalances(
    balances: { source: bigint; destination: bigint },
    { amount, asset }: HubTransferOptions,
  ) {
    const fee = await this.sourceManager.getExtrinsicFee();
    const xcmFee = await this.sourceManager.getXcmFee();
    const totalFee = fee + xcmFee;
    const initialBalance =
      asset === Assets.DOT
        ? INITIAL_BALANCES.DOT
        : asset === Assets.USDT
          ? INITIAL_BALANCES.USDT
          : INITIAL_BALANCES.USDC;

    expect(balances.source).toBe(initialBalance - amount - (asset === Assets.DOT ? totalFee : 0n));
    expect(balances.destination).toBeGreaterThan(0n);
  }
}
