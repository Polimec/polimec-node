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
  ): Promise<void>;

  async testTransfer(options: TransferOptions) {
    const { asset_balances: initialBalances } = await this.getBalances(options);
    const blockNumbers = await this.executeTransfer(options);
    await this.waitForBlocks(blockNumbers);
    await this.verifyExecution();
    const { asset_balances: finalBalances } = await this.getBalances(options);
    await this.verifyFinalBalances(initialBalances, finalBalances, options);
  }

  protected async waitForBlocks({ sourceBlock, destBlock }: TransferResult) {
    await Promise.all([
      this.sourceManager.waitForNextBlock(sourceBlock),
      this.destManager.waitForNextBlock(destBlock),
    ]);
  }

  protected async verifyExecution() {
    const events = await this.destManager.getMessageQueueEvents();
    const v = await this.destManager
      .getApi(Chains.Polimec)
      .event.MessageQueue.ProcessingFailed.pull();
    console.log('MsgQ Events:', v);

    expect(events).not.toBeEmpty();
    expect(events).toBeArray();
    expect(events).toHaveLength(1);
    expect(events[0].payload.success).toBeTrue();
  }
}
