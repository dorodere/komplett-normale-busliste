#![allow(clippy::cast_possible_truncation, clippy::cast_sign_loss)]

use {
    super::{
        authflow::Superuser,
        date_helpers::{figure_out_exact_deadline, time_to_chrono_date, time_to_chrono_datetime},
        format_date, server_error,
        sql_interface::{
            self, DriveFilter, InsertDriveError, Person, Registration, SearchPersonBy,
            SearchRegistrationsBy, UpdateDriveError, VisibilityFilter,
        },
        BususagesDBConn,
    },
    chrono::Utc,
    rocket::{
        form::{Form, Lenient, Strict},
        request::FlashMessage,
        response::{Flash, Redirect},
    },
    rocket_dyn_templates::{context, Template},
    rusqlite::types::Value,
    serde::Serialize,
};

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

#[get("/drives")]
pub async fn drives_panel(
    conn: BususagesDBConn,
    flash: Option<FlashMessage<'_>>,
    _superuser: Superuser,
) -> Result<Template, Flash<Redirect>> {
    #[derive(Clone, Debug, Serialize)]
    struct TemplateDrive {
        date: chrono::NaiveDate,
        deadline: Option<chrono::NaiveDateTime>,
        id: i64,
    }

    let drives = conn.run(sql_interface::list_drives).await.map_err(|err| {
        server_error(
            format!("Error while listing drives: {err}"),
            "an error occured while listing drives",
        )
    })?;

    Ok(Template::render(
        "drives-panel",
        context! {
            flash: flash.map(|flash| flash.message().to_string()),
            future_drives: drives.future,
            past_drives: drives.past,
        },
    ))
}

#[get("/drive/list?<date>")]
pub async fn introspect_drive(
    conn: BususagesDBConn,
    date: time::Date,
    _superuser: Superuser,
) -> Result<Template, Flash<Redirect>> {
    #[derive(Debug, Serialize)]
    struct Context {
        registrations: Vec<Registration>,
        pretty_date: String,
        now: String,
    }

    let pretty_date = super::format_date(time_to_chrono_date(date));
    let registrations = conn
        .run(move |c| {
            sql_interface::search_registrations(
                c,
                &SearchRegistrationsBy::Date(time_to_chrono_date(date)),
            )
        })
        .await
        .map_err(|err| {
            server_error(
                format!("Error listing registrations for date {date}: {err}"),
                "an error occured while listing registrations",
            )
        })?;

    Ok(Template::render(
        "list",
        &Context {
            registrations,
            pretty_date,
            now: Utc::now().format("%A, %d.%m.%Y %H:%M:%S").to_string(),
        },
    ))
}

#[derive(Debug, FromForm)]
pub struct NewDrive {
    date: time::Date,
}

#[post("/drive/new", data = "<form>")]
pub async fn create_new_drive(
    conn: BususagesDBConn,
    form: Form<Strict<NewDrive>>,
    _superuser: Superuser,
) -> Result<Redirect, Flash<Redirect>> {
    let drive_date = time_to_chrono_date(form.date);

    let default_deadline = conn
        .run(|c| sql_interface::get_setting(c, "default-deadline"))
        .await
        .map_err(|err| {
            server_error(
                format!("Error querying default deadline: {err}"),
                "ein Fehler trat auf, während ich nach den Einstellungen geschaut habe",
            )
        })?;
    let deadline = match default_deadline {
        Value::Integer(deadline_weekday) => Some(figure_out_exact_deadline(
            deadline_weekday as u32,
            drive_date,
        )),
        Value::Null => None,
        _ => unreachable!(
            "validation messed up, contained '{:?}' which is not an integer nor null",
            default_deadline
        ),
    };

    match conn
        .run(move |c| sql_interface::insert_new_drive(c, drive_date, deadline))
        .await
    {
        Err(InsertDriveError::AlreadyExists) => Err(Flash::error(
            Redirect::to(uri!(drives_panel)),
            "Bus drive already exists!",
        )),
        Err(err) => {
            return Err(server_error(
                format!("Error inserting new drive: {err}\nDate: {drive_date:?}"),
                "an error occured while inserting a new drive",
            ))
        }
        _ => Ok(Redirect::to(uri!(drives_panel))),
    }
}

#[derive(Debug, FromForm)]
pub struct DeleteDrive {
    id: i64,
}

#[post("/drive/delete", data = "<form>")]
pub async fn delete_drive(
    conn: BususagesDBConn,
    form: Form<Strict<DeleteDrive>>,
    _superuser: Superuser,
) -> Result<Redirect, Flash<Redirect>> {
    let drive_id = form.id;
    conn.run(move |c| sql_interface::delete_drive(c, drive_id))
        .await
        .map(|_| Redirect::to(uri!(drives_panel)))
        .map_err(|err| {
            server_error(
                format!("Error while deleting drive: {err}\nDrive ID: {drive_id}",),
                "an error occured while deleting drive",
            )
        })
}

#[derive(FromForm, Debug)]
pub struct UpdateDrive {
    id: i64,
    date: time::Date,
    deadline: time::PrimitiveDateTime,
    registration_cap: Option<u32>,
}

#[post("/drive/update", data = "<update>")]
pub async fn update_deadline(
    conn: BususagesDBConn,
    update: Option<Form<Strict<UpdateDrive>>>,
    _superuser: Superuser,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    let Some(update) = update else {
        return Err(Flash::error(Redirect::to(uri!(drives_panel)), "Please fill all fields."));
    };

    let update = sql_interface::Drive {
        id: update.id,
        date: time_to_chrono_date(update.date),
        deadline: Some(time_to_chrono_datetime(update.deadline)),
        registration_cap: update.registration_cap,
        already_registered_count: 0,
    };

    let closure_update = update.clone();
    conn.run(move |c| sql_interface::update_drive_deadline(c, closure_update))
        .await
        .map_err(|err| match err {
            UpdateDriveError::DateAlreadyExists => Flash::error(
                Redirect::to(uri!(drives_panel)),
                "Es existiert bereits ein Drive mit diesem Datum, nichts geändert.",
            ),
            UpdateDriveError::RusqliteError(err) => server_error(
                format!(
                    "Error while updating drive {} to deadline {:?}: {}",
                    update.id, update.deadline, err,
                ),
                "ein Fehler trat während der Aktualisierung der Deadline auf",
            ),
        })
        .map(|_| Flash::success(Redirect::to(uri!(drives_panel)), "Änderungen angewandt."))
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
