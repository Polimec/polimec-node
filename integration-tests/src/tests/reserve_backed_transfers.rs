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

fn create_usdt_on_statemint() {
	let usdt_admin_account = Statemint::account_id_of(FERDIE);
	Statemint::execute_with(|| {
		assert_ok!(StatemintAssets::create(
			StatemintOrigin::signed(usdt_admin_account.clone()),
			AcceptedFundingAsset::USDT.to_statemint_id().into(),
			sp_runtime::MultiAddress::Id(usdt_admin_account.clone()),
			0_0_010_000_000u128
		));
	});
}

fn mint_usdt_on_statemint_to(recipient: &StatemintAccountId, amount: u128) {
	Statemint::execute_with(|| {
		assert_ok!(StatemintAssets::mint_into(AcceptedFundingAsset::USDT.to_statemint_id(), recipient, amount,));
	});
}

#[test]
fn reserve_to_polimec() {
	create_usdt_on_statemint();
	let usdt_on_statemint: MultiLocation = (
		PalletInstance(StatemintAssets::index() as u8),
		GeneralIndex(AcceptedFundingAsset::USDT.to_statemint_id() as u128),
	)
		.into();

	let usdt_transfer: MultiAsset = (usdt_on_statemint, RESERVE_TRANSFER_AMOUNT).into();
	let alice_account = Polimec::account_id_of(ALICE.clone());
	let polimec_sibling_account =
		Statemint::sovereign_account_id_of((Parent, Parachain(Polimec::para_id().into())).into());
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);

	mint_usdt_on_statemint_to(&alice_account, 100_0_000_000_000);

	// check Polimec's pre transfer balances and issuance
	let (
		polimec_prev_alice_usdt_balance,
		polimec_prev_alice_plmc_balance,
		polimec_prev_usdt_issuance,
		polimec_prev_plmc_issuance,
	) = Polimec::execute_with(|| {
		(
			PolimecStatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
			PolimecBalances::free_balance(alice_account.clone()),
			PolimecStatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
			PolimecBalances::total_issuance(),
		)
	});

	// check Statemint's pre transfer balances and issuance
	let (statemint_prev_alice_usdt_balance, statemint_prev_polimec_usdt_balance, statemint_prev_usdt_issuance) =
		Statemint::execute_with(|| {
			(
				StatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
				StatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), polimec_sibling_account.clone()),
				StatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
			)
		});

	// do the transfer
	Statemint::execute_with(|| {
		let alice_bytes = <[u8; 32]>::from(alice_account.clone());
		let origin = StatemintOrigin::signed(alice_account.clone());
		let dest: VersionedMultiLocation = ParentThen(X1(Parachain(Polimec::para_id().into()))).into();

		let beneficiary: VersionedMultiLocation = AccountId32 { network: None, id: alice_bytes }.into();
		let assets: VersionedMultiAssets = usdt_transfer.into();
		let fee_asset_item = 0;
		let weight_limit = Unlimited;

		let call = StatemintXcmPallet::limited_reserve_transfer_assets(
			origin,
			bx!(dest),
			bx!(beneficiary),
			bx!(assets),
			fee_asset_item,
			weight_limit,
		);
		assert_ok!(call);

		dbg!(Statemint::events())
	});

	// check the transfer was not blocked by our our xcm configured
	Polimec::execute_with(|| {
		let events = PolimecSystem::events();
		dbg!(events);
		assert_expected_events!(
			Polimec,
			vec![
				PolimecEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success { .. }) => {},
			]
		);
	});

	// check Polimec's post transfer balances and issuance
	let (
		polimec_post_alice_usdt_balance,
		polimec_post_alice_plmc_balance,
		polimec_post_usdt_issuance,
		polimec_post_plmc_issuance,
	) = Polimec::execute_with(|| {
		(
			PolimecStatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
			PolimecBalances::free_balance(alice_account.clone()),
			PolimecStatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
			PolimecBalances::total_issuance(),
		)
	});

	// check Statemint's post transfer balances and issuance
	let (statemint_post_alice_usdt_balance, statemint_post_polimec_usdt_balance, statemint_post_usdt_issuance) =
		Statemint::execute_with(|| {
			(
				StatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
				StatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), polimec_sibling_account.clone()),
				StatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
			)
		});

	let polimec_delta_alice_usdt_balance = polimec_post_alice_usdt_balance - polimec_prev_alice_usdt_balance;
	let polimec_delta_usdt_issuance = polimec_post_usdt_issuance - polimec_prev_usdt_issuance;
	let polimec_delta_alice_plmc_balance = polimec_post_alice_plmc_balance - polimec_prev_alice_plmc_balance;
	let polimec_delta_plmc_issuance = polimec_post_plmc_issuance - polimec_prev_plmc_issuance;

	let statemint_delta_alice_usdt_balance = statemint_prev_alice_usdt_balance - statemint_post_alice_usdt_balance;
	let statemint_delta_polimec_usdt_balance =
		statemint_post_polimec_usdt_balance - statemint_prev_polimec_usdt_balance;
	let statemint_delta_usdt_issuance = statemint_prev_usdt_issuance - statemint_post_usdt_issuance;

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
		statemint_delta_alice_usdt_balance, RESERVE_TRANSFER_AMOUNT,
		"Statemint alice_account.clone() USDT balance should have decreased by the transfer amount"
	);

	assert!(
		statemint_delta_polimec_usdt_balance == RESERVE_TRANSFER_AMOUNT,
		"The USDT balance of Polimec's sovereign account on Statemint should receive the transfer amount"
	);

	assert!(
		statemint_delta_usdt_issuance == 0u128,
		"Statemint's USDT issuance should not change, since it acts as a reserve for that asset"
	);

	assert_eq!(
		polimec_delta_alice_plmc_balance, 0,
		"Polimec alice_account.clone() PLMC balance should not have changed"
	);

	assert_eq!(polimec_delta_plmc_issuance, 0, "Polimec PLMC issuance should not have changed");
}

#[test]
fn polimec_to_reserve() {
	create_usdt_on_statemint();

	let usdt_on_statemint: MultiLocation = ParentThen(X3(
		Parachain(Statemint::para_id().into()),
		PalletInstance(StatemintAssets::index() as u8),
		GeneralIndex(AcceptedFundingAsset::USDT.to_statemint_id() as u128),
	))
	.into();
	let alice_account = Polimec::account_id_of(ALICE.clone());
	let polimec_sibling_account =
		Statemint::sovereign_account_id_of((Parent, Parachain(Polimec::para_id().into())).into());
	let max_weight = Weight::from_parts(MAX_REF_TIME, MAX_PROOF_SIZE);

	// Give some usdt to Polimec on Statemint
	mint_usdt_on_statemint_to(&polimec_sibling_account, RESERVE_TRANSFER_AMOUNT * 3);
	// Represent that usdt just minted in Polimec, by minting to Alice
	Polimec::execute_with(|| {
		assert_ok!(PolimecStatemintAssets::mint_into(
			AcceptedFundingAsset::USDT.to_statemint_id(),
			&alice_account,
			RESERVE_TRANSFER_AMOUNT * 3
		));
		assert_ok!(PolimecStatemintAssets::mint_into(
			AcceptedFundingAsset::DOT.to_statemint_id(),
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
	) = Polimec::execute_with(|| {
		(
			PolimecStatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
			PolimecBalances::free_balance(alice_account.clone()),
			PolimecStatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
			PolimecBalances::total_issuance(),
		)
	});

	// check Statemint's pre transfer balances and issuance
	let (statemint_prev_alice_usdt_balance, statemint_prev_usdt_issuance) = Statemint::execute_with(|| {
		(
			StatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
			StatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
		)
	});

	let usdt_plus_exec_fee: MultiAsset = (usdt_on_statemint, RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000).into();
	let dot_for_xcm_execution: MultiAsset = (Polimec::parent_location(), 1_0_000_000_000u128).into();
	// construct the XCM to transfer from Polimec to Statemint's reserve
	let transfer_xcm: Xcm<PolimecCall> = Xcm(vec![
		WithdrawAsset(vec![usdt_plus_exec_fee.clone(), dot_for_xcm_execution.clone()].into()),
		BuyExecution { fees: usdt_plus_exec_fee.clone(), weight_limit: Limited(max_weight) },
		InitiateReserveWithdraw {
			assets: All.into(),
			reserve: MultiLocation::new(1, X1(Parachain(Statemint::para_id().into()))),
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
	Polimec::execute_with(|| {
		assert_ok!(PolimecXcmPallet::execute(
			PolimecOrigin::signed(alice_account.clone()),
			Box::new(VersionedXcm::V3(transfer_xcm)),
			max_weight,
		));
	});

	// check that the xcm was not blocked
	Statemint::execute_with(|| {
		let events = StatemintSystem::events();
		dbg!(events);
		assert_expected_events!(
			Statemint,
			vec![
				StatemintEvent::XcmpQueue(cumulus_pallet_xcmp_queue::Event::Success {..}) => {},
			]
		);
	});

	// check Polimec's post transfer balances and issuance
	let (
		polimec_post_alice_usdt_balance,
		polimec_post_alice_plmc_balance,
		polimec_post_usdt_issuance,
		polimec_post_plmc_issuance,
	) = Polimec::execute_with(|| {
		(
			PolimecStatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
			PolimecBalances::free_balance(alice_account.clone()),
			PolimecStatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
			PolimecBalances::total_issuance(),
		)
	});

	// check Statemint's post transfer balances and issuance
	let (statemint_post_alice_usdt_balance, statemint_post_usdt_issuance) = Statemint::execute_with(|| {
		(
			StatemintAssets::balance(AcceptedFundingAsset::USDT.to_statemint_id(), alice_account.clone()),
			StatemintAssets::total_issuance(AcceptedFundingAsset::USDT.to_statemint_id()),
		)
	});

	let polimec_delta_usdt_issuance = polimec_prev_usdt_issuance - polimec_post_usdt_issuance;
	let polimec_delta_plmc_issuance = polimec_prev_plmc_issuance - polimec_post_plmc_issuance;
	let polimec_delta_alice_usdt_balance = polimec_prev_alice_usdt_balance - polimec_post_alice_usdt_balance;
	let polimec_delta_alice_plmc_balance = polimec_prev_alice_plmc_balance - polimec_post_alice_plmc_balance;

	let statemint_delta_usdt_issuance = statemint_prev_usdt_issuance - statemint_post_usdt_issuance;
	let statemint_delta_alice_usdt_balance = statemint_post_alice_usdt_balance - statemint_prev_alice_usdt_balance;

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

	dbg!(statemint_delta_alice_usdt_balance);
	assert!(
	    statemint_delta_alice_usdt_balance >=
	        RESERVE_TRANSFER_AMOUNT &&
	        statemint_delta_alice_usdt_balance <= RESERVE_TRANSFER_AMOUNT + 1_0_000_000_000,
	    "Statemint's alice_account DOT balance should increase by at least the transfer amount minus the max allowed fees"
	);

	assert!(
	    statemint_delta_usdt_issuance <= asset_hub_polkadot_runtime::constants::fee::WeightToFee::weight_to_fee(&max_weight),
	    "Statemint's DOT issuance should not change, since it acts as a reserve for that asset (except for fees which are burnt)"
	);
}
