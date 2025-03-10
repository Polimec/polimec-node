import {
  Accounts,
  Asset,
  AssetSourceRelation,
  Chains,
  ParaId,
  type TransferDataParams,
  getVersionedAssets,
} from '@/types';
import {
  XcmV3Instruction,
  XcmV3Junction,
  XcmV3Junctions,
  XcmV3MultiassetAssetId,
  XcmV3MultiassetFungibility,
  XcmV3MultiassetMultiAssetFilter,
  XcmV3MultiassetWildMultiAsset,
  XcmV3WeightLimit,
  XcmVersionedAssetId,
  XcmVersionedLocation,
  XcmVersionedXcm,
} from '@polkadot-api/descriptors';
import type { I5gi8h3e5lkbeq } from '@polkadot-api/descriptors/dist/common-types';
import { Enum, FixedSizeBinary } from 'polkadot-api';
const custom_xcm_on_dest = (): XcmVersionedXcm => {
  return XcmVersionedXcm.V3([
    XcmV3Instruction.DepositReserveAsset({
      assets: XcmV3MultiassetMultiAssetFilter.Wild(XcmV3MultiassetWildMultiAsset.AllCounted(1)),
      dest: {
        parents: 1,
        interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.Polimec])),
      },
      xcm: [
        XcmV3Instruction.BuyExecution({
          fees: {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 1,
              interior: XcmV3Junctions.Here(),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(1_000_000_000n),
          },
          weight_limit: XcmV3WeightLimit.Unlimited(),
        }),
        XcmV3Instruction.DepositAsset({
          assets: XcmV3MultiassetMultiAssetFilter.Wild(XcmV3MultiassetWildMultiAsset.AllCounted(1)),
          beneficiary: {
            parents: 0,
            interior: XcmV3Junctions.X1(
              XcmV3Junction.AccountId32({
                network: undefined,
                id: FixedSizeBinary.fromAccountId32(Accounts.ALICE),
              }),
            ),
          },
        }),
      ],
    }),
  ]);
};

export const createTransferData = ({
  toChain,
  assets,
  recv,
  fee_asset_item,
}: TransferDataParams): I5gi8h3e5lkbeq => {
  if (toChain === Chains.Polkadot) {
    throw new Error('Invalid chain');
  }
  const dest = XcmVersionedLocation.V3({
    parents: 1,
    interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[toChain])),
  });

  const beneficiary = XcmVersionedLocation.V3({
    parents: 0,
    interior: XcmV3Junctions.X1(
      XcmV3Junction.AccountId32({
        network: undefined,
        id: FixedSizeBinary.fromAccountId32(recv || Accounts.ALICE),
      }),
    ),
  });

  return {
    dest,
    beneficiary,
    assets,
    fee_asset_item,
    weight_limit: XcmV3WeightLimit.Unlimited(),
  };
};

export const createDotMultiHopTransferData = (amount: bigint) => {
  const dest = XcmVersionedLocation.V3({
    parents: 0,
    interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub])),
  });

  return {
    dest,
    assets: getVersionedAssets([[Asset.DOT, amount, AssetSourceRelation.Self]]),
    assets_transfer_type: Enum('Teleport'),
    remote_fees_id: XcmVersionedAssetId.V3(
      XcmV3MultiassetAssetId.Concrete({
        parents: 0,
        interior: XcmV3Junctions.Here(),
      }),
    ),
    fees_transfer_type: Enum('Teleport'),
    custom_xcm_on_dest: custom_xcm_on_dest(),
    weight_limit: XcmV3WeightLimit.Unlimited(),
  };
};

export function unwrap<T>(value: T | undefined, errorMessage = 'Value is undefined'): T {
  if (value === undefined) throw new Error(errorMessage);

  return value;
}

export function unwrap_api<T extends { success: boolean }>(
  value: T,
  errorMessage = 'Value is undefined',
): T & { success: true } {
  if (value === undefined) throw new Error(errorMessage);
  if (value === null) throw new Error(errorMessage);
  if (!value.success) throw new Error('Dry run failed');
  return value as T & { success: true };
}

export function flatObject(obj: unknown): unknown {
  if (obj === null || obj === undefined) {
    return obj;
  }
  if (obj instanceof Object && typeof (obj as { asHex?: unknown }).asHex === 'function') {
    return (obj as { asHex: () => unknown }).asHex();
  }
  if (typeof obj === 'object') {
    if (Array.isArray(obj)) {
      return obj.map(flatObject);
    }
    const normalized: Record<string, unknown> = {};
    for (const [key, value] of Object.entries(obj)) {
      normalized[key] = flatObject(value);
    }
    return normalized;
  }
  return obj;
}
export const abs = (n: bigint) => (n < 0n ? -n : n);
