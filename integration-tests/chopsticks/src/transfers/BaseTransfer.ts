import { expect } from 'bun:test';
import type { BaseChainManager } from '@/managers/BaseManager';
import type { Accounts, Asset, AssetSourceRelation, BalanceCheck, TransferResult } from '@/types';
import { sleep } from 'bun';

export interface TransferOptions {
  account: Accounts;
  assets: [Asset, bigint, AssetSourceRelation][];
}

export abstract class BaseTransferTest {
  constructor(
    protected sourceManager: BaseChainManager,
    protected destManager: BaseChainManager,
  ) {}

  abstract executeTransfer(options: TransferOptions): Promise<TransferResult>;
  abstract getBalances(options: TransferOptions): Promise<{ asset_balances: BalanceCheck[] }>;
  abstract verifyFinalBalances(
    initialBalances: BalanceCheck[],
    finalBalances: BalanceCheck[],
    options: TransferOptions,
  ): void;

  async testTransfer(options: TransferOptions) {
    const { asset_balances: initialBalances } = await this.getBalances(options);
    if (options.assets[0][1] > initialBalances[0].source) {
      throw new Error(`Insufficient balance on Source chain for asset: ${options.assets[0][0]}`);
    }
    const _blockNumbers = await this.executeTransfer(options);
    await this.waitForBlocks();
    await this.verifyExecution();
    const { asset_balances: finalBalances } = await this.getBalances(options);
    this.verifyFinalBalances(initialBalances, finalBalances, options);
  }

  // TODO: Wait for the next block to be produced.
  protected async waitForBlocks() {
    await sleep(2000);
  }

  protected async verifyExecution() {
    const events = await this.destManager.getMessageQueueEvents();

    expect(events).not.toBeEmpty();
    expect(events).toBeArray();
    expect(events).toHaveLength(1);
    expect(events[0].payload.success).toBeTrue();
  }
}
