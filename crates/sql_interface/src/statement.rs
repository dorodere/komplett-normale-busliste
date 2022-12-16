use std::{collections::HashSet, marker::PhantomData};

use indoc::formatdoc;
use itertools::Itertools;
use rusqlite::{types::ValueRef, Connection, Params, Result as SqlResult};

use crate::sql_struct::{ReconstructResult, SqlStruct};

pub struct Select<'a, T: SqlStruct> {
    pub conn: &'a mut Connection,
    pub condition: String,
    pub output_type: PhantomData<T>,
}

impl<T: SqlStruct> Select<'_, T> {
    pub fn run(self) -> ReconstructResult<Vec<T>> {
        // figure out which expressions are needed in order to reconstruct T
        let select_exprs = T::select_exprs().join(", ");

        // deduplicate all tables
        let tables: HashSet<_> = T::required_tables().into_iter().collect();
        let tables = tables.into_iter().join(", ");

        // actually build the query
        let statement = formatdoc! {"
            SELECT {select_exprs}
            FROM {tables}
        "};
        let mut statement = self.conn.prepare(&statement)?;

        // run it and convert each row to the target tuple
        let iter = statement.query_map([], |row| {
            // build a proper iterator from the row
            let expr_count = T::select_exprs().len();
            let row = (0..expr_count).map(|idx| row.get_ref(idx));

            // collect into a result first, to catch any errors
            let row: Result<Vec<_>, _> = row.collect();
            let row = row?.into_iter();

            Ok(T::from_row(row))
        })?;

        // collapse all errors (they could be from rusqlite or reconstruction)
        let result: Result<Result<_, _>, _> = iter.collect();
        result?
    }
}
