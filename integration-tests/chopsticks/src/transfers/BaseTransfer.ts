import { expect } from 'bun:test';
import type { BaseChainManager } from '@/managers/BaseManager';
import type { Accounts, BalanceCheck, TransferResult } from '@/types';

export interface BaseTransferOptions {
  amount: bigint;
  account: Accounts;
}

export abstract class BaseTransferTest<T extends BaseTransferOptions = BaseTransferOptions> {
  constructor(
    protected sourceManager: BaseChainManager,
    protected destManager: BaseChainManager,
  ) {}

  abstract executeTransfer(options: T): Promise<TransferResult>;
  abstract checkBalances(options: Omit<T, 'amount'>): Promise<{ balances: BalanceCheck }>;
  abstract verifyFinalBalances(balances: BalanceCheck, options: T): Promise<void>;

  async testTransfer(options: T) {
    const { balances: initialBalances } = await this.checkBalances(options);
    const blockNumbers = await this.executeTransfer(options);
    await this.waitForBlocks(blockNumbers);
    await this.verifyExecution();
    const { balances: finalBalances } = await this.checkBalances(options);
    await this.verifyFinalBalances(finalBalances, options);
    return { initialBalances, finalBalances };
  }

  protected async waitForBlocks({ sourceBlock, destBlock }: TransferResult) {
    await Promise.all([
      this.sourceManager.waitForNextBlock(sourceBlock),
      this.destManager.waitForNextBlock(destBlock),
    ]);
  }

  protected async verifyExecution() {
    const events = await this.destManager.getMessageQueueEvents();
    expect(events).not.toBeEmpty();
    expect(events).toBeArray();
    expect(events).toHaveLength(1);
    expect(events[0].payload.success).toBeTrue();
  }
}
