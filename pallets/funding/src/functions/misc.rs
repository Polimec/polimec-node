#[allow(clippy::wildcard_imports)]
use super::*;
use alloc::string::{String, ToString};
use polimec_common::{assets::AcceptedFundingAsset, ProvideAssetPrice};
use sp_core::{
	ecdsa::{Public as EcdsaPublic, Signature as EcdsaSignature},
	keccak_256,
	sr25519::{Public as SrPublic, Signature as SrSignature},
	ByteArray,
};
use sp_runtime::traits::Verify;

// Helper functions
// ATTENTION: if this is called directly, it will not be transactional
impl<T: Config> Pallet<T> {
	/// The account ID of the project pot.
	///
	/// This actually does computation. If you need to keep using it, then make sure you cache the
	/// value and only call this once.
	#[inline(always)]
	pub fn fund_account_id(index: ProjectId) -> AccountIdOf<T> {
		// since the project_id starts at 0, we need to add 1 to get a different sub_account than the pallet account.
		T::PalletId::get().into_sub_account_truncating(index.saturating_add(One::one()))
	}

	pub fn create_bucket_from_metadata(metadata: &ProjectMetadataOf<T>) -> Result<BucketOf<T>, DispatchError> {
		let auction_allocation_size = metadata.total_allocation_size;
		let bucket_delta_amount = Percent::from_percent(10) * auction_allocation_size;
		let ten_percent_in_price: <T as Config>::Price =
			PriceOf::<T>::checked_from_rational(1, 10).ok_or(Error::<T>::BadMath)?;
		let bucket_delta_price: <T as Config>::Price = metadata.minimum_price.saturating_mul(ten_percent_in_price);

		let bucket: BucketOf<T> =
			Bucket::new(auction_allocation_size, metadata.minimum_price, bucket_delta_price, bucket_delta_amount);

		Ok(bucket)
	}

	pub fn calculate_plmc_bond(ticket_size: Balance, multiplier: MultiplierOf<T>) -> Result<Balance, DispatchError> {
		let plmc_usd_price =
			<PriceProviderOf<T>>::get_decimals_aware_price(Location::here(), USD_DECIMALS, PLMC_DECIMALS)
				.ok_or(Error::<T>::PriceNotFound)?;
		let usd_bond = multiplier.calculate_usd_bonding_requirement::<T>(ticket_size).ok_or(Error::<T>::BadMath)?;
		plmc_usd_price
			.reciprocal()
			.ok_or(Error::<T>::BadMath)?
			.checked_mul_int(usd_bond)
			.ok_or(Error::<T>::BadMath.into())
	}

	pub fn calculate_funding_asset_amount(
		ticket_size: Balance,
		asset_id: AcceptedFundingAsset,
	) -> Result<Balance, DispatchError> {
		let asset_id = asset_id.id();
		let asset_decimals = T::FundingCurrency::decimals(asset_id.clone());
		let asset_usd_price = <PriceProviderOf<T>>::get_decimals_aware_price(asset_id, USD_DECIMALS, asset_decimals)
			.ok_or(Error::<T>::PriceNotFound)?;
		asset_usd_price
			.reciprocal()
			.and_then(|recip| recip.checked_mul_int(ticket_size))
			.ok_or(Error::<T>::BadMath.into())
	}

	// Based on the amount of tokens and price to buy, a desired multiplier, and the type of investor the caller is,
	/// calculate the amount and vesting periods of bonded PLMC and reward CT tokens.
	pub fn calculate_vesting_info(
		_caller: &AccountIdOf<T>,
		multiplier: MultiplierOf<T>,
		bonded_amount: Balance,
	) -> Result<VestingInfo<BlockNumberFor<T>>, DispatchError> {
		let duration: BlockNumberFor<T> = multiplier.calculate_vesting_duration::<T>();
		let duration_as_balance = T::BlockNumberToBalance::convert(duration);
		let amount_per_block = if duration_as_balance == Zero::zero() {
			bonded_amount
		} else {
			bonded_amount.checked_div(duration_as_balance).ok_or(Error::<T>::BadMath)?
		};

		Ok(VestingInfo { total_amount: bonded_amount, amount_per_block, duration })
	}

	pub fn bond_plmc_with_mode(
		who: &T::AccountId,
		project_id: ProjectId,
		amount: Balance,
		mode: ParticipationMode,
		asset: AcceptedFundingAsset,
	) -> DispatchResult {
		match mode {
			ParticipationMode::Classic(_) => Self::try_plmc_participation_lock(who, project_id, amount),
			ParticipationMode::OTM => pallet_proxy_bonding::Pallet::<T>::bond_on_behalf_of(
				project_id,
				who.clone(),
				amount,
				asset.id(),
				HoldReason::Participation.into(),
			),
		}
	}

	pub fn try_plmc_participation_lock(who: &T::AccountId, project_id: ProjectId, amount: Balance) -> DispatchResult {
		// Check if the user has already locked tokens in the evaluation period
		let user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, who));

		let mut to_convert = amount;
		for mut evaluation in user_evaluations {
			if to_convert == Zero::zero() {
				break;
			}
			let slash_deposit = <T as Config>::EvaluatorSlash::get() * evaluation.original_plmc_bond;
			let available_to_convert = evaluation.current_plmc_bond.saturating_sub(slash_deposit);
			let converted = to_convert.min(available_to_convert);
			evaluation.current_plmc_bond = evaluation.current_plmc_bond.saturating_sub(converted);
			Evaluations::<T>::insert((project_id, who, evaluation.id), evaluation);
			T::NativeCurrency::release(&HoldReason::Evaluation.into(), who, converted, Precision::Exact)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			T::NativeCurrency::hold(&HoldReason::Participation.into(), who, converted)
				.map_err(|_| Error::<T>::ImpossibleState)?;
			to_convert = to_convert.saturating_sub(converted)
		}

		T::NativeCurrency::hold(&HoldReason::Participation.into(), who, to_convert)
			.map_err(|_| Error::<T>::ParticipantNotEnoughFunds)?;

		Ok(())
	}

	// TODO(216): use the hold interface of the fungibles::MutateHold once its implemented on pallet_assets.
	pub fn try_funding_asset_hold(
		who: &T::AccountId,
		project_id: ProjectId,
		amount: Balance,
		asset_id: AssetIdOf<T>,
	) -> DispatchResult {
		let fund_account = Self::fund_account_id(project_id);

		T::FundingCurrency::transfer(asset_id, who, &fund_account, amount, Preservation::Preserve)
			.map_err(|_| Error::<T>::ParticipantNotEnoughFunds)?;

		Ok(())
	}

	// Calculate the total fee allocation for a project, based on the funding reached.
	fn calculate_fee_allocation(project_id: ProjectId) -> Result<Balance, DispatchError> {
		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let bucket = Buckets::<T>::get(project_id).ok_or(Error::<T>::BucketNotFound)?;
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// Determine how much funding has been achieved.
		let funding_amount_reached = project_details.funding_amount_reached_usd;
		let fee_usd = Self::compute_total_fee_from_brackets(funding_amount_reached);
		let fee_percentage = Perquintill::from_rational(fee_usd, funding_amount_reached);

		let token_sold = if bucket.current_price == bucket.initial_price {
			project_metadata.total_allocation_size.saturating_sub(bucket.amount_left)
		} else {
			project_metadata.total_allocation_size
		};
		let total_fee_allocation = fee_percentage * token_sold;

		Ok(total_fee_allocation)
	}

	/// Computes the total fee from all defined fee brackets.
	fn compute_total_fee_from_brackets(funding_reached: Balance) -> Balance {
		let mut remaining_for_fee = funding_reached;

		T::FeeBrackets::get()
			.into_iter()
			.map(|(fee, limit)| Self::compute_fee_for_bracket(&mut remaining_for_fee, fee, limit))
			.fold(Balance::zero(), |acc, fee| acc.saturating_add(fee))
	}

	/// Calculate the fee for a particular bracket.
	fn compute_fee_for_bracket(remaining_for_fee: &mut Balance, fee: Percent, limit: Balance) -> Balance {
		if let Some(amount_to_bid) = remaining_for_fee.checked_sub(limit) {
			*remaining_for_fee = amount_to_bid;
			fee * limit
		} else {
			let fee_for_this_bracket = fee * *remaining_for_fee;
			*remaining_for_fee = Balance::zero();
			fee_for_this_bracket
		}
	}

	/// Generate and return evaluator rewards based on a project's funding status.
	///
	/// The function calculates rewards based on several metrics: funding achieved,
	/// total allocations, and issuer fees. It also differentiates between early and
	/// normal evaluators for reward distribution.
	///
	/// Note: Consider refactoring the `RewardInfo` struct to make it more generic and
	/// reusable, not just for evaluator rewards.
	pub fn generate_evaluator_rewards_info(project_id: ProjectId) -> Result<RewardInfo, DispatchError> {
		// Fetching the necessary data for a specific project.
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let total_fee_allocation = Self::calculate_fee_allocation(project_id)?;

		// Calculate rewards.
		let evaluator_rewards = Perquintill::from_percent(30) * total_fee_allocation;

		// Distribute rewards between early and normal evaluators.
		let early_evaluator_reward_pot = Perquintill::from_percent(20) * evaluator_rewards;
		let normal_evaluator_reward_pot = Perquintill::from_percent(80) * evaluator_rewards;

		let normal_evaluator_total_bonded_usd = project_details.evaluation_round_info.total_bonded_usd;
		let early_evaluation_reward_threshold_usd =
			T::EvaluationSuccessThreshold::get() * project_details.fundraising_target_usd;
		let early_evaluator_total_bonded_usd =
			normal_evaluator_total_bonded_usd.min(early_evaluation_reward_threshold_usd);

		// Construct the reward information object.
		let reward_info = RewardInfo {
			early_evaluator_reward_pot,
			normal_evaluator_reward_pot,
			early_evaluator_total_bonded_usd,
			normal_evaluator_total_bonded_usd,
		};

		Ok(reward_info)
	}

	pub fn generate_liquidity_pools_and_long_term_holder_rewards(
		project_id: ProjectId,
	) -> Result<(Balance, Balance), DispatchError> {
		let total_fee_allocation = Self::calculate_fee_allocation(project_id)?;

		let liquidity_pools_percentage = Perquintill::from_percent(50);
		let liquidity_pools_reward_pot = liquidity_pools_percentage * total_fee_allocation;

		let long_term_holder_percentage = Perquintill::from_percent(20);
		let long_term_holder_reward_pot = long_term_holder_percentage * total_fee_allocation;

		Ok((liquidity_pools_reward_pot, long_term_holder_reward_pot))
	}

	pub fn change_migration_status(
		project_id: ProjectId,
		user: T::AccountId,
		status: MigrationStatus,
	) -> DispatchResult {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let (current_status, migrations) =
			UserMigrations::<T>::get((project_id, user.clone())).ok_or(Error::<T>::NoMigrationsFound)?;

		ensure!(current_status == MigrationStatus::NotStarted, Error::<T>::MigrationAlreadyConfirmed);

		UnmigratedCounter::<T>::mutate(project_id, |counter| *counter = counter.saturating_sub(1));
		UserMigrations::<T>::insert((project_id, user), (status, migrations));
		ProjectsDetails::<T>::insert(project_id, project_details);

		Ok(())
	}

	pub(crate) fn transition_project(
		project_id: ProjectId,
		mut project_details: ProjectDetailsOf<T>,
		current_round: ProjectStatus,
		next_round: ProjectStatus,
		maybe_round_duration: Option<BlockNumberFor<T>>,
		skip_end_check: bool,
	) -> DispatchResult {
		/* Verify */
		let now = <frame_system::Pallet<T>>::block_number();
		ensure!(project_details.status == current_round, Error::<T>::IncorrectRound);
		ensure!(project_details.round_duration.ended(now) || skip_end_check, Error::<T>::TooEarlyForRound);

		let round_end =
			maybe_round_duration.map(|round_duration| now.saturating_add(round_duration).saturating_sub(One::one()));
		project_details.round_duration = BlockNumberPair::new(Some(now), round_end);
		project_details.status = next_round.clone();

		// * Update storage *
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::ProjectPhaseTransition { project_id, phase: next_round });

		Ok(())
	}

	pub fn get_substrate_message_to_sign(polimec_account: AccountIdOf<T>, project_id: ProjectId) -> Option<String> {
		let mut message = String::new();

		let polimec_account_ss58_string = T::SS58Conversion::convert(polimec_account.clone());
		let project_id_string = project_id.to_string();
		let nonce_string = frame_system::Pallet::<T>::account_nonce(polimec_account).to_string();

		use alloc::fmt::Write;
		write!(
			&mut message,
			"Polimec account: {} - project id: {} - nonce: {}",
			polimec_account_ss58_string, project_id_string, nonce_string
		)
		.ok()?;
		Some(message)
	}

	pub fn verify_ethereum_account(
		mut signature_bytes: [u8; 65],
		expected_ethereum_account: [u8; 20],
		polimec_account: AccountIdOf<T>,
		project_id: ProjectId,
	) -> bool {
		match signature_bytes[64] {
			27 => signature_bytes[64] = 0x00,
			28 => signature_bytes[64] = 0x01,
			_v => return false,
		}

		let hashed_message = typed_data_v4::get_eip_712_message(
			&T::SS58Conversion::convert(polimec_account.clone()),
			project_id,
			frame_system::Pallet::<T>::account_nonce(polimec_account),
		);

		let ecdsa_signature = EcdsaSignature::from_slice(&signature_bytes).unwrap();
		let public_compressed: EcdsaPublic = ecdsa_signature.recover_prehashed(&hashed_message).unwrap();
		let public_uncompressed = k256::ecdsa::VerifyingKey::from_sec1_bytes(&public_compressed).unwrap();
		let public_uncompressed_point = public_uncompressed.to_encoded_point(false).to_bytes();
		let derived_ethereum_account: [u8; 20] =
			keccak_256(&public_uncompressed_point[1..])[12..32].try_into().unwrap();

		derived_ethereum_account == expected_ethereum_account
	}

	pub fn verify_substrate_account(
		signature_bytes: [u8; 65],
		expected_substrate_account: [u8; 32],
		polimec_account: AccountIdOf<T>,
		project_id: ProjectId,
	) -> bool {
		let message_to_sign = Self::get_substrate_message_to_sign(polimec_account.clone(), project_id).unwrap();
		let message_bytes = message_to_sign.into_bytes();
		let signature = SrSignature::from_slice(&signature_bytes[..64]).unwrap();
		let public = SrPublic::from_slice(&expected_substrate_account).unwrap();
		signature.verify(message_bytes.as_slice(), &public)
	}

	pub fn verify_receiving_account_signature(
		polimec_account: &AccountIdOf<T>,
		project_id: ProjectId,
		receiver_account: &Junction,
		signature_bytes: [u8; 65],
	) -> DispatchResult {
		match receiver_account {
			Junction::AccountId32 { id: substrate_account, .. } => {
				ensure!(
					Self::verify_substrate_account(
						signature_bytes,
						*substrate_account,
						polimec_account.clone(),
						project_id
					),
					Error::<T>::BadReceiverAccountSignature
				);
			},

			Junction::AccountKey20 { key: expected_ethereum_account, .. } => {
				ensure!(
					Self::verify_ethereum_account(
						signature_bytes,
						*expected_ethereum_account,
						polimec_account.clone(),
						project_id,
					),
					Error::<T>::BadReceiverAccountSignature
				);
			},
			_ => return Err(Error::<T>::UnsupportedReceiverAccountJunction.into()),
		};
		Ok(())
	}

	pub fn get_decimals_aware_funding_asset_price(funding_asset: &AcceptedFundingAsset) -> Option<PriceOf<T>> {
		let funding_asset_id = funding_asset.id();
		let funding_asset_decimals = T::FundingCurrency::decimals(funding_asset_id.clone());
		<PriceProviderOf<T>>::get_decimals_aware_price(funding_asset_id, USD_DECIMALS, funding_asset_decimals)
	}
}

pub mod typed_data_v4 {
	use super::*;
	use hex_literal::hex;

	/// Returns the first part needed for a typed data v4 message. It specifies the entity that requires the signature.
	pub fn get_domain_separator(name: &str, version: &str, chain_id: u32, verifying_contract: [u8; 20]) -> [u8; 32] {
		/// EIP-712 domain separator calculation
		/// RFC: https://eips.ethereum.org/EIPS/eip-712
		// keccak_256(b"EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)");
		const DOMAIN_TYPE_HASH: [u8; 32] = hex!("8b73c3c69bb8fe3d512ecc4cf759cc79239f7b179b0ffacaa9a75d522b39400f");

		let mut data = [0u8; 32 * 5];

		// Copy pre-computed domain type hash
		data[..32].copy_from_slice(&DOMAIN_TYPE_HASH);

		// Hash and copy name
		let name_hash = keccak_256(name.as_bytes());
		data[32..64].copy_from_slice(&name_hash);

		// Hash and copy version
		let version_hash = keccak_256(version.as_bytes());
		data[64..96].copy_from_slice(&version_hash);

		// Convert chain_id to big-endian bytes and pad left with zeros
		let chain_id_bytes: [u8; 4] = chain_id.to_be_bytes();
		data[124..128].copy_from_slice(&chain_id_bytes);

		// Copy contract address with proper padding
		data[140..160].copy_from_slice(&verifying_contract);

		// Calculate final hash
		keccak_256(&data)
	}

	/// Returns the second part needed for a typed data v4 message. It specifies the message details with type information.
	pub fn get_message(polimec_account: &str, project_id: u32, nonce: u32) -> [u8; 32] {
		// keccak_256(b"ParticipationAuthorization(string polimecAccount,uint32 projectId,uint32 nonce)");
		const DOMAIN_TYPE_HASH: [u8; 32] = hex!("fd56e61c83fe04559072170cf0f970f35093edf39291b74939109a1c9ded28a3");

		let mut data = [0u8; 32 * 4];

		data[..32].copy_from_slice(&DOMAIN_TYPE_HASH);

		let account_hash = keccak_256(polimec_account.as_bytes());
		data[32..64].copy_from_slice(&account_hash);

		let project_id_bytes: [u8; 4] = project_id.to_be_bytes();
		data[92..96].copy_from_slice(&project_id_bytes);

		let nonce_bytes: [u8; 4] = nonce.to_be_bytes();
		data[124..128].copy_from_slice(&nonce_bytes);

		keccak_256(&data)
	}

	/// Returns the final message hash that will be signed by the user.
	pub fn get_eip_712_message(polimec_account: &str, project_id: u32, nonce: u32) -> [u8; 32] {
		let domain_separator =
			get_domain_separator("Polimec", "1", 1, hex!("0000000000000000000000000000000000003344"));
		let message = get_message(polimec_account, project_id, nonce);

		let mut data = [0u8; 32 * 2 + 2];
		data[0..2].copy_from_slice(b"\x19\x01");
		data[2..34].copy_from_slice(&domain_separator);
		data[34..66].copy_from_slice(&message);

		keccak_256(&data)
	}
}
