use rusqlite::types::{FromSql, FromSqlError, ValueRef};
use thiserror::Error;

pub trait Reconstruct
where
    Self: Sized,
{
    /// The tables this struct is stored in, or depends on.
    fn required_tables() -> Vec<&'static str>;

    /// Which `LEFT OUTER JOIN`s the reconstruction of this struct requires.
    fn required_joins() -> Vec<Join>;

    /// Returns all SQL expressions this struct needs in order to be built in
    /// [`Reconstruct::from_row`].
    fn select_exprs() -> Vec<&'static str>;

    /// Reconstructs the implementor of this trait from a row, following the schema of
    /// [`Reconstruct::select_exprs`] in the same order.
    fn from_row<'a>(row: impl Iterator<Item = ValueRef<'a>>) -> ReconstructResult<Self>;
}

pub struct Join {
    pub table: &'static str,
    pub on: &'static str,
}

#[derive(Debug, Error)]
pub enum ReconstructError {
    #[error("Query or database error: {0}")]
    RusqliteError(#[from] rusqlite::Error),
    #[error("Could not convert value from specific SQL datatype: {0}")]
    FromSqlError(#[from] FromSqlError),
    #[error("Not enough values in row iterator. Likely the schema is invalid, or the mapping machinery has a bug.")]
    NotEnoughValues,
}

pub type ReconstructResult<T> = Result<T, ReconstructError>;

macro_rules! impl_reconstruct_for_tuple {
    ($( ($( $generics:ident ),+ $(,)?) ),* $(,)?) => { $(
        // the comma is important, consider 1 generic
        impl<$( $generics ),*> Reconstruct for ( $( $generics, )* )
        where $(
            $generics: Reconstruct,
        )* {
            fn required_tables() -> Vec<&'static str> {
                [$( $generics::required_tables() ),*]
                    .into_iter()
                    .flatten()
                    .collect()
            }

            fn required_joins() -> Vec<Join> {
                [$( $generics::required_joins() ),*]
                    .into_iter()
                    .flatten()
                    .collect()
            }


            fn select_exprs() -> Vec<&'static str> {
                [$( $generics::select_exprs() ),*]
                    .into_iter()
                    .flatten()
                    .collect()
            }

            fn from_row<'a>(mut row: impl Iterator<Item = ValueRef<'a>>) -> ReconstructResult<Self> {
                Ok(( $(
                    $generics::from_row((&mut row).take($generics::select_exprs().len()))? ,
                )* ))
            }
        }
    )* };
}

impl_reconstruct_for_tuple!(
    (A,),
    (A, B),
    (A, B, C),
    (A, B, C, D),
    (A, B, C, D, E),
    (A, B, C, D, E, F),
    (A, B, C, D, E, F, G),
    (A, B, C, D, E, F, G, H),
    (A, B, C, D, E, F, G, H, I),
    (A, B, C, D, E, F, G, H, I, J),
    (A, B, C, D, E, F, G, H, I, J, K),
    (A, B, C, D, E, F, G, H, I, J, K, L),
);

/// Helper function to retrieve the next cell from a row iterator, while mapping
/// [`None`] to [`Reconstruct::NotEnoughValues`].
pub fn next_converted<'a, T: FromSql>(
    mut row: impl Iterator<Item = ValueRef<'a>>,
) -> ReconstructResult<T> {
    let value = row
        .next()
        .ok_or_else(|| ReconstructError::NotEnoughValues)?;
    let converted = FromSql::column_result(value)?;
    Ok(converted)
}
