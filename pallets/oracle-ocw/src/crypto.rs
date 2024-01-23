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

/// Defines application identifier for crypto keys of this module.
///
/// Every module that deals with signatures needs to declare its unique identifier for
/// its crypto keys.
/// When offchain worker is signing transactions it's going to request keys of type
/// `KeyTypeId` from the keystore and use the ones it finds to sign the transaction.
/// The keys can be inserted manually via RPC (see `author_insertKey`).
use sp_core::crypto::KeyTypeId;
use sp_core::sr25519::Signature as Sr25519Signature;
use sp_runtime::{traits::Verify, MultiSignature, MultiSigner};

pub const POLIMEC_ORACLE: KeyTypeId = KeyTypeId(*b"plmc");

mod app_sr25519 {
	use super::POLIMEC_ORACLE;
	use sp_runtime::app_crypto::{app_crypto, sr25519};
	app_crypto!(sr25519, POLIMEC_ORACLE);
}

pub type AuthorityId = app_sr25519::Public;

pub struct PolimecCrypto;

impl frame_system::offchain::AppCrypto<MultiSigner, MultiSignature> for PolimecCrypto {
	type GenericPublic = sp_core::sr25519::Public;
	type GenericSignature = sp_core::sr25519::Signature;
	type RuntimeAppPublic = AuthorityId;
}

// implemented for mock runtime in test
impl frame_system::offchain::AppCrypto<<Sr25519Signature as Verify>::Signer, Sr25519Signature> for PolimecCrypto {
	type GenericPublic = sp_core::sr25519::Public;
	type GenericSignature = sp_core::sr25519::Signature;
	type RuntimeAppPublic = AuthorityId;
}
