#[macro_use]
extern crate rocket;

mod authflow;
mod config;
mod date_helpers;
mod sql_interface;
mod superuser;
#[cfg(test)]
mod tests;

use {
    authflow::{Superuser, User},
    chrono::Utc,
    date_helpers::*,
    rocket::{
        fairing::AdHoc,
        form::{Form, Strict},
        fs::FileServer,
        http::{Cookie, CookieJar},
        request::FlashMessage,
        response::{Flash, Redirect},
    },
    rocket_dyn_templates::{context, handlebars::handlebars_helper, Template},
    rocket_sync_db_pools::{database, rusqlite},
    serde::Serialize,
    sql_interface::{ApplyRegistrationError, DriveFilter, SearchRegistrationsBy},
    std::fmt,
};

/// A shorthand function for logging an internal server error and redirecting to the page for that.
#[inline]
pub fn server_error(admin_err: impl fmt::Display, user_err: impl AsRef<str>) -> Flash<Redirect> {
    log::error!("{}", admin_err);
    Flash::error(Redirect::to(uri!(server_error_panel)), user_err.as_ref())
}

#[database("bususages")]
pub struct BususagesDBConn(rusqlite::Connection);

#[get("/servererror")]
fn server_error_panel(flash: FlashMessage<'_>) -> Template {
    #[derive(Debug, Serialize)]
    struct Context {
        error: String,
    }

    Template::render(
        "server-error",
        &Context {
            error: flash.message().to_string(),
        },
    )
}

#[get("/")]
async fn dashboard(
    conn: BususagesDBConn,
    user: User,
    superuser: Option<Superuser>,
    flash: Option<FlashMessage<'_>>,
) -> Result<Template, Flash<Redirect>> {
    #[derive(Debug, Serialize)]
    struct TemplateRegistration {
        pretty_date: String,
        locked_reason: Option<String>,
        registration: sql_interface::Registration,
    }

    let mut registrations = [Vec::new(), Vec::new()];
    let person_id = user.person_id();

    for (i, filter) in [DriveFilter::OnlyFuture, DriveFilter::OnlyPast]
        .into_iter()
        .enumerate()
    {
        let from_db = conn
            .run(move |c| {
                sql_interface::search_registrations(
                    c,
                    &SearchRegistrationsBy::PersonId {
                        id: person_id,
                        filter,
                    },
                )
            })
            .await
            .map_err(|err| {
                server_error(
                    format!("Error while loading registrations: {}", err),
                    "an error occured while loading registrations",
                )
            })?;

        let as_template = from_db
            .into_iter()
            .map(|registration| TemplateRegistration {
                pretty_date: format_date(registration.drive.date),
                locked_reason: possible_to_register(&registration.drive, !registration.registered)
                    .err()
                    .map(|reason| reason.to_string()),
                registration,
            })
            .collect();

        registrations[i] = as_template;
    }

    let flash = flash.map(|flashmsg| flashmsg.message().to_string());
    let [future_regs, past_regs] = registrations;

    Ok(Template::render(
        "dashboard",
        context! {
            flash,
            future_regs,
            past_regs,
            show_superuser_controls: superuser.is_some(),
        },
    ))
}

#[post("/logout")]
fn logout(_user: User, jar: &CookieJar<'_>) -> Redirect {
    jar.remove(Cookie::named("auth-token"));
    Redirect::to(uri!(authflow::index))
}

/// A registration form to be returned by the frontend.
#[derive(FromForm, Debug, Clone)]
pub struct Registration {
    date: time::Date,
    new_state: bool,
}

impl Registration {
    #[must_use]
    pub fn to_registration_update(
        &self,
        user: &authflow::User,
    ) -> sql_interface::RegistrationUpdate {
        sql_interface::RegistrationUpdate {
            date: time_to_chrono_date(self.date),
            person_id: user.person_id(),
            registered: self.new_state,
        }
    }
}

#[post("/register", data = "<registration>")]
async fn register(
    conn: BususagesDBConn,
    user: User,
    registration: Form<Strict<Registration>>,
) -> Result<Redirect, Flash<Redirect>> {
    let query_date = time_to_chrono_date(registration.date);
    let drive = conn
        .run(move |c| sql_interface::get_drive(c, query_date))
        .await
        .map_err(|err| {
            server_error(
                format!(
                    "Error while querying drive deadline for '{}': {}",
                    registration.date, err
                ),
                "ein Fehler trat während des Abfragens der Anmeldungsdeadline auf",
            )
        })?
        .ok_or_else(|| {
            Flash::error(
                Redirect::to(uri!(dashboard)),
                "Das Datum der Fahrt ist nicht valide, versuch es nochmal.",
            )
        })?;

    let person_id = user.person_id();
    let currently_registered = conn
        .run(move |c| sql_interface::is_registered(c, person_id, drive.date))
        .await
        .map_err(|err| {
            server_error(
                format!(
                    "Error while querying registration for {} on {}: {}",
                    person_id, registration.date, err
                ),
                "ein Fehler trat während des Abprüfens der aktuellen Registrierung auf",
            )
        })?;

    if currently_registered == registration.new_state {
        // registration would be a no-op
        return Ok(Redirect::to(uri!(dashboard)));
    }

    // before going home with it, let's check if it's even possible to register
    if registration.new_state {
        // TODO: check in the db if the registration is really what the user suggests
        possible_to_register(&drive, currently_registered)
            .map_err(|reason| Flash::error(Redirect::to(uri!(dashboard)), reason.to_string()))?;
    }

    let update = registration.to_registration_update(&user);
    match conn
        .run(move |c| sql_interface::update_registration(c, &update))
        .await
    {
        Err(ApplyRegistrationError::UnknownDriveDate) => {
            return Err(Flash::error(
                Redirect::to(uri!(dashboard)),
                "Ungültiges Fahrdatum, es ist keine Busfahrt an diesem Datum bekannt.",
            ))
        }
        Err(err) => {
            return Err(server_error(
                format!("Error while updating registration: {}", err),
                "ein Fehler trat während der Aktualisierung der Anmeldung auf",
            ))
        }
        _ => (),
    };

    Ok(Redirect::to(uri!(dashboard)))
}

enum ImpossibleReason {
    RegistrationCapReached,
    DeadlineExpired,
}

impl fmt::Display for ImpossibleReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DeadlineExpired => {
                write!(f, "Deadline ist abgelaufen")
            }
            Self::RegistrationCapReached => write!(f, "Maximale Registrierungen erreicht"),
        }
    }
}

fn possible_to_register(
    drive: &sql_interface::Drive,
    wants_to_register: bool,
) -> Result<(), ImpossibleReason> {
    let now = Utc::now().naive_utc();

    if drive
        .deadline
        .map(|deadline| deadline < now)
        .unwrap_or(false)
    {
        Err(ImpossibleReason::DeadlineExpired)
    } else if wants_to_register
        && drive
            .registration_cap
            .map(|cap| cap <= drive.already_registered_count)
            .unwrap_or(false)
    {
        Err(ImpossibleReason::RegistrationCapReached)
    } else {
        Ok(())
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Template::custom(|engines| {
            engines
                .handlebars
                .register_escape_fn(|input| ammonia::clean_text(input));

            handlebars_helper!(equals: |left_hand: String, right_hand: String| left_hand == right_hand);

            engines.handlebars.register_helper("equals", Box::new(equals));
        }))
        .attach(AdHoc::config::<config::Config>())
        .attach(BususagesDBConn::fairing())
        .mount(
            "/",
            routes![
                dashboard,
                logout,
                register,
                server_error_panel,
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
                authflow::verify_token
            ],
        )
        .mount("/static", FileServer::from("./static"))
}
