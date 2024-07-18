use crate::chain_spec::{get_account_id_from_seed, Extensions};
use cumulus_primitives_core::ParaId;
use frame_support::traits::fungible::Inspect;
#[cfg(not(feature = "runtime-benchmarks"))]
use itertools::Itertools;
#[cfg(not(feature = "runtime-benchmarks"))]
use polimec_runtime::MinCandidateStk;
use polimec_runtime::{
	pallet_parachain_staking,
	pallet_parachain_staking::{
		inflation::{perbill_annual_to_perbill_round, BLOCKS_PER_YEAR},
		InflationInfo, Range,
	},
	AccountId, AuraId as AuthorityId, Balance, OracleProvidersMembershipConfig, Runtime, PLMC,
};
use sp_core::{crypto::UncheckedInto, sr25519};
use sp_runtime::{traits::AccountIdConversion, Perbill, Percent};
pub type ChainSpec = sc_service::GenericChainSpec<Extensions>;

/// The default XCM version to set in genesis config.
pub const SAFE_XCM_VERSION: u32 = xcm::prelude::XCM_VERSION;
pub const COLLATOR_COMMISSION: Perbill = Perbill::from_percent(10);
pub const PARACHAIN_BOND_RESERVE_PERCENT: Percent = Percent::from_percent(0);
pub const BLOCKS_PER_ROUND: u32 = 2 * 10;
pub const NUM_SELECTED_CANDIDATES: u32 = 5;

pub fn polimec_inflation_config() -> InflationInfo<Balance> {
	fn to_round_inflation(annual: Range<Perbill>) -> Range<Perbill> {
		perbill_annual_to_perbill_round(
			annual,
			// rounds per year
			BLOCKS_PER_YEAR / BLOCKS_PER_ROUND,
		)
	}

	let annual =
		Range { min: Perbill::from_percent(2), ideal: Perbill::from_percent(3), max: Perbill::from_percent(3) };

	InflationInfo {
		// staking expectations
		expect: Range { min: 100_000 * PLMC, ideal: 200_000 * PLMC, max: 500_000 * PLMC },
		// annual inflation
		annual,
		round: to_round_inflation(annual),
	}
}

pub fn get_polimec_session_keys(keys: AuthorityId) -> polimec_runtime::SessionKeys {
	polimec_runtime::SessionKeys { aura: keys }
}

pub fn alice() -> polimec_runtime::AccountId {
	get_account_id_from_seed::<sr25519::Public>("Alice")
}
pub fn bob() -> polimec_runtime::AccountId {
	get_account_id_from_seed::<sr25519::Public>("Bob")
}
pub fn charlie() -> polimec_runtime::AccountId {
	get_account_id_from_seed::<sr25519::Public>("Charlie")
}
pub fn dave() -> polimec_runtime::AccountId {
	get_account_id_from_seed::<sr25519::Public>("Dave")
}
pub fn eve() -> polimec_runtime::AccountId {
	get_account_id_from_seed::<sr25519::Public>("Eve")
}

pub struct GenesisConfigParams {
	pub stakers: Vec<AccountId>,
	pub council_members: Vec<AccountId>,
	pub technical_committee_members: Vec<AccountId>,
	pub oracle_members: Vec<AccountId>,
	// Do not include system accounts or the funding asset owner.
	pub endowed_accounts: Vec<(AccountId, Balance)>,
	pub funding_assets_owner: AccountId,
	pub id: ParaId,
}
pub fn genesis_config(genesis_config_params: GenesisConfigParams) -> serde_json::Value {
	let GenesisConfigParams {
		stakers,
		council_members,
		technical_committee_members,
		oracle_members,
		mut endowed_accounts,
		funding_assets_owner,
		id,
	} = genesis_config_params;

	let system_accounts = vec![
		(
			<Runtime as pallet_funding::Config>::ContributionTreasury::get(),
			<Runtime as pallet_funding::Config>::NativeCurrency::minimum_balance(),
		),
		// Need this to have enough for staking rewards
		(<Runtime as pallet_parachain_staking::Config>::PayMaster::get(), 10_000_000 * PLMC),
	];
	endowed_accounts.append(&mut system_accounts.clone());

	#[cfg(not(feature = "runtime-benchmarks"))]
	let staking_candidates = stakers.clone().into_iter().map(|account| (account, MinCandidateStk::get())).collect_vec();
	#[cfg(feature = "runtime-benchmarks")]
	let staking_candidates: Vec<(AccountId, Balance)> = vec![];

	let usdt_id = pallet_funding::types::AcceptedFundingAsset::USDT.to_assethub_id();
	let usdc_id = pallet_funding::types::AcceptedFundingAsset::USDC.to_assethub_id();
	let dot_id = pallet_funding::types::AcceptedFundingAsset::DOT.to_assethub_id();

	serde_json::json!({
		"balances": {
			"balances": endowed_accounts.clone()
		},
		"parachainInfo": {
			"parachainId": id
		},
		"foreignAssets":  {
			"assets": vec![(
				pallet_funding::types::AcceptedFundingAsset::USDT.to_assethub_id(),
				&AccountIdConversion::<AccountId>::into_account_truncating(&<Runtime as pallet_funding::Config>::PalletId::get()),
				true,
				70000,
			),
			(
				pallet_funding::types::AcceptedFundingAsset::USDC.to_assethub_id(),
				&AccountIdConversion::<AccountId>::into_account_truncating(&<Runtime as pallet_funding::Config>::PalletId::get()),
				true,
				70000,
			),
			(
				pallet_funding::types::AcceptedFundingAsset::DOT.to_assethub_id(),
				&AccountIdConversion::<AccountId>::into_account_truncating(&<Runtime as pallet_funding::Config>::PalletId::get()),
				true,
				70000,
			)],
			// (id, name, symbol, decimals)
			"metadata": vec![
				(usdt_id, b"Local USDT", b"USDT", 6),
				(usdc_id, b"Local USDC", b"USDC", 6),
				(dot_id, b"Local DOT ", b"DOT ", 10)
			],
			// (id, account_id, amount)
			"accounts": vec![
				(usdt_id, funding_assets_owner.clone(), 1000000000000u64),
				(usdc_id, funding_assets_owner.clone(), 1000000000000u64),
				(dot_id, funding_assets_owner.clone(), 1000000000000u64)
			],
		},
		"parachainStaking": {
			"candidates": staking_candidates,
			"inflationConfig": polimec_inflation_config(),
			"delegations": [],
			"collatorCommission": COLLATOR_COMMISSION,
			"parachainBondReservePercent": PARACHAIN_BOND_RESERVE_PERCENT,
			"blocksPerRound": BLOCKS_PER_ROUND,
			"numSelectedCandidates": NUM_SELECTED_CANDIDATES
		},
		"session": {
			"keys": stakers.iter().map(|account| {
				(
					account.clone(),
					account.clone(),
					get_polimec_session_keys(Into::<[u8; 32]>::into(account.clone()).unchecked_into())
				)
			}).collect::<Vec<_>>()
		},
		"polkadotXcm": {
			"safeXcmVersion": SAFE_XCM_VERSION
		},
		"oracleProvidersMembership": OracleProvidersMembershipConfig {
			members:  oracle_members.try_into().unwrap(),
			phantom: Default::default(),
		},
		"council": {
			"members": council_members
		},
		"technicalCommittee": {
			"members": technical_committee_members
		},
	})
}
