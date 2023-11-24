#[test]
fn test_generate_accounts_macro() {
	macros::generate_accounts!(ALICE, BOB,);

	dbg!(ALICE);
	dbg!(BOB);
	dbg!(names());
}
