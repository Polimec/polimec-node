import {
  Accounts,
  Chains,
  type CreateAssetsParams,
  ParaId,
  type TransferDataParams,
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
  XcmVersionedAssets,
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

// TODO: Modify this function to allow the creation of an XcmVersionedAssets that supports also WETH/bridged assets.
const createHubAssets = ({
  amount,
  assetIndex,
  isFromBridge,
}: CreateAssetsParams): XcmVersionedAssets =>
  XcmVersionedAssets.V3([
    {
      fun: XcmV3MultiassetFungibility.Fungible(amount),
      id: XcmV3MultiassetAssetId.Concrete({
        parents: assetIndex ? 0 : 1,
        interior: assetIndex
          ? XcmV3Junctions.X2([
              XcmV3Junction.PalletInstance(50),
              XcmV3Junction.GeneralIndex(assetIndex),
            ])
          : XcmV3Junctions.Here(),
      }),
    },
  ]);

const createDotAssets = ({ amount }: CreateAssetsParams): XcmVersionedAssets =>
  XcmVersionedAssets.V3([
    {
      fun: XcmV3MultiassetFungibility.Fungible(amount),
      id: XcmV3MultiassetAssetId.Concrete({
        parents: 0,
        interior: XcmV3Junctions.Here(),
      }),
    },
  ]);

const createPolimecAssets = ({ amount, assetIndex }: CreateAssetsParams): XcmVersionedAssets => {
  if (!assetIndex) {
    throw new Error('You need to specify an Asset ID while creating an asset for Polimec');
  }
  return XcmVersionedAssets.V3([
    {
      id: XcmV3MultiassetAssetId.Concrete({
        parents: 1,
        interior:
          assetIndex === 10n
            ? XcmV3Junctions.Here()
            : XcmV3Junctions.X3([
                XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub]),
                XcmV3Junction.PalletInstance(50),
                XcmV3Junction.GeneralIndex(assetIndex),
              ]),
      }),
      fun: XcmV3MultiassetFungibility.Fungible(amount),
    },
  ]);
};

export const createTransferData = ({ amount, toChain, assetIndex, recv }: TransferDataParams) => {
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
    assets:
      toChain === Chains.PolkadotHub
        ? createPolimecAssets({ amount, assetIndex })
        : createHubAssets({ amount, assetIndex }),
    fee_asset_item: 0,
    weight_limit: XcmV3WeightLimit.Unlimited(),
  };
};

export const createMultiHopTransferData = ({ amount }: TransferDataParams) => {
  const dest = XcmVersionedLocation.V3({
    parents: 0,
    interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub])),
  });

  return {
    dest,
    assets: createDotAssets({ amount }),
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
