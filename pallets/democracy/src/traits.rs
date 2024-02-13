// Copyright (C) Parity Technologies (UK) Ltd.

// Polimec Blockchain – https://www.polimec.org/
// Copyright (C) Polimec 2022. All rights reserved.

// This library includes code from Substrate, which is licensed
// under both the GNU General Public License version 3 (GPLv3) and the
// Apache License 2.0. You may choose to redistribute and/or modify this
// code under either the terms of the GPLv3 or the Apache 2.0 License,
// whichever suits your needs.

pub trait GetElectorate<Balance> {
	/// Calculate the total size of the electorate (tokens in circulation that might be used
	/// for voting) in terms of total Balance.
	/// Used for the referendum approval threshold calculation.
	/// Example: Total number of tokens in the system - total number of tokens in the treasury.
	fn get_electorate() -> Balance;
}
