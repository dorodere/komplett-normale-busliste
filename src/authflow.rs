use {
    super::{
        config::Config,
        relative_to_absolute, server_error,
        sql_interface::{self, SearchPersonBy, SearchPersonError},
        BususagesDBConn,
    },
    argon2::{
        password_hash::{PasswordHash, PasswordHasher, PasswordVerifier, SaltString},
        Algorithm, Argon2, Params, Version,
    },
    base64ct::{Base64UrlUnpadded, Encoding},
    chrono::Utc,
    cookie::SameSite,
    jwt::{SignWithKey, VerifyWithKey},
    lettre::{message::Mailbox, transport::smtp::authentication::Credentials, AsyncTransport},
    rand::Rng,
    rocket::{
        form::{Form, Strict},
        http::{Cookie, CookieJar, Status},
        request::{FlashMessage, FromRequest, Outcome, Request},
        response::{Flash, Redirect},
        State,
    },
    rocket_dyn_templates::Template,
    serde::{Deserialize, Serialize},
    std::{collections::HashMap, time::Duration},
    thiserror::Error,
};

#[derive(FromForm)]
pub struct LoginForm {
    email: String,
}

#[get("/", rank = 2)]
pub fn index(flash: Option<FlashMessage<'_>>) -> Template {
    let mut context = HashMap::new();
    if let Some(flash) = flash {
        context.insert("message", flash.message().to_string());
    }
    Template::render("login", &context)
}

/// Generates a token with 128 random bytes, constant-time encoded in URL-safe base64. Also returns
/// the random bytes used.
fn generate_token() -> ([u8; 128], String) {
    let mut rng = rand::thread_rng();
    let mut bytes = [0_u8; 128];

    rng.fill(&mut bytes);

    (bytes, Base64UrlUnpadded::encode_string(&bytes))
}

#[derive(Error, Debug)]
enum SendMailError {
    #[error("SMTP error sending mail: {0}")]
    LettreError(#[from] lettre::transport::smtp::Error),
    #[error("Error while building email: {0}")]
    BuildError(#[from] lettre::error::Error),
}

/// Sends a login mail with the given credentials and mail server.
async fn send_login_mail(
    url: &str,
    from: lettre::Address,
    to: lettre::Address,
    password: &str,
    smtp_server: &str,
) -> Result<(), SendMailError> {
    let email = lettre::Message::builder()
        .from(Mailbox::new(
            Some("Komplett normale Busliste".to_string()),
            from.clone(),
        ))
        .to(Mailbox::new(None, to))
        .subject("[Komplett normale Busliste] Anmeldung")
        .body(format!(
            r#"Hallo,

hier ist dein Link für die Anmeldung in Komplett normale Busliste. Er wird
in einer Stunde automatisch ungültig, aber sobald du einmal angemeldet bist,
bist du das 30 Tage lang.

Hier ist dein Link: {}

Mit freundlichen Grüßen,
Komplett normale Busliste

(P.S. Funktioniert der Link nicht? Entweder bist du bereits angemeldet, oder es
 wurde bereits eine weitere Anmeldung versucht. Nur die zuletzt gesendete Email
 ist gültig. Also überprüfe entweder, ob es eine neuere Email gibt, oder
 versuche erneut, eine Email anzufordern.)

(P.P.S. Zudem kann ein Link nur einmal verwendet werden. Tut mir leid.)"#,
            url,
        ))?;

    let creds = Credentials::new(from.to_string(), password.to_string());
    let conn = lettre::AsyncSmtpTransport::<lettre::Tokio1Executor>::relay(smtp_server)?
        .credentials(creds)
        .build();

    conn.send(email).await?;
    Ok(())
}

// Constructs an [`argon2::Argon2`] instance with reasonable settings.
fn construct_argon2_instance() -> Argon2<'static> {
    Argon2::new(
        Algorithm::Argon2id,
        Version::V0x13,
        Params::new(15_u32 * 1024_u32, 2, 1, None).unwrap(),
    )
}

#[post("/", data = "<login_details>")]
pub async fn login(
    conn: BususagesDBConn,
    config: &State<Config>,
    mut login_details: Form<Strict<LoginForm>>,
) -> Result<Flash<Redirect>, Flash<Redirect>> {
    // strip and normalize a bit
    login_details.email = login_details.email.trim().to_lowercase();

    // first try to find the email in the database
    let search_result = conn
        .run(move |c| {
            sql_interface::search_person(c, &SearchPersonBy::Email(login_details.email.clone()))
        })
        .await;
    let person = match search_result {
        Err(SearchPersonError::NotFound) => {
            return Err(Flash::error(
                Redirect::to(uri!(index)),
                "Emailadresse nicht in der Datenbank gefunden.",
            ))
        }
        Err(err) => {
            return Err(server_error(
                &format!("Non-user error while searching for email: {}", err),
                "ein Fehler trat auf, während ich nach deiner Emailadresse gesucht habe",
            ))
        }
        Ok(address) => address,
    };

    // second, generate the token
    let (raw_token, encoded_token) = generate_token();
    let url = uri!(
        "http://192.168.110.141",
        verify_token(token = encoded_token, person_id = person.id)
    );

    // third, send the email to the search result
    match send_login_mail(
        &url.to_string(),
        config.email.clone(),
        person.email.clone(),
        &config.email_creds,
        &config.smtp_server,
    )
    .await
    {
        Err(SendMailError::LettreError(err)) => {
            let (logmsg, flashmsg) = if err.is_permanent() {
                (
                    format!("Permanent SMTP error while sending email: {}", err),
                    "ein permanenter Fehler trat auf, während ich versuchte, die Anmeldemail zu verschicken",
                )
            } else if err.is_transient() {
                (
                    format!("Transient SMTP error while sending email: {}", err),
                    "ein temporärer Fehler trat auf, während ich versuchte, die Anmeldemail zu verschicken",
                )
            } else {
                (
                    format!("Error occured while trying to send email: {}", err),
                    "ein Fehler trat auf, während ich versuchte, die Anmeldemail zu verschicken",
                )
            };
            return Err(server_error(&logmsg, flashmsg));
        }
        Err(SendMailError::BuildError(err)) => panic!("{}", err),
        _ => (),
    };

    // fourth, hash token and insert into DB
    let salt = SaltString::generate(rand::thread_rng());
    let argon2 = construct_argon2_instance();
    let hashed_token = argon2
        .hash_password(&raw_token, &salt)
        .expect("Could not hash token!")
        .to_string();
    if let Err(err) = conn
        .run(move |c| sql_interface::update_token(c, person.id, Some(hashed_token)))
        .await
    {
        return Err(server_error(
            &format!("Database error while updating token: {}", err),
            "ein Fehler trat auf, während ich versuchte, den Anmeldeversuch abzuspeichern",
        ));
    };

    Ok(Flash::success(
        Redirect::to(uri!(index)),
        "Anmeldelink per Email versendet. Folge diesem, um fortzufahren.\n\nHinweis: Das heißt, die Adresse wurde gefunden und alles ist ok!",
    ))
}

/// Whether a timepoint expired already, measured using the system time.
/// As this doesn't convert between timezones and similar, the timepoint should also be generated
/// on the same machine.
fn timepoint_expired(timepoint: i64) -> bool {
    let now = Utc::now().timestamp();
    now > timepoint
}

/// A stupid helper function because consts are limited to function calls, but I also don't want to
/// type out the error message all the time.
fn verify_failure_flash() -> Flash<Redirect> {
    Flash::error(
        Redirect::to(uri!(index)),
        "Ungültiges Token, ungültiger Nutzer oder anderer Fehler. Wie dem auch sei, bitte versuche es erneut!",
    )
}

#[derive(Debug, Serialize, Deserialize)]
struct Claims {
    exp: i64,
    sub: i64,
    superuser: bool,
}

impl Claims {
    fn expired(&self) -> bool {
        timepoint_expired(self.exp)
    }

    fn try_from_request(req: &Request<'_>) -> Result<Self, AuthError> {
        let config = req
            .rocket()
            .state::<Config>()
            .expect("Config is not set in main!");
        let cookies = req.cookies();
        let claims: Claims = cookies
            .get("auth-token")
            .ok_or(AuthError::CookieNotFound)?
            .value()
            .verify_with_key(&config.jwt_key)?;

        if claims.expired() {
            cookies.remove(Cookie::named("auth-token"));
            return Err(AuthError::JwtExpired);
        }

        Ok(claims)
    }
}

#[get("/login/<token>?<person_id>")]
pub async fn verify_token(
    conn: BususagesDBConn,
    jar: &CookieJar<'_>,
    config: &State<Config>,
    token: String,
    person_id: i64,
) -> Flash<Redirect> {
    // first, find the person (which contains the token hash) in the DB
    let search_result = conn
        .run(move |c| sql_interface::search_person(c, &SearchPersonBy::Id(person_id)))
        .await;
    let person = match search_result {
        Err(SearchPersonError::NotFound) => return verify_failure_flash(),
        Err(err) => {
            log::error!(
                "Database or lettre conversion error while searching for mail: {}",
                err
            );
            // maybe this isn't ideal, but just stay unclear I guess
            return verify_failure_flash();
        }
        Ok(person) => person,
    };

    // second, check if the token expired
    let expiration = if let Some(timestamp) = person.token_expiration {
        timestamp
    } else {
        return verify_failure_flash();
    };
    if timepoint_expired(expiration) {
        return verify_failure_flash();
    }

    let db_token = if let Some(token) = person.token {
        token
    } else {
        return verify_failure_flash();
    };
    let client_token_bytes = match Base64UrlUnpadded::decode_vec(&token) {
        // possibly evil client, but we just friendly say "something happened and idk what"
        Err(_) => return verify_failure_flash(),
        Ok(x) => x,
    };

    // third, verify client token with token hash we got above
    let argon2 = construct_argon2_instance();
    let db_token_hash = PasswordHash::new(&db_token).expect("Invalid token hash in DB!");
    let passed_verification = argon2
        .verify_password(&client_token_bytes, &db_token_hash)
        .is_ok();
    if !passed_verification {
        return verify_failure_flash();
    }

    // the person is authenticated, let's delete the login token because it's useless now
    let person_id = person.id;
    conn.run(move |c| sql_interface::update_token(c, person_id, None))
        .await
        .unwrap();

    // fourth, generate a JWT and store it in a cookie
    let claims = Claims {
        exp: relative_to_absolute(Duration::from_secs(60 * 60 * 24 * 30)),
        sub: person.id,
        superuser: person.is_superuser,
    };
    let jwt = claims.sign_with_key(&config.jwt_key).unwrap();
    jar.add(
        Cookie::build("auth-token", jwt)
            .same_site(SameSite::Lax)
            .max_age(time::Duration::days(30))
            .finish(),
    );

    let redirect = if person.is_superuser {
        Redirect::to(uri!(super::superuser::panel))
    } else {
        Redirect::to(uri!(super::dashboard))
    };
    Flash::success(
        redirect,
        "Erfolgreich angemeldet. Diese Anmeldung gilt 30 Tage ab jetzt, solange du nicht deine Cookies löschst."
    )
}

#[derive(Debug, Error)]
pub enum AuthError {
    #[error("JWT cookie doesn't exist")]
    CookieNotFound,
    #[error("JWT verification error")]
    JwtVerificationError(#[from] jwt::error::Error),
    #[error("JWT cookie expired")]
    JwtExpired,
    #[error("No permission for the wanted role")]
    NoPermission,
    #[error("Person doesn't exist")]
    NonExistentPerson,
    #[error("Server side database failure: {0}")]
    ServerDBFailure(#[from] rusqlite::Error),
    #[error("Server side email parsing failure: {0}")]
    ServerEmailFailure(#[from] lettre::address::AddressError),
}

impl From<SearchPersonError> for AuthError {
    fn from(source: SearchPersonError) -> Self {
        match source {
            SearchPersonError::NotFound => AuthError::NonExistentPerson,
            SearchPersonError::RusqliteError(err) => AuthError::ServerDBFailure(err),
            SearchPersonError::ParseAddressError(err) => AuthError::ServerEmailFailure(err),
        }
    }
}

// A normal authenticated user. You can be ensured a person is authenticated when you have this in
// scope.
pub struct User {
    person_id: i64,
}

impl User {
    fn from_request_result(req: &Request<'_>) -> Result<Self, AuthError> {
        let claims = Claims::try_from_request(req)?;

        // note that a check for a superuser is left out on purpose, a superuser can still do all
        // the things normal users are also able to do
        Ok(User {
            person_id: claims.sub,
        })
    }

    #[inline]
    pub fn person_id(&self) -> i64 {
        self.person_id
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for User {
    type Error = AuthError;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match Self::from_request_result(req) {
            Ok(user) => Outcome::Success(user),
            Err(_) => Outcome::Forward(()), // idk how I could both forward and say "you failed auth"
        }
    }
}

/// An authenticated superuser, which is defined in the database.
///
/// A superuser has the ability to see all persons who registered for a specific date and to
/// register additional drive dates, but nothing more.
pub struct Superuser {
    _person_id: i64,
}

impl Superuser {
    async fn from_request_result(req: &Request<'_>) -> Result<Self, AuthError> {
        let claims = Claims::try_from_request(req)?;

        // might seem unneeded, but a person could have been revoked superuser access
        // then the JWT flag persists, but isn't valid anymore
        let conn = BususagesDBConn::get_one(req.rocket())
            .await
            .expect("Database fairing not attached!");
        let claims_person_id = claims.sub;
        let person = conn
            .run(move |c| sql_interface::search_person(c, &SearchPersonBy::Id(claims_person_id)))
            .await?;

        if !claims.superuser || !person.is_superuser {
            return Err(AuthError::NoPermission);
        }

        Ok(Superuser {
            _person_id: claims.sub,
        })
    }
}

#[async_trait]
impl<'r> FromRequest<'r> for Superuser {
    type Error = &'static str;

    async fn from_request(req: &'r Request<'_>) -> Outcome<Self, Self::Error> {
        match Self::from_request_result(req).await {
            Ok(user) => Outcome::Success(user),
            Err(AuthError::ServerDBFailure(err)) => {
                log::error!("{}", err);
                Outcome::Failure((
                    Status::InternalServerError,
                    "Server side error while validating login token, please notify the administrator of this instance!",
                ))
            }
            Err(AuthError::ServerEmailFailure(err)) => {
                log::error!("{}", err);
                Outcome::Failure((
                    Status::InternalServerError,
                    "Server side error while validating login token, please notify the administrator of this instance!",
                ))
            }
            Err(e) => {
                println!("{}", e);
                Outcome::Forward(())
            }
        }
    }
}
