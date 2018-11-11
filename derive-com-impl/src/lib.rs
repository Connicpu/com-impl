#![recursion_limit = "1024"]

#[macro_use]
extern crate quote;
#[macro_use]
extern crate syn;

extern crate proc_macro;
extern crate proc_macro2;

use proc_macro::TokenStream;
use syn::DeriveInput;
use syn::{AttributeArgs, Item};

mod derive;
mod com_impl;

#[proc_macro_derive(ComImpl, attributes(interfaces, manual_iunknown))]
pub fn derive_com_impl(input: TokenStream) -> TokenStream {
    let input = parse_macro_input!(input as DeriveInput);
    
    derive::expand_derive_com_impl(&input)
        .unwrap_or_else(compile_error)
        .into()
}

#[proc_macro_attribute]
pub fn com_impl(attr: TokenStream, item: TokenStream) -> TokenStream {
    let args = parse_macro_input!(attr as AttributeArgs);
    let item = parse_macro_input!(item as Item);

    com_impl::expand_com_impl(&args, &item)
        .unwrap_or_else(compile_error)
        .into()
}

fn compile_error(message: String) -> proc_macro2::TokenStream {
    quote! {
        compile_error!(#message);
    }
}
