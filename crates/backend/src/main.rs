#![allow(clippy::no_effect_underscore_binding)] // rocket heavily uses those in macros

use rocket::{
    fairing::AdHoc,
    fs::{relative, FileServer},
    launch,
};
use rocket_dyn_templates::{handlebars::handlebars_helper, Template};

use backend::{config::Config, routes, BususagesDBConn};

#[launch]
async fn rocket() -> _ {
    rocket::build()
        .attach(Template::custom(|engines| {
            engines
                .handlebars
                .register_escape_fn(ammonia::clean_text);

            handlebars_helper!(equals: |left_hand: String, right_hand: String| left_hand == right_hand);

            engines.handlebars.register_helper("equals", Box::new(equals));
        }))
        .attach(AdHoc::config::<Config>())
        .attach(BususagesDBConn::fairing())
        .mount("/", routes())
        .mount("/static", FileServer::from(relative!("/static")))
}
