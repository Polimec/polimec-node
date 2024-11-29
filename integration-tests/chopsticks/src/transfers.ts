import { expect } from 'bun:test';
import type { ChainTestManager } from '@/chainManager';
import { INITIAL_BALANCES, TRANSFER_AMOUNTS } from '@/constants';
import { Accounts, Assets, Chains, type Parachain } from '@/types';
import { createTransferData } from '@/utils';

export type TransferDirection = {
  source: Parachain;
  destination: Parachain;
};

export class TransferTest {
  constructor(private chainManager: ChainTestManager) {}

  async testAssetTransfer(
    asset: Assets,
    direction: TransferDirection,
    account: Accounts = Accounts.ALICE,
    initialBalance?: bigint,
  ) {
    const { balances: initialBalances } = await this.checkBalances(asset, account);
    if (initialBalance) this.verifyInitialBalances(initialBalances, initialBalance, direction);

    const blockNumbers = await this.executeTransfer(asset, direction, account);
    await this.waitForBlocks(blockNumbers, direction);

    await this.checkExecutionOn(direction.destination);

    if (
      direction.source === Chains.Polimec &&
      direction.destination === Chains.PolkadotHub &&
      asset === Assets.DOT
    ) {
      const { balances: finalBalances } = await this.checkNativeBalances(account);
      this.verifyFinalNativeBalancesHub(finalBalances, 35930000n);
      return;
    }
    const { balances: finalBalances } = await this.checkBalances(asset, account);
    if (initialBalance) this.verifyFinalBalances(finalBalances, initialBalance, direction);
  }

  async testNativeTransfer(initialBalance: bigint, account: Accounts = Accounts.ALICE) {
    const { balances: initialBalances } = await this.checkNativeBalances(account);
    this.verifyInitialNativeBalances(initialBalances, initialBalance);

    const blockNumbers = await this.executeNativeTransfer(account);
    await this.waitForBlocks(blockNumbers, {
      source: Chains.PolkadotHub,
      destination: Chains.Polimec,
    });

    const { balances: finalBalances } = await this.checkNativeBalances(account);
    await this.checkExecutionOn(Chains.Polimec);
    const fee = await this.chainManager.getExtrinsicFee(Chains.PolkadotHub);
    const xcmFee = await this.chainManager.getXcmFee(Chains.PolkadotHub);
    const totalFee = fee + xcmFee;

    this.verifyFinalNativeBalances(finalBalances, totalFee);
  }

  private async checkBalances(asset: Assets, account: Accounts) {
    const hubBalance = await this.chainManager.getAssetsBalance(Chains.PolkadotHub, account, asset);
    const polimecBalance = await this.chainManager.getAssetsBalance(Chains.Polimec, account, asset);

    return {
      balances: { hub: hubBalance, polimec: polimecBalance },
    };
  }

  private async checkExecutionOn(chain: Parachain) {
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

  private async executeTransfer(asset: Assets, direction: TransferDirection, account: Accounts) {
    const sourceApi = this.chainManager.getApi(direction.source);
    const destApi = this.chainManager.getApi(direction.destination);

    const sourceBlockNumber = await sourceApi.query.System.Number.getValue();
    const destBlockNumber = await destApi.query.System.Number.getValue();
    const amount = asset === Assets.DOT ? TRANSFER_AMOUNTS.NATIVE : TRANSFER_AMOUNTS.TOKENS;

    const data = createTransferData(amount, direction.destination, BigInt(asset), account);

    const res = await sourceApi.tx.PolkadotXcm.transfer_assets(data).signAndSubmit(
      this.chainManager.getSigner(account),
    );

    expect(res.ok).toBeTrue();

    return { sourceBlockNumber, destBlockNumber };
  }

  private async executeNativeTransfer(account: Accounts) {
    const hubApi = this.chainManager.getApi(Chains.PolkadotHub);
    const polimecApi = this.chainManager.getApi(Chains.Polimec);

    const sourceBlockNumber = await hubApi.query.System.Number.getValue();
    const destBlockNumber = await polimecApi.query.System.Number.getValue();

    const data = createTransferData(TRANSFER_AMOUNTS.NATIVE, Chains.Polimec);
    const res = await hubApi.tx.PolkadotXcm.transfer_assets(data).signAndSubmit(
      this.chainManager.getSigner(account),
    );

    expect(res.ok).toBeTrue();

    return { sourceBlockNumber, destBlockNumber };
  }

  private async waitForBlocks(
    { sourceBlockNumber, destBlockNumber }: { sourceBlockNumber: number; destBlockNumber: number },
    direction: TransferDirection,
  ) {
    await Promise.all([
      this.chainManager.waitForNextBlock(direction.source, sourceBlockNumber),
      this.chainManager.waitForNextBlock(direction.destination, destBlockNumber),
    ]);
  }

  private verifyInitialBalances(
    balances: { hub: bigint; polimec: bigint },
    initialBalance: bigint,
    direction: TransferDirection,
  ) {
    const isFromHub = direction.source === Chains.PolkadotHub;
    expect(balances.hub).toBe(isFromHub ? initialBalance : 0n);
    expect(balances.polimec).toBe(isFromHub ? 0n : initialBalance);
  }

  private async verifyFinalBalances(
    balances: { hub: bigint; polimec: bigint },
    initialBalance: bigint,
    direction: TransferDirection,
  ) {
    const isFromHub = direction.source === Chains.PolkadotHub;
    let swapFee = 0n;
    if (!isFromHub) {
      swapFee += await this.chainManager.getSwapCreditExecuted(Chains.PolkadotHub);
    }

    expect(balances.hub).toBe(
      isFromHub ? initialBalance - TRANSFER_AMOUNTS.TOKENS : TRANSFER_AMOUNTS.TOKENS - swapFee,
    );
    expect(balances.polimec).toBe(isFromHub ? 1682500n : initialBalance - TRANSFER_AMOUNTS.TOKENS);
  }

  private verifyInitialNativeBalances(
    balances: { hub: bigint; polimec: bigint },
    initialBalance: bigint,
  ) {
    expect(balances.hub).toBe(initialBalance);
    expect(balances.polimec).toBe(0n);
  }

  private verifyFinalNativeBalances(balances: { hub: bigint; polimec: bigint }, fee: bigint) {
    expect(balances.hub).toBe(INITIAL_BALANCES.DOT - TRANSFER_AMOUNTS.NATIVE - fee);
    expect(balances.polimec).toBe(19365000000n);
  }

  private verifyFinalNativeBalancesHub(balances: { hub: bigint; polimec: bigint }, fee: bigint) {
    expect(balances.hub).toBe(TRANSFER_AMOUNTS.NATIVE - fee);
    expect(balances.polimec).toBe(9999980000000000n);
  }
}
