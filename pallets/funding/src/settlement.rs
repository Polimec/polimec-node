use super::*;
use crate::{
	traits::{BondingRequirementCalculation, ProvideAssetPrice, VestingDurationCalculation},
	ProjectStatus::FundingSuccessful,
};
use frame_support::{
	dispatch::{DispatchErrorWithPostInfo, DispatchResult, DispatchResultWithPostInfo, PostDispatchInfo},
	ensure,
	pallet_prelude::*,
	traits::{
		fungible::{InspectHold, Mutate, MutateHold as FungibleMutateHold},
		fungibles::{
			metadata::{MetadataDeposit, Mutate as MetadataMutate},
			Create, Inspect, Mutate as FungiblesMutate,
		},
		tokens::{Fortitude, Precision, Preservation, Restriction},
		Get,
	},
};
use sp_runtime::{traits::Zero, Perquintill};
use polimec_common::{
	migration_types::{MigrationInfo, MigrationOrigin, Migrations, ParticipationType},
	ReleaseSchedule,
};

impl<T: Config> Pallet<T> {
    pub fn do_settlement_success_bidder(bid: BidInfoOf<T>, project_id: ProjectId) -> DispatchResult {
        
        let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		// Ensure that:
        // 1. The project is in the FundingSuccessful state
        // 2. The bid is in the Accepted or PartiallyAccepted state
        // 3. The contribution token exists
        ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
        ensure!(matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)), Error::<T>::NotAllowed);
        ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

        let bidder = bid.bidder;

        // Calculate the vesting info and add the release schedule
        let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;
		let vest_info =
			Self::calculate_vesting_info(&bidder, bid.multiplier, bid.plmc_bond).map_err(|_| Error::<T>::BadMath)?;
		
        T::Vesting::add_release_schedule(
            &bidder,
            vest_info.total_amount,
            vest_info.amount_per_block,
            funding_end_block,
            HoldReason::Participation(project_id).into(),
        )?;

        // Mint the contribution tokens
        Self::mint_ct_tokens(project_id, &bidder, bid.final_ct_amount)?;

        // Payout the bid funding asset amount to the project account
        Self::release_funding_asset(project_id, &project_details.issuer_account, bid.funding_asset_amount_locked, bid.funding_asset)?;

        // TODO: Create MigrationInfo

        Bids::<T>::remove((project_id, bidder, bid.id));

        // TODO: Emit an event

		Ok(())
    }

    pub fn do_settlement_failure_bidder(bid: BidInfoOf<T>, project_id: ProjectId) -> DispatchResult  {
        let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
        ensure!(
			matches!(project_details.status, ProjectStatus::FundingFailed),
			Error::<T>::NotAllowed
		);

        let bidder = bid.bidder;
		
        // Release the held future ct deposit
        Self::release_future_ct_deposit(project_id, &bidder)?;

        if matches!(bid.status, BidStatus::Accepted | BidStatus::PartiallyAccepted(..)) {
            // Return the funding assets to the bidder
            Self::release_funding_asset(project_id, &bidder, bid.funding_asset_amount_locked, bid.funding_asset)?;

            // Release the held PLMC bond
            Self::release_bond(project_id, &bidder, bid.plmc_bond)?;
        }
        
        // Remove the bid from the storage
        Bids::<T>::remove((project_id, bidder, bid.id));

        // TODO: Emit an event

        Ok(())
    }

    pub fn do_settlement_success_contributor(contribution: ContributionInfoOf<T>, project_id: ProjectId) -> DispatchResult {
        let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		// Ensure that:
        // 1. The project is in the FundingSuccessful state
        // 2. The contribution token exists
        ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
        ensure!(T::ContributionTokenCurrency::asset_exists(project_id), Error::<T>::CannotClaimYet);

        let contributor = contribution.contributor;

        // Calculate the vesting info and add the release schedule
        let funding_end_block = project_details.funding_end_block.ok_or(Error::<T>::ImpossibleState)?;
		let vest_info =
			Self::calculate_vesting_info(&contributor, contribution.multiplier, contribution.plmc_bond).map_err(|_| Error::<T>::BadMath)?;

        T::Vesting::add_release_schedule(
            &contributor,
            vest_info.total_amount,
            vest_info.amount_per_block,
            funding_end_block,
            HoldReason::Participation(project_id).into(),
        )?;

         // Mint the contribution tokens
        Self::mint_ct_tokens(project_id, &contributor, contribution.ct_amount)?;

        // Payout the bid funding asset amount to the project account
        Self::release_funding_asset(project_id, &project_details.issuer_account, contribution.funding_asset_amount, contribution.funding_asset)?;

        Contributions::<T>::remove((project_id, contributor, contribution.id));

        Ok(())
    }

    pub fn do_setllement_failure_contributor(contribution: ContributionInfoOf<T>, project_id: ProjectId) -> DispatchResult {
        let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
        ensure!(
			matches!(project_details.status, ProjectStatus::FundingFailed),
			Error::<T>::NotAllowed
		);

        // Check if the bidder has a future deposit held
        let contributor = contribution.contributor;
		
        // Release the held future ct deposit
        Self::release_future_ct_deposit(project_id, &contributor)?;

        // Return the funding assets to the contributor
        Self::release_funding_asset(project_id, &contributor, contribution.funding_asset_amount, contribution.funding_asset)?;

        // Release the held PLMC bond
        Self::release_bond(project_id, &contributor, contribution.plmc_bond)?;


         // Remove the bid from the storage
         Contributions::<T>::remove((project_id, contributor, contribution.id));

         // TODO: Emit an event
 
         Ok(())
    }

    pub fn do_settlement_success_evaluator(evaluation: EvaluationInfoOf<T>, project_id: ProjectId) -> DispatchResult {
        let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
        ensure!(matches!(project_details.status, ProjectStatus::FundingSuccessful), Error::<T>::NotAllowed);

        // Based on the results of the funding round, the evaluator is either:
        // 1. Slashed
        // 2. Rewarded with CT tokens
        // 3. Not slashed or Rewarded.
        let bond = match project_details.evaluation_round_info.evaluators_outcome {
            EvaluatorsOutcome::Slashed => Self::slash_evaluator(project_id, &evaluation)?,
            EvaluatorsOutcome::Rewarded(info) => Self::reward_evaluator(project_id, &evaluation, info)?,
            EvaluatorsOutcome::Unchanged => evaluation.current_plmc_bond,
        };

        // Release the held PLMC bond
        T::NativeCurrency::release(
            &HoldReason::Evaluation(project_id).into(),
            &evaluation.evaluator,
            bond,
            Precision::Exact,
        )?;

        Evaluations::<T>::remove((project_id, evaluation.evaluator, evaluation.id));

        // TODO: Emit an event

        Ok(())
    }

    pub fn do_settlement_failure_evaluator(evaluation: EvaluationInfoOf<T>, project_id: ProjectId) -> DispatchResult {
        let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
        ensure!(
			matches!(project_details.status, ProjectStatus::FundingFailed | ProjectStatus::EvaluationFailed),
			Error::<T>::NotAllowed
		);

        let bond;
        if matches!(project_details.evaluation_round_info.evaluators_outcome, EvaluatorsOutcome::Slashed) {
            bond = Self::slash_evaluator(project_id, &evaluation)?;
        } else {
            bond = evaluation.current_plmc_bond;
        }
        
        // Release the held future ct deposit
        Self::release_future_ct_deposit(project_id, &evaluation.evaluator)?;

        // Release the held PLMC bond
        T::NativeCurrency::release(
			&HoldReason::Evaluation(project_id).into(),
			&evaluation.evaluator,
			bond,
			Precision::Exact,
		)?;

        Evaluations::<T>::remove((project_id, evaluation.evaluator, evaluation.id));

        // TODO: Emit an event

        Ok(())
    }

    fn mint_ct_tokens(project_id: ProjectId, participant: &AccountIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
        if !T::ContributionTokenCurrency::contains(&project_id, participant) {
            Self::release_future_ct_deposit(project_id, participant)?;
            T::ContributionTokenCurrency::touch(project_id, participant.clone(), participant.clone())?;
        }
        T::ContributionTokenCurrency::mint_into(project_id, participant, amount)?;
        Ok(())
    }

    fn release_future_ct_deposit(project_id: ProjectId, participant: &AccountIdOf<T>) -> DispatchResult {
        let held_plmc = T::NativeCurrency::balance_on_hold(&HoldReason::FutureDeposit(project_id).into(), participant);
        ensure!(held_plmc > Zero::zero(), Error::<T>::NoFutureDepositHeld);

        // Return the held deposit to the bidder
        T::NativeCurrency::release(
			&HoldReason::FutureDeposit(project_id).into(),
			participant,
			T::ContributionTokenCurrency::deposit_required(project_id),
			Precision::Exact,
		)?;
        Ok(())
    }

    fn release_funding_asset(project_id: ProjectId, participant: &AccountIdOf<T>, amount: BalanceOf<T>, asset: AcceptedFundingAsset) -> DispatchResult {
        let project_pot = Self::fund_account_id(project_id);
        T::FundingCurrency::transfer(
            asset.to_assethub_id(),
            &project_pot,
            &participant,
            amount,
            Preservation::Expendable,
        )?;
        Ok(())
    }

    fn release_bond(project_id: ProjectId, participant: &AccountIdOf<T>, amount: BalanceOf<T>) -> DispatchResult {
        // Release the held PLMC bond
        T::NativeCurrency::release(
            &HoldReason::Participation(project_id).into(),
            &participant,
            amount,
            Precision::Exact,
        )?;
        Ok(())
    }

    fn slash_evaluator(project_id: ProjectId, evaluation: &EvaluationInfoOf<T>) -> Result<BalanceOf<T>, DispatchError> {
        
        let slash_percentage = T::EvaluatorSlash::get();
		let treasury_account = T::ProtocolGrowthTreasury::get();

		// * Calculate variables *
		// We need to make sure that the current PLMC bond is always >= than the slash amount.
		let slashed_amount = slash_percentage * evaluation.original_plmc_bond;

		T::NativeCurrency::transfer_on_hold(
			&HoldReason::Evaluation(project_id).into(),
			&evaluation.evaluator,
			&treasury_account,
			slashed_amount,
			Precision::Exact,
			Restriction::Free,
			Fortitude::Force,
		)?;

        Ok(evaluation.current_plmc_bond.saturating_sub(slashed_amount))
    }

    fn reward_evaluator(project_id: ProjectId, evaluation: &EvaluationInfoOf<T>, info: RewardInfoOf<T>) -> Result<BalanceOf<T>, DispatchError> {
        
        // * Calculate variables *
		let early_reward_weight = Perquintill::from_rational(
            evaluation.early_usd_amount, 
            info.early_evaluator_total_bonded_usd
        );
        let normal_reward_weight = Perquintill::from_rational(
            evaluation.late_usd_amount.saturating_add(evaluation.early_usd_amount),
            info.normal_evaluator_total_bonded_usd,
        );
        let early_evaluators_rewards = early_reward_weight * info.early_evaluator_reward_pot;
        let normal_evaluators_rewards = normal_reward_weight * info.normal_evaluator_reward_pot;
        let total_reward_amount = early_evaluators_rewards.saturating_add(normal_evaluators_rewards);

        Self::mint_ct_tokens(project_id, &evaluation.evaluator, total_reward_amount)?;

        Ok(evaluation.current_plmc_bond)
    }
}