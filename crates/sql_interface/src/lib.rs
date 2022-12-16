#![allow(clippy::cast_possible_truncation)] // Irrelevant in this context

use rusqlite::Result;

mod sql_struct;
mod statement;
mod types;

#[cfg(test)]
mod tests;

pub fn init_db(conn: &mut rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("./init_db.sql"))
}
