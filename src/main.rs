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
        fs::{relative, FileServer},
        http::{Cookie, CookieJar},
        request::FlashMessage,
        response::{Flash, Redirect},
    },
    rocket_dyn_templates::Template,
    rocket_sync_db_pools::{database, rusqlite},
    serde::Serialize,
    sql_interface::{ApplyRegistrationError, SearchRegistrationsBy},
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
        registration: sql_interface::Registration,
    }

    #[derive(Debug, Serialize)]
    struct Context {
        flash: Option<String>,
        past_regs: Vec<TemplateRegistration>,
        future_regs: Vec<TemplateRegistration>,
        show_superuser_controls: bool,
    }

    let registrations = match conn
        .run(move |c| {
            sql_interface::search_registrations(
                c,
                &SearchRegistrationsBy::PersonId {
                    id: user.person_id(),
                    ignore_past: false,
                },
            )
        })
        .await
    {
        Err(err) => {
            return Err(server_error(
                &format!("Error while loading registrations: {}", err),
                "an error occured while loading registrations",
            ))
        }
        Ok(x) => x,
    };

    let flash = flash.map(|flashmsg| flashmsg.message().to_string());

    let now = Utc::now().naive_local().date();
    let mut past_regs = Vec::new();
    let future_regs = registrations
        .into_iter()
        .filter_map(|r| {
            let date = r.date;
            let template_reg = TemplateRegistration {
                pretty_date: format_date(date),
                registration: r,
            };
            if date <= now {
                past_regs.push(template_reg);
                None
            } else {
                Some(template_reg)
            }
        })
        .collect();

    Ok(Template::render(
        "dashboard",
        &Context {
            flash,
            past_regs,
            future_regs,
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
    let now = time::OffsetDateTime::now_utc().date();
    if registration.date <= now {
        return Err(Flash::error(
            Redirect::to(uri!(dashboard)),
            "Du kannst deine Anmeldung nicht mehr am Tag der Fahrt und danach ändern.",
        ));
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
                &format!("Error while updating registration: {}", err),
                "ein Fehler trat während der Aktualisierung der Anmeldung auf",
            ))
        }
        _ => (),
    };

    Ok(Redirect::to(uri!(dashboard)))
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .attach(Template::custom(|engines| {
            engines
                .handlebars
                .register_escape_fn(|input| ammonia::clean_text(input));
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
                superuser::introspect_drive,
                superuser::registrations_panel,
                superuser::person_panel,
                superuser::create_new_person,
                superuser::update_person,
                superuser::delete_person,
                superuser::introspect_person,
                superuser::register_person,
                superuser::settings,
                authflow::index,
                authflow::login,
                authflow::verify_token
            ],
        )
        .mount("/static", FileServer::from(relative!("/static")))
}
