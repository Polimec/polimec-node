// import { afterAll, beforeAll, beforeEach, describe, test } from 'bun:test';
// import { TRANSFER_AMOUNTS } from '@/constants';
// import { createChainManager } from '@/managers/Factory';
// import { polimec_storage } from '@/polimec';
// import { ChainSetup } from '@/setup';
// import { PolimecToHubTransfer } from '@/transfers/PolimecToHub';
// import { Accounts, Asset, AssetSourceRelation, Chains } from '@/types';

// describe('Polimec -> Hub Transfer Tests', () => {
//   const sourceManager = createChainManager(Chains.Polimec);
//   const destManager = createChainManager(Chains.PolkadotHub);
//   const transferTest = new PolimecToHubTransfer(sourceManager, destManager);
//   const chainSetup = new ChainSetup();

//   beforeAll(async () => await chainSetup.initialize(polimec_storage));
//   beforeEach(() => {
//     sourceManager.connect();
//     destManager.connect();
//   });
//   afterAll(async () => await chainSetup.cleanup());

//   async function getBalance(account: Accounts, asset: Asset) {
//     return await sourceManager.getAssetBalanceOf(account, asset);
//   }
//   test('Balance query', () => getBalance(Accounts.BOB, Asset.USDT), { timeout: 250000000 });

//   test(
//     'Send USDC to Hub',
//     () =>
//       transferTest.testTransfer({
//         amount: TRANSFER_AMOUNTS.TOKENS,
//         account: Accounts.BOB,
//         asset: Asset.USDC,
//         assetSourceRelation: AssetSourceRelation.Sibling,
//       }),
//     { timeout: 25000 },
//   );

//   test(
//     'Send USDT to Hub',
//     () =>
//       transferTest.testTransfer({
//         amount: TRANSFER_AMOUNTS.TOKENS,
//         account: Accounts.BOB,
//         asset: Asset.USDT,
//         assetSourceRelation: AssetSourceRelation.Sibling,
//       }),
//     { timeout: 25000 },
//   );

//   test(
//     'Send DOT to Hub',
//     () =>
//       transferTest.testTransfer({
//         amount: TRANSFER_AMOUNTS.NATIVE,
//         account: Accounts.BOB,
//         asset: Asset.DOT,
//         assetSourceRelation: AssetSourceRelation.Parent,
//       }),
//     { timeout: 25000 },
//   );
// });
