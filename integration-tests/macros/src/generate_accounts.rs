#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_macros)]

use core::panic;
use proc_macro::TokenStream;
use quote::{format_ident, quote, ToTokens};
use sp_core::U256;
use syn::{
	parse::{Parse, ParseBuffer, ParseStream},
	parse2, parse_macro_input,
	punctuated::Punctuated,
	token::Comma,
	Expr, GenericArgument, GenericParam, Generics, Ident, ItemMod, Result, Token, Type, Visibility, WhereClause,
};

pub fn generate_accounts_impl(input: TokenStream) -> TokenStream {
	let inputs = parse_macro_input!(input with Punctuated::<Ident, Token![,]>::parse_terminated);
	let mut output = quote! {};
	let mut insertions = Vec::new();

	for input in inputs {
		let name = input.to_string();

		// Ensure the name is all uppercase
		if name != name.to_uppercase() {
			panic!("Name must be in all uppercase");
		}

		// Generate a unique [u8; 32] value for the constant
		let mut value = [0u8; 32];
		for (i, byte) in name.bytes().enumerate() {
			value[i % 32] ^= byte;
		}

		let ident = format_ident!("{}", name);

		// Convert the array into a tuple for the quote macro
		let value_iter = value.clone().into_iter();

		output.extend(quote! {
			pub const #input: [u8; 32] = [#(#value_iter), *];
		});

		insertions.push(quote! {
			names.insert(#input, stringify!(#ident));
		});
	}

	output.extend(quote! {
		pub fn names() -> std::collections::HashMap<[u8; 32], &'static str> {
			let mut names = std::collections::HashMap::new();
			#(#insertions)*
			names
		}
	});

	TokenStream::from(output)
}
