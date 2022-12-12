#![allow(
    clippy::no_effect_underscore_binding, // Rocket heavily uses those in macros
    clippy::needless_pass_by_value,  // The request guards should take them by value anyways
)]

#[macro_use]
extern crate rocket;

pub mod config;
pub mod date_helpers;
pub mod routes;

use std::fmt;

use rocket::{
    response::{Flash, Redirect},
    Route,
};
use rocket_sync_db_pools::database;

pub use routes::routes;

/// A shorthand function for logging an internal server error and redirecting to the page for that.
#[inline]
pub fn server_error(admin_err: impl fmt::Display, user_err: impl AsRef<str>) -> Flash<Redirect> {
    log::error!("{}", admin_err);
    Flash::error(
        Redirect::to(uri!(routes::error::server_error_panel)),
        user_err.as_ref(),
    )
}

pub fn flatten_routes(
    routes: impl IntoIterator<Item = impl IntoIterator<Item = Route>>,
) -> Vec<Route> {
    routes.into_iter().flatten().collect()
}

#[database("bususages")]
pub struct BususagesDBConn(rusqlite::Connection);