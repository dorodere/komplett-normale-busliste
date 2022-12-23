use std::iter;

use proc_macro2::TokenStream as TokenStream2;
use quote::quote;
use syn::{Data, DeriveInput, Error, Ident, Result};

use crate::attr::ParsedAttributes;

use self::{
    field_column::{Complexity, FieldColumn, JoinClause, SqlLink},
    struct_attr::StructAttr,
};

mod field_column;
mod struct_attr;

pub fn generate_impl(input: DeriveInput) -> Result<TokenStream2> {
    let Data::Struct(target) = input.data else {
        return Err(Error::new_spanned(input, "only structs can be derived from Reconstruct, for now"));
    };

    let attr = ParsedAttributes::try_from(input.attrs)?;
    let struct_attr = StructAttr::try_from(attr)?;

    let table = struct_attr
        .table
        .unwrap_or_else(|| input.ident.to_string().to_lowercase());
    let ident = input.ident;

    let fields: Result<Vec<_>> = target
        .fields
        .into_iter()
        .map(FieldColumn::from_syn_field)
        .collect();
    let fields = fields?;

    let tables = iter::once(quote! { vec![#table] })
        .chain(
            fields
                .clone()
                .into_iter()
                .filter_map(expand_table_if_complex),
        )
        .collect();

    let joins = fields
        .clone()
        .into_iter()
        .filter_map(expand_join_if_complex)
        .collect();

    let (select_exprs, reconstruct_exprs) = fields
        .into_iter()
        .map(|mapping| {
            (
                expand_select_expr(mapping.clone(), &table),
                expand_reconstruct_expr(mapping),
            )
        })
        .unzip();

    Ok(expand(
        ident,
        tables,
        joins,
        select_exprs,
        reconstruct_exprs,
    ))
}

fn expand_table_if_complex(
    FieldColumn { ty, complexity, .. }: FieldColumn,
) -> Option<TokenStream2> {
    if let Complexity::Complex { .. } = complexity {
        Some(quote! { <#ty>::required_tables() })
    } else {
        None
    }
}

fn expand_join_if_complex(FieldColumn { ty, complexity, .. }: FieldColumn) -> Option<TokenStream2> {
    let Complexity::Complex {
        join: Some(join_clause),
    } = complexity else {
        return None;
    };

    let clause = match join_clause {
        JoinClause::On(clause) => quote! { crate::sql_struct::JoinClause::On(#clause) },
        JoinClause::Condition => quote! { crate::sql_struct::JoinClause::Condition },
    };

    Some(quote! {
        ::std::vec![
            crate::sql_struct::Join {
                table: <#ty>::required_tables()[0],
                clause: #clause,
            }
        ],
        <#ty>::required_joins()
    })
}

fn expand_select_expr(
    FieldColumn { ty, complexity, .. }: FieldColumn,
    table: impl AsRef<str>,
) -> TokenStream2 {
    match complexity {
        Complexity::Complex { .. } => quote! { <#ty>::select_exprs() },
        Complexity::Primitive { link } => {
            let expr = match link {
                SqlLink::Column(column) => {
                    let table = table.as_ref();
                    format!("{table}.{column}")
                }
                SqlLink::Expression(expr) => expr,
            };

            quote! { vec![#expr] }
        }
    }
}

fn expand_reconstruct_expr(
    FieldColumn {
        field_ident,
        ty,
        complexity,
        ..
    }: FieldColumn,
) -> TokenStream2 {
    if let Complexity::Complex { .. } = complexity {
        quote! {
            #field_ident: <#ty>::from_row(
                (&mut row).take(<#ty>::select_exprs().len())
            )?,
        }
    } else {
        quote! {
            #field_ident: crate::sql_struct::next_converted(&mut row)?,
        }
    }
}

fn expand(
    target_ident: Ident,
    tables: Vec<TokenStream2>,
    joins: Vec<TokenStream2>,
    select_exprs: Vec<TokenStream2>,
    reconstruct_exprs: Vec<TokenStream2>,
) -> TokenStream2 {
    let tables = if tables.is_empty() {
        quote! { ::std::vec::Vec::new() }
    } else {
        quote! {
            [ #(
                #tables,
            )* ]
                .into_iter()
                .flatten()
                .collect()
        }
    };

    let joins = if joins.is_empty() {
        quote! { ::std::vec::Vec::new() }
    } else {
        quote! {
            [ #(
                #joins,
            )* ]
                .into_iter()
                .flatten()
                .collect()
        }
    };

    quote! {
        impl crate::sql_struct::Reconstruct for #target_ident {
            fn required_tables() -> ::std::vec::Vec<&'static str> {
                #tables
            }

            fn required_joins() -> ::std::vec::Vec<crate::sql_struct::Join> {
                #joins
            }

            fn select_exprs() -> std::vec::Vec<&'static str> {
                [ #(
                    #select_exprs,
                )* ]
                    .into_iter()
                    .flatten()
                    .collect()
            }

            fn from_row<'a>(
                mut row: impl ::std::iter::Iterator<Item = ::rusqlite::types::ValueRef<'a>>
            ) -> crate::sql_struct::ReconstructResult<Self> {
                ::std::result::Result::Ok(
                    Self { #(
                        #reconstruct_exprs
                    )* }
                )
            }
        }
    }
}
