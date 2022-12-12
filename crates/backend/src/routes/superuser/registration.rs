use rocket::{
    response::{Flash, Redirect},
    routes, Route,
};
use rocket_dyn_templates::Template;

use crate::{
    date_helpers::time_to_chrono_date, routes::authflow::Superuser, server_error, sql_interface,
    BususagesDBConn,
};

pub fn routes() -> Vec<Route> {
    routes![registrations_panel]
}

#[get("/registrations?<from>&<to>")]
pub async fn registrations_panel(
    conn: BususagesDBConn,
    _superuser: Superuser,
    from: Option<time::Date>,
    to: Option<time::Date>,
) -> Result<Template, Flash<Redirect>> {
    let persons_with_counts = conn
        .run(move |c| {
            sql_interface::list_persons_counted_registrations(
                c,
                from.map(time_to_chrono_date),
                to.map(time_to_chrono_date),
            )
        })
        .await
        .map_err(|err| {
            server_error(
                format!("Error while counting registrations: {err}"),
                "an error occurred while loading persons",
            )
        })?;

    Ok(Template::render("registrations-panel", persons_with_counts))
}
