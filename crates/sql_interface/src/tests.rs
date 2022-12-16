use std::marker::PhantomData;

use indoc::indoc;
use rusqlite::{
    params,
    types::{FromSql, FromSqlError, FromSqlResult, ToSql, ToSqlOutput, ValueRef},
    Connection,
};
use time::macros::datetime;

use crate::{statement::Select, types::Drive};

fn empty_test_database() -> rusqlite::Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    crate::init_db(&mut conn).unwrap();
    conn
}

#[test]
fn drive_roundtrip() {
    let mut conn = empty_test_database();

    let first = Drive {
        id: 0,
        date: datetime!(2022-12-14 20:00:00 UTC),
        deadline: Some(datetime!(2022-12-12 16:00:00 UTC)),
        registration_cap: None,
    };
    let second = Drive {
        id: 1,
        date: datetime!(2022-12-17 19:30:00 UTC),
        deadline: None,
        ..first
    };

    // insert these example drives into the db (eventually this be cleaner, hopefully)
    conn.execute(
        indoc! {"
            INSERT INTO drive(drive_id, drivedate, deadline, registration_cap)
            VALUES
                (?, ?, ?, ?),
                (?, ?, ?, ?)
        "},
        params![
            first.id,
            first.date,
            first.deadline,
            first.registration_cap,
            second.id,
            second.date,
            second.deadline,
            second.registration_cap
        ],
    )
    .unwrap();

    let query = Select {
        conn: &mut conn,
        output_type: PhantomData::<Drive>,
        condition: String::new(),
    };
    let rows = query.run().unwrap();

    assert_eq!(rows, vec![first, second]);
}
