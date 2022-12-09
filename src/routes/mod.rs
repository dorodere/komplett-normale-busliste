use rocket::{routes, Route};

pub mod authflow;
pub mod dashboard;
pub mod error;
pub mod superuser;

#[must_use]
pub fn routes() -> Vec<Route> {
    [
        routes![
            superuser::panel,
            superuser::drives_panel,
            superuser::create_new_drive,
            superuser::delete_drive,
            superuser::update_deadline,
            superuser::introspect_drive,
            superuser::registrations_panel,
            superuser::person_panel,
            superuser::create_new_person,
            superuser::update_person,
            superuser::delete_person,
            superuser::introspect_person,
            superuser::register_person,
            superuser::settings,
            superuser::set_setting,
            authflow::index,
            authflow::login,
            authflow::verify_token,
        ],
        dashboard::routes(),
        error::routes(),
    ]
    .into_iter()
    .flatten()
    .collect()
}
