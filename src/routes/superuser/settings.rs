use rocket::{
    form::{Form, Strict},
    request::FlashMessage,
    response::{Flash, Redirect},
    routes, Route,
};
use rocket_dyn_templates::Template;
use rusqlite::types::Value;

use crate::{routes::authflow::Superuser, server_error, sql_interface, BususagesDBConn};

#[must_use]
pub fn routes() -> Vec<Route> {
    routes![panel, set]
}

#[get("/settings")]
pub async fn panel(
    conn: BususagesDBConn,
    flash: Option<FlashMessage<'_>>,
    _superuser: Superuser,
) -> Result<Template, Flash<Redirect>> {
    let mut settings = conn.run(sql_interface::all_settings).await.map_err(|err| {
        server_error(
            format!("Error while fetching current setting values: {err}"),
            "ein Fehler trat während des Abfragen der Werte der aktuellen Einstellungen auf",
        )
    })?;
    settings.insert(
        "flash".to_string(),
        flash.map_or_else(String::new, |flash| flash.message().to_string()),
    );
    Ok(Template::render("settings", settings))
}

#[derive(FromForm, Debug, Clone)]
pub struct Setting {
    name: String,
    value: String,
}

#[post("/settings/set", data = "<update>")]
pub async fn set(
    conn: BususagesDBConn,
    update: Form<Strict<Setting>>,
    _superuser: Superuser,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    // probably want to perform some additional validation here for new settings, but for now this is fine
    let value = match update.name.as_ref() {
        "login-message" => Value::Text(update.value.clone()),
        "default-deadline" => match (update.value.len(), update.value.chars().next()) {
            (0, None) => Value::Null,
            (1, Some('0'..='6')) => Value::Integer(update.value.parse().unwrap()),
            _ => {
                return Err(server_error(
                    format!("User wanted to set default deadline to '{}', which is invalid (validation/UI out of sync?)", update.value),
                    "ein Fehler trat während der Anwendung der Default-Deadline auf",
                ))
            }
        },
        "default-registration-cap" => {
            let cap = update.value.parse::<u32>().map_err(|_| {
                Flash::error(Redirect::to(uri!(panel)), "Die Zahl ist nicht valide, oder zu groß.")
            })?;
            Value::Integer(i64::from(cap))
        },
        _ => {
            return Err(server_error(
                format!(
                    "User wanted to set setting '{}' to '{}', which isn't validated for (but may exist in the database, in that case validation + database are out of sync)",
                    update.name, update.value
                ),
                "ein Fehler trat während des Setzens der Einstellung auf",
            ));
        }
    };
    let name = update.name.clone();

    conn.run(move |c| sql_interface::set_setting(c, name, value))
        .await
        .map_err(|err| {
            server_error(
                format!(
                    "Error while setting '{}' to '{}': {err}",
                    update.name, update.value
                ),
                "ein Fehler trat während des Updates der Einstellung auf",
            )
        })?;

    Ok(Flash::success(
        Redirect::to(uri!(panel)),
        "Einstellung angewandt.",
    ))
}
