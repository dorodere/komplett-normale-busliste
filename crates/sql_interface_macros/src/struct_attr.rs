use syn::{Error, Lit, Result};

use crate::attr::ParsedAttributes;

#[derive(Default)]
pub struct StructAttr {
    pub table: Option<String>,
}

impl TryFrom<ParsedAttributes> for StructAttr {
    type Error = Error;

    fn try_from(value: ParsedAttributes) -> Result<Self> {
        Ok(Self {
            table: extract_value_from_lit!(
                "string" as Lit::Str,
                from value named "table",
            ),
        })
    }
}
