extern crate proc_macro;
use proc_macro::TokenStream;

mod generate_accounts;

#[proc_macro]
pub fn generate_accounts(input: TokenStream) -> TokenStream {
    generate_accounts::generate_accounts_impl(input)
}

