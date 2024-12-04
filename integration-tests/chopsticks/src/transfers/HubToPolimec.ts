import { expect } from 'bun:test';
import { INITIAL_BALANCES } from '@/constants';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import { type Accounts, Assets, Chains, type PolimecBalanceCheck } from '@/types';
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

  async getBalances({
    account,
    asset,
  }: Omit<HubTransferOptions, 'amount'>): Promise<{ balances: PolimecBalanceCheck }> {
    const isNativeTransfer = asset === Assets.DOT;
    const treasuryAccount = this.destManager.getTreasuryAccount();
    return {
      balances: {
        source: isNativeTransfer
          ? await this.sourceManager.getNativeBalanceOf(account)
          : await this.sourceManager.getAssetBalanceOf(account, asset),
        destination: await this.destManager.getAssetBalanceOf(account, asset),
        treasury: await this.destManager.getAssetBalanceOf(treasuryAccount, asset),
      },
    };
  }

  async verifyFinalBalances(
    initialBalances: PolimecBalanceCheck,
    finalBalances: PolimecBalanceCheck,
    { amount, asset }: HubTransferOptions,
  ) {
    // TODO: At the moment we exclude fees from the balance check since the PAPI team is wotking on some utilies to calculate fees.
    const initialBalance =
      asset === Assets.DOT
        ? INITIAL_BALANCES.DOT
        : asset === Assets.USDT
          ? INITIAL_BALANCES.USDT
          : INITIAL_BALANCES.USDC;
    // Note: Initially every account on destination is empty.
    expect(initialBalances.destination).toBe(0n);
    expect(initialBalances.source).toBe(initialBalance);
    expect(finalBalances.source).toBeLessThan(initialBalances.source);
    expect(finalBalances.destination).toBeGreaterThan(initialBalances.destination);
    expect(finalBalances.treasury).toBeGreaterThan(initialBalances.treasury);
  }
}
