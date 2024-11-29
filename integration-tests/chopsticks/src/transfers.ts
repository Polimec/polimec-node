import { expect } from 'bun:test';
import type { ChainTestManager } from './chainManager';
import { TRANSFER_AMOUNTS } from './constants';
import { Accounts, Assets, Chains, type Parachain } from './types';
import { createTransferData } from './utils';

export class TransferTest {
  constructor(private chainManager: ChainTestManager) {}

  async testAssetTransfer(asset: Assets, initialBalance?: bigint) {
    const { balances: initialBalances } = await this.checkBalances(asset, Accounts.ALICE);
    if (initialBalance) this.verifyInitialBalances(initialBalances, initialBalance);

    const blockNumbers = await this.executeTransfer(asset);
    await this.waitForBlocks(blockNumbers);
    await this.checkExecution(Chains.Polimec);

    const { balances: finalBalances } = await this.checkBalances(asset, Accounts.ALICE);
    if (initialBalance) this.verifyFinalBalances(finalBalances, initialBalance);
  }

  async testNativeTransfer(initialBalance: bigint) {
    const { balances: initialBalances } = await this.checkNativeBalances(Accounts.BOB);
    this.verifyInitialNativeBalances(initialBalances, initialBalance);

    const blockNumbers = await this.executeNativeTransfer();
    await this.waitForBlocks(blockNumbers);

    const { balances: finalBalances } = await this.checkNativeBalances(Accounts.ALICE);
    this.verifyFinalNativeBalances(finalBalances);
  }

  private async checkBalances(asset: Assets, account: Accounts) {
    const hubBalance = await this.chainManager.getAssetsBalance(Chains.PolkadotHub, account, asset);
    const polimecBalance = await this.chainManager.getAssetsBalance(Chains.Polimec, account, asset);

    return {
      balances: { hub: hubBalance, polimec: polimecBalance },
    };
  }

  private async checkExecution(chain: Parachain) {
    const events = await this.chainManager.getMessageQueueEvents(chain);
    expect(events).not.toBeEmpty();
    expect(events).toBeArray();
    expect(events).toHaveLength(1);
    expect(events[0].payload.success).toBeTrue();
  }

  private async checkNativeBalances(account: Accounts) {
    const hubBalance = await this.chainManager.getFreeBalance(Chains.PolkadotHub, account);
    const polimecBalance = await this.chainManager.getAssetsBalance(
      Chains.Polimec,
      account,
      Assets.DOT,
    );

    return {
      balances: { hub: hubBalance, polimec: polimecBalance },
    };
  }

  private async executeTransfer(asset: Assets) {
    const api = this.chainManager.getApi(Chains.PolkadotHub);
    const polimecApi = this.chainManager.getApi(Chains.Polimec);

    const blockNumber = await api.query.System.Number.getValue();
    const polimecBlockNumber = await polimecApi.query.System.Number.getValue();

    const data = createTransferData(TRANSFER_AMOUNTS.TOKENS, BigInt(asset));
    const res = await api.tx.PolkadotXcm.transfer_assets(data).signAndSubmit(
      this.chainManager.getSigner(Accounts.ALICE),
    );

    expect(res.ok).toBeTrue();

    return { blockNumber, polimecBlockNumber };
  }

  private async executeNativeTransfer() {
    const api = this.chainManager.getApi(Chains.PolkadotHub);
    const polimecApi = this.chainManager.getApi(Chains.Polimec);

    const blockNumber = await api.query.System.Number.getValue();
    const polimecBlockNumber = await polimecApi.query.System.Number.getValue();

    const data = createTransferData(TRANSFER_AMOUNTS.NATIVE);
    const res = await api.tx.PolkadotXcm.transfer_assets(data).signAndSubmit(
      this.chainManager.getSigner(Accounts.ALICE),
    );

    expect(res.ok).toBeTrue();

    return { blockNumber, polimecBlockNumber };
  }

  private async waitForBlocks({
    blockNumber,
    polimecBlockNumber,
  }: { blockNumber: number; polimecBlockNumber: number }) {
    await Promise.all([
      this.chainManager.waitForNextBlock(Chains.PolkadotHub, blockNumber),
      this.chainManager.waitForNextBlock(Chains.Polimec, polimecBlockNumber),
    ]);
  }

  private verifyInitialBalances(
    balances: { hub: bigint; polimec: bigint },
    initialBalance: bigint,
  ) {
    expect(balances.hub).toBe(initialBalance);
    expect(balances.polimec).toBe(0n);
  }

  private verifyFinalBalances(balances: { hub: bigint; polimec: bigint }, initialBalance: bigint) {
    expect(balances.hub).toBe(initialBalance - TRANSFER_AMOUNTS.TOKENS);
    expect(balances.polimec).toBe(1682500n);
  }

  private verifyInitialNativeBalances(
    balances: { hub: bigint; polimec: bigint },
    initialBalance: bigint,
  ) {
    expect(balances.hub).toBe(initialBalance);
    expect(balances.polimec).toBe(0n);
  }

  private verifyFinalNativeBalances(balances: { hub: bigint; polimec: bigint }) {
    expect(balances.hub).toBe(9999979047536876n);
    expect(balances.polimec).toBe(19365000000n);
  }
}
