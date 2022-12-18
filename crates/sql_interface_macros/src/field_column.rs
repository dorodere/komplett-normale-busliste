use std::fmt;

use quote::ToTokens;
use syn::{
    spanned::Spanned, Attribute, Error, Field, Ident, Lit, Meta, MetaList, MetaNameValue,
    NestedMeta, Result,
};

/// Maps from a field to a column, or the other way around if desired.
#[derive(Clone, Debug)]
pub struct FieldColumn {
    pub field_ident: Ident,
    pub column: String,
}

impl FieldColumn {
    pub fn from_syn_field(field: Field) -> Result<Self> {
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

        Ok(Self {
            field_ident,
            column,
        })
    }
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
            return error(attr, error_message);
        };

        let mut result = Self { column: None };

        for pair in nested {
            let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                path, lit: Lit::Str(value_lit), ..
            })) = &pair else {
                return error(attr, error_message);
            };
            let Some(key) = path.get_ident() else { continue; };
            let value = value_lit.value();

            let previous = match key.to_string().as_ref() {
                "column" => result.column.replace(value),
                _ => None,
            };

            if previous.is_some() {
                return error(pair, "same key specified multiple times");
            }
        }

        Some(Ok(result))
    }
}

fn error<T: ToTokens, U: fmt::Display>(on: T, message: U) -> Option<Result<FieldAttr>> {
    Some(Err(Error::new_spanned(on, message)))
}
