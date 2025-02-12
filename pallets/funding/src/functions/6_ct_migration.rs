#[allow(clippy::wildcard_imports)]
use super::*;

impl<T: Config> Pallet<T> {
	#[transactional]
	pub fn do_start_offchain_migration(project_id: ProjectId, caller: AccountIdOf<T>) -> DispatchResultWithPostInfo {
		let project_details = ProjectsDetails::<T>::get(project_id).ok_or(Error::<T>::ProjectDetailsNotFound)?;

		ensure!(project_details.issuer_account == caller, Error::<T>::NotIssuer);

		Self::transition_project(
			project_id,
			project_details,
			ProjectStatus::SettlementFinished(FundingOutcome::Success),
			ProjectStatus::CTMigrationStarted,
			None,
			false,
		)?;

		Ok(PostDispatchInfo { actual_weight: None, pays_fee: Pays::No })
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
