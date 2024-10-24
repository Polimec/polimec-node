use crate::*;
use hex_literal::hex;
use sp_runtime::traits::Convert;

generate_accounts!(ETH_BUYER);

#[test]
fn test_hardcoded_signatures() {
	let polimec_account: PolimecAccountId = ETH_BUYER.into();
	let project_id = 1;

	// Values generated with `https://github.com/lrazovic/ethsigner`
	let polimec_account_ss58 = polimec_runtime::SS58Converter::convert(polimec_account.clone());
	let ethereum_account: [u8; 20] = hex!("FCAd0B19bB29D4674531d6f115237E16AfCE377c");
	let signature: [u8; 65] = hex!("4fa35369a2d654112d3fb419e24dc0d7d61b7e3f23936d6d4df0ac8608fa4530795971d4d1967da60853aa974ad57252a521f97bcd5a68ddea5f8959a5c60b471c");

	PolimecNet::execute_with(|| {
		assert_ok!(PolimecFunding::verify_receiving_account_signature(
			&polimec_account,
			project_id,
			&Junction::AccountKey20 { network: None, key: ethereum_account },
			signature,
		));
	});

	let polkadot_signature: [u8; 64] = hex!("32b486f3944c6345295777c84113c5339b786a6c1a4505ab876349e07a7938646b01aff5c7ceda542af794cde03c14eb255d905da8019aa7264fa1a766ab0188");
	let mut signature: [u8; 65] = [0u8; 65];
	signature[..64].copy_from_slice(polkadot_signature.as_slice());
	signature[64] = 0;
	let polkadot_address: [u8; 32] = hex!("d43593c715fdd31c61141abd04a99fd6822c8558854ccde39a5684e7a56da27d");

	PolimecNet::execute_with(|| {
		assert_ok!(PolimecFunding::verify_receiving_account_signature(
			&polimec_account,
			project_id,
			&Junction::AccountId32 { network: None, id: polkadot_address },
			signature,
		));
	});
}
