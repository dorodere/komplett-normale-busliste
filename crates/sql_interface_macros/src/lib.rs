extern crate proc_macro;

use std::fmt;

use proc_macro::TokenStream;
use proc_macro2::TokenStream as TokenStream2;
use quote::{quote, ToTokens};
use syn::{
    parse_macro_input, spanned::Spanned, Attribute, Data, DeriveInput, Error, Field, Ident, Lit,
    Meta, MetaList, MetaNameValue, NestedMeta, Result,
};

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
        .map(|field| column_and_ident(&table, field))
        .collect();

    let (select_exprs, field_idents): (Vec<_>, Vec<_>) = fields?.into_iter().unzip();

    Ok(expand(ident, table, select_exprs, field_idents))
}

fn column_and_ident(table: impl fmt::Display, field: Field) -> Result<(String, Ident)> {
    let span = field.span();
    let field_ident = field
        .ident
        .ok_or_else(|| Error::new(span, "fields need to have explicit identifiers"))?;

    // see if there's an attribute which overrides the field ident as column
    let column = field
        .attrs
        .into_iter()
        .find_map(|raw_attr| {
            FieldAttr::parse_if_relevant(raw_attr)?
                .map(|attr| attr.column)
                .transpose()
        })
        .unwrap_or_else(|| Ok(field_ident.to_string()))?;

    Ok((format!("{table}.{column}"), field_ident))
}

struct FieldAttr {
    column: Option<String>,
}

impl FieldAttr {
    fn parse_if_relevant(attr: Attribute) -> Option<Result<Self>> {
        if attr.path.get_ident()? != "sql" {
            return None;
        }

        let error_message = r#"attribute needs to be in the form of `#[sql(column = "...")]` "#;

        let Ok(Meta::List(MetaList { nested, .. })) = attr.parse_meta() else {
            return Some(Err(Error::new_spanned(attr, error_message)));
        };

        let mut result = Self { column: None };

        for pair in nested {
            let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                path, lit: Lit::Str(value_lit), ..
            })) = &pair else {
                return Some(Err(Error::new_spanned(attr, error_message)));
            };
            let Some(key) = path.get_ident() else { continue; };
            let value = value_lit.value();

            let previous_value = match key.to_string().as_ref() {
                "column" => result.column.replace(value),
                _ => None,
            };

            if previous_value.is_some() {
                return error(pair, "same key specified multiple times");
            }
        }

        Some(Ok(result))
    }
}

fn error<T: ToTokens, U: fmt::Display>(on: T, message: U) -> Option<Result<FieldAttr>> {
    Some(Err(Error::new_spanned(on, message)))
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
