use macros::generate_accounts;

#[test]
fn test_generate_accounts_macro() {
	generate_accounts!(ALICE, BOB,);

	dbg!(ALICE);
	dbg!(BOB);
	dbg!(names());
}
