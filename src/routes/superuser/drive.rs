use chrono::Utc;
use rocket::{
    form::Form,
    response::Redirect,
    routes, Route,
    {form::Strict, request::FlashMessage, response::Flash},
};
use rocket_dyn_templates::{context, Template};
use rusqlite::types::Value;
use serde::Serialize;

use crate::{
    date_helpers::{
        figure_out_exact_deadline, format_date, time_to_chrono_date, time_to_chrono_datetime,
    },
    routes::authflow::Superuser,
    server_error,
    sql_interface::{self, InsertDriveError, SearchRegistrationsBy, UpdateDriveError},
    BususagesDBConn,
};

#[must_use]
pub fn routes() -> Vec<Route> {
    routes![
        drives_panel,
        introspect_drive,
        create_new_drive,
        delete_drive,
        update_deadline
    ]
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
    let pretty_date = format_date(time_to_chrono_date(date));
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
        context! {
            registrations,
            pretty_date,
            now: Utc::now().format("%A, %d.%m.%Y %H:%M:%S").to_string(),
        },
    ))
}

#[derive(Debug, FromForm)]
pub struct NewDrive {
    pub(crate) date: time::Date,
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
    pub(crate) id: i64,
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
    pub(crate) id: i64,
    pub(crate) date: time::Date,
    pub(crate) deadline: time::PrimitiveDateTime,
    pub(crate) registration_cap: Option<u32>,
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
