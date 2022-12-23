use indoc::indoc;
use rusqlite::{named_params, params, Connection};
use sql_interface_macros::Reconstruct;
use time::{macros::datetime, OffsetDateTime as DateTime};

use crate::{
    statement::{OrderBy, Select},
    types::{Drive, Person},
};

fn test_database() -> rusqlite::Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    crate::init_db(&mut conn).unwrap();

    conn.execute(
        indoc! {r#"
            INSERT INTO drive(drive_id, drivedate, deadline, registration_cap)
            VALUES
                (0, "2022-12-22 00:00:00.0Z", "2022-12-21 23:00:00.0Z", 64)
        "#},
        (),
    )
    .unwrap();

    conn
}

#[test]
fn drive_roundtrip() {
    let mut conn = test_database();

    let first = Drive {
        id: 1,
        date: datetime!(2022-12-14 20:00:00 UTC),
        deadline: Some(datetime!(2022-12-12 16:00:00 UTC)),
        registration_cap: None,
    };
    let second = Drive {
        id: 2,
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
        condition: None,
        params: named_params! {},
        order: None,
    };
    let rows: Vec<Drive> = query.run().unwrap();
    // nope this is not an off-by-one error, the test DB contains one default drive
    assert_eq!(rows[1], first);
    assert_eq!(rows[2], second);

    let query = Select {
        condition: Some("drivedate == :date"),
        params: named_params! {
            ":date": first.date,
        },
        ..query
    };
    let rows: Vec<Drive> = query.run().unwrap();
    assert_eq!(rows, vec![first]);
}

#[allow(dead_code)]
#[derive(Debug, Reconstruct)]
#[sql(table = "drive")]
struct SomeWeirdlyNamedStruct {
    #[sql(column = "drive_id")]
    id: i64,
    #[sql(column = "drivedate")]
    date: DateTime,
    deadline: Option<DateTime>,
    registration_cap: Option<u32>,
}

#[test]
fn table_but_named_differently() {
    let mut conn = test_database();

    let query = Select {
        conn: &mut conn,
        condition: None,
        params: (),
        order: None,
    };
    // only check if the query fails
    let rows: Vec<SomeWeirdlyNamedStruct> = query.run().unwrap();
    assert_eq!(rows.len(), 1);
}

#[test]
fn order_by() {
    let mut conn = test_database();

    conn.execute(
        indoc! {r#"
            INSERT INTO person(prename, name, email, is_superuser, is_visible)
            VALUES
                ("Alice", "Beta", "alice_beta@non-existent-domain", false, true),
                ("Alice", "Beta", "alice_beta2@non-existent-domain", false, true),
                ("Bob", "Echo", "bob_echo@non-existent-domain", false, true),
                ("Carol", "Delta", "carol_delta@non-existent-domain", false, true)
        "#},
        (),
    )
    .unwrap();

    let query = Select {
        conn: &mut conn,
        condition: None,
        params: (),
        order: Some(OrderBy::Descending("person.name")),
    };
    let names: Vec<_> = query
        .run()
        .unwrap()
        .into_iter()
        .map(|person: Person| person.name)
        .collect();

    assert_eq!(names, vec!["Echo", "Delta", "Beta", "Beta"]);
}
