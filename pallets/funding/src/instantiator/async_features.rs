use super::*;
use assert_matches2::assert_matches;
use futures::FutureExt;
use std::{
	collections::HashMap,
	sync::{
		atomic::{AtomicBool, AtomicU32, Ordering},
		Arc,
	},
	time::Duration,
};
use tokio::{
	sync::{Mutex, Notify},
	time::sleep,
};

pub struct BlockOrchestrator<T: Config, AllPalletsWithoutSystem, RuntimeEvent> {
	pub current_block: Arc<AtomicU32>,
	// used for resuming execution of a project that is waiting for a certain block to be reached
	pub awaiting_projects: Mutex<HashMap<BlockNumberFor<T>, Vec<Arc<Notify>>>>,
	pub should_continue: Arc<AtomicBool>,
	pub instantiator_phantom: PhantomData<(T, AllPalletsWithoutSystem, RuntimeEvent)>,
}
pub async fn block_controller<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
) {
	loop {
		if !block_orchestrator.continue_running() {
			break;
		}

		let maybe_target_reached = block_orchestrator.advance_to_next_target(instantiator.clone()).await;

		if let Some(target_reached) = maybe_target_reached {
			block_orchestrator.execute_callbacks(target_reached).await;
		}
		// leaves some time for the projects to submit their targets to the orchestrator
		sleep(Duration::from_millis(100)).await;
	}
}

impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> Default for BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	fn default() -> Self {
		Self::new()
	}
}

impl<
		T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
		AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
		RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
	> BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>
{
	pub fn new() -> Self {
		BlockOrchestrator::<T, AllPalletsWithoutSystem, RuntimeEvent> {
			current_block: Arc::new(AtomicU32::new(0)),
			awaiting_projects: Mutex::new(HashMap::new()),
			should_continue: Arc::new(AtomicBool::new(true)),
			instantiator_phantom: PhantomData,
		}
	}

	pub async fn add_awaiting_project(&self, block_number: BlockNumberFor<T>, notify: Arc<Notify>) {
		let mut awaiting_projects = self.awaiting_projects.lock().await;
		awaiting_projects.entry(block_number).or_default().push(notify);
		drop(awaiting_projects);
	}

	pub async fn advance_to_next_target(
		&self,
		instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	) -> Option<BlockNumberFor<T>> {
		let mut inst = instantiator.lock().await;
		let now: u32 = inst.current_block().try_into().unwrap_or_else(|_| panic!("Block number should fit into u32"));
		self.current_block.store(now, Ordering::SeqCst);

		let awaiting_projects = self.awaiting_projects.lock().await;

		if let Some(&next_block) = awaiting_projects.keys().min() {
			drop(awaiting_projects);

			while self.get_current_block() < next_block {
				inst.advance_time(One::one()).unwrap();
				let current_block: u32 =
					self.get_current_block().try_into().unwrap_or_else(|_| panic!("Block number should fit into u32"));
				self.current_block.store(current_block + 1u32, Ordering::SeqCst);
			}
			Some(next_block)
		} else {
			None
		}
	}

	pub async fn execute_callbacks(&self, block_number: BlockNumberFor<T>) {
		let mut awaiting_projects = self.awaiting_projects.lock().await;
		if let Some(notifies) = awaiting_projects.remove(&block_number) {
			for notify in notifies {
				notify.notify_one();
			}
		}
	}

	pub async fn is_empty(&self) -> bool {
		self.awaiting_projects.lock().await.is_empty()
	}

	// Method to check if the loop should continue
	pub fn continue_running(&self) -> bool {
		self.should_continue.load(Ordering::SeqCst)
	}

	// Method to get the current block number
	pub fn get_current_block(&self) -> BlockNumberFor<T> {
		self.current_block.load(Ordering::SeqCst).into()
	}
}

// async instantiations for parallel testing
pub async fn async_create_new_project<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	project_metadata: ProjectMetadataOf<T>,
	issuer: AccountIdOf<T>,
) -> ProjectId {
	let mut inst = instantiator.lock().await;

	let now = inst.current_block();
	let ed = inst.get_ed();
	// One ED for the issuer, one for the escrow account
	inst.mint_plmc_to(vec![UserToPLMCBalance::new(issuer.clone(), ed * 2u64.into())]);
	inst.execute(|| {
		crate::Pallet::<T>::do_create_project(
			&issuer.clone(),
			project_metadata.clone(),
			generate_did_from_account(issuer.clone()),
		)
		.unwrap();
		let last_project_metadata = ProjectsMetadata::<T>::iter().last().unwrap();
		log::trace!("Last project metadata: {:?}", last_project_metadata);
	});

	let created_project_id = inst.execute(|| NextProjectId::<T>::get().saturating_sub(One::one()));
	inst.creation_assertions(created_project_id, project_metadata, now);
	created_project_id
}

pub async fn async_create_evaluating_project<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	project_metadata: ProjectMetadataOf<T>,
	issuer: AccountIdOf<T>,
) -> ProjectId {
	let project_id = async_create_new_project(instantiator.clone(), project_metadata, issuer.clone()).await;

	let mut inst = instantiator.lock().await;

	inst.start_evaluation(project_id, issuer).unwrap();
	let now = inst.current_block();
	project_id
}

pub async fn async_start_auction<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_id: ProjectId,
	caller: AccountIdOf<T>,
) -> Result<(), DispatchError> {
	let mut inst = instantiator.lock().await;

	let project_details = inst.get_project_details(project_id);

	if project_details.status == ProjectStatus::EvaluationRound {
		let update_block = inst.get_update_block(project_id, &UpdateType::EvaluationEnd).unwrap();
		let notify = Arc::new(Notify::new());
		block_orchestrator.add_awaiting_project(update_block, notify.clone()).await;

		// Wait for the notification that our desired block was reached to continue
		drop(inst);

		notify.notified().await;

		inst = instantiator.lock().await;
	};

	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionInitializePeriod);

	inst.execute(|| crate::Pallet::<T>::do_auction_opening(caller.clone(), project_id).unwrap());

	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::AuctionOpening);

	Ok(())
}

pub async fn async_create_auctioning_project<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_metadata: ProjectMetadataOf<T>,
	issuer: AccountIdOf<T>,
	evaluations: Vec<UserToUSDBalance<T>>,
	bids: Vec<BidParams<T>>,
) -> ProjectId {
	let project_id =
		async_create_evaluating_project(instantiator.clone(), project_metadata.clone(), issuer.clone()).await;

	let mut inst = instantiator.lock().await;

	let evaluators = evaluations.accounts();
	let prev_supply = inst.get_plmc_total_supply();
	let prev_plmc_balances = inst.get_free_plmc_balances_for(evaluators.clone());

	let plmc_eval_deposits: Vec<UserToPLMCBalance<T>> = inst.calculate_evaluation_plmc_spent(evaluations.clone());
	let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = evaluators.existential_deposits();

	let expected_remaining_plmc: Vec<UserToPLMCBalance<T>> =
		inst.generic_map_operation(vec![prev_plmc_balances, plmc_existential_deposits.clone()], MergeOperation::Add);

	inst.mint_plmc_to(plmc_eval_deposits.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());

	inst.evaluate_for_users(project_id, evaluations).unwrap();

	let expected_evaluator_balances =
		inst.sum_balance_mappings(vec![plmc_eval_deposits.clone(), plmc_existential_deposits.clone()]);

	let expected_total_supply = prev_supply + expected_evaluator_balances;

	inst.evaluation_assertions(project_id, expected_remaining_plmc, plmc_eval_deposits, expected_total_supply);

	drop(inst);

	async_start_auction(instantiator.clone(), block_orchestrator, project_id, issuer).await.unwrap();

	inst = instantiator.lock().await;
	let plmc_for_bids =
		inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata.clone(), None);
	let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = bids.accounts().existential_deposits();
	let usdt_for_bids =
		inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata, None);

	inst.mint_plmc_to(plmc_for_bids.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());
	inst.mint_foreign_asset_to(usdt_for_bids.clone());

	inst.bid_for_users(project_id, bids).unwrap();
	drop(inst);

	project_id
}

pub async fn async_start_community_funding<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_id: ProjectId,
) -> Result<(), DispatchError> {
	let mut inst = instantiator.lock().await;

	let update_block = inst.get_update_block(project_id, &UpdateType::AuctionClosingStart).unwrap();
	let closing_start = update_block;

	let notify = Arc::new(Notify::new());

	block_orchestrator.add_awaiting_project(closing_start, notify.clone()).await;

	// Wait for the notification that our desired block was reached to continue

	drop(inst);

	notify.notified().await;

	inst = instantiator.lock().await;
	let update_block = inst.get_update_block(project_id, &UpdateType::CommunityFundingStart).unwrap();
	let community_start = update_block;

	let notify = Arc::new(Notify::new());

	block_orchestrator.add_awaiting_project(community_start, notify.clone()).await;

	drop(inst);

	notify.notified().await;

	inst = instantiator.lock().await;

	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);

	Ok(())
}

pub async fn async_create_community_contributing_project<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_metadata: ProjectMetadataOf<T>,
	issuer: AccountIdOf<T>,
	evaluations: Vec<UserToUSDBalance<T>>,
	bids: Vec<BidParams<T>>,
) -> (ProjectId, Vec<BidParams<T>>) {
	if bids.is_empty() {
		panic!("Cannot start community funding without bids")
	}

	let project_id = async_create_auctioning_project(
		instantiator.clone(),
		block_orchestrator.clone(),
		project_metadata.clone(),
		issuer,
		evaluations.clone(),
		vec![],
	)
	.await;

	let mut inst = instantiator.lock().await;

	let bidders = bids.accounts();
	let asset_id = bids[0].asset.to_assethub_id();
	let prev_plmc_balances = inst.get_free_plmc_balances_for(bidders.clone());
	let prev_funding_asset_balances = inst.get_free_foreign_asset_balances_for(asset_id, bidders.clone());
	let plmc_evaluation_deposits: Vec<UserToPLMCBalance<T>> = inst.calculate_evaluation_plmc_spent(evaluations);
	let plmc_bid_deposits: Vec<UserToPLMCBalance<T>> =
		inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(&bids, project_metadata.clone(), None);
	let participation_usable_evaluation_deposits = plmc_evaluation_deposits
		.into_iter()
		.map(|mut x| {
			x.plmc_amount = x.plmc_amount.saturating_sub(<T as Config>::EvaluatorSlash::get() * x.plmc_amount);
			x
		})
		.collect::<Vec<UserToPLMCBalance<T>>>();
	let necessary_plmc_mint = inst.generic_map_operation(
		vec![plmc_bid_deposits.clone(), participation_usable_evaluation_deposits],
		MergeOperation::Subtract,
	);
	let total_plmc_participation_locked = plmc_bid_deposits;
	let plmc_existential_deposits: Vec<UserToPLMCBalance<T>> = bidders.existential_deposits();
	let funding_asset_deposits = inst.calculate_auction_funding_asset_charged_from_all_bids_made_or_with_bucket(
		&bids,
		project_metadata.clone(),
		None,
	);

	let bidder_balances =
		inst.sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

	let expected_free_plmc_balances =
		inst.generic_map_operation(vec![prev_plmc_balances, plmc_existential_deposits.clone()], MergeOperation::Add);

	let prev_supply = inst.get_plmc_total_supply();
	let post_supply = prev_supply + bidder_balances;

	inst.mint_plmc_to(necessary_plmc_mint.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());
	inst.mint_foreign_asset_to(funding_asset_deposits.clone());

	inst.bid_for_users(project_id, bids.clone()).unwrap();

	inst.do_reserved_plmc_assertions(
		total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
		HoldReason::Participation(project_id).into(),
	);
	inst.do_bid_transferred_foreign_asset_assertions(
		funding_asset_deposits.merge_accounts(MergeOperation::Add),
		project_id,
	);
	inst.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
	inst.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
	assert_eq!(inst.get_plmc_total_supply(), post_supply);

	drop(inst);
	async_start_community_funding(instantiator.clone(), block_orchestrator, project_id).await.unwrap();
	let mut inst = instantiator.lock().await;

	let _weighted_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let accepted_bids = inst.filter_bids_after_auction(
		bids,
		project_metadata.auction_round_allocation_percentage * project_metadata.total_allocation_size,
	);
	let bid_expectations = accepted_bids
		.iter()
		.map(|bid| BidInfoFilter::<T> {
			bidder: Some(bid.bidder.clone()),
			final_ct_amount: Some(bid.amount),
			..Default::default()
		})
		.collect_vec();

	let total_ct_sold = accepted_bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);

	inst.finalized_bids_assertions(project_id, bid_expectations, total_ct_sold);

	(project_id, accepted_bids)
}

pub async fn async_start_remainder_or_end_funding<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_id: ProjectId,
) -> Result<(), DispatchError> {
	let mut inst = instantiator.lock().await;

	assert_eq!(inst.get_project_details(project_id).status, ProjectStatus::CommunityRound);

	let update_block = inst.get_update_block(project_id, &UpdateType::RemainderFundingStart).unwrap();
	let remainder_start = update_block;

	let notify = Arc::new(Notify::new());

	block_orchestrator.add_awaiting_project(remainder_start, notify.clone()).await;

	// Wait for the notification that our desired block was reached to continue

	drop(inst);

	notify.notified().await;

	let mut inst = instantiator.lock().await;

	assert_matches!(
		inst.get_project_details(project_id).status,
		(ProjectStatus::RemainderRound | ProjectStatus::FundingSuccessful)
	);
	Ok(())
}

pub async fn async_create_remainder_contributing_project<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_metadata: ProjectMetadataOf<T>,
	issuer: AccountIdOf<T>,
	evaluations: Vec<UserToUSDBalance<T>>,
	bids: Vec<BidParams<T>>,
	contributions: Vec<ContributionParams<T>>,
) -> (ProjectId, Vec<BidParams<T>>) {
	let (project_id, accepted_bids) = async_create_community_contributing_project(
		instantiator.clone(),
		block_orchestrator.clone(),
		project_metadata,
		issuer,
		evaluations.clone(),
		bids,
	)
	.await;

	if contributions.is_empty() {
		async_start_remainder_or_end_funding(instantiator.clone(), block_orchestrator.clone(), project_id)
			.await
			.unwrap();
		return (project_id, accepted_bids);
	}

	let mut inst = instantiator.lock().await;

	let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let contributors = contributions.accounts();
	let asset_id = contributions[0].asset.to_assethub_id();
	let prev_plmc_balances = inst.get_free_plmc_balances_for(contributors.clone());
	let prev_funding_asset_balances = inst.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

	let plmc_evaluation_deposits = inst.calculate_evaluation_plmc_spent(evaluations);
	let plmc_bid_deposits = inst.calculate_auction_plmc_charged_with_given_price(&accepted_bids, ct_price);

	let plmc_contribution_deposits = inst.calculate_contributed_plmc_spent(contributions.clone(), ct_price);

	let necessary_plmc_mint = inst.generic_map_operation(
		vec![plmc_contribution_deposits.clone(), plmc_evaluation_deposits],
		MergeOperation::Subtract,
	);
	let total_plmc_participation_locked =
		inst.generic_map_operation(vec![plmc_bid_deposits, plmc_contribution_deposits], MergeOperation::Add);
	let plmc_existential_deposits = contributors.existential_deposits();

	let funding_asset_deposits = inst.calculate_contributed_funding_asset_spent(contributions.clone(), ct_price);
	let contributor_balances =
		inst.sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

	let expected_free_plmc_balances =
		inst.generic_map_operation(vec![prev_plmc_balances, plmc_existential_deposits.clone()], MergeOperation::Add);

	let prev_supply = inst.get_plmc_total_supply();
	let post_supply = prev_supply + contributor_balances;

	inst.mint_plmc_to(necessary_plmc_mint.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());
	inst.mint_foreign_asset_to(funding_asset_deposits.clone());

	inst.contribute_for_users(project_id, contributions).expect("Contributing should work");

	inst.do_reserved_plmc_assertions(
		total_plmc_participation_locked.merge_accounts(MergeOperation::Add),
		HoldReason::Participation(project_id).into(),
	);
	inst.do_contribution_transferred_foreign_asset_assertions(
		funding_asset_deposits.merge_accounts(MergeOperation::Add),
		project_id,
	);
	inst.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
	inst.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
	assert_eq!(inst.get_plmc_total_supply(), post_supply);
	drop(inst);
	async_start_remainder_or_end_funding(instantiator.clone(), block_orchestrator.clone(), project_id).await.unwrap();
	(project_id, accepted_bids)
}

pub async fn async_finish_funding<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_id: ProjectId,
) -> Result<(), DispatchError> {
	let mut inst = instantiator.lock().await;
	let update_block = inst.get_update_block(project_id, &UpdateType::FundingEnd).unwrap();

	let notify = Arc::new(Notify::new());
	block_orchestrator.add_awaiting_project(update_block, notify.clone()).await;
	drop(inst);
	notify.notified().await;
	Ok(())
}

pub async fn async_create_finished_project<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	project_metadata: ProjectMetadataOf<T>,
	issuer: AccountIdOf<T>,
	evaluations: Vec<UserToUSDBalance<T>>,
	bids: Vec<BidParams<T>>,
	community_contributions: Vec<ContributionParams<T>>,
	remainder_contributions: Vec<ContributionParams<T>>,
) -> ProjectId {
	let (project_id, accepted_bids) = async_create_remainder_contributing_project(
		instantiator.clone(),
		block_orchestrator.clone(),
		project_metadata.clone(),
		issuer,
		evaluations.clone(),
		bids.clone(),
		community_contributions.clone(),
	)
	.await;

	let mut inst = instantiator.lock().await;

	let total_ct_sold_in_bids = bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);
	let total_ct_sold_in_community_contributions =
		community_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);
	let total_ct_sold_in_remainder_contributions =
		remainder_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);

	let total_ct_sold =
		total_ct_sold_in_bids + total_ct_sold_in_community_contributions + total_ct_sold_in_remainder_contributions;
	let total_ct_available = project_metadata.total_allocation_size;
	assert!(
        total_ct_sold <= total_ct_available,
        "Some CT buys are getting less than expected due to running out of CTs. This is ok in the runtime, but likely unexpected from the parameters of this instantiation"
    );

	match inst.get_project_details(project_id).status {
		ProjectStatus::FundingSuccessful => return project_id,
		ProjectStatus::RemainderRound if remainder_contributions.is_empty() => {
			drop(inst);
			async_finish_funding(instantiator.clone(), block_orchestrator.clone(), project_id).await.unwrap();
			return project_id;
		},
		_ => {},
	};

	let ct_price = inst.get_project_details(project_id).weighted_average_price.unwrap();
	let contributors = remainder_contributions.accounts();
	let asset_id = remainder_contributions[0].asset.to_assethub_id();
	let prev_plmc_balances = inst.get_free_plmc_balances_for(contributors.clone());
	let prev_funding_asset_balances = inst.get_free_foreign_asset_balances_for(asset_id, contributors.clone());

	let plmc_evaluation_deposits = inst.calculate_evaluation_plmc_spent(evaluations);
	let plmc_bid_deposits = inst.calculate_auction_plmc_charged_from_all_bids_made_or_with_bucket(
		&accepted_bids,
		project_metadata.clone(),
		None,
	);
	let plmc_community_contribution_deposits =
		inst.calculate_contributed_plmc_spent(community_contributions.clone(), ct_price);
	let plmc_remainder_contribution_deposits =
		inst.calculate_contributed_plmc_spent(remainder_contributions.clone(), ct_price);

	let necessary_plmc_mint = inst.generic_map_operation(
		vec![plmc_remainder_contribution_deposits.clone(), plmc_evaluation_deposits],
		MergeOperation::Subtract,
	);
	let total_plmc_participation_locked = inst.generic_map_operation(
		vec![plmc_bid_deposits, plmc_community_contribution_deposits, plmc_remainder_contribution_deposits],
		MergeOperation::Add,
	);
	let plmc_existential_deposits = contributors.existential_deposits();
	let funding_asset_deposits =
		inst.calculate_contributed_funding_asset_spent(remainder_contributions.clone(), ct_price);

	let contributor_balances =
		inst.sum_balance_mappings(vec![necessary_plmc_mint.clone(), plmc_existential_deposits.clone()]);

	let expected_free_plmc_balances =
		inst.generic_map_operation(vec![prev_plmc_balances, plmc_existential_deposits.clone()], MergeOperation::Add);

	let prev_supply = inst.get_plmc_total_supply();
	let post_supply = prev_supply + contributor_balances;

	inst.mint_plmc_to(necessary_plmc_mint.clone());
	inst.mint_plmc_to(plmc_existential_deposits.clone());
	inst.mint_foreign_asset_to(funding_asset_deposits.clone());

	inst.contribute_for_users(project_id, remainder_contributions.clone()).expect("Remainder Contributing should work");

	let merged = total_plmc_participation_locked.merge_accounts(MergeOperation::Add);

	inst.do_reserved_plmc_assertions(merged, HoldReason::Participation(project_id).into());

	inst.do_contribution_transferred_foreign_asset_assertions(
		funding_asset_deposits.merge_accounts(MergeOperation::Add),
		project_id,
	);
	inst.do_free_plmc_assertions(expected_free_plmc_balances.merge_accounts(MergeOperation::Add));
	inst.do_free_foreign_asset_assertions(prev_funding_asset_balances.merge_accounts(MergeOperation::Add));
	assert_eq!(inst.get_plmc_total_supply(), post_supply);

	drop(inst);
	async_finish_funding(instantiator.clone(), block_orchestrator.clone(), project_id).await.unwrap();
	let mut inst = instantiator.lock().await;

	if inst.get_project_details(project_id).status == ProjectStatus::FundingSuccessful {
		// Check that remaining CTs are updated
		let project_details = inst.get_project_details(project_id);
		let auction_bought_tokens =
			accepted_bids.iter().map(|bid| bid.amount).fold(Zero::zero(), |acc, item| item + acc);
		let community_bought_tokens =
			community_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);
		let remainder_bought_tokens =
			remainder_contributions.iter().map(|cont| cont.amount).fold(Zero::zero(), |acc, item| item + acc);

		assert_eq!(
			project_details.remaining_contribution_tokens,
			project_metadata.total_allocation_size -
				auction_bought_tokens -
				community_bought_tokens -
				remainder_bought_tokens,
			"Remaining CTs are incorrect"
		);
	}

	project_id
}

pub async fn create_project_at<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	test_project_params: TestProjectParams<T>,
) -> ProjectId {
	match test_project_params.expected_state {
		ProjectStatus::FundingSuccessful =>
			async_create_finished_project(
				instantiator,
				block_orchestrator,
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
				test_project_params.community_contributions,
				test_project_params.remainder_contributions,
			)
			.await,
		ProjectStatus::RemainderRound =>
			async_create_remainder_contributing_project(
				instantiator,
				block_orchestrator,
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
				test_project_params.community_contributions,
			)
			.map(|(project_id, _)| project_id)
			.await,
		ProjectStatus::CommunityRound =>
			async_create_community_contributing_project(
				instantiator,
				block_orchestrator,
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
			)
			.map(|(project_id, _)| project_id)
			.await,
		ProjectStatus::AuctionOpening =>
			async_create_auctioning_project(
				instantiator,
				block_orchestrator,
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
			)
			.await,
		ProjectStatus::EvaluationRound =>
			async_create_evaluating_project(instantiator, test_project_params.metadata, test_project_params.issuer)
				.await,
		ProjectStatus::Application =>
			async_create_new_project(instantiator, test_project_params.metadata, test_project_params.issuer).await,
		_ => panic!("unsupported project creation in that status"),
	}
}

pub async fn async_create_project_at<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>>,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	mutex_inst: Arc<Mutex<Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>>>,
	block_orchestrator: Arc<BlockOrchestrator<T, AllPalletsWithoutSystem, RuntimeEvent>>,
	test_project_params: TestProjectParams<T>,
) -> ProjectId {
	let time_to_new_project: BlockNumberFor<T> = Zero::zero();
	let time_to_evaluation: BlockNumberFor<T> = time_to_new_project + Zero::zero();
	// we immediately start the auction, so we dont wait for T::AuctionInitializePeriodDuration.
	let time_to_auction: BlockNumberFor<T> = time_to_evaluation + <T as Config>::EvaluationDuration::get();
	let time_to_community: BlockNumberFor<T> =
		time_to_auction + <T as Config>::AuctionOpeningDuration::get() + <T as Config>::AuctionClosingDuration::get();
	let time_to_remainder: BlockNumberFor<T> = time_to_community + <T as Config>::CommunityFundingDuration::get();
	let time_to_finish: BlockNumberFor<T> = time_to_remainder +
		<T as Config>::RemainderFundingDuration::get() +
		<T as Config>::SuccessToSettlementTime::get();
	let mut inst = mutex_inst.lock().await;
	let now = inst.current_block();
	drop(inst);

	match test_project_params.expected_state {
		ProjectStatus::Application => {
			let notify = Arc::new(Notify::new());
			block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_new_project, notify.clone()).await;
			// Wait for the notification that our desired block was reached to continue
			notify.notified().await;
			async_create_new_project(mutex_inst.clone(), test_project_params.metadata, test_project_params.issuer).await
		},
		ProjectStatus::EvaluationRound => {
			let notify = Arc::new(Notify::new());
			block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_evaluation, notify.clone()).await;
			// Wait for the notification that our desired block was reached to continue
			notify.notified().await;
			let now = mutex_inst.lock().await.current_block();
			async_create_evaluating_project(
				mutex_inst.clone(),
				test_project_params.metadata,
				test_project_params.issuer,
			)
			.await
		},
		ProjectStatus::AuctionOpening | ProjectStatus::AuctionClosing => {
			let notify = Arc::new(Notify::new());
			block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_auction, notify.clone()).await;
			// Wait for the notification that our desired block was reached to continue
			notify.notified().await;
			async_create_auctioning_project(
				mutex_inst.clone(),
				block_orchestrator.clone(),
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
			)
			.await
		},
		ProjectStatus::CommunityRound => {
			let notify = Arc::new(Notify::new());
			block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_community, notify.clone()).await;
			// Wait for the notification that our desired block was reached to continue
			notify.notified().await;
			async_create_community_contributing_project(
				mutex_inst.clone(),
				block_orchestrator.clone(),
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
			)
			.map(|(project_id, _)| project_id)
			.await
		},
		ProjectStatus::RemainderRound => {
			let notify = Arc::new(Notify::new());
			block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_remainder, notify.clone()).await;
			// Wait for the notification that our desired block was reached to continue
			notify.notified().await;
			async_create_remainder_contributing_project(
				mutex_inst.clone(),
				block_orchestrator.clone(),
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
				test_project_params.community_contributions,
			)
			.map(|(project_id, _)| project_id)
			.await
		},
		ProjectStatus::FundingSuccessful => {
			let notify = Arc::new(Notify::new());
			block_orchestrator.add_awaiting_project(now + time_to_finish - time_to_finish, notify.clone()).await;
			// Wait for the notification that our desired block was reached to continue
			notify.notified().await;
			async_create_finished_project(
				mutex_inst.clone(),
				block_orchestrator.clone(),
				test_project_params.metadata,
				test_project_params.issuer,
				test_project_params.evaluations,
				test_project_params.bids,
				test_project_params.community_contributions,
				test_project_params.remainder_contributions,
			)
			.await
		},
		_ => unimplemented!("Unsupported project creation in that status"),
	}
}

pub fn create_multiple_projects_at<
	T: Config + pallet_balances::Config<Balance = BalanceOf<T>>,
	AllPalletsWithoutSystem: OnFinalize<BlockNumberFor<T>> + OnIdle<BlockNumberFor<T>> + OnInitialize<BlockNumberFor<T>> + 'static + 'static,
	RuntimeEvent: From<Event<T>> + TryInto<Event<T>> + Parameter + Member + IsType<<T as frame_system::Config>::RuntimeEvent>,
>(
	instantiator: Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>,
	projects: Vec<TestProjectParams<T>>,
) -> (Vec<ProjectId>, Instantiator<T, AllPalletsWithoutSystem, RuntimeEvent>) {
	use tokio::runtime::Builder;
	let tokio_runtime = Builder::new_current_thread().enable_all().build().unwrap();
	let local = tokio::task::LocalSet::new();
	let execution = local.run_until(async move {
		let block_orchestrator = Arc::new(BlockOrchestrator::new());
		let mutex_inst = Arc::new(Mutex::new(instantiator));

		let project_futures = projects.into_iter().map(|project| {
			let block_orchestrator = block_orchestrator.clone();
			let mutex_inst = mutex_inst.clone();
			tokio::task::spawn_local(async { async_create_project_at(mutex_inst, block_orchestrator, project).await })
		});

		// Wait for all project creation tasks to complete
		let joined_project_futures = futures::future::join_all(project_futures);
		let controller_handle =
			tokio::task::spawn_local(block_controller(block_orchestrator.clone(), mutex_inst.clone()));
		let projects = joined_project_futures.await;

		// Now that all projects have been set up, signal the block_controller to stop
		block_orchestrator.should_continue.store(false, Ordering::SeqCst);

		// Wait for the block controller to finish
		controller_handle.await.unwrap();

		let inst = Arc::try_unwrap(mutex_inst).unwrap_or_else(|_| panic!("mutex in use")).into_inner();
		let project_ids = projects.into_iter().map(|project| project.unwrap()).collect_vec();

		(project_ids, inst)
	});
	tokio_runtime.block_on(execution)
}
