#![allow(clippy::cast_possible_truncation)] // Irrelevant in this context

use rusqlite::Result;

pub mod sql_struct;
pub mod statement;
pub mod types;

#[cfg(test)]
mod tests;

pub fn init_db(conn: &mut rusqlite::Connection) -> Result<()> {
    conn.execute_batch(include_str!("./init_db.sql"))
}
