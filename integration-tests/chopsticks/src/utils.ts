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

export const createTransferData = ({ toChain, assets, recv }: TransferDataParams) => {
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
    fee_asset_item: 0,
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
    assets: getVersionedAssets([[Asset.DOT, amount]], AssetSourceRelation.Self),
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
  if (value === undefined) {
    throw new Error(errorMessage);
  }
  return value;
}
