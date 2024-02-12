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
		fungible::{Inspect as FungibleInspect, Unbalanced},
		PalletInfoAccess,
	},
	weights::WeightToFee,
};
use sp_runtime::DispatchError;

const RESERVE_TRANSFER_AMOUNT: u128 = 10_0_000_000_000; // 10 DOT
const MAX_REF_TIME: u64 = 5_000_000_000;
const MAX_PROOF_SIZE: u64 = 200_000;

fn create_asset_on_asset_hub(asset_id: u32) {
	if asset_id == 0 {
		return;
	}
	let usdt_admin_account = AssetHub::account_id_of(FERDIE);
	AssetHub::execute_with(|| {
		assert_ok!(AssetHubAssets::force_create(
			AssetHubOrigin::root(),
			asset_id.into(),
			sp_runtime::MultiAddress::Id(usdt_admin_account.clone()),
			true,
			0_0_010_000_000u128
		));
	});
}

fn mint_asset_on_asset_hub_to(asset_id: u32, recipient: &AssetHubAccountId, amount: u128) {
	AssetHub::execute_with(|| {
		match asset_id {
			0 => {
				assert_ok!(AssetHubBalances::write_balance(recipient, amount));
			}
			_ => {
				assert_ok!(AssetHubAssets::mint_into(asset_id, recipient, amount));
			}
		}
		AssetHubSystem::reset_events();
	});
}

fn get_polimec_balances(asset_id: u32, user_account: AccountId) -> (u128, u128, u128, u128) {
	PolimecBase::execute_with(|| {
		(
			BaseForeignAssets::balance(asset_id, user_account.clone()),
			BaseBalances::free_balance(user_account.clone()),
			BaseForeignAssets::total_issuance(asset_id),
			BaseBalances::total_issuance(),
		)
	})
}

fn get_asset_hub_balances(asset_id: u32, user_account: AccountId, polimec_account: AccountId) -> (u128, u128, u128) {
	AssetHub::execute_with(|| {
		match asset_id {
			// Asset id 0 equals Dot
			0 => (
				AssetHubBalances::balance(&user_account),
				AssetHubBalances::balance(&polimec_account),
				AssetHubBalances::total_issuance(),
			),
			_ => (
				AssetHubAssets::balance(asset_id, user_account.clone()),
				AssetHubAssets::balance(asset_id, polimec_account.clone()),
				AssetHubAssets::total_issuance(asset_id),
			),
		}
	})
}

/// Test the reserve based transfer from asset_hub to Polimec. Depending of the asset_id we
/// transfer either USDT, USDC and DOT.
fn test_reserve_to_polimec(asset_id: u32) {
	create_asset_on_asset_hub(asset_id);
	let asset_hub_asset_id: MultiLocation = match asset_id {
		0 => Parent.into(),
		_ => (PalletInstance(AssetHubAssets::index() as u8), GeneralIndex(asset_id as u128)).into()
	};

	let alice_account = PolimecBase::account_id_of(ALICE.clone());
	let polimec_sibling_account =
		AssetHub::sovereign_account_id_of((Parent, Parachain(PolimecBase::para_id().into())).into());
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);

	mint_asset_on_asset_hub_to(asset_id, &alice_account, 100_0_000_000_000);

	let (
		polimec_prev_alice_asset_balance,
		polimec_prev_alice_plmc_balance,
		polimec_prev_asset_issuance,
		polimec_prev_plmc_issuance,
	) = get_polimec_balances(asset_id, alice_account.clone());

	// check AssetHub's pre transfer balances and issuance
	let (asset_hub_prev_alice_asset_balance, asset_hub_prev_polimec_asset_balance, asset_hub_prev_asset_issuance) =
		get_asset_hub_balances(asset_id, alice_account.clone(), polimec_sibling_account.clone());
		

	AssetHub::execute_with(|| {
		let asset_transfer: MultiAsset = (asset_hub_asset_id, RESERVE_TRANSFER_AMOUNT).into();
		let origin = AssetHubOrigin::signed(alice_account.clone());
		let dest: VersionedMultiLocation = ParentThen(X1(Parachain(PolimecBase::para_id().into()))).into();

		let beneficiary: VersionedMultiLocation = AccountId32 { network: None, id: alice_account.clone().into() }.into();
		let assets: VersionedMultiAssets = asset_transfer.into();
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

	
	let (
		polimec_post_alice_asset_balance,
		polimec_post_alice_plmc_balance,
		polimec_post_asset_issuance,
		polimec_post_plmc_issuance,
	) = get_polimec_balances(asset_id, alice_account.clone());

	let (asset_hub_post_alice_asset_balance, asset_hub_post_polimec_asset_balance, asset_hub_post_asset_issuance) =
		get_asset_hub_balances(asset_id, alice_account.clone(), polimec_sibling_account.clone());

	
	let polimec_delta_alice_asset_balance = polimec_post_alice_asset_balance.abs_diff(polimec_prev_alice_asset_balance);
	let polimec_delta_alice_plmc_balance = polimec_post_alice_plmc_balance.abs_diff(polimec_prev_alice_plmc_balance);
	let polimec_delta_asset_issuance = polimec_post_asset_issuance.abs_diff(polimec_prev_asset_issuance);
	let polimec_delta_plmc_issuance = polimec_post_plmc_issuance.abs_diff(polimec_prev_plmc_issuance);
	let asset_hub_delta_alice_asset_balance = asset_hub_post_alice_asset_balance.abs_diff(asset_hub_prev_alice_asset_balance);
	let asset_hub_delta_polimec_asset_balance = asset_hub_post_polimec_asset_balance.abs_diff(asset_hub_prev_polimec_asset_balance);
	let asset_hub_delta_asset_issuance = asset_hub_post_asset_issuance.abs_diff(asset_hub_prev_asset_issuance);

	assert!(
	    polimec_delta_alice_asset_balance >= RESERVE_TRANSFER_AMOUNT - polimec_parachain_runtime::WeightToFee::weight_to_fee(&max_weight) &&
	    polimec_delta_alice_asset_balance <= RESERVE_TRANSFER_AMOUNT,
	    "Polimec alice_account.clone() Asset balance should have increased by at least the transfer amount minus the XCM execution fee"
	);

	assert!(
		polimec_delta_asset_issuance >=
			RESERVE_TRANSFER_AMOUNT - polimec_parachain_runtime::WeightToFee::weight_to_fee(&max_weight) &&
			polimec_delta_asset_issuance <= RESERVE_TRANSFER_AMOUNT,
		"Polimec Asset issuance should have increased by at least the transfer amount minus the XCM execution fee"
	);

	assert_eq!(
		asset_hub_delta_alice_asset_balance, RESERVE_TRANSFER_AMOUNT,
		"AssetHub alice_account.clone() Asset balance should have decreased by the transfer amount"
	);

	assert!(
		asset_hub_delta_polimec_asset_balance == RESERVE_TRANSFER_AMOUNT,
		"The USDT balance of Polimec's sovereign account on AssetHub should receive the transfer amount"
	);

	assert!(
		asset_hub_delta_asset_issuance == 0u128,
		"AssetHub's USDT issuance should not change, since it acts as a reserve for that asset"
	);

	assert_eq!(
		polimec_delta_alice_plmc_balance, 0,
		"Polimec alice_account.clone() PLMC balance should not have changed"
	);

	assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec PLMC issuance should not have changed");
}

fn test_polimec_to_reserve(asset_id: u32) {
	create_asset_on_asset_hub(asset_id);
	let asset_hub_asset_id: MultiLocation = match asset_id {
		0 => Parent.into(),
		_ => ParentThen(X3(Parachain(AssetHub::para_id().into()), PalletInstance(AssetHubAssets::index() as u8), GeneralIndex(asset_id as u128))).into()
	};

	let alice_account = PolimecBase::account_id_of(ALICE.clone());
	let polimec_sibling_account =
		AssetHub::sovereign_account_id_of((Parent, Parachain(PolimecBase::para_id().into())).into());
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);

	mint_asset_on_asset_hub_to(asset_id, &polimec_sibling_account, RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000);

	PolimecBase::execute_with(|| {
		assert_ok!(BaseForeignAssets::mint_into(
			asset_id,
			&alice_account,
			RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000
		));
	});

	let (
		polimec_prev_alice_asset_balance,
		polimec_prev_alice_plmc_balance,
		polimec_prev_asset_issuance,
		polimec_prev_plmc_issuance,
	) = get_polimec_balances(asset_id, alice_account.clone());

	// check AssetHub's pre transfer balances and issuance
	let (asset_hub_prev_alice_asset_balance, asset_hub_prev_polimec_asset_balance, asset_hub_prev_asset_issuance) =
		get_asset_hub_balances(asset_id, alice_account.clone(), polimec_sibling_account.clone());

	let transferable_asset_plus_exec_fee: MultiAsset = (asset_hub_asset_id, RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000).into();
	let mut asset_hub_exec_fee: MultiAsset = (asset_hub_asset_id.clone(), 1_0_000_000_000u128).into();
	asset_hub_exec_fee.reanchor(
		&(ParentThen(X1(Parachain(AssetHub::para_id().into()))).into()),
		Here
	).unwrap();
	// let dot_for_xcm_execution: MultiAsset = (PolimecBase::parent_location(), 1_0_000_000_000u128).into();

	// construct the XCM to transfer from Polimec to AssetHub's reserve
	let transfer_xcm: Xcm<BaseCall> = Xcm(vec![
		WithdrawAsset(transferable_asset_plus_exec_fee.clone().into()),
		BuyExecution { fees: transferable_asset_plus_exec_fee.clone(), weight_limit: Limited(max_weight) },
		InitiateReserveWithdraw {
			assets: All.into(),
			reserve: MultiLocation::new(1, X1(Parachain(AssetHub::para_id().into()))),
			xcm: Xcm(vec![
				BuyExecution { fees: asset_hub_exec_fee, weight_limit: Limited(max_weight) },
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
	
	let (
		polimec_post_alice_asset_balance,
		polimec_post_alice_plmc_balance,
		polimec_post_asset_issuance,
		polimec_post_plmc_issuance,
	) = get_polimec_balances(asset_id, alice_account.clone());

	let (asset_hub_post_alice_asset_balance, asset_hub_post_polimec_asset_balance, asset_hub_post_asset_issuance) =
		get_asset_hub_balances(asset_id, alice_account.clone(), polimec_sibling_account.clone());

	
	let polimec_delta_alice_asset_balance = polimec_post_alice_asset_balance.abs_diff(polimec_prev_alice_asset_balance);
	let polimec_delta_alice_plmc_balance = polimec_post_alice_plmc_balance.abs_diff(polimec_prev_alice_plmc_balance);
	let polimec_delta_asset_issuance = polimec_post_asset_issuance.abs_diff(polimec_prev_asset_issuance);
	let polimec_delta_plmc_issuance = polimec_post_plmc_issuance.abs_diff(polimec_prev_plmc_issuance);
	let asset_hub_delta_alice_asset_balance = asset_hub_post_alice_asset_balance.abs_diff(asset_hub_prev_alice_asset_balance);
	let asset_hub_delta_polimec_asset_balance = asset_hub_post_polimec_asset_balance.abs_diff(asset_hub_prev_polimec_asset_balance);
	let asset_hub_delta_asset_issuance = asset_hub_post_asset_issuance.abs_diff(asset_hub_prev_asset_issuance);

	assert_eq!(
		polimec_delta_alice_asset_balance,
		RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
		"Polimec's alice_account Asset balance should decrease by the transfer amount"
	);

	assert_eq!(
		polimec_delta_asset_issuance,
		RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
		"Polimec's Asset issuance should decrease by transfer amount due to burn"
	);

	assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec's PLMC issuance should not change, since all xcm token transfer are done in Asset, and no fees are burnt since no extrinsics are dispatched");
	assert_eq!(polimec_delta_alice_plmc_balance, 0, "Polimec's Alice PLMC should not change");

	assert!(
	    asset_hub_delta_alice_asset_balance >=
	        RESERVE_TRANSFER_AMOUNT &&
	        asset_hub_delta_alice_asset_balance <= RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
	    "AssetHub's alice_account Asset balance should increase by at least the transfer amount minus the max allowed fees"
	);

	assert!(
	    asset_hub_delta_polimec_asset_balance >=
	        RESERVE_TRANSFER_AMOUNT &&
	        asset_hub_delta_polimec_asset_balance <= RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
	    "Polimecs sovereign account on asset hub should have transferred Asset amount to Alice"
	);

	assert!(
	    asset_hub_delta_asset_issuance <= asset_hub_polkadot_runtime::constants::fee::WeightToFee::weight_to_fee(&max_weight),
	    "AssetHub's Asset issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
	);
}

/// Test reserve based transfer of USDT from AssetHub to Polimec.
#[test]
fn reserve_usdt_to_polimec() {
	let asset_id = 1984;
	test_reserve_to_polimec(asset_id);
}

/// Test reserve based transfer of USDC from AssetHub to Polimec.
#[test]
fn reserve_usdc_to_polimec() {
	let asset_id = 1337;
	test_reserve_to_polimec(asset_id);
}

/// Test reserve based transfer of DOT from AssetHub to Polimec.
#[test]
fn reserve_dot_to_polimec() {
	let asset_id = 0;
	test_reserve_to_polimec(asset_id);
}

/// Test that reserve based transfer of random asset from AssetHub to Polimec fails.
#[test]
#[should_panic]
fn reserve_random_asset_to_polimec() {
	let asset_id = 69;
	test_reserve_to_polimec(asset_id);
}

/// Test transfer of reserve-based DOT from Polimec back to AssetHub.
#[test]
fn polimec_usdt_to_reserve() {
	let asset_id = 1984;
	test_polimec_to_reserve(asset_id);
}

/// Test transfer of reserve-based DOT from Polimec back to AssetHub.
#[test]
fn polimec_usdc_to_reserve() {
	let asset_id = 1337;
	test_polimec_to_reserve(asset_id);
}

/// Test transfer of reserve-based DOT from Polimec back to AssetHub.
#[test]
fn polimec_dot_to_reserve() {
	let asset_id = 0;
	test_polimec_to_reserve(asset_id);
}

#[test]
fn test_user_cannot_create_foreign_asset_on_polimec() {
	PolimecBase::execute_with(|| {
		let admin = AssetHub::account_id_of(ALICE);
		assert_noop!(
			BaseForeignAssets::create(
				BaseOrigin::signed(admin.clone()),
				69.into(),
				sp_runtime::MultiAddress::Id(admin),
				0_0_010_000_000u128,
			), 
		DispatchError::BadOrigin);
	});
}