import {
  XcmV3Junction,
  XcmV3JunctionNetworkId,
  XcmV3Junctions,
  XcmV3MultiassetFungibility,
  XcmVersionedAssets,
  XcmVersionedLocation,
  type bridge,
  type pah,
  type polimec,
  type polkadot,
} from '@polkadot-api/descriptors';
import { FixedSizeBinary, type PolkadotClient, type TypedApi } from 'polkadot-api';
import { WETH_ADDRESS } from './constants';

type Polimec = typeof polimec;
type PolkadotHub = typeof pah;
type Polkadot = typeof polkadot;
type BridgeHub = typeof bridge;

export const Relay = {
  Polkadot: 'ws://localhost:8002',
} as const;

export const Parachains = {
  Polimec: 'ws://localhost:8000',
  PolkadotHub: 'ws://localhost:8001',
  BridgeHub: 'ws://localhost:8003',
} as const;

export const Chains = {
  ...Relay,
  ...Parachains,
} as const;

// Create a type for the combined object
export type Chains = (typeof Chains)[keyof typeof Chains];

export type Parachains = (typeof Parachains)[keyof typeof Parachains];

export type ChainClient<T extends Chains> = {
  api: TypedApi<ChainToDefinition[T]>;
  client: PolkadotClient;
};

const PARACHAIN_IDS = {
  [Parachains.Polimec]: 3344,
  [Parachains.PolkadotHub]: 1000,
  [Parachains.BridgeHub]: 1002,
} as const;

export const ParaId = PARACHAIN_IDS;

export enum AssetSourceRelation {
  Parent = 0,
  Sibling = 1,
  Self = 2,
}

export enum Accounts {
  BOB = '5FHneW46xGXgs5mUiveU4sbTyGBzmstUspZC92UhjJM694ty',
  ALICE = '5GrwvaEF5zXb26Fz9rcQpDWS57CtERHpNehXCPcNoHGKutQY',
  POLIMEC = '5Eg2fnsdZ37LtdXnum4Lp9M1pAmBeThPxbqGMnCe4Lh77jdD'
}

export type ChainToDefinition = {
  [Chains.Polimec]: Polimec;
  [Chains.PolkadotHub]: PolkadotHub;
  [Chains.Polkadot]: Polkadot;
  [Chains.BridgeHub]: BridgeHub;
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
  toChain: Chains;
  assets: XcmVersionedAssets;
  recv?: Accounts;
  isMultiHop?: boolean;
}

export enum Asset {
  DOT = 10,
  USDC = 1337,
  USDT = 1984,
  WETH = 10000, // Note: This is not the real Asset ID - we should improve this.
}

export function AssetHubAssetLocation(
  assetId: bigint,
  source_relation: AssetSourceRelation,
): XcmVersionedLocation {
  switch (source_relation) {
    case AssetSourceRelation.Sibling:
      return XcmVersionedLocation.V4({
        parents: 1,
        interior: XcmV3Junctions.X3([
          XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub]),
          XcmV3Junction.PalletInstance(50),
          XcmV3Junction.GeneralIndex(assetId),
        ]),
      });
    case AssetSourceRelation.Self:
      return XcmVersionedLocation.V4({
        parents: 0,
        interior: XcmV3Junctions.X2([
          XcmV3Junction.PalletInstance(50),
          XcmV3Junction.GeneralIndex(assetId),
        ]),
      });
    case AssetSourceRelation.Parent:
      return XcmVersionedLocation.V4({
        parents: 0,
        interior: XcmV3Junctions.X3([
          XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub]),
          XcmV3Junction.PalletInstance(50),
          XcmV3Junction.GeneralIndex(assetId),
        ]),
      });
  }
}

export function NativeAssetLocation(
  source_relation: AssetSourceRelation,
  paraId?: number,
): XcmVersionedLocation {
  switch (source_relation) {
    case AssetSourceRelation.Sibling:
      if (!paraId) {
        throw new Error('You need to specify a paraId with SourceRelation.Sibling');
      }
      return XcmVersionedLocation.V4({
        parents: 1,
        interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(paraId)),
      });
    case AssetSourceRelation.Self:
      return XcmVersionedLocation.V4({
        parents: 0,
        interior: XcmV3Junctions.Here(),
      });
    case AssetSourceRelation.Parent:
      return XcmVersionedLocation.V4({
        parents: 1,
        interior: XcmV3Junctions.Here(),
      });
  }
}

export function EthereumAssetLocation(contract_address: FixedSizeBinary<20>): XcmVersionedLocation {
  return XcmVersionedLocation.V4({
    parents: 2,
    interior: XcmV3Junctions.X2([
      XcmV3Junction.GlobalConsensus(XcmV3JunctionNetworkId.Ethereum({ chain_id: 1n })),
      XcmV3Junction.AccountKey20({ network: undefined, key: contract_address }),
    ]),
  });
}

export function AssetLocation(
  asset: Asset,
  assetSourceRelation: AssetSourceRelation,
): XcmVersionedLocation {
  const baseLocation =
    asset === Asset.WETH
      ? EthereumAssetLocation(FixedSizeBinary.fromHex(WETH_ADDRESS))
      : asset === Asset.DOT
        ? NativeAssetLocation(assetSourceRelation)
        : AssetHubAssetLocation(BigInt(asset), assetSourceRelation);

  return baseLocation;
}

export function getVersionedAssets(
  assets: [Asset, bigint, AssetSourceRelation][],
): XcmVersionedAssets {
  const final_assets: {
    id: { parents: number; interior: XcmV3Junctions };
    fun: XcmV3MultiassetFungibility;
  }[] = [];
  for (const [asset, amount, asset_source_relation] of assets) {
    const location = AssetLocation(asset, asset_source_relation);
    const id = {
      parents: location.value.parents,
      interior: location.value.interior as XcmV3Junctions, // We assume that this is not an XCM v2 MultiLocation.
    };
    final_assets.push({
      id,
      fun: XcmV3MultiassetFungibility.Fungible(amount),
    });
  }

  return XcmVersionedAssets.V4(final_assets);
}
