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

// If you feel like getting in touch with us, you can do so at info@polimec.org
use super::*;

impl<T: Config> Pallet<T> {
    pub fn do_handle_channel_open_request(message: Instruction) -> XcmResult {
		// TODO: set these constants with a proper value
		const EXECUTION_DOT: MultiAsset = MultiAsset {
			id: Concrete(MultiLocation { parents: 0, interior: Here }),
			fun: Fungible(1_0_000_000_000u128),
		};
		const MAX_WEIGHT: Weight = Weight::from_parts(20_000_000_000, 1_000_000);

		let max_message_size_thresholds = T::MaxMessageSizeThresholds::get();
		let max_capacity_thresholds = T::MaxCapacityThresholds::get();

		log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest received: {:?}", message);

		match message {
			Instruction::HrmpNewChannelOpenRequest { sender, max_message_size, max_capacity }
				if max_message_size >= max_message_size_thresholds.0 &&
					max_message_size <= max_message_size_thresholds.1 &&
					max_capacity >= max_capacity_thresholds.0 &&
					max_capacity <= max_capacity_thresholds.1 =>
			{
				log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest accepted");

				let (project_id, mut project_details) = ProjectsDetails::<T>::iter()
					.find(|(_id, details)| {
						details.parachain_id == Some(ParaId::from(sender)) && details.status == FundingSuccessful
					})
					.ok_or(XcmError::BadOrigin)?;

				let accept_channel_relay_call =
					polkadot_runtime::RuntimeCall::Hrmp(polkadot_runtime_parachains::hrmp::Call::<
						polkadot_runtime::Runtime,
					>::hrmp_accept_open_channel {
						sender: ParaId::from(sender),
					})
					.encode();

				let request_channel_relay_call =
					polkadot_runtime::RuntimeCall::Hrmp(polkadot_runtime_parachains::hrmp::Call::<
						polkadot_runtime::Runtime,
					>::hrmp_init_open_channel {
						recipient: ParaId::from(sender),
						proposed_max_capacity: T::RequiredMaxCapacity::get(),
						proposed_max_message_size: T::RequiredMaxMessageSize::get(),
					})
					.encode();

				let xcm: Xcm<()> = Xcm(vec![
					WithdrawAsset(vec![EXECUTION_DOT.clone()].into()),
					BuyExecution { fees: EXECUTION_DOT.clone(), weight_limit: Unlimited },
					Transact {
						origin_kind: OriginKind::Native,
						require_weight_at_most: MAX_WEIGHT,
						call: accept_channel_relay_call.into(),
					},
					Transact {
						origin_kind: OriginKind::Native,
						require_weight_at_most: MAX_WEIGHT,
						call: request_channel_relay_call.into(),
					},
					RefundSurplus,
					DepositAsset {
						assets: Wild(All),
						beneficiary: MultiLocation { parents: 0, interior: X1(Parachain(POLIMEC_PARA_ID)) },
					},
				]);
				let mut message = Some(xcm);

				let dest_loc = MultiLocation { parents: 1, interior: Here };
				let mut destination = Some(dest_loc);
				let (ticket, _price) = T::XcmRouter::validate(&mut destination, &mut message)?;

				match T::XcmRouter::deliver(ticket) {
					Ok(_) => {
						log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest: acceptance successfully sent");
						project_details.hrmp_channel_status.project_to_polimec = ChannelStatus::Open;
						project_details.hrmp_channel_status.polimec_to_project = ChannelStatus::AwaitingAcceptance;
						ProjectsDetails::<T>::insert(project_id, project_details);

						Pallet::<T>::deposit_event(Event::<T>::HrmpChannelAccepted {
							project_id,
							para_id: ParaId::from(sender),
						});
						Ok(())
					},
					Err(e) => {
						log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest: acceptance sending failed - {:?}", e);
						Err(XcmError::Unimplemented)
					},
				}
			},
			instr => {
				log::trace!(target: "pallet_funding::hrmp", "Bad instruction: {:?}", instr);
				Err(XcmError::Unimplemented)
			},
		}
	}

	pub fn do_handle_channel_accepted(message: Instruction) -> XcmResult {
		match message {
			Instruction::HrmpChannelAccepted { recipient } => {
				log::trace!(target: "pallet_funding::hrmp", "HrmpChannelAccepted received: {:?}", message);
				let (project_id, mut project_details) = ProjectsDetails::<T>::iter()
					.find(|(_id, details)| {
						details.parachain_id == Some(ParaId::from(recipient)) && details.status == FundingSuccessful
					})
					.ok_or(XcmError::BadOrigin)?;

				project_details.hrmp_channel_status.polimec_to_project = ChannelStatus::Open;
				ProjectsDetails::<T>::insert(project_id, project_details);
				Pallet::<T>::deposit_event(Event::<T>::HrmpChannelEstablished {
					project_id,
					para_id: ParaId::from(recipient),
				});

				Pallet::<T>::do_start_migration_readiness_check(
					&(T::PalletId::get().into_account_truncating()),
					project_id,
				)
				.map_err(|_| XcmError::NoDeal)?;
				Ok(())
			},
			instr => {
				log::trace!(target: "pallet_funding::hrmp", "Bad instruction: {:?}", instr);
				Err(XcmError::Unimplemented)
			},
		}
	}

	pub fn do_start_migration_readiness_check(
		caller: &AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
	) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let parachain_id: u32 = project_details.parachain_id.ok_or(Error::<T>::ImpossibleState)?.into();
		let project_multilocation = ParentThen(X1(Parachain(parachain_id)));
		let now = <frame_system::Pallet<T>>::block_number();

		// TODO: check these values
		let max_weight = Weight::from_parts(700_000_000, 10_000);

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::NotAllowed);
		ensure!(
			project_details.hrmp_channel_status ==
				HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Open,
					polimec_to_project: ChannelStatus::Open
				},
			Error::<T>::CommsNotEstablished
		);
		if project_details.migration_readiness_check.is_none() {
			ensure!(caller.clone() == T::PalletId::get().into_account_truncating(), Error::<T>::NotAllowed);
		} else if matches!(
			project_details.migration_readiness_check,
			Some(MigrationReadinessCheck {
				holding_check: (_, CheckOutcome::Failed),
				pallet_check: (_, CheckOutcome::Failed),
				..
			})
		) {
			ensure!(caller == &project_details.issuer, Error::<T>::NotAllowed);
		}

		// * Update storage *
		let call: <T as Config>::RuntimeCall =
			Call::migration_check_response { query_id: Default::default(), response: Default::default() }.into();

		let query_id_holdings = pallet_xcm::Pallet::<T>::new_notify_query(
			project_multilocation.clone(),
			call.clone().into(),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);
		let query_id_pallet = pallet_xcm::Pallet::<T>::new_notify_query(
			project_multilocation.clone(),
			call.into(),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);

		project_details.migration_readiness_check = Some(MigrationReadinessCheck {
			holding_check: (query_id_holdings, CheckOutcome::AwaitingResponse),
			pallet_check: (query_id_pallet, CheckOutcome::AwaitingResponse),
		});
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Send the migration query *
		let expected_tokens: MultiAsset =
			(MultiLocation { parents: 0, interior: Here }, 1_000_000_0_000_000_000u128).into(); // 1MM units for migrations
		let xcm = Xcm(vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			WithdrawAsset(vec![expected_tokens].into()),
			ReportHolding {
				response_info: QueryResponseInfo {
					destination: ParentThen(Parachain(POLIMEC_PARA_ID).into()).into(),
					query_id: 0,
					max_weight,
				},
				assets: Wild(All),
			},
			QueryPallet {
				module_name: Vec::from("polimec_receiver"),
				response_info: QueryResponseInfo {
					destination: ParentThen(Parachain(POLIMEC_PARA_ID).into()).into(),
					query_id: 1,
					max_weight,
				},
			},
			DepositAsset { assets: Wild(All), beneficiary: ParentThen(Parachain(POLIMEC_PARA_ID).into()).into() },
		]);
		<pallet_xcm::Pallet<T>>::send_xcm(Here, project_multilocation, xcm).map_err(|_| Error::<T>::XcmFailed)?;

		// * Emit events *
		Self::deposit_event(Event::<T>::MigrationReadinessCheckStarted { project_id, caller: caller.clone() });

		Ok(())
	}

	pub fn do_migration_check_response(
		location: MultiLocation,
		query_id: xcm::v3::QueryId,
		response: xcm::v3::Response,
	) -> DispatchResult {
		use xcm::v3::prelude::*;
		// TODO: check if this is too low performance. Maybe we want a new map of query_id -> project_id
		let (project_id, mut project_details, mut migration_check) = ProjectsDetails::<T>::iter()
			.find_map(|(project_id, details)| {
				if let Some(check @ MigrationReadinessCheck { holding_check, pallet_check }) =
					details.migration_readiness_check
				{
					if holding_check.0 == query_id || pallet_check.0 == query_id {
						return Some((project_id, details, check))
					}
				}
				None
			})
			.ok_or(Error::<T>::NotAllowed)?;

		let para_id = if let MultiLocation { parents: 1, interior: X1(Parachain(para_id)) } = location {
			ParaId::from(para_id)
		} else {
			return Err(Error::<T>::NotAllowed.into())
		};

		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectNotFound)?;
		let contribution_tokens_sold = project_metadata
			.total_allocation_size
			.0
			.saturating_add(project_metadata.total_allocation_size.1)
			.saturating_sub(project_details.remaining_contribution_tokens.0)
			.saturating_sub(project_details.remaining_contribution_tokens.1);

		ensure!(project_details.parachain_id == Some(para_id), Error::<T>::NotAllowed);

		match (response.clone(), migration_check) {
			(
				Response::Assets(assets),
				MigrationReadinessCheck { holding_check: (_, CheckOutcome::AwaitingResponse), .. },
			) => {
				let ct_sold_as_u128: u128 = contribution_tokens_sold.try_into().map_err(|_| Error::<T>::BadMath)?;
				let assets: Vec<MultiAsset> = assets.into_inner();
				let asset_1 = assets[0].clone();
				match asset_1 {
					MultiAsset {
						id: Concrete(MultiLocation { parents: 1, interior: X1(Parachain(pid)) }),
						fun: Fungible(amount),
					} if amount >= ct_sold_as_u128 && pid == u32::from(para_id) => {
						migration_check.holding_check.1 = CheckOutcome::Passed;
						Self::deposit_event(Event::<T>::MigrationCheckResponseAccepted {
							project_id,
							query_id,
							response,
						});
					},
					_ => {
						migration_check.holding_check.1 = CheckOutcome::Failed;
						Self::deposit_event(Event::<T>::MigrationCheckResponseRejected {
							project_id,
							query_id,
							response,
						});
					},
				}
			},

			(
				Response::PalletsInfo(pallets_info),
				MigrationReadinessCheck { pallet_check: (_, CheckOutcome::AwaitingResponse), .. },
			) =>
				if pallets_info.len() == 1 && pallets_info[0] == T::PolimecReceiverInfo::get() {
					migration_check.pallet_check.1 = CheckOutcome::Passed;
					Self::deposit_event(Event::<T>::MigrationCheckResponseAccepted { project_id, query_id, response });
				} else {
					migration_check.pallet_check.1 = CheckOutcome::Failed;
					Self::deposit_event(Event::<T>::MigrationCheckResponseRejected { project_id, query_id, response });
				},
			_ => return Err(Error::<T>::NotAllowed.into()),
		};

		project_details.migration_readiness_check = Some(migration_check);
		ProjectsDetails::<T>::insert(project_id, project_details);
		Ok(())
	}

	pub fn do_start_migration(caller: &AccountIdOf<T>, project_id: T::ProjectIdentifier) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let migration_readiness_check = project_details.migration_readiness_check.ok_or(Error::<T>::NotAllowed)?;

		// * Validity Checks *
		ensure!(caller.clone() == project_details.issuer, Error::<T>::NotAllowed);
		ensure!(migration_readiness_check.is_ready(), Error::<T>::NotAllowed);

		// Start automated migration process

		// * Emit events *
		Self::deposit_event(Event::<T>::MigrationStarted { project_id });

		Ok(())
	}

	pub fn do_migrate_one_participant(
		caller: AccountIdOf<T>,
		project_id: T::ProjectIdentifier,
		participant: AccountIdOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let migration_readiness_check = project_details.migration_readiness_check.ok_or(Error::<T>::NotAllowed)?;
		let user_evaluations = Evaluations::<T>::iter_prefix_values((project_id, participant.clone()));
		let user_bids = Bids::<T>::iter_prefix_values((project_id, participant.clone()));
		let user_contributions = Contributions::<T>::iter_prefix_values((project_id, participant.clone()));
		let project_para_id = project_details.parachain_id.ok_or(Error::<T>::ImpossibleState)?;
		let now = <frame_system::Pallet<T>>::block_number();

		// * Validity Checks *
		ensure!(migration_readiness_check.is_ready(), Error::<T>::NotAllowed);

		// * Process Data *
		// u128 is a balance, u64 is now a BlockNumber, but will be a Moment/Timestamp in the future
		let evaluation_migrations =
			user_evaluations.filter_map(|evaluation| MigrationGenerator::<T>::evaluation_migration(evaluation));
		let bid_migrations = user_bids.filter_map(|bid| MigrationGenerator::<T>::bid_migration(bid));
		let contribution_migrations =
			user_contributions.filter_map(|contribution| MigrationGenerator::<T>::contribution_migration(contribution));

		let migrations = evaluation_migrations.chain(bid_migrations).chain(contribution_migrations).collect_vec();
		let migrations = Migrations::from(migrations);

		let constructed_migrations = Self::construct_migration_xcm_messages(migrations);
		for (migrations, xcm) in constructed_migrations {
			let project_multilocation = MultiLocation { parents: 1, interior: X1(Parachain(project_para_id.into())) };
			let project_migration_origins = ProjectMigrationOriginsOf::<T> {
				project_id,
				migration_origins: migrations
					.origins()
					.try_into()
					.expect("construct function uses same constraint T::MaxMigrationsPerXcm"),
			};

			let call: <T as Config>::RuntimeCall =
				Call::confirm_migrations { query_id: Default::default(), response: Default::default() }.into();
			let transact_response_query_id =
				pallet_xcm::Pallet::<T>::new_notify_query(project_multilocation, call.into(), now + 20u32.into(), Here);
			// TODO: check these values
			let max_weight = Weight::from_parts(700_000_000, 10_000);

			let mut instructions = xcm.into_inner();
			instructions.push(ReportTransactStatus(QueryResponseInfo {
				destination: ParentThen(X1(Parachain(POLIMEC_PARA_ID))).into(),
				query_id: transact_response_query_id,
				max_weight,
			}));
			let xcm = Xcm(instructions);

			<pallet_xcm::Pallet<T>>::send_xcm(Here, project_multilocation, xcm).map_err(|_| Error::<T>::XcmFailed)?;
			Self::mark_migrations_as_sent(project_migration_origins.clone(), transact_response_query_id);
			UnconfirmedMigrations::<T>::insert(transact_response_query_id, project_migration_origins);

			Self::deposit_event(Event::<T>::UserMigrationSent {
				project_id,
				caller: caller.clone(),
				participant: participant.clone(),
			});
		}
		Ok(())
	}

	pub fn do_confirm_migrations(location: MultiLocation, query_id: QueryId, response: Response) -> DispatchResult {
		use xcm::v3::prelude::*;
		let unconfirmed_migrations = UnconfirmedMigrations::<T>::take(query_id).ok_or(Error::<T>::NotAllowed)?;
		let project_id = unconfirmed_migrations.project_id;
		let para_id = if let MultiLocation { parents: 1, interior: X1(Parachain(para_id)) } = location {
			ParaId::from(para_id)
		} else {
			return Err(Error::<T>::NotAllowed.into())
		};
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(project_details.parachain_id == Some(para_id), Error::<T>::NotAllowed);

		match response {
			Response::DispatchResult(MaybeErrorCode::Success) => {
				Self::mark_migrations_as_confirmed(unconfirmed_migrations.clone());
				Self::deposit_event(Event::MigrationsConfirmed {
					project_id,
					migration_origins: unconfirmed_migrations.migration_origins,
				});
				// Self::deposit_event(Event::MigrationsConfirmed { project_id });
				Ok(())
			},
			Response::DispatchResult(MaybeErrorCode::Error(e)) |
			Response::DispatchResult(MaybeErrorCode::TruncatedError(e)) => {
				Self::mark_migrations_as_failed(unconfirmed_migrations.clone(), e);
				Self::deposit_event(Event::MigrationsFailed {
					project_id,
					migration_origins: unconfirmed_migrations.migration_origins,
				});
				// Self::deposit_event(Event::MigrationsFailed { project_id});
				Ok(())
			},
			_ => Err(Error::<T>::NotAllowed.into()),
		}
	}

    pub fn migrations_per_xcm_message_allowed() -> u32 {
		const MAX_WEIGHT: Weight = Weight::from_parts(20_000_000_000, 1_000_000);

		let one_migration_bytes = (0u128, 0u64).encode().len() as u32;

		// our encoded call starts with pallet index 51, and call index 0
		let mut encoded_call = vec![51u8, 0];
		let encoded_first_param = [0u8; 32].encode();
		let encoded_second_param = Vec::<MigrationInfo>::new().encode();
		// we append the encoded parameters, with our migrations vec being empty for now
		encoded_call.extend_from_slice(encoded_first_param.as_slice());
		encoded_call.extend_from_slice(encoded_second_param.as_slice());

		let base_xcm_message: Xcm<()> = Xcm(vec![
			UnpaidExecution { weight_limit: WeightLimit::Unlimited, check_origin: None },
			Transact { origin_kind: OriginKind::Native, require_weight_at_most: MAX_WEIGHT, call: encoded_call.into() },
			ReportTransactStatus(QueryResponseInfo {
				destination: Parachain(3344).into(),
				query_id: 0,
				max_weight: MAX_WEIGHT,
			}),
		]);
		let xcm_size = base_xcm_message.encode().len();

		let available_bytes_for_migration_per_message =
			T::RequiredMaxMessageSize::get().saturating_sub(xcm_size as u32);

		available_bytes_for_migration_per_message.saturating_div(one_migration_bytes)
	}

    pub fn mark_migrations_as_sent(project_migration_origins: ProjectMigrationOriginsOf<T>, query_id: QueryId) {
		let project_id = project_migration_origins.project_id;
		let migration_origins = project_migration_origins.migration_origins;
		for MigrationOrigin { user, id, participation_type } in migration_origins {
			match participation_type {
				ParticipationType::Evaluation => {
					Evaluations::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_evaluation| {
							if let Some(evaluation) = maybe_evaluation {
								evaluation.ct_migration_status = MigrationStatus::Sent(query_id);
							}
						},
					);
				},
				ParticipationType::Bid => {
					Bids::<T>::mutate((project_id, T::AccountId32Conversion::convert_back(user), id), |maybe_bid| {
						if let Some(bid) = maybe_bid {
							bid.ct_migration_status = MigrationStatus::Sent(query_id);
						}
					});
				},
				ParticipationType::Contribution => {
					Contributions::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_contribution| {
							if let Some(contribution) = maybe_contribution {
								contribution.ct_migration_status = MigrationStatus::Sent(query_id);
							}
						},
					);
				},
			}
		}
	}

	pub fn mark_migrations_as_confirmed(project_migration_origins: ProjectMigrationOriginsOf<T>) {
		let project_id = project_migration_origins.project_id;
		let migration_origins = project_migration_origins.migration_origins;
		for MigrationOrigin { user, id, participation_type } in migration_origins {
			match participation_type {
				ParticipationType::Evaluation => {
					Evaluations::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_evaluation| {
							if let Some(evaluation) = maybe_evaluation {
								evaluation.ct_migration_status = MigrationStatus::Confirmed;
							}
						},
					);
				},
				ParticipationType::Bid => {
					Bids::<T>::mutate((project_id, T::AccountId32Conversion::convert_back(user), id), |maybe_bid| {
						if let Some(bid) = maybe_bid {
							bid.ct_migration_status = MigrationStatus::Confirmed;
						}
					});
				},
				ParticipationType::Contribution => {
					Contributions::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_contribution| {
							if let Some(contribution) = maybe_contribution {
								contribution.ct_migration_status = MigrationStatus::Confirmed;
							}
						},
					);
				},
			}
		}
	}

	pub fn mark_migrations_as_failed(
		project_migration_origins: ProjectMigrationOriginsOf<T>,
		error: BoundedVec<u8, MaxDispatchErrorLen>,
	) {
		let project_id = project_migration_origins.project_id;
		let migration_origins = project_migration_origins.migration_origins;
		for MigrationOrigin { user, id, participation_type } in migration_origins {
			match participation_type {
				ParticipationType::Evaluation => {
					Evaluations::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_evaluation| {
							if let Some(evaluation) = maybe_evaluation {
								evaluation.ct_migration_status = MigrationStatus::Failed(error.clone());
							}
						},
					);
				},
				ParticipationType::Bid => {
					Bids::<T>::mutate((project_id, T::AccountId32Conversion::convert_back(user), id), |maybe_bid| {
						if let Some(bid) = maybe_bid {
							bid.ct_migration_status = MigrationStatus::Failed(error.clone());
						}
					});
				},
				ParticipationType::Contribution => {
					Contributions::<T>::mutate(
						(project_id, T::AccountId32Conversion::convert_back(user), id),
						|maybe_contribution| {
							if let Some(contribution) = maybe_contribution {
								contribution.ct_migration_status = MigrationStatus::Failed(error.clone());
							}
						},
					);
				},
			}
		}
	}

}