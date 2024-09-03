
//! Autogenerated weights for `pallet_identity`
//!
//! THIS FILE WAS AUTO-GENERATED USING THE SUBSTRATE BENCHMARK CLI VERSION 39.0.0
//! DATE: 2024-08-30, STEPS: `50`, REPEAT: `20`, LOW RANGE: `[]`, HIGH RANGE: `[]`
//! WORST CASE MAP SIZE: `1000000`
//! HOSTNAME: `ip-172-31-23-147`, CPU: `AMD EPYC 9R14`
//! WASM-EXECUTION: `Compiled`, CHAIN: `Some("polimec-paseo-local")`, DB CACHE: 1024

// Executed Command:
// ./target/production/polimec-node
// benchmark
// pallet
// --chain=polimec-paseo-local
// --wasm-execution=compiled
// --pallet=pallet_identity
// --extrinsic=*
// --steps=50
// --repeat=20
// --output=./runtimes/polimec/src/weights/pallet_identity.rs

#![cfg_attr(rustfmt, rustfmt_skip)]
#![allow(unused_parens)]
#![allow(unused_imports)]
#![allow(missing_docs)]

use frame_support::{traits::Get, weights::Weight};
use core::marker::PhantomData;

/// Weight functions for `pallet_identity`.
pub struct WeightInfo<T>(PhantomData<T>);
impl<T: frame_system::Config> pallet_identity::WeightInfo for WeightInfo<T> {
	/// Storage: `Identity::Registrars` (r:1 w:1)
	/// Proof: `Identity::Registrars` (`max_values`: Some(1), `max_size`: Some(172), added: 667, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 2]`.
	fn add_registrar(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `29 + r * (58 ±0)`
		//  Estimated: `1657`
		// Minimum execution time: 6_240_000 picoseconds.
		Weight::from_parts(6_610_804, 0)
			.saturating_add(Weight::from_parts(0, 1657))
			// Standard Error: 25_426
			.saturating_add(Weight::from_parts(66_097, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 3]`.
	fn set_identity(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6977 + r * (5 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 110_801_000 picoseconds.
		Weight::from_parts(113_591_971, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 33_516
			.saturating_add(Weight::from_parts(100_867, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::IdentityOf` (r:1 w:0)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SubsOf` (r:1 w:1)
	/// Proof: `Identity::SubsOf` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SuperOf` (r:2 w:2)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[0, 2]`.
	fn set_subs_new(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `100`
		//  Estimated: `10680 + s * (2589 ±0)`
		// Minimum execution time: 6_720_000 picoseconds.
		Weight::from_parts(7_183_870, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 8_735
			.saturating_add(Weight::from_parts(3_260_648, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().reads((1_u64).saturating_mul(s.into())))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(s.into())))
			.saturating_add(Weight::from_parts(0, 2589).saturating_mul(s.into()))
	}
	/// Storage: `Identity::IdentityOf` (r:1 w:0)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SubsOf` (r:1 w:1)
	/// Proof: `Identity::SubsOf` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SuperOf` (r:0 w:2)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// The range of component `p` is `[0, 2]`.
	fn set_subs_old(p: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `100 + p * (124 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 6_720_000 picoseconds.
		Weight::from_parts(7_251_080, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 9_354
			.saturating_add(Weight::from_parts(2_033_971, 0).saturating_mul(p.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
			.saturating_add(T::DbWeight::get().writes((1_u64).saturating_mul(p.into())))
	}
	/// Storage: `Identity::SubsOf` (r:1 w:1)
	/// Proof: `Identity::SubsOf` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SuperOf` (r:0 w:2)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 3]`.
	/// The range of component `s` is `[0, 2]`.
	fn clear_identity(r: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6886 + r * (5 ±0) + s * (124 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 44_640_000 picoseconds.
		Weight::from_parts(46_291_053, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 19_500
			.saturating_add(Weight::from_parts(269_645, 0).saturating_mul(r.into()))
			// Standard Error: 19_500
			.saturating_add(Weight::from_parts(2_062_698, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(s.into())))
	}
	/// Storage: `Identity::Registrars` (r:1 w:0)
	/// Proof: `Identity::Registrars` (`max_values`: Some(1), `max_size`: Some(172), added: 667, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 3]`.
	fn request_judgement(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6965 + r * (58 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 69_021_000 picoseconds.
		Weight::from_parts(70_637_997, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 21_973
			.saturating_add(Weight::from_parts(203_043, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 3]`.
	fn cancel_request(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `6998`
		//  Estimated: `10680`
		// Minimum execution time: 71_561_000 picoseconds.
		Weight::from_parts(73_521_316, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 30_273
			.saturating_add(Weight::from_parts(156_236, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::Registrars` (r:1 w:1)
	/// Proof: `Identity::Registrars` (`max_values`: Some(1), `max_size`: Some(172), added: 667, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 2]`.
	fn set_fee(_r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `88 + r * (57 ±0)`
		//  Estimated: `1657`
		// Minimum execution time: 4_930_000 picoseconds.
		Weight::from_parts(5_394_134, 0)
			.saturating_add(Weight::from_parts(0, 1657))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::Registrars` (r:1 w:1)
	/// Proof: `Identity::Registrars` (`max_values`: Some(1), `max_size`: Some(172), added: 667, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 2]`.
	fn set_account_id(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `88 + r * (57 ±0)`
		//  Estimated: `1657`
		// Minimum execution time: 4_300_000 picoseconds.
		Weight::from_parts(4_608_024, 0)
			.saturating_add(Weight::from_parts(0, 1657))
			// Standard Error: 17_526
			.saturating_add(Weight::from_parts(66_987, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::Registrars` (r:1 w:1)
	/// Proof: `Identity::Registrars` (`max_values`: Some(1), `max_size`: Some(172), added: 667, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 2]`.
	fn set_fields(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `88 + r * (57 ±0)`
		//  Estimated: `1657`
		// Minimum execution time: 4_280_000 picoseconds.
		Weight::from_parts(4_600_659, 0)
			.saturating_add(Weight::from_parts(0, 1657))
			// Standard Error: 17_573
			.saturating_add(Weight::from_parts(66_220, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::Registrars` (r:1 w:0)
	/// Proof: `Identity::Registrars` (`max_values`: Some(1), `max_size`: Some(172), added: 667, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 2]`.
	fn provide_judgement(r: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `7045 + r * (57 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 87_501_000 picoseconds.
		Weight::from_parts(84_905_451, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 105_588
			.saturating_add(Weight::from_parts(4_450_124, 0).saturating_mul(r.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::SubsOf` (r:1 w:1)
	/// Proof: `Identity::SubsOf` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `System::Account` (r:2 w:2)
	/// Proof: `System::Account` (`max_values`: None, `max_size`: Some(128), added: 2603, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SuperOf` (r:0 w:2)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// The range of component `r` is `[1, 3]`.
	/// The range of component `s` is `[0, 2]`.
	fn kill_identity(r: u32, s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `7129 + r * (5 ±0) + s * (124 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 55_800_000 picoseconds.
		Weight::from_parts(55_999_794, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 22_851
			.saturating_add(Weight::from_parts(564_960, 0).saturating_mul(r.into()))
			// Standard Error: 22_851
			.saturating_add(Weight::from_parts(2_208_880, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(2))
			.saturating_add(T::DbWeight::get().writes((2_u64).saturating_mul(s.into())))
	}
	/// Storage: `Identity::IdentityOf` (r:1 w:0)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SuperOf` (r:1 w:1)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SubsOf` (r:1 w:1)
	/// Proof: `Identity::SubsOf` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[0, 1]`.
	fn add_sub(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `100 + s * (216 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 10_750_000 picoseconds.
		Weight::from_parts(11_405_397, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 37_456
			.saturating_add(Weight::from_parts(2_813_602, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Identity::IdentityOf` (r:1 w:0)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SuperOf` (r:1 w:1)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[1, 2]`.
	fn rename_sub(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `237 + s * (37 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 9_360_000 picoseconds.
		Weight::from_parts(9_128_724, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 29_582
			.saturating_add(Weight::from_parts(769_687, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::IdentityOf` (r:1 w:0)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SuperOf` (r:1 w:1)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SubsOf` (r:1 w:1)
	/// Proof: `Identity::SubsOf` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[1, 2]`.
	fn remove_sub(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `283 + s * (70 ±0)`
		//  Estimated: `10680`
		// Minimum execution time: 13_780_000 picoseconds.
		Weight::from_parts(13_595_093, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			// Standard Error: 43_152
			.saturating_add(Weight::from_parts(965_553, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(3))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Identity::SuperOf` (r:1 w:1)
	/// Proof: `Identity::SuperOf` (`max_values`: None, `max_size`: Some(114), added: 2589, mode: `MaxEncodedLen`)
	/// Storage: `Identity::SubsOf` (r:1 w:1)
	/// Proof: `Identity::SubsOf` (`max_values`: None, `max_size`: Some(121), added: 2596, mode: `MaxEncodedLen`)
	/// The range of component `s` is `[0, 1]`.
	fn quit_sub(s: u32, ) -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `293 + s * (70 ±0)`
		//  Estimated: `3586`
		// Minimum execution time: 11_620_000 picoseconds.
		Weight::from_parts(12_259_695, 0)
			.saturating_add(Weight::from_parts(0, 3586))
			// Standard Error: 37_807
			.saturating_add(Weight::from_parts(743_404, 0).saturating_mul(s.into()))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(2))
	}
	/// Storage: `Identity::UsernameAuthorities` (r:0 w:1)
	/// Proof: `Identity::UsernameAuthorities` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn add_username_authority() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `0`
		//  Estimated: `0`
		// Minimum execution time: 4_040_000 picoseconds.
		Weight::from_parts(4_390_000, 0)
			.saturating_add(Weight::from_parts(0, 0))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::UsernameAuthorities` (r:1 w:1)
	/// Proof: `Identity::UsernameAuthorities` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	fn remove_username_authority() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `79`
		//  Estimated: `3534`
		// Minimum execution time: 7_190_000 picoseconds.
		Weight::from_parts(7_510_000, 0)
			.saturating_add(Weight::from_parts(0, 3534))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::UsernameAuthorities` (r:1 w:1)
	/// Proof: `Identity::UsernameAuthorities` (`max_values`: None, `max_size`: Some(69), added: 2544, mode: `MaxEncodedLen`)
	/// Storage: `Identity::AccountOfUsername` (r:1 w:1)
	/// Proof: `Identity::AccountOfUsername` (`max_values`: None, `max_size`: Some(81), added: 2556, mode: `MaxEncodedLen`)
	/// Storage: `Identity::PendingUsernames` (r:1 w:0)
	/// Proof: `Identity::PendingUsernames` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	fn set_username_for() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `79`
		//  Estimated: `10680`
		// Minimum execution time: 54_251_000 picoseconds.
		Weight::from_parts(55_760_000, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			.saturating_add(T::DbWeight::get().reads(4))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Identity::PendingUsernames` (r:1 w:1)
	/// Proof: `Identity::PendingUsernames` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	/// Storage: `Identity::AccountOfUsername` (r:0 w:1)
	/// Proof: `Identity::AccountOfUsername` (`max_values`: None, `max_size`: Some(81), added: 2556, mode: `MaxEncodedLen`)
	fn accept_username() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `114`
		//  Estimated: `10680`
		// Minimum execution time: 15_610_000 picoseconds.
		Weight::from_parts(16_160_000, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(3))
	}
	/// Storage: `Identity::PendingUsernames` (r:1 w:1)
	/// Proof: `Identity::PendingUsernames` (`max_values`: None, `max_size`: Some(85), added: 2560, mode: `MaxEncodedLen`)
	fn remove_expired_approval() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `114`
		//  Estimated: `3550`
		// Minimum execution time: 8_040_000 picoseconds.
		Weight::from_parts(8_280_000, 0)
			.saturating_add(Weight::from_parts(0, 3550))
			.saturating_add(T::DbWeight::get().reads(1))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::AccountOfUsername` (r:1 w:0)
	/// Proof: `Identity::AccountOfUsername` (`max_values`: None, `max_size`: Some(81), added: 2556, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:1)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	fn set_primary_username() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `256`
		//  Estimated: `10680`
		// Minimum execution time: 13_340_000 picoseconds.
		Weight::from_parts(13_780_000, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
	/// Storage: `Identity::AccountOfUsername` (r:1 w:1)
	/// Proof: `Identity::AccountOfUsername` (`max_values`: None, `max_size`: Some(81), added: 2556, mode: `MaxEncodedLen`)
	/// Storage: `Identity::IdentityOf` (r:1 w:0)
	/// Proof: `Identity::IdentityOf` (`max_values`: None, `max_size`: Some(7215), added: 9690, mode: `MaxEncodedLen`)
	fn remove_dangling_username() -> Weight {
		// Proof Size summary in bytes:
		//  Measured:  `97`
		//  Estimated: `10680`
		// Minimum execution time: 8_960_000 picoseconds.
		Weight::from_parts(9_260_000, 0)
			.saturating_add(Weight::from_parts(0, 10680))
			.saturating_add(T::DbWeight::get().reads(2))
			.saturating_add(T::DbWeight::get().writes(1))
	}
}
