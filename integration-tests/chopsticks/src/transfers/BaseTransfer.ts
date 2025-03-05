import { expect } from 'bun:test';
import type { BaseChainManager } from '@/managers/BaseManager';
import {
  type Accounts,
  type Asset,
  type AssetSourceRelation,
  type BalanceCheck,
  Chains,
  type TransferResult,
} from '@/types';
import { sleep } from 'bun';

export interface TransferOptions {
  account: Accounts;
  assets: [Asset, bigint, AssetSourceRelation][];
  fee_asset_item?: number;
}

export abstract class BaseTransferTest {
  constructor(
    protected sourceManager: BaseChainManager,
    protected destManager: BaseChainManager,
  ) {
    this.sourceManager = sourceManager;
    this.destManager = destManager;
  }

  abstract executeTransfer(options: TransferOptions): Promise<TransferResult>;
  abstract getBalances(options: TransferOptions): Promise<{ asset_balances: BalanceCheck[] }>;
  abstract verifyFinalBalances(
    initialBalances: BalanceCheck[],
    finalBalances: BalanceCheck[],
    options: TransferOptions,
  ): void;

  async testTransfer(options: TransferOptions) {
    // Note: For the bridged tests we use the dry-run Runtime API, so we don't write any data to the chain.
    const isBridged = this.sourceManager.getChainType() === Chains.BridgeHub;

    let initialBalances: BalanceCheck[] = [];
    if (!isBridged) {
      const { asset_balances } = await this.getBalances(options);
      initialBalances = asset_balances; // Assign within the block
      if (options.assets[0][1] > asset_balances[0].source) {
        throw new Error(`Insufficient balance on Source chain for asset: ${options.assets[0][0]}`);
      }
    }

    await this.executeTransfer(options);

    if (!isBridged) {
      await this.waitForBlocks();
      await this.verifyExecution();
      const { asset_balances: finalBalances } = await this.getBalances(options);
      this.verifyFinalBalances(initialBalances, finalBalances, options);
    }
  }

  // TODO: Wait for the next block to be produced.
  protected async waitForBlocks() {
    await sleep(2000);
  }

  protected async verifyExecution() {
    const events = await this.destManager.getMessageQueueEvents();

    console.dir(events, { depth: null });
    expect(events).not.toBeEmpty();
    expect(events).toBeArray();
    expect(events).toHaveLength(1);
    expect(events[0].payload.success).toBeTrue();
  }
}
