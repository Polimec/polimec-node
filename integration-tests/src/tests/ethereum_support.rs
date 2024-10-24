use crate::*;
use hex_literal::hex;
use sp_runtime::traits::Convert;

generate_accounts!(ETH_BUYER);

#[test]
fn test_hardcoded_signatures() {
	let polimec_account: PolimecAccountId = ETH_BUYER.into();
	let project_id = 0;

	// Values generated with `https://github.com/lrazovic/ethsigner`
	let polimec_account_ss58 = polimec_runtime::SS58Converter::convert(polimec_account.clone());
	dbg!(polimec_account_ss58);
	let ethereum_account: [u8; 20] = hex!("FCAd0B19bB29D4674531d6f115237E16AfCE377c");
	let signature: [u8; 65] = hex!("4fa35369a2d654112d3fb419e24dc0d7d61b7e3f23936d6d4df0ac8608fa4530795971d4d1967da60853aa974ad57252a521f97bcd5a68ddea5f8959a5c60b471c");

	PolimecNet::execute_with(|| {
		assert_ok!(PolimecFunding::verify_receiving_account_signature(
			&polimec_account,
			project_id,
			&Junction::AccountKey20 { network: Some(NetworkId::Ethereum { chain_id: 1 }), key: ethereum_account },
			signature,
		));
	});

	let polkadot_signature: [u8; 64] = hex!("7efee88bb61b74c91e6dc0ad48ea5b0118db77a579da8a8a753933d76cdc9e029c11f32a51b00fd3a1e3ce5b56cd1e275b179d4b195e7d527eebc60680291b81");
	let mut signature: [u8; 65] = [0u8; 65];
	signature[..64].copy_from_slice(polkadot_signature.as_slice());
	signature[64] = 0;
	let polkadot_address: [u8; 32] = hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d");

	PolimecNet::execute_with(|| {
		assert_ok!(PolimecFunding::verify_receiving_account_signature(
			&polimec_account,
			project_id,
			&Junction::AccountId32 { network: Some(NetworkId::Polkadot), id: polkadot_address },
			signature,
		));
	});
}
