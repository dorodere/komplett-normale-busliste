use std::{env, path::PathBuf};

use rusqlite::Connection;
use sql_interface::{statement::Select, types::Drive};

fn crate_root() -> PathBuf {
    env::var("CARGO_MANIFEST_DIR")
        .map(|path| path.into())
        .or_else(|_| env::current_dir())
        .unwrap()
}

fn database_path() -> PathBuf {
    [
        crate_root(),
        "..".into(),
        "backend".into(),
        "testing-database.db".into(),
    ]
    .iter()
    .collect::<PathBuf>()
    .canonicalize()
    .unwrap()
}

fn main() {
    let mut conn = Connection::open(database_path()).unwrap();

    let query = Select {
        conn: &mut conn,
        joins: Vec::new(),
        condition: None,
        params: (),
    };
    let items: Vec<Drive> = query.run().unwrap();

    dbg!(items);
}
