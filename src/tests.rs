use {
    super::sql_interface::{
        self, NewPerson, RegistrationUpdate,
        SearchPersonBy::{Email, Id},
        SearchRegistrationsBy::{Date, PersonId},
        UpdatePerson,
    },
    chrono::NaiveDate,
    rusqlite::Connection,
};

/// Creates a fresh empty database with tables defined.
fn init_db() -> Connection {
    let conn = Connection::open_in_memory().unwrap();
    conn.execute_batch(
        "CREATE TABLE person(
            person_id INTEGER,
            prename TEXT NOT NULL,
            name TEXT NOT NULL,
            email TEXT NOT NULL,
            token TEXT,
            token_expiration INTEGER,
            is_superuser BOOLEAN NOT NULL,
            UNIQUE(email),
            PRIMARY KEY (person_id AUTOINCREMENT)
        );
        CREATE TABLE drive(
            drive_id INTEGER,
            drivedate DATE NOT NULL,
            UNIQUE(drivedate),
            PRIMARY KEY (drive_id AUTOINCREMENT)
        );
        CREATE TABLE registration(
                id INTEGER,
                person_id INTEGER NOT NULL,
                drive_id INTEGER NOT NULL,
                registered BOOLEAN NOT NULL,
                UNIQUE(person_id, drive_id),
                FOREIGN KEY (person_id) REFERENCES person(person_id)
                        ON DELETE CASCADE
                        ON UPDATE CASCADE,
                FOREIGN KEY (drive_id) REFERENCES drive(drive_id)
                        ON DELETE CASCADE
                        ON UPDATE CASCADE,
                PRIMARY KEY (id AUTOINCREMENT) 
        );",
    )
    .unwrap();
    conn
}

#[test]
fn persons() {
    let mut conn = init_db();

    for person in [
        ("Alice", "Beta", "alice_beta@non-existent-domain"),
        ("Alice", "Beta", "alice_beta2@non-existent-domain"),
        ("Bob", "Echo", "bob_echo@non-existent-domain"),
        ("Carol", "Delta", "carol_delta@non-existent-domain"),
    ] {
        sql_interface::insert_new_person(
            &mut conn,
            &NewPerson {
                prename: person.0.to_string(),
                name: person.1.to_string(),
                email: person.2.parse().unwrap(),
            },
        )
        .unwrap();
    }

    let bob = sql_interface::search_person(
        &mut conn,
        &Email("bob_echo@non-existent-domain".to_string()),
    )
    .unwrap();
    let bob_by_id = sql_interface::search_person(&mut conn, &Id(bob.id)).unwrap();
    assert_eq!(bob, bob_by_id);
    assert_eq!(bob.prename, "Bob");
    assert_eq!(bob.name, "Echo");
    assert_eq!(bob.is_superuser, false);
    assert_eq!(bob.token, None);

    // whoops, confused them with another one
    sql_interface::update_person(
        &mut conn,
        &UpdatePerson {
            id: bob.id,
            prename: "Jackie".to_string(),
            name: "Hotel".to_string(),
            email: "jackie_hotel@non-existent-domain".parse().unwrap(),
        },
    )
    .unwrap();
    let jackie = sql_interface::search_person(&mut conn, &Id(bob.id)).unwrap();
    assert_eq!(jackie.prename, "Jackie");
    assert_eq!(jackie.name, "Hotel");
    assert_eq!(jackie.is_superuser, false);
    assert_eq!(jackie.token, None);

    let all_persons = sql_interface::list_all_persons(&mut conn).unwrap();
    for person in all_persons {
        sql_interface::delete_person(&mut conn, person.id).unwrap();
    }
    let all_persons = sql_interface::list_all_persons(&mut conn).unwrap();
    assert!(all_persons.is_empty());
}

#[test]
fn register() {
    let mut conn = init_db();

    for person in [
        ("Alice", "Beta", "alice_beta@non-existent-domain"),
        ("Bob", "Echo", "bob_echo@non-existent-domain"),
    ] {
        sql_interface::insert_new_person(
            &mut conn,
            &NewPerson {
                prename: person.0.to_string(),
                name: person.1.to_string(),
                email: person.2.parse().unwrap(),
            },
        )
        .unwrap();
    }
    let bob = sql_interface::search_person(
        &mut conn,
        &Email("bob_echo@non-existent-domain".to_string()),
    )
    .unwrap();

    // do you know that date? (no it's not a historical or political reference, just a very special
    // release date)
    let date = NaiveDate::from_ymd(2009, 1, 16);
    sql_interface::insert_new_drive(&mut conn, date).unwrap();

    let regupdate = RegistrationUpdate {
        date,
        person_id: bob.id,
        registered: true,
    };
    sql_interface::update_registration(&mut conn, &regupdate).unwrap();

    let regs = sql_interface::search_registrations(&mut conn, &Date(date)).unwrap();
    assert_eq!(regs.len(), 2);
    let reg = &regs[1]; // relying explicitly on sorting
    assert_eq!(reg.date, date);
    assert_eq!(reg.person.prename, "Bob");
    assert!(reg.registered);

    let regs = sql_interface::search_registrations(
        &mut conn,
        &PersonId {
            id: bob.id,
            ignore_past: false,
        },
    )
    .unwrap();
    assert_eq!(regs.len(), 1);
    let reg = &regs[0];
    assert_eq!(reg.date, date);
    assert_eq!(reg.person.prename, "Bob");
    assert!(reg.registered);
}
