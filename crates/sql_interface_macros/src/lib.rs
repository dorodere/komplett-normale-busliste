extern crate proc_macro;

#[macro_use]
mod attr;
mod reconstruct;

use proc_macro::TokenStream;

use syn::{parse_macro_input, DeriveInput, Error};

#[proc_macro_derive(Reconstruct, attributes(sql))]
pub fn derive_reconstruct(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    reconstruct::generate_impl(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}
