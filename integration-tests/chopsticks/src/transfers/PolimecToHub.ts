import { expect } from 'bun:test';
import { INITIAL_BALANCES } from '@/constants';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import { Assets, type BalanceCheck, Chains } from '@/types';
import { createTransferData } from '@/utils';
import { type BaseTransferOptions, BaseTransferTest } from './BaseTransfer';

interface PolimecTransferOptions extends BaseTransferOptions {
  asset: Assets;
}

export class PolimecToHubTransfer extends BaseTransferTest<PolimecTransferOptions> {
  constructor(
    protected override sourceManager: PolimecManager,
    protected override destManager: PolkadotHubManager,
  ) {
    super(sourceManager, destManager);
  }

  async executeTransfer({ amount, account, asset }: PolimecTransferOptions) {
    const [sourceBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
    ]);

    const data = createTransferData({
      amount,
      toChain: Chains.PolkadotHub,
      assetIndex: BigInt(asset),
      recv: account,
    });

    const res = await this.sourceManager
      .getXcmPallet()
      .transfer_assets(data)
      .signAndSubmit(this.sourceManager.getSigner(account));

    expect(res.ok).toBeTrue();
    return { sourceBlock, destBlock };
  }

  async getBalances({
    account,
    asset,
  }: Omit<PolimecTransferOptions, 'amount'>): Promise<{ balances: BalanceCheck }> {
    const isNativeTransfer = asset === Assets.DOT;
    return {
      balances: {
        source: await this.sourceManager.getAssetBalanceOf(account, asset),
        destination: isNativeTransfer
          ? await this.destManager.getNativeBalanceOf(account)
          : await this.destManager.getAssetBalanceOf(account, asset),
      },
    };
  }

  async verifyFinalBalances(
    initialBalances: BalanceCheck,
    finalBalances: BalanceCheck,
    { asset }: PolimecTransferOptions,
  ) {
    // TODO: At the moment we exclude fees from the balance check since the PAPI team is wotking on some utilies to calculate fees.
    const initialBalance =
      asset === Assets.DOT
        ? INITIAL_BALANCES.DOT
        : asset === Assets.USDT
          ? INITIAL_BALANCES.USDT
          : INITIAL_BALANCES.USDC;

    expect(initialBalances.destination).toBe(0n);
    expect(initialBalances.source).toBe(initialBalance);
    expect(finalBalances.source).toBeLessThan(initialBalances.source);
    expect(finalBalances.destination).toBeGreaterThan(initialBalances.destination);
  }
}
