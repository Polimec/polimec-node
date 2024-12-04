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

  // Note: here are also checking if the extrinsic is executed successfully on the source chain.
  abstract executeTransfer(options: T): Promise<TransferResult>;
  abstract checkBalances(options: Omit<T, 'amount'>): Promise<{ balances: BalanceCheck }>;
  abstract verifyFinalBalances(balances: BalanceCheck, options: T): Promise<void>;

  async testTransfer(options: T) {
    const { balances: initialBalances } = await this.checkBalances(options);
    const blockNumbers = await this.executeTransfer(options);
    // Note: here we wait for the blocks to be finalized on both chains.
    await this.waitForBlocks(blockNumbers);
    // Note: here we check if the extrinsic is executed successfully on the destination chain.
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
    // Note: If in the same block there are multiple events, we should check if OUR message is processed correctly.
    // Here we are just assuming that the first event is the one we are interested in.
    expect(events[0].payload.success).toBeTrue();
  }
}
