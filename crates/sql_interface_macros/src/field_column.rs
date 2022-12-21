use std::fmt;

use quote::ToTokens;
use syn::{
    spanned::Spanned, Attribute, Error, Field, Ident, Lit, Meta, MetaList, MetaNameValue,
    NestedMeta, Result, Type,
};

/// Maps from a field to a column, or the other way around if desired.
#[derive(Clone)]
pub struct FieldColumn {
    pub field_ident: Ident,
    pub ty: Type,
    pub complexity: Complexity,
}

/// How the field is to be rebuilt from columns.
#[derive(Clone)]
pub enum Complexity {
    /// The field has a complex type and needs multiple columns to be represented. Consult
    /// `<ty>::select_exprs()` for them.
    Complex { joined_on: Option<String> },
    /// The field is representable through exactly one column.
    Primitive { column: String },
}

impl FieldColumn {
    pub fn from_syn_field(field: Field) -> Result<Self> {
        let span = field.span();
        let field_ident = field
            .ident
            .ok_or_else(|| Error::new(span, "fields need to have explicit identifiers"))?;

        // parse the attributes from the field
        let attr = field
            .attrs
            .into_iter()
            .find_map(FieldAttr::parse_if_relevant)
            .unwrap_or_else(|| Ok(FieldAttr::default()))?;

        let complexity = match attr.is_complex {
            Some(true) => Complexity::Complex {
                joined_on: attr.joined_on,
            },
            _ => Complexity::Primitive {
                column: attr.column.unwrap_or_else(|| field_ident.to_string()),
            },
        };

        Ok(Self {
            field_ident,
            ty: field.ty,
            complexity,
        })
    }
}

#[derive(Default)]
struct FieldAttr {
    column: Option<String>,
    is_complex: Option<bool>,
    joined_on: Option<String>,
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

        let mut result = Self::default();

        for pair in nested {
            let NestedMeta::Meta(Meta::NameValue(MetaNameValue {
                path, lit, ..
            })) = &pair else {
                return error(attr, error_message);
            };
            let Some(key) = path.get_ident() else { continue; };

            let already_seen = match key.to_string().as_ref() {
                "column" => {
                    let Lit::Str(lit) = lit else {
                        return error(lit, "`column` requires a string literal");
                    };

                    result.column.replace(lit.value()).is_some()
                }
                "complex" => {
                    let Lit::Bool(lit) = lit else {
                        return error(lit, "`complex` requires a boolean literal");
                    };

                    result.is_complex.replace(lit.value()).is_some()
                }
                "joined_on" => {
                    let Lit::Str(lit) = lit else {
                        return error(lit, "`joined_on` requires a string literal");
                    };

                    result.joined_on.replace(lit.value()).is_some()
                }
                _ => false,
            };

            if already_seen {
                return error(pair, "same key specified multiple times");
            }
        }

        Some(Ok(result))
    }
}

fn error<T: ToTokens, U: fmt::Display>(on: T, message: U) -> Option<Result<FieldAttr>> {
    Some(Err(Error::new_spanned(on, message)))
}
