use syn::{spanned::Spanned, Error, Field, Ident, Lit, Result, Type};

use crate::attr::ParsedAttributes;

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
        let attr: ParsedAttributes = field.attrs.try_into()?;
        let attr = FieldAttr::try_from(attr)?;

        let complexity = match attr.complex {
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
    complex: Option<bool>,
    joined_on: Option<String>,
}

impl TryFrom<ParsedAttributes> for FieldAttr {
    type Error = Error;

    fn try_from(value: ParsedAttributes) -> Result<Self> {
        Ok(Self {
            column: extract_value_from_lit!(
                "string" as Lit::Str,
                from value named "column",
            ),
            complex: extract_value_from_lit!(
                "boolean" as Lit::Bool,
                from value named "complex",
            ),
            joined_on: extract_value_from_lit!(
                "string" as Lit::Str,
                from value named "joined_on",
            ),
        })
    }
}
