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

export type Parachain = Chains.Polimec | Chains.PolkadotHub;

export enum Accounts {
  BOB = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
  ALICE = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
}

export enum Assets {
  USDT = 1984,
  DOT = 10,
  USDC = 1337,
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
