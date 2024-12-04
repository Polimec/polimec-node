import { Accounts, Chains, ParaId } from '@/types';
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

// On Polkadot Hub, the XcmVersionedAssets could be DOT or an asset in the Pallet Assets.
const createHubAssets = (amount: bigint, assetIndex?: bigint): XcmVersionedAssets =>
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

// On Polkadot, the XcmVersionedAssets could only be the native currency, DOT.
const createPolkadotAssets = (amount: bigint): XcmVersionedAssets =>
  XcmVersionedAssets.V3([
    {
      fun: XcmV3MultiassetFungibility.Fungible(amount),
      id: XcmV3MultiassetAssetId.Concrete({
        parents: 0,
        interior: XcmV3Junctions.Here(),
      }),
    },
  ]);

// On Polimec, the XcmVersionedAssets is an asset in the Pallet Assets.
const createPolimecAssets = (amount: bigint, assetIndex = 1984n): XcmVersionedAssets =>
  XcmVersionedAssets.V3([
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

interface TransferDataParams {
  amount: bigint;
  toChain: Chains;
  assetIndex?: bigint;
  recv?: Accounts;
  isMultiHop?: boolean;
}

// Note: This should be used if the destination AND the soure is either Polimec or Polkadot Hub.
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
        ? createPolimecAssets(amount, assetIndex)
        : createHubAssets(amount, assetIndex),
    fee_asset_item: 0,
    weight_limit: XcmV3WeightLimit.Unlimited(),
  };
};

// Note: This should be used if the destination is Polimec and the source is Polkadot.
export const createMultiHopTransferData = ({ amount, toChain }: TransferDataParams) => {
  if (toChain === Chains.Polkadot) {
    throw new Error('The Multi Hop destination cannot be Polkadot');
  }
  const dest = XcmVersionedLocation.V3({
    parents: 0,
    interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub])),
  });
  return {
    dest,
    assets: createPolkadotAssets(amount),
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
