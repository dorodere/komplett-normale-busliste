use crate::sql_interface;
use rocket_dyn_templates::{context, Template};
/*
fn getSeats() -> INTEGER {
    conn.execute(
        "SELECT  registration.registered, :date
        FROM person
        LEFT OUTER JOIN drive ON (drive.drivedate == :date)
        LEFT OUTER JOIN registration ON (
            registration.person_id == person.person_id AND
            registration.drive_id == drive.drive_id
        )
        ORDER BY person.name"
    )
}
*/
#[get("/mensa")]
pub fn mensa() -> Template {
    Template::render(
        "mensa",
        context!(
            test0: "test0",
            test1: "test1"
        ),
    )
}
