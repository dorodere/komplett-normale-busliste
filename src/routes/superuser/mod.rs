#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

mod drive;
mod person;
mod registration;
mod settings;

use rocket::{request::FlashMessage, Route};
use rocket_dyn_templates::{context, Template};

use crate::routes::authflow::Superuser;

#[must_use]
pub fn routes() -> Vec<Route> {
    crate::flatten_routes([
        drive::routes(),
        person::routes(),
        registration::routes(),
        settings::routes(),
        routes![panel],
    ])
}

#[get("/superuser")]
pub fn panel(flash: Option<FlashMessage<'_>>, _superuser: Superuser) -> Template {
    Template::render(
        "superuser-panel",
        context! {
            flash: flash.map(|flash| flash.message().to_string()),
        },
    )
}
