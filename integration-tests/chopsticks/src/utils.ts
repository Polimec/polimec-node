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
import { Accounts } from './types';

/**
 * Helper to create XCM assets.
 * @param amount - The asset amount as bigint.
 * @param assetIndex - The asset id.
 */
const createAssets = (amount: bigint, assetIndex?: bigint): XcmVersionedAssets => ({
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

/**
 * Creates transfer data for XCM calls.
 * @param amount - The amount to transfer as bigint.
 * @param assetIndex - Optional asset index for multi-assets.
 */
export const createTransferData = (amount: bigint, assetIndex?: bigint) => {
  const dest: XcmVersionedLocation = {
    type: 'V3',
    value: {
      parents: 1,
      interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(3344)),
    },
  };

  const beneficiary: XcmVersionedLocation = {
    type: 'V3',
    value: {
      parents: 0,
      interior: XcmV3Junctions.X1(
        XcmV3Junction.AccountId32({
          network: undefined,
          id: FixedSizeBinary.fromAccountId32(Accounts.ALICE),
        }),
      ),
    },
  };

  return {
    dest,
    beneficiary,
    assets: createAssets(amount, assetIndex),
    fee_asset_item: 0,
    weight_limit: XcmV3WeightLimit.Unlimited(),
  };
};
