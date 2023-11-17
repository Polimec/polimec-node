#![allow(dead_code)]
#![allow(unused_imports)]
#![allow(unused_variables)]
#![allow(unused_macros)]

use core::panic;
use proc_macro::TokenStream;
use quote::{quote, ToTokens};
use sp_core::U256;
use syn::{parse::{Parse, ParseBuffer, ParseStream}, parse2, parse_macro_input, punctuated::Punctuated, token::Comma, Expr, GenericArgument, GenericParam, Generics, Ident, Result, Token, Type, Visibility, WhereClause, ItemMod};

pub fn generate_accounts_impl(_args: TokenStream, input: TokenStream) -> TokenStream {
    let item = parse_macro_input!(input as ItemMod);
    let name = &item.ident;
    let content = item.content.expect("Expected accounts defined").1;
    let names = parse_macro_input!(content as Punctuated<Ident, Comma>);

    let mut starting_value = U256::from(0u32);

    let generated_consts = names.iter().enumerate().map(|(i, name)| {
        let val = starting_value.checked_add(U256::from(i)).unwrap();
        let const_value: [u8; 32] = val.try_into().unwrap();
        quote! {
                pub const #name: [u8; 32] = #const_value;
            }
    });

    let output = quote! {
        pub mod #name {
            #(#generated_consts)*
        }
    };

    output.into()
}
