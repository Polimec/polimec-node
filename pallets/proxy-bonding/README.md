<!-- cargo-rdme start -->

# Proxy Bonding Pallet

A FRAME pallet that facilitates token bonding operations with fee management capabilities. This pallet allows users to bond tokens from a configurable account (we call Treasury) while paying fees in various assets.
This pallet is intended to be used as an alternative to a direct bonding mechanism. In this way, the user does not need to own or hold the tokens, but can still participate in various activities by paying a fee.

## Overview

The Bonding Pallet provides functionality to:
- Bond treasury tokens on behalf of users
- Pay a bonding fee in different assets (e.g., DOT)
- Set the bond release to either immediate refund or time-locked release

## Features

### Token Bonding
- Bond tokens from a treasury account into sub-accounts
- Support for existential deposit management
- Hold-based bonding mechanism using runtime-defined hold reasons

### Fee Management
- Accept fees in configurable assets (e.g., DOT)
- Calculate fees based on bond amount and current token prices
- Support both fee refunds and fee transfers to recipients
- Percentage-based fee calculation in USD terms

### Release Mechanisms
Two types of release mechanisms are supported:
- Immediate refund: Bonds can be immediately returned to treasury, and fees await refunding to users.
- Time-locked release: Bonds are locked until a specific block number, and fees can be sent to the configured fee recipient.

### Example Configuration (Similar on how it's configured on the Polimec Runtime)

```rust
parameter_types! {
	// Fee is defined as 1.5% of the USD Amount. Since fee is applied to the PLMC amount, and that is always 5 times
	// less than the usd_amount (multiplier of 5), we multiply the 1.5 by 5 to get 7.5%
	pub FeePercentage: Perbill = Perbill::from_rational(75u32, 1000u32);
	pub FeeRecipient: AccountId =  AccountId::from(hex_literal::hex!("3ea952b5fa77f4c67698e79fe2d023a764a41aae409a83991b7a7bdd9b74ab56"));
	pub RootId: PalletId = PalletId(*b"treasury");
}

impl pallet_proxy_bonding::Config for Runtime {
	type BondingToken = Balances; // The Balances pallet is used for the bonding token
	type BondingTokenDecimals = ConstU8<10>; // The PLMC token has 10 decimals
	type BondingTokenId = ConstU32<X>; // TODO: Replace with a proper number and explanation.
	type FeePercentage = FeePercentage; // The fee kept by the treasury
	type FeeRecipient = FeeRecipient; // THe account that receives the fee
	type FeeToken = ForeignAssets; // The Asset pallet is used for the fee token
	type Id = PalletId; // The ID type used for the ... account
	type PriceProvider = OraclePriceProvider<AssetId, Price, Oracle>; // The Oracle pallet is used for the price provider
	type RootId = TreasuryId; // The treasury account ID
	type Treasury = TreasuryAccount; // The treasury account
	type UsdDecimals = ConstU8<X>; // TODO: Replace with a proper number and explanation.
	type RuntimeEvent = RuntimeEvent;
	type RuntimeHoldReason = RuntimeHoldReason;
}
```


## Extrinsics
`transfer_bonds_back_to_treasury`
`transfer_fees_to_recipient`

## Public Functions
`calculate_fee`
`get_bonding_account`
`bond_on_behalf_of`
`set_release_type`
`refund_fee`


### transfer_bonds_back_to_treasury
Transfer bonded tokens back to the treasury when release conditions are met.

Parameters:
- `derivation_path`: The sub-account derivation path
- `hold_reason`: The reason for the hold
- `origin`: Signed origin

### transfer_fees_to_recipient
Transfer collected fees to the designated fee recipient.

Parameters:
- `derivation_path`: The sub-account derivation path
- `hold_reason`: The reason for the hold
- `fee_asset`: The asset ID of the fee token
- `origin`: Signed origin

## Public Functions

### bond_on_behalf_of
Bonds tokens from the treasury into a sub-account on behalf of a user.

Parameters:
- `derivation_path`: Sub-account derivation path
- `account`: Account ID of the user
- `bond_amount`: Amount of tokens to bond
- `fee_asset`: Asset ID of the fee token
- `hold_reason`: Reason for the hold

### calculate_fee
Calculates the fee amount in the specified fee asset based on the bond amount.

### refund_fee
Refunds the fee to the specified account.

## Events

- `BondsTransferredBackToTreasury`: Emitted when bonds are transferred back to treasury
- `FeesTransferredToFeeRecipient`: Emitted when fees are transferred to the fee recipient

## Errors

- `ReleaseTypeNotSet`: Release type not configured for the given derivation path/hold reason
- `TooEarlyToUnlock`: Attempted to unlock tokens before the configured release block
- `FeeToRecipientDisallowed`: Fee transfer to recipient not allowed for refunded release type
- `FeeRefundDisallowed`: Fee refund not allowed for locked release type
- `PriceNotAvailable`: Price information unavailable for fee calculation

## Example integration

The Proxy Bonding Pallet work seamlessly with the Funding Pallet to handle OTM (One-Token-Model) participation modes in project funding. Here's how the integration works:

### Contribution Flow
1. When a user contributes to a project using OTM mode:
- The Funding Pallet calls `bond_on_behalf_of` with:
- Project ID as the derivation path
- User's account
- PLMC bond amount
- Funding asset ID
- Participation hold reason

2. During project settlement phase:
- For successful projects:
- An OTM release type is set with a time-lock based on the multiplier
- Bonds remain locked until the vesting duration completes
- For failed projects:
- Release type is set to `Refunded`
- Allows immediate return of bonds to treasury
- Enables fee refunds to participants

### Key Interactions
```rust
// In Funding Pallet
pub fn bond_plmc_with_mode(
	who: &T::AccountId,
	project_id: ProjectId,
	amount: Balance,
	mode: ParticipationMode,
	asset: AcceptedFundingAsset,
) -> DispatchResult {
	match mode {
		ParticipationMode::OTM => pallet_proxy_bonding::Pallet::<T>::bond_on_behalf_of(
			project_id,
			who.clone(),
			amount,
			asset.id(),
			HoldReason::Participation.into(),
		),
		ParticipationMode::Classic(_) => // ... other handling
	}
}
```

### Settlement Process
The settlement process determines the release conditions for bonded tokens:
- Success: Tokens remain locked with a time-based release schedule
- Failure: Tokens are marked for immediate return to treasury with fee refunds

## License

License: GPL-3.0

<!-- cargo-rdme end -->
