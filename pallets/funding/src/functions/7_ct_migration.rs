#[allow(clippy::wildcard_imports)]
use super::*;
use xcm::v4::MaxPalletNameLen;

// Offchain migration functions
impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_start_offchain_migration(project_id: ProjectId, caller: AccountIdOf<T>) -> DispatchResult {
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(project_details.issuer_account == caller, Error::<T>::NotIssuer);

		project_details.migration_type = Some(MigrationType::Offchain);

		Self::transition_project(
			project_id,
			project_details,
			ProjectStatus::SettlementFinished(FundingOutcome::Success),
			ProjectStatus::CTMigrationStarted,
			None,
			false,
		)?;

		Ok(())
	}

	#[transactional]
	pub fn do_confirm_offchain_migration(
		project_id: ProjectId,
		caller: AccountIdOf<T>,
		participant: AccountIdOf<T>,
	) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::CTMigrationStarted, Error::<T>::IncorrectRound);
		ensure!(project_details.issuer_account == caller, Error::<T>::NotIssuer);

		// * Update storage *
		Self::change_migration_status(project_id, participant.clone(), MigrationStatus::Confirmed)?;

		Ok(())
	}
}

// Pallet migration functions
impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_start_pallet_migration(
		caller: &AccountIdOf<T>,
		project_id: ProjectId,
		para_id: ParaId,
	) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(&(project_details.issuer_account) == caller, Error::<T>::NotIssuer);
		match project_details.status {
			ProjectStatus::SettlementFinished(FundingOutcome::Success) => (),
			ProjectStatus::FundingSuccessful | ProjectStatus::SettlementStarted(FundingOutcome::Success) =>
				return Err(Error::<T>::SettlementNotComplete.into()),
			_ => return Err(Error::<T>::IncorrectRound.into()),
		}

		// * Update storage *
		let parachain_receiver_pallet_info = PalletMigrationInfo {
			parachain_id: para_id,
			hrmp_channel_status: HRMPChannelStatus {
				project_to_polimec: ChannelStatus::Closed,
				polimec_to_project: ChannelStatus::Closed,
			},
			migration_readiness_check: None,
		};
		project_details.migration_type = Some(MigrationType::Pallet(parachain_receiver_pallet_info));

		Self::transition_project(
			project_id,
			project_details,
			ProjectStatus::SettlementFinished(FundingOutcome::Success),
			ProjectStatus::CTMigrationStarted,
			None,
			false,
		)?;

		Ok(())
	}

	/// Handle the channel open request from the relay on behalf of a parachain.
	/// If the parachain id belongs to a funded project with the same project id, then send an acceptance message and a request for a
	/// channel in the opposite direction to the relay.
	pub fn do_handle_channel_open_request(sender: u32) -> XcmResult {
		// TODO: set these constants with a proper value
		const EXECUTION_DOT: Asset = Asset { id: AssetId(Location::here()), fun: Fungible(1_0_000_000_000u128) };
		const MAX_WEIGHT: Weight = Weight::from_parts(20_000_000_000, 1_000_000);

		log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest accepted");

		let (project_id, mut project_details) = ProjectsDetails::<T>::iter()
			.find(|(_id, details)| {
				matches!(
							&details.migration_type,
							Some(MigrationType::Pallet(info)) if
								info.parachain_id == ParaId::from(sender) && details.status == ProjectStatus::CTMigrationStarted)
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
			DepositAsset { assets: Wild(All), beneficiary: Location::new(0, Parachain(POLIMEC_PARA_ID)) },
		]);
		let mut message = Some(xcm);

		let dest_loc = Location::new(1, Here);
		let mut destination = Some(dest_loc);
		let (ticket, _price) = T::XcmRouter::validate(&mut destination, &mut message)?;

		match T::XcmRouter::deliver(ticket) {
			Ok(_) => {
				log::trace!(target: "pallet_funding::hrmp", "HrmpNewChannelOpenRequest: acceptance successfully sent");
				match project_details.migration_type {
					Some(MigrationType::Pallet(ref mut info)) => {
						info.hrmp_channel_status.project_to_polimec = ChannelStatus::Open;
						info.hrmp_channel_status.polimec_to_project = ChannelStatus::AwaitingAcceptance;
					},
					_ => return Err(XcmError::Transport("Migration type not set")),
				}

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
	}

	/// Handle the channel accepted message of project->Polimec from the relay on behalf of the project parachain.
	/// Start the migration readiness check for the project.
	pub fn do_handle_channel_accepted(recipient: u32) -> XcmResult {
		log::trace!(target: "pallet_funding::hrmp", "HrmpChannelAccepted received: {:?}", recipient);
		let (project_id, mut project_details) = ProjectsDetails::<T>::iter()
			.find(|(_id, details)| {
				matches!(
							&details.migration_type,
							Some(MigrationType::Pallet(info)) if
								info.parachain_id == ParaId::from(recipient) && details.status == ProjectStatus::CTMigrationStarted)
			})
			.ok_or(XcmError::BadOrigin)?;

		match project_details.migration_type {
			Some(MigrationType::Pallet(ref mut info)) => {
				info.hrmp_channel_status.polimec_to_project = ChannelStatus::Open;
			},
			_ => return Err(XcmError::Transport("Unexpected automatic flow")),
		}

		ProjectsDetails::<T>::insert(project_id, project_details);
		Pallet::<T>::deposit_event(Event::<T>::HrmpChannelEstablished { project_id, para_id: ParaId::from(recipient) });

		Pallet::<T>::do_start_pallet_migration_readiness_check(
			&(T::PalletId::get().into_account_truncating()),
			project_id,
		)
		.map_err(|_| XcmError::Transport("Unexpected automatic flow"))?;
		Ok(())
	}

	/// After the bidirectional HRMP channels are established, check that the project chain has the Polimec receiver pallet,
	/// and has minted the amount of CTs sold to the Polimec sovereign account.
	#[transactional]
	pub fn do_start_pallet_migration_readiness_check(caller: &AccountIdOf<T>, project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let Some(MigrationType::Pallet(ref mut migration_info)) = project_details.migration_type else {
			return Err(Error::<T>::NotAllowed.into())
		};
		let parachain_id: u32 = migration_info.parachain_id.into();
		let project_location = ParentThen(Parachain(parachain_id).into());
		let now = <frame_system::Pallet<T>>::block_number();

		// TODO: check these values
		let max_weight = Weight::from_parts(700_000_000, 10_000);

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::CTMigrationStarted, Error::<T>::IncorrectRound);
		ensure!(
			migration_info.hrmp_channel_status ==
				HRMPChannelStatus {
					project_to_polimec: ChannelStatus::Open,
					polimec_to_project: ChannelStatus::Open
				},
			Error::<T>::ChannelNotOpen
		);
		if migration_info.migration_readiness_check.is_none() {
			ensure!(caller.clone() == T::PalletId::get().into_account_truncating(), Error::<T>::NotAllowed);
		} else if matches!(
			migration_info.migration_readiness_check,
			Some(PalletMigrationReadinessCheck {
				holding_check: (_, CheckOutcome::Failed),
				pallet_check: (_, CheckOutcome::Failed),
				..
			})
		) {
			ensure!(caller == &project_details.issuer_account, Error::<T>::NotIssuer);
		}

		// * Update storage *
		let call = Call::<T>::pallet_migration_readiness_response {
			query_id: Default::default(),
			response: Default::default(),
		};

		let query_id_holdings = pallet_xcm::Pallet::<T>::new_notify_query(
			project_location.clone(),
			<T as Config>::RuntimeCall::from(call.clone()),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);
		let query_id_pallet = pallet_xcm::Pallet::<T>::new_notify_query(
			project_location.clone(),
			<T as Config>::RuntimeCall::from(call),
			now + QUERY_RESPONSE_TIME_WINDOW_BLOCKS.into(),
			Here,
		);

		migration_info.migration_readiness_check = Some(PalletMigrationReadinessCheck {
			holding_check: (query_id_holdings, CheckOutcome::AwaitingResponse),
			pallet_check: (query_id_pallet, CheckOutcome::AwaitingResponse),
		});
		ProjectsDetails::<T>::insert(project_id, project_details.clone());

		let total_cts_minted = <T as Config>::ContributionTokenCurrency::total_issuance(project_id);

		// * Send the migration query *
		let expected_tokens: Asset = (Location::here(), total_cts_minted.into()).into();
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
		<pallet_xcm::Pallet<T>>::send_xcm(Here, project_location, xcm).map_err(|_| Error::<T>::XcmFailed)?;

		// * Emit events *
		Self::deposit_event(Event::<T>::MigrationReadinessCheckStarted { project_id, caller: caller.clone() });

		Ok(())
	}

	/// Handle the migration readiness check response from the project chain.
	#[transactional]
	pub fn do_pallet_migration_readiness_response(
		location: Location,
		query_id: QueryId,
		response: Response,
	) -> DispatchResult {
		use xcm::v4::prelude::*;
		// TODO: check if this is too low performance. Maybe we want a new map of query_id -> project_id
		let (project_id, mut migration_info, mut project_details) = ProjectsDetails::<T>::iter()
			.find_map(|(project_id, details)| {
				if let Some(MigrationType::Pallet(ref info)) = details.migration_type {
					if let Some(check) = info.migration_readiness_check {
						if check.holding_check.0 == query_id || check.pallet_check.0 == query_id {
							return Some((project_id, info.clone(), details));
						}
					}
				}
				None
			})
			.ok_or(Error::<T>::ProjectDetailsNotFound)?;

		let para_id = match location.unpack() {
			(1, &[Parachain(para_id)]) => ParaId::from(para_id),
			_ => return Err(Error::<T>::WrongParaId.into()),
		};
		ensure!(migration_info.parachain_id == para_id, Error::<T>::WrongParaId);

		let project_metadata = ProjectsMetadata::<T>::get(project_id).ok_or(Error::<T>::ProjectMetadataNotFound)?;
		let contribution_tokens_sold =
			project_metadata.total_allocation_size.saturating_sub(project_details.remaining_contribution_tokens);

		match (response.clone(), &mut migration_info.migration_readiness_check) {
			(
				Response::Assets(assets),
				&mut Some(
					ref mut check @ PalletMigrationReadinessCheck {
						holding_check: (_, CheckOutcome::AwaitingResponse),
						..
					},
				),
			) => {
				let ct_sold_as_u128: u128 = contribution_tokens_sold.into();
				let assets: Vec<Asset> = assets.into_inner();
				let asset_1 = assets[0].clone();
				match asset_1 {
					Asset { id: AssetId(location), fun: Fungible(amount) }
						if amount >= ct_sold_as_u128 &&
							get_parachain_id(&location.clone()).unwrap() == u32::from(para_id) =>
					{
						// FIXME: Remove the `unwrap()` here
						check.holding_check.1 = CheckOutcome::Passed(None);
						Self::deposit_event(Event::<T>::MigrationCheckResponseAccepted {
							project_id,
							query_id,
							response,
						});
					},
					_ => {
						check.holding_check.1 = CheckOutcome::Failed;
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
				Some(
					ref mut check @ PalletMigrationReadinessCheck {
						pallet_check: (_, CheckOutcome::AwaitingResponse),
						..
					},
				),
			) => {
				let expected_module_name: BoundedVec<u8, MaxPalletNameLen> =
					BoundedVec::try_from("polimec_receiver".as_bytes().to_vec()).map_err(|_| Error::<T>::NotAllowed)?;
				let Some(PalletInfo { index, module_name, .. }) = pallets_info.first() else {
					return Err(Error::<T>::NotAllowed.into());
				};
				let u8_index: u8 = (*index).try_into().map_err(|_| Error::<T>::NotAllowed)?;
				if pallets_info.len() == 1 && module_name == &expected_module_name {
					check.pallet_check.1 = CheckOutcome::Passed(Some(u8_index));
					Self::deposit_event(Event::<T>::MigrationCheckResponseAccepted { project_id, query_id, response });
				} else {
					check.pallet_check.1 = CheckOutcome::Failed;
					Self::deposit_event(Event::<T>::MigrationCheckResponseRejected { project_id, query_id, response });
				}
			},
			_ => return Err(Error::<T>::NotAllowed.into()),
		};

		project_details.migration_type = Some(MigrationType::Pallet(migration_info));
		ProjectsDetails::<T>::insert(project_id, project_details);
		Ok(())
	}

	/// Migrate all the CTs of a project for a single participant
	/// This entails transferring the funds from the Polimec sovereign account to the participant account, and applying
	/// a vesting schedule if necessary.
	#[transactional]
	pub fn do_send_pallet_migration_for(project_id: ProjectId, participant: AccountIdOf<T>) -> DispatchResult {
		// * Get variables *
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let migration_info = match project_details.migration_type {
			Some(MigrationType::Pallet(info)) => info,
			_ => return Err(Error::<T>::NotAllowed.into()),
		};
		let migration_readiness_check = migration_info.migration_readiness_check.ok_or(Error::<T>::ChannelNotReady)?;
		let project_para_id = migration_info.parachain_id;
		let now = <frame_system::Pallet<T>>::block_number();
		ensure!(
			Self::user_has_no_participations(project_id, participant.clone()),
			Error::<T>::ParticipationsNotSettled
		);
		let (_, migrations) =
			UserMigrations::<T>::get((project_id, participant.clone())).ok_or(Error::<T>::NoMigrationsFound)?;

		// * Validity Checks *
		ensure!(migration_readiness_check.is_ready(), Error::<T>::ChannelNotReady);

		let project_location = Location::new(1, Parachain(project_para_id.into()));
		let call: <T as Config>::RuntimeCall =
			Call::confirm_pallet_migrations { query_id: Default::default(), response: Default::default() }.into();
		let query_id =
			pallet_xcm::Pallet::<T>::new_notify_query(project_location.clone(), call.into(), now + 20u32.into(), Here);

		let CheckOutcome::Passed(Some(pallet_index)) = migration_readiness_check.pallet_check.1 else {
			return Err(Error::<T>::NotAllowed.into());
		};

		Self::change_migration_status(project_id, participant.clone(), MigrationStatus::Sent(query_id))?;

		// * Process Data *
		let xcm = Self::construct_migration_xcm_message(migrations, query_id, pallet_index);

		<pallet_xcm::Pallet<T>>::send_xcm(Here, project_location, xcm).map_err(|_| Error::<T>::XcmFailed)?;
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
	pub fn do_confirm_pallet_migrations(location: Location, query_id: QueryId, response: Response) -> DispatchResult {
		let (project_id, participant) =
			ActiveMigrationQueue::<T>::take(query_id).ok_or(Error::<T>::NoActiveMigrationsFound)?;
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;
		let migration_info = match project_details.migration_type {
			Some(MigrationType::Pallet(info)) => info,
			_ => return Err(Error::<T>::NotAllowed.into()),
		};

		ensure!(
			matches!(location.unpack(), (1, &[Parachain(para_id)]) if ParaId::from(para_id) == migration_info.parachain_id),
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

// Common migration functions
impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_mark_project_ct_migration_as_finished(project_id: ProjectId) -> DispatchResult {
		// * Get variables *
		let mut project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		// * Validity checks *
		ensure!(project_details.status == ProjectStatus::CTMigrationStarted, Error::<T>::IncorrectRound);

		let unmigrated_participants = UnmigratedCounter::<T>::get(project_id);
		ensure!(unmigrated_participants == 0, Error::<T>::MigrationsStillPending);

		// * Update storage *
		project_details.status = ProjectStatus::CTMigrationFinished;
		ProjectsDetails::<T>::insert(project_id, project_details);

		// * Emit events *
		Self::deposit_event(Event::CTMigrationFinished { project_id });

		Ok(())
	}
}

fn get_parachain_id(loc: &Location) -> Option<u32> {
	match loc.unpack() {
		(0, [Parachain(id)]) => Some(*id),
		_ => None,
	}
}
