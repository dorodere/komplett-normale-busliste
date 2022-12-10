#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

mod drive;

use rocket::{
    form::{Form, Lenient, Strict},
    request::FlashMessage,
    response::{Flash, Redirect},
    Route,
};
use rocket_dyn_templates::{context, Template};
use rusqlite::types::Value;
use serde::Serialize;

use crate::{
    date_helpers::{format_date, time_to_chrono_date},
    routes::authflow::Superuser,
    server_error,
    sql_interface::{
        self, DriveFilter, Person, SearchPersonBy, SearchRegistrationsBy, VisibilityFilter,
    },
    BususagesDBConn,
};

#[must_use]
pub fn routes() -> Vec<Route> {
    crate::flatten_routes([
        drive::routes(),
        routes![
            panel,
            registrations_panel,
            person_panel,
            create_new_person,
            update_person,
            delete_person,
            introspect_person,
            register_person,
            settings,
            set_setting,
        ],
    ])
}

#[get("/superuser")]
pub fn panel(flash: Option<FlashMessage<'_>>, _superuser: Superuser) -> Template {
    #[derive(Debug, Serialize)]
    struct Context {
        flash: Option<String>,
    }

    Template::render(
        "superuser-panel",
        &Context {
            flash: flash.map(|flash| flash.message().to_string()),
        },
    )
}

/// Just a shorthand for an error flash containing a redirect.
#[inline]
fn flash_error(message: &str) -> Flash<Redirect> {
    Flash::error(Redirect::to(uri!(person_panel)), message)
}

#[get("/person")]
pub async fn person_panel(
    conn: BususagesDBConn,
    _superuser: Superuser,
    flash: Option<FlashMessage<'_>>,
) -> Result<Template, Flash<Redirect>> {
    #[derive(Debug, Serialize)]
    struct Context {
        flash: Option<String>,
        persons: Vec<Person>,
    }

    let persons = conn
        .run(|c| sql_interface::list_all_persons(c, VisibilityFilter::IncludingInvisible))
        .await
        .map_err(|err| {
            server_error(
                format!("Error while listing persons: {err}"),
                "an error occurred while loading persons",
            )
        })?;

    Ok(Template::render(
        "personcontrol",
        &Context {
            flash: flash.map(|flash| flash.message().to_string()),
            persons,
        },
    ))
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

#[derive(Debug, FromForm)]
pub struct NewPerson {
    prename: String,
    name: String,
    email: String,
}

impl TryFrom<NewPerson> for sql_interface::NewPerson {
    type Error = lettre::address::AddressError;

    fn try_from(source: NewPerson) -> Result<sql_interface::NewPerson, Self::Error> {
        Ok(sql_interface::NewPerson {
            prename: source.prename,
            name: source.name,
            email: source.email.parse()?,
        })
    }
}

#[post("/person/new", data = "<form>")]
pub async fn create_new_person(
    conn: BususagesDBConn,
    form: Form<Strict<NewPerson>>,
    _superuser: Superuser,
) -> Result<Redirect, Flash<Redirect>> {
    let new_person: sql_interface::NewPerson = form
        .into_inner()
        .into_inner()
        .try_into()
        .map_err(|_| flash_error("Invalid email!"))?;
    let debug = new_person.clone();
    match conn
        .run(move |c| sql_interface::insert_new_person(c, &new_person))
        .await
    {
        Err(sql_interface::PersonCreationError::EmailAlreadyInUse) => Err(flash_error(
            "This email is already in use by another person. Perhaps it already exists?",
        )),
        Err(err) => Err(server_error(
            format!("Error while inserting new person: {err}\n{debug:#?}"),
            "an error occured while inserting the new person",
        )),
        _ => Ok(Redirect::to(uri!(person_panel))),
    }
}

#[derive(Debug, FromForm)]
pub struct UpdatePerson {
    id: i64,
    prename: String,
    name: String,
    email: String,
    is_visible: Lenient<bool>,
}

impl TryFrom<UpdatePerson> for sql_interface::UpdatePerson {
    type Error = lettre::address::AddressError;

    fn try_from(source: UpdatePerson) -> Result<sql_interface::UpdatePerson, Self::Error> {
        Ok(sql_interface::UpdatePerson {
            id: source.id,
            prename: source.prename,
            name: source.name,
            email: source.email.parse()?,
            is_visible: source.is_visible.into_inner(),
        })
    }
}

#[post("/person/update", data = "<form>")]
pub async fn update_person(
    conn: BususagesDBConn,
    form: Form<Strict<UpdatePerson>>,
    _superuser: Superuser,
) -> Result<Redirect, Flash<Redirect>> {
    let update_person: sql_interface::UpdatePerson = form
        .into_inner()
        .into_inner()
        .try_into()
        .map_err(|_| flash_error("Invalid email!"))?;
    let debug = update_person.clone();
    conn.run(move |c| sql_interface::update_person(c, &update_person))
        .await
        .map(|_| Redirect::to(uri!(person_panel)))
        .map_err(|err| {
            server_error(
                format!("Error while updating person: {err}\n{debug:#?}"),
                "an error occured while updating person",
            )
        })
}

#[derive(FromForm)]
pub struct DeletePerson {
    id: i64,
}

#[post("/person/delete", data = "<form>")]
pub async fn delete_person(
    conn: BususagesDBConn,
    form: Form<Strict<DeletePerson>>,
    _superuser: Superuser,
) -> Result<Redirect, Flash<Redirect>> {
    let person_id = form.id;
    conn.run(move |c| sql_interface::delete_person(c, person_id))
        .await
        .map(|_| Redirect::to(uri!(person_panel)))
        .map_err(|err| {
            server_error(
                format!("Error while deleting person: {err}\nPerson ID: {person_id}",),
                "an error occured while deleting person",
            )
        })
}

#[get("/person/list?<id>")]
pub async fn introspect_person(
    conn: BususagesDBConn,
    id: i64,
    _superuser: Superuser,
) -> Result<Template, Flash<Redirect>> {
    #[derive(Debug, Serialize)]
    struct TemplateRegistration {
        pretty_date: String,
        registration: sql_interface::Registration,
    }

    let registrations = conn
        .run(move |c| {
            sql_interface::search_registrations(
                c,
                &SearchRegistrationsBy::PersonId {
                    id,
                    filter: DriveFilter::ListAll,
                },
            )
        })
        .await
        .map_err(|err| {
            server_error(
                format!("Error occurred while introspecting {id} (registration search): {err}"),
                "an error occurred while introspecting that person",
            )
        })?;

    let person = conn
        .run(move |c| sql_interface::search_person(c, &SearchPersonBy::Id(id)))
        .await
        .map_err(|err| {
            server_error(
                format!("Error occurred while introspecting {id} (name search): {err}"),
                "an error occurred while introspecting that person",
            )
        })?;

    let registrations: Vec<_> = registrations
        .into_iter()
        .map(|r| TemplateRegistration {
            pretty_date: format_date(r.drive.date),
            registration: r,
        })
        .collect();

    Ok(Template::render(
        "personintrospect",
        context! {
            prename: person.prename,
            name: person.name,
            registrations,
        },
    ))
}

#[derive(FromForm, Debug, Clone)]
pub struct RegistrationForm {
    id: i64,
    date: time::Date,
    new_state: bool,
}

impl RegistrationForm {
    #[must_use]
    pub fn to_registration_update(&self) -> sql_interface::RegistrationUpdate {
        sql_interface::RegistrationUpdate {
            date: time_to_chrono_date(self.date),
            person_id: self.id,
            registered: self.new_state,
        }
    }
}

/// /register, but superuser version
#[post("/person/register", data = "<registration>")]
pub async fn register_person(
    conn: BususagesDBConn,
    registration: Form<Strict<RegistrationForm>>,
    _superuser: Superuser,
) -> Result<Redirect, Flash<Redirect>> {
    let update = registration.to_registration_update();
    conn.run(move |c| sql_interface::update_registration(c, &update))
        .await
        .map_err(|err| {
            server_error(
                format!("Error while updating registration (issued by superuser): {err}"),
                "ein Fehler trat während der Aktualisierung der Anmeldung auf",
            )
        })?;

    let id = registration.id;
    Ok(Redirect::to(uri!(introspect_person(id = id))))
}

#[get("/settings")]
pub async fn settings(
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
pub async fn set_setting(
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
                Flash::error(Redirect::to(uri!(settings)), "Die Zahl ist nicht valide, oder zu groß.")
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
        Redirect::to(uri!(settings)),
        "Einstellung angewandt.",
    ))
}
