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

	assert_eq!(
		output.status.success(),
		true,
		"Make sure you compile the node with \"std\" and \"fast-gov\" feature enabled before running this test."
	);

	dbg!(output);
}
