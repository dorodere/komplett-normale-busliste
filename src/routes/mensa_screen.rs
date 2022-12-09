use crate::sql_interface;
use rocket_dyn_templates::{context, Template};

#[get("/mensa")]
pub fn mensa() -> Template {
    let mut next_drive;
    let mut next_next_drive;
    Template::render(
        "mensa",
        context!(
            test0: "test0",
            test1: "test1"
        ),
    )
}
