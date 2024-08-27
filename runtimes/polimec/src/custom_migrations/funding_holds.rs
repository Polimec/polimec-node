use crate::{Balance, Funding, Runtime, RuntimeHoldReason};
use alloc::vec::Vec;
use frame_support::traits::{GetStorageVersion, OnRuntimeUpgrade, VariantCount, VariantCountOf};
use pallet_balances::IdAmount;
use pallet_funding::ProjectId;
use parity_scale_codec::{Decode, Encode};
use scale_info::TypeInfo;
use sp_core::{MaxEncodedLen, RuntimeDebug};
use sp_runtime::BoundedVec;

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum OldFundingHoldReason {
	Evaluation(ProjectId),
	Participation(ProjectId),
}

#[derive(Clone, Copy, Encode, Decode, Eq, PartialEq, RuntimeDebug, TypeInfo, MaxEncodedLen)]
pub enum OldRuntimeHoldReason {
	#[codec(index = 25u8)]
	ParachainStaking(pallet_parachain_staking::HoldReason),

	#[codec(index = 41u8)]
	Democracy(pallet_democracy::HoldReason),

	#[codec(index = 44u8)]
	Elections(pallet_elections_phragmen::HoldReason),

	#[codec(index = 45u8)]
	Preimage(pallet_preimage::HoldReason),

	#[codec(index = 80u8)]
	Funding(OldFundingHoldReason),
}

impl VariantCount for OldRuntimeHoldReason {
	const VARIANT_COUNT: u32 = 2 + 1 + 1 + 1 + 2;
}

type OldIdAmount = IdAmount<OldRuntimeHoldReason, Balance>;
type NewIdAmount = IdAmount<RuntimeHoldReason, Balance>;
type OldHoldsItem = BoundedVec<OldIdAmount, VariantCountOf<OldRuntimeHoldReason>>;
type NewHoldsItem = BoundedVec<NewIdAmount, VariantCountOf<RuntimeHoldReason>>;

pub struct FromFundingV4Migration;
impl OnRuntimeUpgrade for FromFundingV4Migration {
	fn on_runtime_upgrade() -> frame_support::weights::Weight {
		let on_chain_version = Funding::on_chain_storage_version();
		if on_chain_version != 4 {
			log::warn!("Funding Holds migration can be removed. Skipping it now...",);
			return <Runtime as frame_system::Config>::DbWeight::get().reads(1)
		}
		let mut items = 0;
		let mut translate = |_key, old_user_holds: OldHoldsItem| -> Option<NewHoldsItem> {
			items += 1;
			log::info!("Migrating hold {:?}", items);
			let mut new_user_holds = Vec::new();
			for user_hold in old_user_holds.iter() {
				let new_id = match user_hold.id {
					OldRuntimeHoldReason::ParachainStaking(reason) => RuntimeHoldReason::ParachainStaking(reason),
					OldRuntimeHoldReason::Democracy(reason) => RuntimeHoldReason::Democracy(reason),
					OldRuntimeHoldReason::Elections(reason) => RuntimeHoldReason::Elections(reason),
					OldRuntimeHoldReason::Preimage(reason) => RuntimeHoldReason::Preimage(reason),
					OldRuntimeHoldReason::Funding(OldFundingHoldReason::Evaluation(_)) =>
						RuntimeHoldReason::Funding(pallet_funding::HoldReason::Evaluation),
					OldRuntimeHoldReason::Funding(OldFundingHoldReason::Participation(_)) =>
						RuntimeHoldReason::Funding(pallet_funding::HoldReason::Participation),
				};
				new_user_holds.push(IdAmount { id: new_id, amount: user_hold.amount })
			}
			let output = NewHoldsItem::try_from(new_user_holds);

			debug_assert!(output.is_ok(), "Failed to convert holds");
			if let Err(err) = &output {
				log::error!(
					"Holds conversion failed with err {:?} for the following user holds: {:?}",
					err,
					old_user_holds
				);
			}
			// If we failed to convert the holds, we delete them
			output.ok()
		};

		pallet_balances::Holds::<Runtime>::translate(|key, object: OldHoldsItem| translate(key, object));

		log::info!("Number of users migrated: {}", items);
		let weight = <Runtime as frame_system::Config>::DbWeight::get().reads_writes(items, items);
		log::info!("holds weight: {:?}", weight);
		weight
	}
}
