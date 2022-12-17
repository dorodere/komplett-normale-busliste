extern crate proc_macro;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, spanned::Spanned, Data, DeriveInput, Error, Ident, Result};

#[proc_macro_derive(Reconstruct)]
pub fn derive_reconstruct(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    generate_impl(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn generate_impl(input: DeriveInput) -> Result<TokenStream2> {
    let Data::Struct(target) = input.data else {
        return Err(Error::new_spanned(input, "Only structs can be derived, for now"));
    };

    let table = input.ident.to_string().to_lowercase();
    let ident = input.ident;

    let fields: Result<Vec<_>> = target
        .fields
        .iter()
        .map(|field| {
            let span = field.span();
            let column = field
                .ident
                .as_ref()
                .ok_or_else(|| Error::new(span, "Fields need to be named"))?;

            Ok((format!("{table}.{column}"), column))
        })
        .collect();

    let (select_exprs, field_idents): (Vec<_>, Vec<_>) = fields?.into_iter().unzip();

    Ok(expand(ident, table, select_exprs, field_idents))
}

fn expand(
    target_ident: Ident,
    table: String,
    select_exprs: Vec<String>,
    field_idents: Vec<&Ident>,
) -> TokenStream2 {
    quote! {
        impl crate::sql_struct::Reconstruct for #target_ident {
            fn required_tables() -> ::std::vec::Vec<&'static str> {
                ::std::vec![#table]
            }

            fn select_exprs() -> std::vec::Vec<&'static str> {
                ::std::vec![ #(
                    #select_exprs,
                )* ]
            }

            fn from_row<'a>(
                mut row: impl ::std::iter::Iterator<Item = ::rusqlite::types::ValueRef<'a>>
            ) -> crate::sql_struct::ReconstructResult<Self> {
                ::std::result::Result::Ok(
                    Self { #(
                        #field_idents: crate::sql_struct::next_converted(&mut row)?,
                    )* }
                )
            }
        }
    }
}
