import type { pah, polimec, polkadot } from '@polkadot-api/descriptors';
import type { PolkadotClient, TypedApi } from 'polkadot-api';

type Polimec = typeof polimec;
type PolkadotHub = typeof pah;
type Polkadot = typeof polkadot;

export enum Chains {
  Polimec = 'ws://localhost:8000',
  PolkadotHub = 'ws://localhost:8001',
  Polkadot = 'ws://localhost:8002',
}

export const ParaId = {
  [Chains.Polimec]: 3344,
  [Chains.PolkadotHub]: 1000,
};

export enum Accounts {
  BOB = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
  ALICE = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
}

export enum Assets {
  USDT = 1984,
  DOT = 10,
  USDC = 1337,
  UNKNOWN = 42,
}

export type ChainToDefinition = {
  [Chains.Polimec]: Polimec;
  [Chains.PolkadotHub]: PolkadotHub;
  [Chains.Polkadot]: Polkadot;
};

export type ChainClient<T extends Chains> = {
  api: TypedApi<ChainToDefinition[T]>;
  client: PolkadotClient;
};

export interface TransferResult {
  sourceBlock: number;
  destBlock: number;
}

export interface BalanceCheck {
  source: bigint;
  destination: bigint;
}

export interface PolimecBalanceCheck extends BalanceCheck {
  treasury: bigint;
}

export interface TransferDataParams {
  amount: bigint;
  toChain: Chains;
  assetIndex?: bigint;
  recv?: Accounts;
  isMultiHop?: boolean;
  // TODO: Check if this flag is actually needed.
  isFromBridge?: boolean;
}

export interface CreateAssetsParams {
  amount: bigint;
  assetIndex?: bigint;
  isFromBridge?: boolean;
}
