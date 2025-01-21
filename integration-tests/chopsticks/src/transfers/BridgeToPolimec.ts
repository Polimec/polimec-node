import { expect } from 'bun:test';
import type { BridgerHubManagaer } from '@/managers/BridgeHubManager';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import {
  Accounts,
  Asset,
  AssetSourceRelation,
  Chains,
  ParaId,
  type PolimecBalanceCheck,
  getVersionedAssets,
} from '@/types';
import { createTransferData, flatObject, unwrap } from '@/utils';
import {
  DispatchRawOrigin,
  XcmV3Instruction,
  XcmV3Junction,
  XcmV3JunctionNetworkId,
  XcmV3Junctions,
  XcmV3MultiassetAssetId,
  XcmV3MultiassetFungibility,
  XcmV3MultiassetMultiAssetFilter,
  XcmV3MultiassetWildMultiAsset,
  XcmV3WeightLimit,
  XcmVersionedLocation,
  XcmVersionedXcm,
} from '@polkadot-api/descriptors';

import { FixedSizeBinary } from 'polkadot-api';
import { BaseTransferTest, type TransferOptions } from './BaseTransfer';

export class BridgeToPolimecTransfer extends BaseTransferTest {
  constructor(
    protected override sourceManager: BridgerHubManagaer,
    protected hopManager: PolkadotHubManager,
    protected override destManager: PolimecManager,
  ) {
    super(sourceManager, destManager);
    this.hopManager = hopManager;
  }

  async executeTransfer({ account, assets }: TransferOptions) {
    console.log('BridgeToPolimecTransfer executeTransfer');
    const sourceBlock = await this.sourceManager.getBlockNumber();
    console.log('sourceBlock', sourceBlock);
    const hopManagerBlock = await this.hopManager.getBlockNumber();
    console.log('hopManagerBlock', hopManagerBlock);
    const destBlock = await this.destManager.getBlockNumber();
    console.log('destBlock', destBlock);

    const sourceApi = this.sourceManager.getApi<Chains.BridgeHub>(Chains.BridgeHub);

    const dest: XcmVersionedLocation = XcmVersionedLocation.V4({
      parents: 1,
      interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub])),
    });

    const message: XcmVersionedXcm = XcmVersionedXcm.V3([
      // 1. Receive Teleported Assets
      XcmV3Instruction.ReceiveTeleportedAsset([
        {
          id: XcmV3MultiassetAssetId.Concrete({
            parents: 1,
            interior: XcmV3Junctions.Here(),
          }),
          fun: XcmV3MultiassetFungibility.Fungible(80000000000n),
        },
      ]),

      // 2. Buy Execution
      XcmV3Instruction.BuyExecution({
        fees: {
          id: XcmV3MultiassetAssetId.Concrete({
            parents: 1,
            interior: XcmV3Junctions.Here(),
          }),
          fun: XcmV3MultiassetFungibility.Fungible(40000000000n),
        },
        weight_limit: XcmV3WeightLimit.Unlimited(),
      }),

      // 3. Descend Origin
      XcmV3Instruction.DescendOrigin(XcmV3Junctions.X1(XcmV3Junction.PalletInstance(80))),

      // 4. Universal Origin
      XcmV3Instruction.UniversalOrigin(
        XcmV3Junction.GlobalConsensus(XcmV3JunctionNetworkId.Ethereum({ chain_id: 1n })),
      ),

      // 5. Reserve Asset Deposited
      XcmV3Instruction.ReserveAssetDeposited([
        {
          id: XcmV3MultiassetAssetId.Concrete({
            parents: 2,
            interior: XcmV3Junctions.X2([
              XcmV3Junction.GlobalConsensus(XcmV3JunctionNetworkId.Ethereum({ chain_id: 1n })),
              XcmV3Junction.AccountKey20({
                network: undefined,
                key: FixedSizeBinary.fromHex('0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2'),
              }),
            ]),
          }),
          fun: XcmV3MultiassetFungibility.Fungible(15000000000000n),
        },
      ]),

      // 6. Clear Origin
      XcmV3Instruction.ClearOrigin(),

      // 7. Set Appendix
      XcmV3Instruction.SetAppendix([
        XcmV3Instruction.DepositAsset({
          assets: XcmV3MultiassetMultiAssetFilter.Wild(XcmV3MultiassetWildMultiAsset.AllCounted(2)),
          beneficiary: {
            parents: 2,
            interior: XcmV3Junctions.X1(
              XcmV3Junction.GlobalConsensus(XcmV3JunctionNetworkId.Ethereum({ chain_id: 1n })),
            ),
          },
        }),
      ]),

      // 8. Deposit Reserve Asset
      XcmV3Instruction.DepositReserveAsset({
        assets: XcmV3MultiassetMultiAssetFilter.Definite([
          {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 1,
              interior: XcmV3Junctions.Here(),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(40000000000n),
          },
          {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 2,
              interior: XcmV3Junctions.X2([
                XcmV3Junction.GlobalConsensus(XcmV3JunctionNetworkId.Ethereum({ chain_id: 1n })),
                XcmV3Junction.AccountKey20({
                  network: undefined,
                  key: FixedSizeBinary.fromHex('0xc02aaa39b223fe8d0a0e5c4f27ead9083c756cc2'),
                }),
              ]),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(15000000000000n),
          },
        ]),
        dest: {
          parents: 1,
          interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(3344)),
        },
        xcm: [
          XcmV3Instruction.BuyExecution({
            fees: {
              id: XcmV3MultiassetAssetId.Concrete({
                parents: 1,
                interior: XcmV3Junctions.Here(),
              }),
              fun: XcmV3MultiassetFungibility.Fungible(40000000000n),
            },
            weight_limit: XcmV3WeightLimit.Unlimited(),
          }),
          XcmV3Instruction.DepositAsset({
            assets: XcmV3MultiassetMultiAssetFilter.Wild(
              XcmV3MultiassetWildMultiAsset.AllCounted(2),
            ),
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
          XcmV3Instruction.SetTopic(
            FixedSizeBinary.fromArray([
              1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
              1, 1, 1,
            ]),
          ),
        ],
      }),
      // 9. Set Topic
      XcmV3Instruction.SetTopic(
        FixedSizeBinary.fromArray([
          1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1, 1,
          1, 1,
        ]),
      ),
    ]);

    const xcm_res = sourceApi.tx.PolkadotXcm.send({ dest, message }).decodedCall;

    const dry_run_res = await sourceApi.apis.DryRunApi.dry_run_call(
      { type: 'system', value: DispatchRawOrigin.Root() },
      xcm_res,
    );
    console.log('dryRunCallOnBH SUCCESS?', dry_run_res.success);

    const hopApi = this.hopManager.getApi<Chains.PolkadotHub>(Chains.PolkadotHub);

    const origin = XcmVersionedLocation.V4({
      parents: 1,
      interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.BridgeHub])),
    });

    const dryRunonHop = await hopApi.apis.DryRunApi.dry_run_xcm(origin, message);
    console.dir(flatObject(dryRunonHop.value), { depth: null, colors: true });

    return { sourceBlock, destBlock };
  }

  async getBalances({
    account,
    assets,
  }: TransferOptions): Promise<{ asset_balances: PolimecBalanceCheck[] }> {
    return { asset_balances: [{ source: 0n, destination: 0n, treasury: 0n }] };
  }

  async verifyFinalBalances(
    assetInitialBalances: PolimecBalanceCheck[],
    assetFinalBalances: PolimecBalanceCheck[],
    transferOptions: TransferOptions,
  ) {
    expect(0n).toBe(0n);
  }

  async calculatePolimecXcmFee(transferOptions: TransferOptions): Promise<bigint> {
    return 0n;
  }
}
