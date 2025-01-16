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
	let ethereum_account: [u8; 20] = hex!("796afe7b8933ee8cf337f17887e5c19b657f9ab8");
	let signature: [u8; 65] = hex!("952e312ac892fefc7c69051521e78a3bc2727fbb495585bdb5fb77e662b8a3de2b1254058d824e85034710e338c2590e2f92d74ce3c60292ed25c7537d94ed621b");

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
