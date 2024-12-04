import { expect } from 'bun:test';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotManager } from '@/managers/PolkadotManager';
import { Assets, Chains, type PolimecBalanceCheck } from '@/types';
import { createMultiHopTransferData } from '@/utils';
import { type BaseTransferOptions, BaseTransferTest } from './BaseTransfer';

interface PolkadotTransferOptions extends BaseTransferOptions {
  asset: Assets.DOT;
}

export class PolkadotToPolimecTransfer extends BaseTransferTest<PolkadotTransferOptions> {
  constructor(
    protected override sourceManager: PolkadotManager,
    protected override destManager: PolimecManager,
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

  async getBalances({
    account,
  }: Omit<PolkadotTransferOptions, 'amount'>): Promise<{ balances: PolimecBalanceCheck }> {
    const treasuryAccount = this.destManager.getTreasuryAccount();
    return {
      balances: {
        source: await this.sourceManager.getNativeBalanceOf(account),
        destination: await this.destManager.getAssetBalanceOf(account, Assets.DOT),
        treasury: await this.destManager.getAssetBalanceOf(treasuryAccount, Assets.DOT),
      },
    };
  }

  async verifyFinalBalances(
    initialBalances: PolimecBalanceCheck,
    finalBalances: PolimecBalanceCheck,
    { amount }: PolkadotTransferOptions,
  ) {
    // TODO: At the moment we exclude fees from the balance check since the PAPI team is wotking on some utilies to calculate fees.
    expect(initialBalances.destination).toBe(0n);
    expect(finalBalances.source).toBeLessThan(initialBalances.source);
    expect(finalBalances.destination).toBeGreaterThan(initialBalances.destination);
    expect(finalBalances.treasury).toBeGreaterThan(initialBalances.treasury);
  }
}
