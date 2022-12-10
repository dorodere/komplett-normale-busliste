use std::fmt;

use chrono::Utc;
use rocket::{
    form::{Form, Strict},
    request::FlashMessage,
    response::{Flash, Redirect},
    Route,
};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

use crate::{
    date_helpers::{format_date, time_to_chrono_date},
    routes::authflow::{Superuser, User},
    server_error,
    sql_interface::{self, ApplyRegistrationError, DriveFilter, SearchRegistrationsBy},
    BususagesDBConn,
};

#[must_use]
pub fn routes() -> Vec<Route> {
    routes![dashboard, register]
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
                    format!("Error while loading registrations: {err}"),
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

/// A registration form to be returned by the frontend.
#[derive(FromForm, Debug, Clone)]
pub struct Registration {
    date: time::Date,
    new_state: bool,
}

impl Registration {
    #[must_use]
    pub fn to_registration_update(&self, user: &User) -> sql_interface::RegistrationUpdate {
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
                format!("Error while updating registration: {err}"),
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

    if drive.deadline.map_or(false, |deadline| deadline < now) {
        Err(ImpossibleReason::DeadlineExpired)
    } else if wants_to_register
        && drive
            .registration_cap
            .map_or(false, |cap| cap <= drive.already_registered_count)
    {
        Err(ImpossibleReason::RegistrationCapReached)
    } else {
        Ok(())
    }
}
