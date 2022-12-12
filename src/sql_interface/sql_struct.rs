use rusqlite::types::{FromSqlResult, ValueRef};
use thiserror::Error;

pub trait SqlStruct
where
    Self: Sized,
{
    /// The tables this struct is stored in, or depends on.
    fn required_tables() -> Vec<&'static str>;

    /// Returns all SQL expressions this struct needs in order to be built in
    /// [`SqlStruct::from_row`].
    fn select_exprs() -> Vec<&'static str>;

    /// Reconstructs the implementor of this trait from a row, following the schema of
    /// [`SqlStruct::select_exprs`] in the same order.
    fn from_row<'a>(row: impl Iterator<Item = ValueRef<'a>>) -> FromSqlResult<Self>;
}

#[derive(Debug, Error)]
#[error("Not enough values in row iterator. Likely the schema is invalid, or the mapping machinery has a bug.")]
pub struct NotEnoughValues;

macro_rules! impl_sqlstruct_for_tuple {
    ($( ($( $generics:ident ),+ $(,)?) ),* $(,)?) => { $(
        // the comma is important, consider 1 generic
        impl<$( $generics ),*> SqlStruct for ( $( $generics, )* )
        where $(
            $generics: SqlStruct,
        )* {
            fn required_tables() -> Vec<&'static str> {
                [$( $generics::required_tables() ),*]
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

            fn from_row<'a>(mut row: impl Iterator<Item = ValueRef<'a>>) -> FromSqlResult<Self> {
                Ok(( $(
                    $generics::from_row((&mut row).take($generics::select_exprs().len()))? ,
                )* ))
            }
        }
    )* };
}

impl_sqlstruct_for_tuple!(
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
