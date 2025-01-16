import { expect } from 'bun:test';
import { INITIAL_BALANCES } from '@/constants';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import { Asset, type BalanceCheck, Chains, getVersionedAssets } from '@/types';
import { createTransferData } from '@/utils';
import { BaseTransferTest, type TransferOptions } from './BaseTransfer';

export class PolimecToHubTransfer extends BaseTransferTest {
  constructor(
    protected override sourceManager: PolimecManager,
    protected override destManager: PolkadotHubManager,
  ) {
    super(sourceManager, destManager);
  }

  async executeTransfer({ account, assets }: TransferOptions) {
    const [sourceBlock, destBlock] = await Promise.all([
      this.sourceManager.getBlockNumber(),
      this.destManager.getBlockNumber(),
    ]);

    const versioned_assets = getVersionedAssets(assets);
    const data = createTransferData({
      toChain: Chains.PolkadotHub,
      assets: versioned_assets,
      recv: account,
    });

    const res = await this.sourceManager
      .getXcmPallet()
      .transfer_assets(data)
      .signAndSubmit(this.sourceManager.getSigner(account));

    expect(res.ok).toBeTrue();
    return { sourceBlock, destBlock };
  }

  async getBalances(options: TransferOptions): Promise<{ asset_balances: BalanceCheck[] }> {
    const source = await this.sourceManager.getAssetBalanceOf(
      options.account,
      options.assets[0][0],
    );
    const destination = await this.destManager.getAssetBalanceOf(
      options.account,
      options.assets[0][0],
    );
    return { asset_balances: [{ source, destination }] };
  }

  verifyFinalBalances(
    initialBalances: BalanceCheck[],
    finalBalances: BalanceCheck[],
    options: TransferOptions,
  ) {
    // TODO: At the moment we exclude fees from the balance check since the PAPI team is wotking on some utilies to calculate fees.
    const initialBalance =
      options.assets[0][0] === Asset.DOT
        ? INITIAL_BALANCES.DOT
        : options.assets[0][0] === Asset.USDT
          ? INITIAL_BALANCES.USDT
          : INITIAL_BALANCES.USDC;
    for (let i = 0; i < options.assets.length; i++) {
      expect(initialBalances[i].destination).toBe(0n);
      expect(initialBalances[i].source).toBe(initialBalance);
      expect(finalBalances[i].source).toBeLessThan(initialBalances[i].source);
      expect(finalBalances[i].destination).toBeGreaterThan(initialBalances[i].destination);
    }
  }
}