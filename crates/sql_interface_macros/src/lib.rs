extern crate proc_macro;

mod field_column;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{parse_macro_input, Data, DeriveInput, Error, Ident, Result};

use field_column::FieldColumn;

#[proc_macro_derive(Reconstruct, attributes(sql))]
pub fn derive_reconstruct(item: TokenStream) -> TokenStream {
    let input = parse_macro_input!(item as DeriveInput);

    generate_impl(input)
        .unwrap_or_else(Error::into_compile_error)
        .into()
}

fn generate_impl(input: DeriveInput) -> Result<TokenStream2> {
    let Data::Struct(target) = input.data else {
        return Err(Error::new_spanned(input, "only structs can be derived from Reconstruct, for now"));
    };

    let table = input.ident.to_string().to_lowercase();
    let ident = input.ident;

    let fields: Result<Vec<_>> = target
        .fields
        .into_iter()
        .map(FieldColumn::from_syn_field)
        .collect();

    let (select_exprs, field_idents) = fields?
        .clone()
        .into_iter()
        .map(|mapping| (mapping.column, mapping.field_ident))
        .unzip();

    Ok(expand(ident, table, select_exprs, field_idents))
}

fn expand(
    target_ident: Ident,
    table: String,
    select_exprs: Vec<String>,
    field_idents: Vec<Ident>,
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
