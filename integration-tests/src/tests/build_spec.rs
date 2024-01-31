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
#[ignore]
#[test]
fn build_spec_testing_node() {
	// run the polimec-parachain-node compiled with "std" with the build-spec command and --raw flag
	// This makes sure our async parallel instantiation of projects is working as intended.
	// We need this to test the protocol with the UI.

	match std::env::current_dir() {
		Ok(path) => {
			println!("The current directory is {}", path.display());
		},
		Err(e) => {
			println!("Error getting the current directory: {}", e);
		},
	}

	let output = std::process::Command::new("../target/release/polimec-parachain-node")
		.arg("build-spec")
		.arg("--chain=polimec-testing")
		.arg("--disable-default-bootnode")
		.arg("--raw")
		.output()
		.expect("failed to execute process");
	
	dbg!(output.clone());
	assert_eq!(
		output.status.success(),
		true,
		"Make sure you compile the polimec-parachain-node with \"--release\" and \"--features std,fast-mode\" before running this test."
	);
}
