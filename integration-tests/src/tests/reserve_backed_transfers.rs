// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// The Polimec Blockchain is free software: you can redistribute it and/or modify
// it under the terms of the GNU General Public License as published by
// the Free Software Foundation, either version 3 of the License, or
// (at your option) any later version.

// The Polimec Blockchain is distributed in the hope that it will be useful,
// but WITHOUT ANY WARRANTY; without even the implied warranty of
// MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
// GNU General Public License for more details.

// You should have received a copy of the GNU General Public License
// along with this program.  If not, see <https://www.gnu.org/licenses/>.

use crate::*;
use frame_support::{
	traits::{
		fungibles::{Inspect, Mutate},
		PalletInfoAccess,
	},
	weights::WeightToFee,
};
use pallet_funding::types::AcceptedFundingAsset;
const RESERVE_TRANSFER_AMOUNT: u128 = 10_0_000_000_000; // 10 DOT
const MAX_REF_TIME: u64 = 5_000_000_000;
const MAX_PROOF_SIZE: u64 = 200_000;

fn create_usdt_on_asset_hub() {
	let usdt_admin_account = AssetHub::account_id_of(FERDIE);
	AssetHub::execute_with(|| {
		assert_ok!(AssetHubAssets::create(
			AssetHubOrigin::signed(usdt_admin_account.clone()),
			AcceptedFundingAsset::USDT.to_assethub_id().into(),
			sp_runtime::MultiAddress::Id(usdt_admin_account.clone()),
			0_0_010_000_000u128
		));
	});
}

fn mint_usdt_on_asset_hub_to(recipient: &AssetHubAccountId, amount: u128) {
	AssetHub::execute_with(|| {
		assert_ok!(AssetHubAssets::mint_into(AcceptedFundingAsset::USDT.to_assethub_id(), recipient, amount,));
	});
}

#[test]
fn reserve_to_polimec() {
	create_usdt_on_asset_hub();
	let usdt_on_asset_hub: MultiLocation = (
		PalletInstance(AssetHubAssets::index() as u8),
		GeneralIndex(AcceptedFundingAsset::USDT.to_assethub_id() as u128),
	)
		.into();

	let usdt_transfer: MultiAsset = (usdt_on_asset_hub, RESERVE_TRANSFER_AMOUNT).into();
	let alice_account = PolimecBase::account_id_of(ALICE.clone());
	let polimec_sibling_account =
		AssetHub::sovereign_account_id_of((Parent, Parachain(PolimecBase::para_id().into())).into());
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);

	mint_usdt_on_asset_hub_to(&alice_account, 100_0_000_000_000);

	// check Polimec's pre transfer balances and issuance
	let (
		polimec_prev_alice_usdt_balance,
		polimec_prev_alice_plmc_balance,
		polimec_prev_usdt_issuance,
		polimec_prev_plmc_issuance,
	) = PolimecBase::execute_with(|| {
		(
			BaseForeignAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
			BaseBalances::free_balance(alice_account.clone()),
			BaseForeignAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
			BaseBalances::total_issuance(),
		)
	});

	// check AssetHub's pre transfer balances and issuance
	let (asset_hub_prev_alice_usdt_balance, asset_hub_prev_polimec_usdt_balance, asset_hub_prev_usdt_issuance) =
		AssetHub::execute_with(|| {
			(
				AssetHubAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
				AssetHubAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), polimec_sibling_account.clone()),
				AssetHubAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
			)
		});

	// do the transfer
	AssetHub::execute_with(|| {
		let origin = AssetHubOrigin::signed(alice_account.clone());
		let dest: VersionedMultiLocation = ParentThen(X1(Parachain(PolimecBase::para_id().into()))).into();

		let beneficiary: VersionedMultiLocation = AccountId32 { network: None, id: alice_account.clone().into() }.into();
		let assets: VersionedMultiAssets = usdt_transfer.into();
		let fee_asset_item = 0;
		let weight_limit = Unlimited;

		let call = AssetHubXcmPallet::limited_reserve_transfer_assets(
			origin,
			bx!(dest),
			bx!(beneficiary),
			bx!(assets),
			fee_asset_item,
			weight_limit,
		);
		assert_ok!(call);
	});

	// check the transfer was not blocked by our our xcm configured
	PolimecBase::execute_with(|| {
		assert_expected_events!(
			PolimecBase,
			vec![
				BaseEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. }) => {},
			]
		);
	});

	// check Polimec's post transfer balances and issuance
	let (
		polimec_post_alice_usdt_balance,
		polimec_post_alice_plmc_balance,
		polimec_post_usdt_issuance,
		polimec_post_plmc_issuance,
	) = PolimecBase::execute_with(|| {
		(
			BaseForeignAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
			BaseBalances::free_balance(alice_account.clone()),
			BaseForeignAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
			BaseBalances::total_issuance(),
		)
	});

	// check AssetHub's post transfer balances and issuance
	let (asset_hub_post_alice_usdt_balance, asset_hub_post_polimec_usdt_balance, asset_hub_post_usdt_issuance) =
		AssetHub::execute_with(|| {
			(
				AssetHubAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
				AssetHubAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), polimec_sibling_account.clone()),
				AssetHubAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
			)
		});

	let polimec_delta_alice_usdt_balance = polimec_post_alice_usdt_balance - polimec_prev_alice_usdt_balance;
	let polimec_delta_usdt_issuance = polimec_post_usdt_issuance - polimec_prev_usdt_issuance;
	let polimec_delta_alice_plmc_balance = polimec_post_alice_plmc_balance - polimec_prev_alice_plmc_balance;
	let polimec_delta_plmc_issuance = polimec_post_plmc_issuance - polimec_prev_plmc_issuance;

	let asset_hub_delta_alice_usdt_balance = asset_hub_prev_alice_usdt_balance - asset_hub_post_alice_usdt_balance;
	let asset_hub_delta_polimec_usdt_balance =
		asset_hub_post_polimec_usdt_balance - asset_hub_prev_polimec_usdt_balance;
	let asset_hub_delta_usdt_issuance = asset_hub_prev_usdt_issuance - asset_hub_post_usdt_issuance;

	assert!(
	    polimec_delta_alice_usdt_balance >= RESERVE_TRANSFER_AMOUNT - polimec_parachain_runtime::WeightToFee::weight_to_fee(&max_weight) &&
	    polimec_delta_alice_usdt_balance <= RESERVE_TRANSFER_AMOUNT,
	    "Polimec alice_account.clone() USDT balance should have increased by at least the transfer amount minus the XCM execution fee"
	);

	assert!(
		polimec_delta_usdt_issuance >=
			RESERVE_TRANSFER_AMOUNT - polimec_parachain_runtime::WeightToFee::weight_to_fee(&max_weight) &&
			polimec_delta_usdt_issuance <= RESERVE_TRANSFER_AMOUNT,
		"Polimec USDT issuance should have increased by at least the transfer amount minus the XCM execution fee"
	);

	assert_eq!(
		asset_hub_delta_alice_usdt_balance, RESERVE_TRANSFER_AMOUNT,
		"AssetHub alice_account.clone() USDT balance should have decreased by the transfer amount"
	);

	assert!(
		asset_hub_delta_polimec_usdt_balance == RESERVE_TRANSFER_AMOUNT,
		"The USDT balance of Polimec's sovereign account on AssetHub should receive the transfer amount"
	);

	assert!(
		asset_hub_delta_usdt_issuance == 0u128,
		"AssetHub's USDT issuance should not change, since it acts as a reserve for that asset"
	);

	assert_eq!(
		polimec_delta_alice_plmc_balance, 0,
		"Polimec alice_account.clone() PLMC balance should not have changed"
	);

	assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec PLMC issuance should not have changed");
}

#[test]
fn polimec_to_reserve() {
	create_usdt_on_asset_hub();

	let usdt_on_asset_hub: MultiLocation = ParentThen(X3(
		Parachain(AssetHub::para_id().into()),
		PalletInstance(AssetHubAssets::index() as u8),
		GeneralIndex(AcceptedFundingAsset::USDT.to_assethub_id() as u128),
	))
	.into();
	let alice_account = PolimecBase::account_id_of(ALICE.clone());
	let polimec_sibling_account =
		AssetHub::sovereign_account_id_of((Parent, Parachain(PolimecBase::para_id().into())).into());
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);

	// Give some usdt to Polimec on AssetHub
	mint_usdt_on_asset_hub_to(&polimec_sibling_account, RESERVE_TRANSFER_AMOUNT * 3);
	// Represent that usdt just minted in Polimec, by minting to Alice
	PolimecBase::execute_with(|| {
		assert_ok!(BaseForeignAssets::mint_into(
			AcceptedFundingAsset::USDT.to_assethub_id(),
			&alice_account,
			RESERVE_TRANSFER_AMOUNT * 3
		));
		assert_ok!(BaseForeignAssets::mint_into(
			AcceptedFundingAsset::DOT.to_assethub_id(),
			&alice_account,
			RESERVE_TRANSFER_AMOUNT * 3
		));
	});

	// Check Polimec's pre transfer balances and issuance
	let (
		polimec_prev_alice_usdt_balance,
		polimec_prev_alice_plmc_balance,
		polimec_prev_usdt_issuance,
		polimec_prev_plmc_issuance,
	) = PolimecBase::execute_with(|| {
		(
			BaseForeignAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
			PolimecBalances::free_balance(alice_account.clone()),
			BaseForeignAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
			PolimecBalances::total_issuance(),
		)
	});

	// check AssetHub's pre transfer balances and issuance
	let (asset_hub_prev_alice_usdt_balance, asset_hub_prev_usdt_issuance) = AssetHub::execute_with(|| {
		(
			AssetHubAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
			AssetHubAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
		)
	});

	let usdt_plus_exec_fee: MultiAsset = (usdt_on_asset_hub, RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000).into();
	let dot_for_xcm_execution: MultiAsset = (PolimecBase::parent_location(), 1_0_000_000_000u128).into();

	// construct the XCM to transfer from Polimec to AssetHub's reserve
	let transfer_xcm: Xcm<BaseCall> = Xcm(vec![
		WithdrawAsset(vec![usdt_plus_exec_fee.clone(), dot_for_xcm_execution.clone()].into()),
		BuyExecution { fees: usdt_plus_exec_fee.clone(), weight_limit: Limited(max_weight) },
		InitiateReserveWithdraw {
			assets: All.into(),
			reserve: MultiLocation::new(1, X1(Parachain(AssetHub::para_id().into()))),
			xcm: Xcm(vec![
				BuyExecution { fees: dot_for_xcm_execution.clone(), weight_limit: Limited(max_weight) },
				DepositAsset {
					assets: All.into(),
					beneficiary: MultiLocation::new(0, AccountId32 { network: None, id: alice_account.clone().into() }),
				},
			]),
		},
	]);

	// do the transfer
	PolimecBase::execute_with(|| {
		assert_ok!(BaseXcmPallet::execute(
			BaseOrigin::signed(alice_account.clone()),
			Box::new(VersionedXcm::V3(transfer_xcm)),
			max_weight,
		));
	});

	// check that the xcm was not blocked
	AssetHub::execute_with(|| {
		assert_expected_events!(
			AssetHub,
			vec![
				AssetHubEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success {..}) => {},
			]
		);
	});

	// check Polimec's post transfer balances and issuance
	let (
		polimec_post_alice_usdt_balance,
		polimec_post_alice_plmc_balance,
		polimec_post_usdt_issuance,
		polimec_post_plmc_issuance,
	) = PolimecBase::execute_with(|| {
		(
			BaseForeignAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
			PolimecBalances::free_balance(alice_account.clone()),
			BaseForeignAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
			PolimecBalances::total_issuance(),
		)
	});

	// check AssetHub's post transfer balances and issuance
	let (asset_hub_post_alice_usdt_balance, asset_hub_post_usdt_issuance) = AssetHub::execute_with(|| {
		(
			AssetHubAssets::balance(AcceptedFundingAsset::USDT.to_assethub_id(), alice_account.clone()),
			AssetHubAssets::total_issuance(AcceptedFundingAsset::USDT.to_assethub_id()),
		)
	});

	let polimec_delta_usdt_issuance = polimec_prev_usdt_issuance - polimec_post_usdt_issuance;
	let polimec_delta_plmc_issuance = polimec_prev_plmc_issuance - polimec_post_plmc_issuance;
	let polimec_delta_alice_usdt_balance = polimec_prev_alice_usdt_balance - polimec_post_alice_usdt_balance;
	let polimec_delta_alice_plmc_balance = polimec_prev_alice_plmc_balance - polimec_post_alice_plmc_balance;

	let asset_hub_delta_usdt_issuance = asset_hub_prev_usdt_issuance - asset_hub_post_usdt_issuance;
	let asset_hub_delta_alice_usdt_balance = asset_hub_post_alice_usdt_balance - asset_hub_prev_alice_usdt_balance;

	assert_eq!(
		polimec_delta_alice_usdt_balance,
		RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
		"Polimec's alice_account.clone() DOT balance should decrease by the transfer amount"
	);

	assert_eq!(
		polimec_delta_usdt_issuance,
		RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
		"Polimec's DOT issuance should decrease by transfer amount due to burn"
	);

	assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec's PLMC issuance should not change, since all xcm token transfer are done in DOT, and no fees are burnt since no extrinsics are dispatched");
	assert_eq!(polimec_delta_alice_plmc_balance, 0, "Polimec's Alice PLMC should not change");

	dbg!(asset_hub_delta_alice_usdt_balance);
	assert!(
	    asset_hub_delta_alice_usdt_balance >=
	        RESERVE_TRANSFER_AMOUNT &&
	        asset_hub_delta_alice_usdt_balance <= RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
	    "AssetHub's alice_account DOT balance should increase by at least the transfer amount minus the max allowed fees"
	);

	assert!(
	    asset_hub_delta_usdt_issuance <= asset_hub_polkadot_runtime::constants::fee::WeightToFee::weight_to_fee(&max_weight),
	    "AssetHub's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
	);
}
