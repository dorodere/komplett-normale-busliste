use std::collections::HashSet;

use indoc::formatdoc;
use itertools::Itertools;
use rusqlite::{Connection, Params};

use crate::sql_struct::{Reconstruct, ReconstructResult};

pub struct Select<'a, P: Params + Clone> {
    pub conn: &'a mut Connection,

    /// What should go into the `WHERE` clause.
    pub condition: Option<&'static str>,

    /// Which parameters to insert in the query.
    pub params: P,
}

impl<P: Params + Clone> Select<'_, P> {
    pub fn run<T: Reconstruct>(&self) -> ReconstructResult<Vec<T>> {
        // figure out which expressions are needed in order to reconstruct T
        let select_exprs = T::select_exprs().join(", ");

        // deduplicate all tables
        let mut tables: HashSet<_> = T::required_tables().into_iter().collect();

        // generate the filtering + join clauses
        let where_clause: &str = self.condition.unwrap_or("true");
        let join_clauses = self
            .joins
            .iter()
            .inspect(|join| {
                // table doesn't need to be in FROM if it's joined already
                tables.remove(join.table);
            })
            .map(|join| format!("LEFT OUTER JOIN {} ON ({})", join.table, join.on))
            .join("\n");

        let tables = tables.into_iter().join(", ");

        // actually build the query
        let statement = formatdoc! {"
            SELECT {select_exprs}
            FROM {tables}
            WHERE {where_clause}
            {join_clauses}
        "};
        let mut statement = self.conn.prepare(&statement)?;

        // run it and convert each row to the target tuple
        let iter = statement.query_map(self.params.clone(), |row| {
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
