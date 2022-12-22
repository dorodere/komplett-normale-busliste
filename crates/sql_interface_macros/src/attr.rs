use std::{collections::HashMap, fmt};

use proc_macro2::Span;
use syn::{
    spanned::Spanned, Attribute, Error, Lit, Meta, MetaList, MetaNameValue, NestedMeta, Result,
};

#[allow(unused)] // is used in field_column and struct_attr
macro_rules! extract_value_from_lit {
    ($desc:literal as $target:path, from $from:ident named $key:literal $(,)?) => {
        $from
            .0
            .get($key)
            .map(|attr| {
                if let $target(lit) = attr.content.clone() {
                    Ok(lit.value())
                } else {
                    Err(Error::new(attr.span, format!("expected {} literal", $desc)))
                }
            })
            .transpose()?
    };
}

#[derive(Default)]
pub struct ParsedAttributes(pub HashMap<String, Attr>);

pub struct Attr {
    pub content: Lit,
    pub span: Span,
}

impl TryFrom<Vec<Attribute>> for ParsedAttributes {
    type Error = Error;

    fn try_from(attrs: Vec<Attribute>) -> Result<Self> {
        attrs
            .into_iter()
            .find_map(Self::parse_if_relevant)
            .unwrap_or_else(|| Ok(Self::default()))
    }
}

impl ParsedAttributes {
    fn parse_if_relevant(attr: Attribute) -> Option<Result<Self>> {
        if attr.path.get_ident()? != "sql" {
            return None;
        }

        let error_message =
            r#"sql attribute needs to be in the form of `#[sql(key = "value literal", ...)]` "#;

        let Ok(Meta::List(MetaList { nested, .. })) = attr.parse_meta() else {
            return error(attr, error_message);
        };

        let mut store = HashMap::new();

        for pair in nested {
            let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                path, lit, ..
            })) = pair else {
                return error(attr, error_message);
            };

            let Some(key) = path.get_ident() else { continue; };
            let span = lit.span();

            let attr = Attr { content: lit, span };

            let already_seen = store.insert(key.to_string(), attr).is_some();

            if already_seen {
                return error(span, "same key specified multiple times");
            }
        }

        Some(Ok(Self(store)))
    }
}

fn error<T: Spanned, U: fmt::Display>(on: T, message: U) -> Option<Result<ParsedAttributes>> {
    Some(Err(Error::new(on.span(), message)))
}

// don't ask why this is on top, textual scope is a weird thing
// this is the only way to use this in the child modules
macro_rules! extract_value_from_lit {
    ($desc:literal as $target:path, from $from:ident named $key:literal $(,)?) => {
        $from
            .0
            .get($key)
            .map(|attr| {
                if let $target(lit) = attr.content.clone() {
                    Ok(lit.value())
                } else {
                    Err(Error::new(attr.span, format!("expected {} literal", $desc)))
                }
            })
            .transpose()?
    };
}
