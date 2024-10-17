#[allow(clippy::wildcard_imports)]
use crate::*;
use crate::traits::BondingRequirementCalculation;
use alloc::collections::BTreeMap;
use frame_support::traits::fungibles::{Inspect, InspectEnumerable};
use itertools::Itertools;
use parity_scale_codec::{Decode, Encode};
use polimec_common::{credentials::InvestorType, ProvideAssetPrice, USD_DECIMALS};
use scale_info::TypeInfo;
use sp_core::Get;
use sp_runtime::traits::Zero;

#[derive(Debug, Clone, PartialEq, Eq, Encode, Decode, TypeInfo)]
pub struct ProjectParticipationIds<T: Config> {
	account: AccountIdOf<T>,
	evaluation_ids: Vec<u32>,
	bid_ids: Vec<u32>,
	contribution_ids: Vec<u32>,
}

sp_api::decl_runtime_apis! {
	#[api_version(1)]
	pub trait Leaderboards<T: Config> {
		/// Get the top evaluations made for a project by the amount of PLMC bonded
		fn top_evaluations(project_id: ProjectId, amount: u32) -> Vec<EvaluationInfoOf<T>>;

		/// Get the top bids for a project by the amount of CTs bought.
		fn top_bids(project_id: ProjectId, amount: u32) -> Vec<BidInfoOf<T>>;

		/// Get the top contributions for a project by the amount of CTs bought.
		fn top_contributions(project_id: ProjectId, amount: u32) -> Vec<ContributionInfoOf<T>>;

		/// Get the top projects by the absolute USD value raised
		fn top_projects_by_usd_raised(amount: u32) -> Vec<(ProjectId, ProjectMetadataOf<T>, ProjectDetailsOf<T>)>;

		/// Get the top project by the highest percentage of the target reached
		fn top_projects_by_usd_target_percent_reached(amount: u32) -> Vec<(ProjectId, ProjectMetadataOf<T>, ProjectDetailsOf<T>)>;
	}

	#[api_version(1)]
	pub trait UserInformation<T: Config> {
		/// Get all the contribution token balances for the participated projects
		fn contribution_tokens(account: AccountIdOf<T>) -> Vec<(ProjectId, Balance)>;

		/// Get all the project participations made by a single DID.
		fn all_project_participations_by_did(project_id: ProjectId, did: Did) -> Vec<ProjectParticipationIds<T>>;
	}

	#[api_version(1)]
	pub trait ProjectInformation<T: Config> {
		/// Get the percentage of the target reached for a project
		fn usd_target_percent_reached(project_id: ProjectId) -> FixedU128;

		/// Get all the projects created by a single DID.
		fn projects_by_did(did: Did) -> Vec<ProjectId>;
	}

	#[api_version(3)]
	pub trait ExtrinsicHelpers<T: Config> {
		/// Get the current price of a contribution token (either current bucket in the auction, or WAP in contribution phase),
		/// and calculate the amount of tokens that can be bought with the given amount USDT/USDC/DOT.
		fn funding_asset_to_ct_amount_classic(project_id: ProjectId, funding_asset: AcceptedFundingAsset, funding_asset_amount: Balance) -> Balance;

		/// Calculate how many CTs and what the OTM fee is for a given project and funding asset amount.
		fn funding_asset_to_ct_amount_otm(project_id: ProjectId, funding_asset: AcceptedFundingAsset, funding_asset_amount: Balance) -> (Balance, Balance);

		/// Get the indexes of vesting schedules that are good candidates to be merged.
		/// Schedules that have not yet started are de-facto bad candidates.
		fn get_next_vesting_schedule_merge_candidates(account_id: AccountIdOf<T>, hold_reason: <T as Config>::RuntimeHoldReason, end_max_delta: Balance) -> Option<(u32, u32)>;

		/// Calculate the OTM fee for a project, using a given asset and amount.
		fn calculate_otm_fee(funding_asset: AcceptedFundingAsset, funding_asset_amount: Balance) -> Option<Balance>;

		/// Gets the minimum and maximum amount of FundingAsset a user can input in the UI.
		fn get_funding_asset_min_max_amounts(project_id: ProjectId, did: Did, funding_asset: AcceptedFundingAsset, investor_type: InvestorType) -> Option<(Balance, Balance)>;

	}
}

impl<T: Config> Pallet<T> {
	pub fn top_evaluations(project_id: ProjectId, amount: u32) -> Vec<EvaluationInfoOf<T>> {
		Evaluations::<T>::iter_prefix_values((project_id,))
			.sorted_by(|a, b| b.original_plmc_bond.cmp(&a.original_plmc_bond))
			.take(amount as usize)
			.collect_vec()
	}

	pub fn top_bids(project_id: ProjectId, amount: u32) -> Vec<BidInfoOf<T>> {
		Bids::<T>::iter_prefix_values((project_id,))
			.sorted_by(|a, b| b.final_ct_amount().cmp(&a.final_ct_amount()))
			.take(amount as usize)
			.collect_vec()
	}

	pub fn top_contributions(project_id: ProjectId, amount: u32) -> Vec<ContributionInfoOf<T>> {
		Contributions::<T>::iter_prefix_values((project_id,))
			.sorted_by(|a, b| b.ct_amount.cmp(&a.ct_amount))
			.take(amount as usize)
			.collect_vec()
	}

	pub fn top_projects_by_usd_raised(amount: u32) -> Vec<(ProjectId, ProjectMetadataOf<T>, ProjectDetailsOf<T>)> {
		ProjectsDetails::<T>::iter()
			.sorted_by(|a, b| b.1.funding_amount_reached_usd.cmp(&a.1.funding_amount_reached_usd))
			.take(amount as usize)
			.map(|(project_id, project_details)| {
				let project_metadata = ProjectsMetadata::<T>::get(project_id).expect("Project not found");
				(project_id, project_metadata, project_details)
			})
			.collect_vec()
	}

	pub fn top_projects_by_usd_target_percent_reached(
		amount: u32,
	) -> Vec<(ProjectId, ProjectMetadataOf<T>, ProjectDetailsOf<T>)> {
		ProjectsDetails::<T>::iter()
			.map(|(project_id, project_details)| {
				let funding_reached = project_details.funding_amount_reached_usd;
				let funding_target = project_details.fundraising_target_usd;
				let funding_ratio = FixedU128::from_rational(funding_reached, funding_target);
				(project_id, project_details, funding_ratio)
			})
			.sorted_by(|a, b| b.2.cmp(&a.2))
			.take(amount as usize)
			.map(|(project_id, project_details, _funding_ratio)| {
				let project_metadata = ProjectsMetadata::<T>::get(project_id).expect("Project not found");
				(project_id, project_metadata, project_details)
			})
			.collect_vec()
	}

	pub fn contribution_tokens(account: AccountIdOf<T>) -> Vec<(ProjectId, Balance)> {
		let asset_ids = <T as Config>::ContributionTokenCurrency::asset_ids();
		asset_ids
			.filter_map(|asset_id| {
				let balance = <T as Config>::ContributionTokenCurrency::balance(asset_id, &account);
				if balance > Zero::zero() {
					Some((asset_id, balance))
				} else {
					None
				}
			})
			.sorted_by(|a, b| b.1.cmp(&a.1))
			.collect_vec()
	}

	pub fn funding_asset_to_ct_amount_classic(
		project_id: ProjectId,
		asset: AcceptedFundingAsset,
		asset_amount: Balance,
	) -> Balance {
		let project_details = ProjectsDetails::<T>::get(project_id).expect("Project not found");
		let funding_asset_usd_price =
			Pallet::<T>::get_decimals_aware_funding_asset_price(&asset).expect("Price not found");
		let usd_ticket_size = funding_asset_usd_price.saturating_mul_int(asset_amount);

		let mut ct_amount = Zero::zero();

		// Contribution phase
		if let Some(wap) = project_details.weighted_average_price {
			ct_amount = wap.reciprocal().expect("Bad math").saturating_mul_int(usd_ticket_size);
		}
		// Auction phase, we need to consider multiple buckets
		else {
			let mut usd_to_spend = usd_ticket_size;
			let mut current_bucket = Buckets::<T>::get(project_id).expect("Bucket not found");
			while usd_to_spend > Zero::zero() {
				let bucket_price = current_bucket.current_price;

				let ct_to_buy = bucket_price.reciprocal().expect("Bad math").saturating_mul_int(usd_to_spend);
				let ct_to_buy = ct_to_buy.min(current_bucket.amount_left);

				ct_amount = ct_amount.saturating_add(ct_to_buy);
				// if usd spent is 0, we will have an infinite loop
				let usd_spent = bucket_price.saturating_mul_int(ct_to_buy).max(One::one());
				usd_to_spend = usd_to_spend.saturating_sub(usd_spent);

				current_bucket.update(ct_to_buy)
			}
		}

		ct_amount
	}

	pub fn funding_asset_to_ct_amount_otm(
		project_id: ProjectId,
		funding_asset: AcceptedFundingAsset,
		total_funding_asset_amount: Balance,
	) -> (Balance, Balance) {
		let project_details = ProjectsDetails::<T>::get(project_id).expect("Project not found");
		let funding_asset_usd_price =
			Pallet::<T>::get_decimals_aware_funding_asset_price(&funding_asset).expect("Price not found");
		let otm_multiplier = ParticipationMode::OTM.multiplier();
		let otm_fee_plmc_percentage = <T as pallet_proxy_bonding::Config>::FeePercentage::get();
		let otm_fee_usd_percentage = otm_fee_plmc_percentage / otm_multiplier;

		let divisor = FixedU128::from_perbill(otm_fee_usd_percentage) + FixedU128::from_rational(1, 1);
		let participating_funding_asset_amount =
			divisor.reciprocal().unwrap().saturating_mul_int(total_funding_asset_amount);
		let fee_funding_asset_amount = total_funding_asset_amount.saturating_sub(participating_funding_asset_amount);

		let participating_usd_ticket_size =
			funding_asset_usd_price.saturating_mul_int(participating_funding_asset_amount);

		let mut ct_amount = Zero::zero();

		// Contribution phase
		if let Some(wap) = project_details.weighted_average_price {
			ct_amount = wap.reciprocal().expect("Bad math").saturating_mul_int(participating_usd_ticket_size);
		}
		// Auction phase, we need to consider multiple buckets
		else {
			let mut usd_to_spend = participating_usd_ticket_size;
			let mut current_bucket = Buckets::<T>::get(project_id).expect("Bucket not found");
			while usd_to_spend > Zero::zero() {
				let bucket_price = current_bucket.current_price;

				let ct_to_buy = bucket_price.reciprocal().expect("Bad math").saturating_mul_int(usd_to_spend);
				let ct_to_buy = ct_to_buy.min(current_bucket.amount_left);

				ct_amount = ct_amount.saturating_add(ct_to_buy);
				// if usd spent is 0, we will have an infinite loop
				let usd_spent = bucket_price.saturating_mul_int(ct_to_buy).max(One::one());
				usd_to_spend = usd_to_spend.saturating_sub(usd_spent);

				current_bucket.update(ct_to_buy)
			}
		}

		(ct_amount, fee_funding_asset_amount)
	}

	pub fn get_next_vesting_schedule_merge_candidates(
		account_id: AccountIdOf<T>,
		hold_reason: <T as Config>::RuntimeHoldReason,
		end_max_delta: Balance,
	) -> Option<(u32, u32)> {
		let schedules = pallet_linear_release::Vesting::<T>::get(account_id, hold_reason)?
			.into_iter()
			.enumerate()
			// Filter out schedules with future starting blocks before collecting them into a vector.
			.filter_map(|(i, schedule)| {
				if schedule.starting_block > <frame_system::Pallet<T>>::block_number() {
					None
				} else {
					Some((i, schedule.ending_block_as_balance::<BlockNumberToBalanceOf<T>>()))
				}
			})
			.collect::<Vec<_>>();

		let mut inspected_schedules = BTreeMap::new();

		for (i, schedule_end) in schedules {
			let range_start = schedule_end.saturating_sub(end_max_delta);
			let range_end = schedule_end.saturating_add(end_max_delta);

			//  All entries where the ending_block is between range_start and range_end.
			if let Some((_, &j)) = inspected_schedules.range(range_start..=range_end).next() {
				return Some((j as u32, i as u32));
			}

			inspected_schedules.insert(schedule_end, i);
		}

		None
	}

	pub fn calculate_otm_fee(funding_asset: AcceptedFundingAsset, funding_asset_amount: Balance) -> Option<Balance> {
		let plmc_price = <PriceProviderOf<T>>::get_decimals_aware_price(PLMC_FOREIGN_ID, USD_DECIMALS, PLMC_DECIMALS)
			.expect("Price not found");
		let funding_asset_usd_price = Pallet::<T>::get_decimals_aware_funding_asset_price(&funding_asset).unwrap();
		let usd_amount = funding_asset_usd_price.saturating_mul_int(funding_asset_amount);
		let otm_multiplier: MultiplierOf<T> = ParticipationMode::OTM.multiplier().try_into().ok()?;
		let required_usd_bond = otm_multiplier.calculate_usd_bonding_requirement::<T>(usd_amount)?;
		let plmc_bond = plmc_price.reciprocal()?.saturating_mul_int(required_usd_bond);
		pallet_proxy_bonding::Pallet::<T>::calculate_fee(plmc_bond, funding_asset.id()).ok()
	}

	pub fn get_funding_asset_min_max_amounts(
		project_id: ProjectId,
		did: Did,
		funding_asset: AcceptedFundingAsset,
		investor_type: InvestorType,
	) -> Option<(Balance, Balance)> {
		let project_details = ProjectsDetails::<T>::get(project_id)?;
		let project_metadata = ProjectsMetadata::<T>::get(project_id)?;
		let funding_asset_price = Pallet::<T>::get_decimals_aware_funding_asset_price(&funding_asset)?;

		let (min_usd_ticket, maybe_max_usd_ticket, already_spent_usd, total_cts_usd_amount) =
			match project_details.status {
				ProjectStatus::AuctionRound => {
					let ticket_sizes = match investor_type {
						InvestorType::Institutional => project_metadata.bidding_ticket_sizes.institutional,
						InvestorType::Professional => project_metadata.bidding_ticket_sizes.professional,
						_ => return None,
					};
					let already_spent_usd = AuctionBoughtUSD::<T>::get((project_id, did));
					let mut max_contribution_tokens =
						project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size;

					let mut total_cts_usd_amount = 0;

					let mut current_bucket = Buckets::<T>::get(project_id)?;
					while max_contribution_tokens > 0u128 {
						let bucket_price = current_bucket.current_price;
						let ct_to_buy = max_contribution_tokens.min(current_bucket.amount_left);
						let usd_spent = bucket_price.saturating_mul_int(ct_to_buy);

						max_contribution_tokens -= ct_to_buy;
						total_cts_usd_amount += usd_spent;
						current_bucket.update(ct_to_buy);
					}

					(
						ticket_sizes.usd_minimum_per_participation,
						ticket_sizes.usd_maximum_per_did,
						already_spent_usd,
						total_cts_usd_amount,
					)
				},
				ProjectStatus::CommunityRound(..) => {
					let ticket_sizes = match investor_type {
						InvestorType::Institutional => project_metadata.contributing_ticket_sizes.institutional,
						InvestorType::Professional => project_metadata.contributing_ticket_sizes.professional,
						InvestorType::Retail => project_metadata.contributing_ticket_sizes.retail,
					};
					let already_spent_usd = ContributionBoughtUSD::<T>::get((project_id, did));
					let max_contribution_tokens = project_details.remaining_contribution_tokens;
					let price = project_details.weighted_average_price?;
					let total_cts_usd_amount = price.saturating_mul_int(max_contribution_tokens);
					(
						ticket_sizes.usd_minimum_per_participation,
						ticket_sizes.usd_maximum_per_did,
						already_spent_usd,
						total_cts_usd_amount,
					)
				},
				_ => return None,
			};

		let max_usd_ticket = if let Some(issuer_set_max_usd_ticket) = maybe_max_usd_ticket {
			total_cts_usd_amount.min(issuer_set_max_usd_ticket.saturating_sub(already_spent_usd))
		} else {
			total_cts_usd_amount
		};

		let funding_asset_min_ticket = funding_asset_price.reciprocal()?.saturating_mul_int(min_usd_ticket);
		let funding_asset_max_ticket = funding_asset_price.reciprocal()?.saturating_mul_int(max_usd_ticket);

		Some((funding_asset_min_ticket, funding_asset_max_ticket))
	}

	pub fn all_project_participations_by_did(project_id: ProjectId, did: Did) -> Vec<ProjectParticipationIds<T>> {
		let evaluations = Evaluations::<T>::iter_prefix((project_id,))
			.filter(|((_account_id, _evaluation_id), evaluation)| evaluation.did == did)
			.map(|((account_id, evaluation_id), _evaluation)| (account_id, evaluation_id))
			.collect_vec();

		let bids = Bids::<T>::iter_prefix((project_id,))
			.filter(|((_account_id, _bid_id), bid)| bid.did == did)
			.map(|((account_id, bid_id), _bid)| (account_id, bid_id))
			.collect_vec();

		let contributions = Contributions::<T>::iter_prefix((project_id,))
			.filter(|((_account_id, _contribution_id), contribution)| contribution.did == did)
			.map(|((account_id, contribution_id), _contribution)| (account_id, contribution_id))
			.collect_vec();

		#[allow(clippy::type_complexity)]
		let mut map: BTreeMap<AccountIdOf<T>, (Vec<u32>, Vec<u32>, Vec<u32>)> = BTreeMap::new();

		for (account_id, evaluation_id) in evaluations {
			map.entry(account_id).or_insert_with(|| (Vec::new(), Vec::new(), Vec::new())).0.push(evaluation_id);
		}

		for (account_id, bid_id) in bids {
			map.entry(account_id).or_insert_with(|| (Vec::new(), Vec::new(), Vec::new())).1.push(bid_id);
		}

		for (account_id, contribution_id) in contributions {
			map.entry(account_id).or_insert_with(|| (Vec::new(), Vec::new(), Vec::new())).2.push(contribution_id);
		}

		map.into_iter()
			.map(|(account, (evaluation_ids, bid_ids, contribution_ids))| ProjectParticipationIds {
				account,
				evaluation_ids,
				bid_ids,
				contribution_ids,
			})
			.collect()
	}

	pub fn usd_target_percent_reached(project_id: ProjectId) -> FixedU128 {
		let project_details = ProjectsDetails::<T>::get(project_id).expect("Project not found");
		let funding_reached = project_details.funding_amount_reached_usd;
		let funding_target = project_details.fundraising_target_usd;
		FixedU128::from_rational(funding_reached, funding_target)
	}

	pub fn projects_by_did(did: Did) -> Vec<ProjectId> {
		ProjectsDetails::<T>::iter()
			.filter(|(_project_id, project_details)| project_details.issuer_did == did)
			.map(|(project_id, _)| project_id)
			.collect()
	}
}
