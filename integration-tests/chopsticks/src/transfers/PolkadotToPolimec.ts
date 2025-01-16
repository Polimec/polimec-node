import { expect } from 'bun:test';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotManager } from '@/managers/PolkadotManager';
import { Asset, type BalanceCheck, Chains, type PolimecBalanceCheck } from '@/types';
import { createDotMultiHopTransferData } from '@/utils';
import { BaseTransferTest, type TransferOptions } from './BaseTransfer';

export class PolkadotToPolimecTransfer extends BaseTransferTest {
  constructor(
    protected override sourceManager: PolkadotManager,
    protected override destManager: PolimecManager,
  ) {
    super(sourceManager, destManager);
  }

  async executeTransfer({ account, assets }: TransferOptions) {
    const [sourceBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
    ]);

    const amount = assets[0][1];
    const data = createDotMultiHopTransferData(amount);

    const api = this.sourceManager.getApi(Chains.Polkadot);
    const res = await api.tx.XcmPallet.transfer_assets_using_type_and_then(data).signAndSubmit(
      this.sourceManager.getSigner(account),
    );

    expect(res.ok).toBeTrue();
    return { sourceBlock, destBlock };
  }

  async getBalances(options: TransferOptions): Promise<{ asset_balances: PolimecBalanceCheck[] }> {
    throw new Error('Method not implemented.');
  }

  async verifyFinalBalances(
    initialBalances: PolimecBalanceCheck[],
    finalBalances: PolimecBalanceCheck[],
    options: TransferOptions,
  ): Promise<void> {
    throw new Error('Method not implemented.');
  }

  // async getBalances({
  //   account,
  // }: Omit<BaseTransferOptions, 'amount'>): Promise<{ balances: PolimecBalanceCheck }> {
  //   const treasuryAccount = this.destManager.getTreasuryAccount();
  //   return {
  //     balances: {
  //       source: await this.sourceManager.getAssetBalanceOf(account, Asset.DOT),
  //       destination: await this.destManager.getAssetBalanceOf(account, Asset.DOT),
  //       treasury: await this.destManager.getAssetBalanceOf(treasuryAccount, Asset.DOT),
  //     },
  //   };
  // }

  // async verifyFinalBalances(
  //   initialBalances: PolimecBalanceCheck,
  //   finalBalances: PolimecBalanceCheck,
  // ) {
  //   // TODO: At the moment we exclude fees from the balance check since the PAPI team is wotking on some utilies to calculate fees.
  //   expect(initialBalances.destination).toBe(0n);
  //   expect(finalBalances.source).toBeLessThan(initialBalances.source);
  //   expect(finalBalances.destination).toBeGreaterThan(initialBalances.destination);
  //   expect(finalBalances.treasury).toBeGreaterThan(initialBalances.treasury);
  // }
}