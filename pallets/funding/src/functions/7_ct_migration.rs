use super::*;
use xcm::v3::MaxPalletNameLen;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_set_para_id_for_project(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		para_id: ParaId,
	) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(&(project_details.issuer_account) == caller, Error::<T>::NotIssuer);

		// * Update storage *
		project_details.parachain_id = Some(para_id);
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::ProjectParaIdSet { project_id, para_id });

		Ok(())
	}

	/// Handle the channel open request from the relay on behalf of a parachain.
	/// If the para id belongs to a funded project with that id, then send an acceptance message and a request for a
	/// channel in the opposite direction to the relay.
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

				let mut accept_channel_relay_call = vec![60u8, 1];
				let sender_id = ParaId::from(sender).encode();
				accept_channel_relay_call.extend_from_slice(&sender_id);

				let mut request_channel_relay_call = vec![60u8, 0];
				let recipient = ParaId::from(sender).encode();
				request_channel_relay_call.extend_from_slice(&recipient);
				let proposed_max_capacity = T::RequiredMaxCapacity::get().encode();
				request_channel_relay_call.extend_from_slice(&proposed_max_capacity);
				let proposed_max_message_size = T::RequiredMaxMessageSize::get().encode();
				request_channel_relay_call.extend_from_slice(&proposed_max_message_size);

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

	/// Handle the channel accepted message of project->polimec from the relay on behalf of the project parachain.
	/// Start the migration readiness check for the project.
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

	/// After the bidirectional HRMP channels are established, check that the project chain has the receiver pallet,
	/// and has minted the amount of CTs sold to the polimec sovereign account.
	#[transactional]
	pub fn do_start_migration_readiness_check(caller: &AccountIdOf<T>, project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let parachain_id: u32 = project_details.parachain_id.ok_or(Error::<T>::ImpossibleState)?.into();
		let project_multilocation = ParentThen(X1(Parachain(parachain_id)));
		let now = <frame_system::Pallet<T>>::block_number();

		// TODO: check these values
		let max_weight = Weight::from_parts(700_000_000, 10_000);

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::FundingSuccessful, Error::<T>::IncorrectRound);
		ensure!(
			project_details.hrmp_channel_status ==
				HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Open,
					polimec_to_project: ChannelStatus::Open
				},
			Error::<T>::ChannelNotOpen
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
			ensure!(caller == &project_details.issuer_account, Error::<T>::NotIssuer);
		}

		// * Update storage *
		let call = Call::<T>::migration_check_response { query_id: Default::default(), response: Default::default() };

		let query_id_holdings = pallet_xcm::Pallet::<T>::new_notify_query(
			project_multilocation.clone(),
			<T as Config>::RuntimeCall::from(call.clone()),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);
		let query_id_pallet = pallet_xcm::Pallet::<T>::new_notify_query(
			project_multilocation.clone(),
			<T as Config>::RuntimeCall::from(call),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);

		project_details.migration_readiness_check = Some(MigrationReadinessCheck {
			holding_check: (query_id_holdings, CheckOutcome::AwaitingResponse),
			pallet_check: (query_id_pallet, CheckOutcome::AwaitingResponse),
		});
		ProjectsDetails::<T>::insert(project_id, project_details.clone());

		let total_cts_minted = <T as Config>::ContributionTokenCurrency::total_issuance(project_id);

		// * Send the migration query *
		let expected_tokens: MultiAsset =
			(MultiLocation { parents: 0, interior: Here }, total_cts_minted.into()).into();
		log::info!("expected_tokens sold for migrations: {:?}", total_cts_minted);
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

	/// Handle the migration readiness check response from the project chain.
	#[transactional]
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
						return Some((project_id, details, check));
					}
				}
				None
			})
			.ok_or(Error::<T>::ProjectDetailsNotFound)?;

		let para_id = if let MultiLocation { parents: 1, interior: X1(Parachain(para_id)) } = location {
			ParaId::from(para_id)
		} else {
			return Err(Error::<T>::WrongParaId.into());
		};

		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let contribution_tokens_sold =
			project_metadata.total_allocation_size.saturating_sub(project_details.remaining_contribution_tokens);
		ensure!(project_details.parachain_id == Some(para_id), Error::<T>::WrongParaId);

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
						migration_check.holding_check.1 = CheckOutcome::Passed(None);
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
			) => {
				let expected_module_name: BoundedVec<u8, MaxPalletNameLen> =
					BoundedVec::try_from("polimec_receiver".as_bytes().to_vec()).map_err(|_| Error::<T>::NotAllowed)?;
				let Some(PalletInfo { index, module_name, .. }) = pallets_info.first() else {
					return Err(Error::<T>::NotAllowed.into());
				};
				let u8_index: u8 = (*index).try_into().map_err(|_| Error::<T>::NotAllowed)?;
				if pallets_info.len() == 1 && module_name == &expected_module_name {
					migration_check.pallet_check.1 = CheckOutcome::Passed(Some(u8_index));
					Self::deposit_event(Event::<T>::MigrationCheckResponseAccepted { project_id, query_id, response });
				} else {
					migration_check.pallet_check.1 = CheckOutcome::Failed;
					Self::deposit_event(Event::<T>::MigrationCheckResponseRejected { project_id, query_id, response });
				}
			},
			_ => return Err(Error::<T>::NotAllowed.into()),
		};

		project_details.migration_readiness_check = Some(migration_check);
		ProjectsDetails::<T>::insert(project_id, project_details);
		Ok(())
	}

	/// Migrate all the CTs of a project for a single participant
	/// This entails transferring the funds from the polimec sovereign account to the participant account, and applying
	/// a vesting schedule if necessary.
	#[transactional]
	pub fn do_migrate_one_participant(project_id: ProjectId, participant: AccountIdOf<T>) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let migration_readiness_check = project_details.migration_readiness_check.ok_or(Error::<T>::ChannelNotReady)?;
		let project_para_id = project_details.parachain_id.ok_or(Error::<T>::ImpossibleState)?;
		let now = <frame_system::Pallet<T>>::block_number();
		ensure!(
			Self::user_has_no_participations(project_id, participant.clone()),
			Error::<T>::ParticipationsNotSettled
		);
		let (_, migrations) =
			UserMigrations::<T>::get(project_id, participant.clone()).ok_or(Error::<T>::NoMigrationsFound)?;

		// * Validity Checks *
		ensure!(migration_readiness_check.is_ready(), Error::<T>::ChannelNotReady);

		let project_multilocation = MultiLocation { parents: 1, interior: X1(Parachain(project_para_id.into())) };
		let call: <T as Config>::RuntimeCall =
			Call::confirm_migrations { query_id: Default::default(), response: Default::default() }.into();
		let query_id =
			pallet_xcm::Pallet::<T>::new_notify_query(project_multilocation, call.into(), now + 20u32.into(), Here);

		let CheckOutcome::Passed(Some(pallet_index)) = migration_readiness_check.pallet_check.1 else {
			return Err(Error::<T>::NotAllowed.into());
		};

		Self::change_migration_status(project_id, participant.clone(), MigrationStatus::Sent(query_id))?;

		// * Process Data *
		let xcm = Self::construct_migration_xcm_message(migrations.into(), query_id, pallet_index);

		<pallet_xcm::Pallet<T>>::send_xcm(Here, project_multilocation, xcm).map_err(|_| Error::<T>::XcmFailed)?;
		ActiveMigrationQueue::<T>::insert(query_id, (project_id, participant.clone()));

		Self::deposit_event(Event::<T>::MigrationStatusUpdated {
			project_id,
			account: participant,
			status: MigrationStatus::Sent(query_id),
		});

		Ok(())
	}

	/// Mark the migration item that corresponds to a single participation as confirmed or failed.
	#[transactional]
	pub fn do_confirm_migrations(location: MultiLocation, query_id: QueryId, response: Response) -> DispatchResult {
		use xcm::v3::prelude::*;
		let (project_id, participant) =
			ActiveMigrationQueue::<T>::take(query_id).ok_or(Error::<T>::NoActiveMigrationsFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(
			matches!(location, MultiLocation { parents: 1, interior: X1(Parachain(para_id))} if Some(ParaId::from(para_id)) == project_details.parachain_id),
			Error::<T>::WrongParaId
		);


		let status = match response {
			Response::DispatchResult(MaybeErrorCode::Success) => {
				Self::change_migration_status(project_id, participant.clone(), MigrationStatus::Confirmed)?;
				MigrationStatus::Confirmed
			},
			Response::DispatchResult(MaybeErrorCode::Error(_)) |
			Response::DispatchResult(MaybeErrorCode::TruncatedError(_)) => {
				Self::change_migration_status(project_id, participant.clone(), MigrationStatus::Failed)?;
				MigrationStatus::Failed
			},
			_ => return Err(Error::<T>::NotAllowed.into()),
		};
		Self::deposit_event(Event::<T>::MigrationStatusUpdated { project_id, account: participant, status });
		Ok(())
	}
}
