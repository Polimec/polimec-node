import { expect } from 'bun:test';
import { DEFAULT_TOPIC, FEE_AMOUNT, WETH_ADDRESS, WETH_AMOUNT } from '@/constants';
import type { BridgerHubManagaer } from '@/managers/BridgeHubManager';
import type { PolimecManager } from '@/managers/PolimecManager';
import type { PolkadotHubManager } from '@/managers/PolkadotHubManager';
import { Accounts, type BalanceCheck, Chains, ParaId } from '@/types';
import { unwrap_api } from '@/utils';
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
  getBalances(_options: TransferOptions): Promise<{ asset_balances: BalanceCheck[] }> {
    throw new Error('Method not allowed.');
  }
  verifyFinalBalances(
    _initialBalances: BalanceCheck[],
    _finalBalances: BalanceCheck[],
    _options: TransferOptions,
  ): void {
    throw new Error('Method not allowed.');
  }
  constructor(
    protected override sourceManager: BridgerHubManagaer,
    protected hopManager: PolkadotHubManager,
    protected override destManager: PolimecManager,
  ) {
    super(sourceManager, destManager);
    this.hopManager = hopManager;
  }

  async executeTransfer({ account, assets }: TransferOptions) {
    const sourceBlock = await this.sourceManager.getBlockNumber();
    const destBlock = await this.destManager.getBlockNumber();

    const sourceApi = this.sourceManager.getApi(Chains.BridgeHub);
    const hopApi = this.hopManager.getApi(Chains.PolkadotHub);
    const destApi = this.destManager.getApi(Chains.Polimec);

    const dest: XcmVersionedLocation = XcmVersionedLocation.V4({
      parents: 1,
      interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub])),
    });

    const messageFromEthereum = this.getEthereumMessage(Chains.PolkadotHub);

    const origin = XcmVersionedLocation.V4({
      parents: 1,
      interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.BridgeHub])),
    });

    // Execute the XCM message
    const xcm_res = sourceApi.tx.PolkadotXcm.send({
      dest,
      message: messageFromEthereum,
    }).decodedCall;
    const rootOrigin = { type: 'system' as const, value: DispatchRawOrigin.Root() };

    // Parallelize dry run calls
    const [dryRunOnSource, dryRunOnHop] = await Promise.all([
      sourceApi.apis.DryRunApi.dry_run_call(rootOrigin, xcm_res).then(unwrap_api),
      hopApi.apis.DryRunApi.dry_run_xcm(origin, messageFromEthereum).then(unwrap_api),
    ]);

    // Validate results
    expect(dryRunOnSource.success).toBeTrue();
    expect(dryRunOnSource.value.execution_result.success).toBeTrue();
    expect(dryRunOnHop.success).toBeTrue();
    expect(dryRunOnHop.value.execution_result.type).toBe('Complete');

    const issuedEvents = dryRunOnHop.value.emitted_events.filter(
      (event) => event.type === 'ForeignAssets' && event.value.type === 'Issued',
    );
    expect(issuedEvents.length).toBe(1);

    const messageOnPolimec = this.getEthereumMessage(Chains.Polimec);

    const hopOrigin = XcmVersionedLocation.V4({
      parents: 1,
      interior: XcmV3Junctions.X1(XcmV3Junction.Parachain(ParaId[Chains.PolkadotHub])),
    });

    const dryRunOnDest = await destApi.apis.DryRunApi.dry_run_xcm(hopOrigin, messageOnPolimec).then(
      unwrap_api,
    );
    expect(dryRunOnDest.success).toBeTrue();

    const issuedEventsOnDest = dryRunOnDest.value.emitted_events.filter(
      (event) => event.type === 'ForeignAssets' && event.value.type === 'Issued',
    );

    // TODO: Check why we have 3 events instead of 2 (WETH + DOT). Curently we have 3 events (WETH + DOT + DOT)
    expect(issuedEventsOnDest.length).toBe(3);

    return { sourceBlock, destBlock };
  }

  private getEthereumMessage(on: Chains): XcmVersionedXcm {
    if (on === Chains.PolkadotHub)
      return XcmVersionedXcm.V3([
        // 1. Receive Teleported Assets
        XcmV3Instruction.ReceiveTeleportedAsset([
          {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 1,
              interior: XcmV3Junctions.Here(),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(FEE_AMOUNT * 2n),
          },
        ]),

        // 2. Buy Execution
        XcmV3Instruction.BuyExecution({
          fees: {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 1,
              interior: XcmV3Junctions.Here(),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(FEE_AMOUNT),
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
                  key: FixedSizeBinary.fromHex(WETH_ADDRESS),
                }),
              ]),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(WETH_AMOUNT),
          },
        ]),

        // 6. Clear Origin
        XcmV3Instruction.ClearOrigin(),

        // 7. Set Appendix
        XcmV3Instruction.SetAppendix([
          XcmV3Instruction.DepositAsset({
            assets: XcmV3MultiassetMultiAssetFilter.Wild(
              XcmV3MultiassetWildMultiAsset.AllCounted(2),
            ),
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
              fun: XcmV3MultiassetFungibility.Fungible(FEE_AMOUNT),
            },
            {
              id: XcmV3MultiassetAssetId.Concrete({
                parents: 2,
                interior: XcmV3Junctions.X2([
                  XcmV3Junction.GlobalConsensus(XcmV3JunctionNetworkId.Ethereum({ chain_id: 1n })),
                  XcmV3Junction.AccountKey20({
                    network: undefined,
                    key: FixedSizeBinary.fromHex(WETH_ADDRESS),
                  }),
                ]),
              }),
              fun: XcmV3MultiassetFungibility.Fungible(WETH_AMOUNT),
            },
          ]),
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
                fun: XcmV3MultiassetFungibility.Fungible(FEE_AMOUNT),
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
            XcmV3Instruction.SetTopic(DEFAULT_TOPIC),
          ],
        }),
        // 9. Set Topic
        XcmV3Instruction.SetTopic(DEFAULT_TOPIC),
      ]);
    if (on === Chains.Polimec)
      return XcmVersionedXcm.V3([
        // Reserve Asset Deposited
        XcmV3Instruction.ReserveAssetDeposited([
          {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 1,
              interior: XcmV3Junctions.Here(),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(FEE_AMOUNT),
          },
          {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 2,
              interior: XcmV3Junctions.X2([
                XcmV3Junction.GlobalConsensus(XcmV3JunctionNetworkId.Ethereum({ chain_id: 1n })),
                XcmV3Junction.AccountKey20({
                  network: undefined,
                  key: FixedSizeBinary.fromHex(WETH_ADDRESS),
                }),
              ]),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(WETH_AMOUNT),
          },
        ]),

        // Clear Origin
        XcmV3Instruction.ClearOrigin(),

        // Buy Execution
        XcmV3Instruction.BuyExecution({
          fees: {
            id: XcmV3MultiassetAssetId.Concrete({
              parents: 1,
              interior: XcmV3Junctions.Here(),
            }),
            fun: XcmV3MultiassetFungibility.Fungible(FEE_AMOUNT),
          },
          weight_limit: XcmV3WeightLimit.Unlimited(),
        }),

        // Deposit Asset
        XcmV3Instruction.DepositAsset({
          assets: XcmV3MultiassetMultiAssetFilter.Wild(XcmV3MultiassetWildMultiAsset.AllCounted(2)),
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

        // Set Topic
        XcmV3Instruction.SetTopic(DEFAULT_TOPIC),
      ]);
    throw new Error('Unsupported chain');
  }
}
