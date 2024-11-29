import { Accounts, Chains, ParaId, type Parachain } from '@/types';
import {
  XcmV3Junction,
  XcmV3Junctions,
  XcmV3MultiassetAssetId,
  XcmV3MultiassetFungibility,
  XcmV3WeightLimit,
  type XcmVersionedAssets,
  type XcmVersionedLocation,
} from '@polkadot-api/descriptors';
import { FixedSizeBinary } from 'polkadot-api';

/**
 * Helper to create XCM assets.
 * @param amount - The asset amount as bigint.
 * @param assetIndex - The asset id.
 */
const createHubAssets = (amount: bigint, assetIndex?: bigint): XcmVersionedAssets => ({
  type: 'V3',
  value: [
    {
      id: XcmV3MultiassetAssetId.Concrete({
        parents: assetIndex ? 0 : 1,
        interior: assetIndex
          ? XcmV3Junctions.X2([
              XcmV3Junction.PalletInstance(50),
              XcmV3Junction.GeneralIndex(assetIndex),
            ])
          : XcmV3Junctions.Here(),
      }),
      fun: XcmV3MultiassetFungibility.Fungible(amount),
    },
  ],
});

const createPolimecAssets = (amount: bigint, assetIndex = 1984n): XcmVersionedAssets => ({
  type: 'V3',
  value: [
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
  ],
});

/**
 * Creates transfer data for XCM calls.
 * @param amount - The amount to transfer as bigint.
 * @param assetIndex - Optional asset index for multi-assets.
 */
export const createTransferData = (
  amount: bigint,
  toChain: Parachain,
  assetIndex?: bigint,
  recv?: Accounts,
) => {
  const dest: XcmVersionedLocation = {
    type: 'V3',
    value: {
      parents: 1,
      interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[toChain])),
    },
  };

  const beneficiary: XcmVersionedLocation = {
    type: 'V3',
    value: {
      parents: 0,
      interior: XcmV3Junctions.X1(
        XcmV3Junction.AccountId32({
          network: undefined,
          id: FixedSizeBinary.fromAccountId32(recv || Accounts.ALICE),
        }),
      ),
    },
  };

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
