use chrono::{Days, NaiveDate};
use rusqlite::{types::Value, Connection};

use crate::sql_interface::{
    self, DriveFilter, NewPerson, RegistrationUpdate,
    SearchPersonBy::{Email, Id},
    SearchRegistrationsBy::{Date, PersonId},
    UpdatePerson, VisibilityFilter,
};

/// Creates a fresh empty database with tables defined.
fn init_db() -> Connection {
    let mut conn = Connection::open_in_memory().unwrap();
    sql_interface::init_db_if_necessary(&mut conn).unwrap();
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
            is_visible: true,
        },
    )
    .unwrap();
    let jackie = sql_interface::search_person(&mut conn, &Id(bob.id)).unwrap();
    assert_eq!(jackie.prename, "Jackie");
    assert_eq!(jackie.name, "Hotel");
    assert_eq!(jackie.is_superuser, false);
    assert_eq!(jackie.token, None);

    let all_persons =
        sql_interface::list_all_persons(&mut conn, VisibilityFilter::OnlyVisible).unwrap();
    for person in all_persons {
        sql_interface::delete_person(&mut conn, person.id).unwrap();
    }
    let all_persons =
        sql_interface::list_all_persons(&mut conn, VisibilityFilter::IncludingInvisible).unwrap();
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
    let date = NaiveDate::from_ymd_opt(2009, 1, 16).unwrap();
    let deadline = (date - Days::new(2)).and_hms_opt(19, 2, 00).unwrap();
    sql_interface::insert_new_drive(&mut conn, date, Some(deadline)).unwrap();

    let regupdate = RegistrationUpdate {
        date,
        person_id: bob.id,
        registered: true,
    };
    sql_interface::update_registration(&mut conn, &regupdate).unwrap();

    let regs = sql_interface::search_registrations(&mut conn, &Date(date)).unwrap();
    assert_eq!(regs.len(), 2);
    let reg = &regs[1]; // relying explicitly on sorting
    assert_eq!(reg.drive.date, date);
    assert_eq!(reg.person.prename, "Bob");
    assert!(reg.registered);

    let regs = sql_interface::search_registrations(
        &mut conn,
        &PersonId {
            id: bob.id,
            filter: DriveFilter::ListAll,
        },
    )
    .unwrap();
    assert_eq!(regs.len(), 1);
    let reg = &regs[0];
    assert_eq!(reg.drive.date, date);
    assert_eq!(reg.person.prename, "Bob");
    assert!(reg.registered);
}

#[test]
fn settings() {
    let mut conn = init_db();

    let _ = sql_interface::get_setting(&mut conn, "login-message").unwrap();

    let very_special_message =
        "this is totally not text that'd ever appear on the login page would it";
    sql_interface::set_setting(&mut conn, "login-message", very_special_message).unwrap();

    // some hypothetical business logic

    let retrieved = sql_interface::get_setting(&mut conn, "login-message").unwrap();

    if let Value::Text(content) = retrieved {
        assert_eq!(content, very_special_message);
    } else {
        panic!(
            "login message stored in database wasn't a text, but rather {:?}",
            retrieved
        );
    }

    let all_settings = sql_interface::all_settings(&mut conn).unwrap();
    assert_eq!(all_settings["login-message"], very_special_message);
}
