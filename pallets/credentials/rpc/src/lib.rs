use codec::Codec;
use jsonrpsee::{
	core::{async_trait, Error as JsonRpseeError, RpcResult},
	proc_macros::rpc,
	types::error::{CallError, ErrorObject},
};
use polimec_traits::MemberRole;
use sp_api::ProvideRuntimeApi;
use sp_blockchain::HeaderBackend;
use sp_runtime::generic::BlockId;
use std::{marker::PhantomData, sync::Arc};

pub use pallet_credentials_runtime_api::CredentialsApi as CredentialsRuntimeApi;

#[rpc(client, server)]
pub trait CredentialsApi<BlockHash, AccountId> {
	#[method(name = "credentials_is_in")]
	fn get_value(&self, at: Option<BlockHash>, role: MemberRole, who: AccountId)
		-> RpcResult<bool>;
	#[method(name = "credentials_get_members_of")]
	fn get_members_of(
		&self,
		at: Option<BlockHash>,
		role: MemberRole,
	) -> RpcResult<Vec<AccountId>>;
	#[method(name = "credentials_get_roles_of")]
	fn get_roles_of(
		&self,
		at: Option<BlockHash>,
		who: AccountId,
	) -> RpcResult<Vec<MemberRole>>;
}

pub struct Credentials<Client, Block> {
	client: Arc<Client>,
	_marker: PhantomData<Block>,
}

type HashOf<Block> = <Block as sp_runtime::traits::Block>::Hash;

impl<Client, Block> Credentials<Client, Block>
where
	Block: sp_runtime::traits::Block,
	Client: HeaderBackend<Block>,
{
	pub fn new(client: Arc<Client>) -> Self {
		Self { client, _marker: Default::default() }
	}
}

#[async_trait]
impl<Client, Block, AccountId> CredentialsApiServer<HashOf<Block>, AccountId>
	for Credentials<Client, Block>
where
	Block: sp_runtime::traits::Block,
	Client: ProvideRuntimeApi<Block> + HeaderBackend<Block> + Send + Sync + 'static,
	Client::Api: CredentialsRuntimeApi<Block, AccountId>,
	AccountId: Codec,
{
	fn get_value(
		&self,
		at: Option<<Block>::Hash>,
		role: MemberRole,
		who: AccountId,
	) -> RpcResult<bool> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.is_in(&at, role, who).map_err(runtime_error_into_rpc_err)
	}

	fn get_members_of(
		&self,
		at: Option<<Block>::Hash>,
		role: MemberRole,
	) -> RpcResult<Vec<AccountId>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_members_of(&at, role).map_err(runtime_error_into_rpc_err)
	}

	fn get_roles_of(
		&self,
		at: Option<<Block>::Hash>,
		who: AccountId,
	) -> RpcResult<Vec<MemberRole>> {
		let api = self.client.runtime_api();
		let at = BlockId::hash(at.unwrap_or_else(|| self.client.info().best_hash));

		api.get_roles_of(&at, who).map_err(runtime_error_into_rpc_err)
	}
}

const RUNTIME_ERROR: i32 = 1;

/// Converts a runtime trap into an RPC error.
fn runtime_error_into_rpc_err(err: impl std::fmt::Debug) -> JsonRpseeError {
	CallError::Custom(ErrorObject::owned(
		RUNTIME_ERROR,
		"Runtime error",
		Some(format!("{:?}", err)),
	))
	.into()
}
