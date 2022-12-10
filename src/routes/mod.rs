use rocket::Route;

pub mod authflow;
pub mod dashboard;
pub mod error;
pub mod superuser;

#[must_use]
pub fn routes() -> Vec<Route> {
    [
        authflow::routes(),
        dashboard::routes(),
        error::routes(),
        superuser::routes(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
