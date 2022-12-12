use rocket::{
    form::{Form, Lenient, Strict},
    request::FlashMessage,
    response::{Flash, Redirect},
    Route,
};
use rocket_dyn_templates::{context, Template};
use serde::Serialize;

use crate::{
    date_helpers::{format_date, time_to_chrono_date},
    routes::authflow::Superuser,
    server_error,
    sql_interface::{self, DriveFilter, SearchPersonBy, SearchRegistrationsBy, VisibilityFilter},
    BususagesDBConn,
};

#[must_use]
pub fn routes() -> Vec<Route> {
    routes![panel, create, update, delete, introspect, register]
}

/// Just a shorthand for an error flash containing a redirect.
#[inline]
fn flash_error(message: &str) -> Flash<Redirect> {
    Flash::error(Redirect::to(uri!(panel)), message)
}

#[get("/person")]
pub async fn panel(
    conn: BususagesDBConn,
    _superuser: Superuser,
    flash: Option<FlashMessage<'_>>,
) -> Result<Template, Flash<Redirect>> {
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
        context! {
            flash: flash.map(|flash| flash.message().to_string()),
            persons,
        },
    ))
}

#[derive(Debug, FromForm)]
pub struct Create {
    prename: String,
    name: String,
    email: String,
}

impl TryFrom<Create> for sql_interface::NewPerson {
    type Error = lettre::address::AddressError;

    fn try_from(source: Create) -> Result<sql_interface::NewPerson, Self::Error> {
        Ok(sql_interface::NewPerson {
            prename: source.prename,
            name: source.name,
            email: source.email.parse()?,
        })
    }
}

#[post("/person/new", data = "<form>")]
pub async fn create(
    conn: BususagesDBConn,
    form: Form<Strict<Create>>,
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
        _ => Ok(Redirect::to(uri!(panel))),
    }
}

#[derive(Debug, FromForm)]
pub struct Update {
    id: i64,
    prename: String,
    name: String,
    email: String,
    is_visible: Lenient<bool>,
}

impl TryFrom<Update> for sql_interface::UpdatePerson {
    type Error = lettre::address::AddressError;

    fn try_from(source: Update) -> Result<sql_interface::UpdatePerson, Self::Error> {
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
pub async fn update(
    conn: BususagesDBConn,
    form: Form<Strict<Update>>,
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
        .map(|_| Redirect::to(uri!(panel)))
        .map_err(|err| {
            server_error(
                format!("Error while updating person: {err}\n{debug:#?}"),
                "an error occured while updating person",
            )
        })
}

#[derive(FromForm)]
pub struct Delete {
    id: i64,
}

#[post("/person/delete", data = "<form>")]
pub async fn delete(
    conn: BususagesDBConn,
    form: Form<Strict<Delete>>,
    _superuser: Superuser,
) -> Result<Redirect, Flash<Redirect>> {
    let person_id = form.id;
    conn.run(move |c| sql_interface::delete_person(c, person_id))
        .await
        .map(|_| Redirect::to(uri!(panel)))
        .map_err(|err| {
            server_error(
                format!("Error while deleting person: {err}\nPerson ID: {person_id}",),
                "an error occured while deleting person",
            )
        })
}

#[get("/person/list?<id>")]
pub async fn introspect(
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
pub async fn register(
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
    Ok(Redirect::to(uri!(introspect(id = id))))
}
