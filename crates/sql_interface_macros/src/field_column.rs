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
    Complex { joined_on: JoinKind },
    /// The field is representable through exactly one column.
    Primitive { column: String },
}

#[derive(Clone)]
pub enum JoinKind {
    On(String),
    Condition,
    None,
}

impl FieldColumn {
    pub fn from_syn_field(field: Field) -> Result<Self> {
        let span = field.span();
        let field_ident = field
            .ident
            .ok_or_else(|| Error::new(span, "fields need to have explicit identifiers"))?;

        // parse the attributes from the field
        let attr = ParsedAttributes::try_from(field.attrs)?;
        let attr = FieldAttr::try_from(attr)?;

        let complexity = match attr.complex {
            Some(true) => Complexity::Complex {
                joined_on: match (attr.joined_on, attr.condition_in_join) {
                    (Some(clause), _) => JoinKind::On(clause),
                    (_, Some(true)) => JoinKind::Condition,
                    _ => JoinKind::None,
                },
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
    condition_in_join: Option<bool>,
}

impl TryFrom<ParsedAttributes> for FieldAttr {
    type Error = Error;

    fn try_from(value: ParsedAttributes) -> Result<Self> {
        let parsed = Self {
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
            condition_in_join: extract_value_from_lit!(
                "boolean" as Lit::Bool,
                from value named "condition_in_join",
            ),
        };

        if parsed.joined_on.is_some() && parsed.condition_in_join.unwrap_or(false) {
            return Err(Error::new(
                value.0.get("joined_on").unwrap().content.span(),
                r#"`joined_on` and `condition_in_join` conflict with each other, use only one"#,
            ));
        }

        Ok(parsed)
    }
}
