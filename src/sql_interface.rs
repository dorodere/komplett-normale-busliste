use {
    super::relative_to_absolute,
    chrono::Utc,
    lettre::Address,
    rocket_sync_db_pools::rusqlite,
    rusqlite::{named_params, types::Type, types::Value, ToSql},
    serde::{Deserialize, Serialize},
    std::{collections::BTreeMap, fmt, time::Duration},
    thiserror::Error,
};

macro_rules! match_constraint_violation {
    ($statement:expr, $custom_error:expr) => {
        match $statement {
            Err(rusqlite::Error::SqliteFailure(
                libsqlite3_sys::Error {
                    code: libsqlite3_sys::ErrorCode::ConstraintViolation,
                    ..
                },
                _,
            )) => Err($custom_error),
            Err(err) => Err(err.into()),
            _ => Ok(()),
        }
    };
}

/// A person in the SQL database.
#[derive(Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Person {
    pub id: i64,
    pub prename: String,
    pub name: String,
    pub email: lettre::Address,

    /// The token used to authenticate a login. Is always set to [`Option::None`] in case no token
    /// is set or it's unnecessary for the query.
    pub token: Option<String>,

    /// A UNIX timestamp in seconds marking on which timepoint the token expires and should not be
    /// accepted anymore.
    pub token_expiration: Option<i64>,

    /// If the person has elevated previliges, like being allowed to see registration entries and
    /// create new drive dates.
    ///
    /// Automatically set to false if it is not needed for the current query, like querying
    /// registrations themselves.
    pub is_superuser: bool,

    /// Whether or not the person shows up in the registration list. They can log in regardless of this.
    pub is_visible: bool,
}

// Note: Only usable in context here, since the columns are hardcoded
#[doc(hidden)]
fn row_to_full_person(row: &rusqlite::Row) -> rusqlite::Result<Person> {
    Ok(Person {
        id: row.get(0)?,
        prename: row.get(1)?,
        name: row.get(2)?,
        email: row
            .get::<_, String>(3)?
            .parse()
            .expect("Invalid email in database!"),
        token: row.get(4)?,
        token_expiration: row.get(5)?,
        is_superuser: row.get(6)?,
        is_visible: row.get(7)?,
    })
}

// See note of row_to_full_person
#[doc(hidden)]
fn row_to_person(row: &rusqlite::Row) -> rusqlite::Result<Person> {
    Ok(Person {
        id: row.get(0)?,
        prename: row.get(1)?,
        name: row.get(2)?,
        email: row
            .get::<_, String>(3)?
            .parse()
            .expect("Invalid email in database!"),
        token: None,
        token_expiration: None,
        is_superuser: false,
        is_visible: row.get(4)?,
    })
}

/// A drive a user can register for and a registration then refers to.
#[derive(Debug, Serialize, Deserialize)]
pub struct Drive {
    pub id: i64,
    pub date: chrono::NaiveDate,
}

/// How a person uses the bus on a specfic date.
#[derive(Debug, Serialize, Deserialize)]
pub struct Registration {
    // The person which registered the bususage. `token` and `token_expiration` are set to
    // [`Option::None`] because they're irrelevant.
    pub person: Person,

    pub date: chrono::NaiveDate,
    pub registered: bool,
}

/// Parameters needed to update a specific registration.
#[derive(Debug, Serialize, Deserialize)]
pub struct RegistrationUpdate {
    pub date: chrono::NaiveDate,
    pub person_id: i64,
    pub registered: bool,
}

/// Returns `Ok(false)` if the given Result is an error noting that here is Null (or more precisely,
/// `Err(rusqlite::Error::InvalidColumnType(_, _, Type::Null))`).
#[doc(hidden)]
fn false_if_null(cell_query_result: rusqlite::Result<bool>) -> rusqlite::Result<bool> {
    match cell_query_result {
        Err(rusqlite::Error::InvalidColumnType(_, _, Type::Null)) => Ok(false),
        x => x,
    }
}

pub enum DatabaseStatus {
    AlreadyExistent,
    Created,
}

#[allow(unused)]
pub fn init_db_if_necessary(
    conn: &mut rusqlite::Connection,
) -> Result<DatabaseStatus, rusqlite::Error> {
    // dummy query to see if the db has a table in it
    // yeah, we could query sqlite_master, but this way we can also directly ask for the
    // columns
    if conn
        .execute(
            "SELECT person_id, name, email
            FROM person
            WHERE false",
            [],
        )
        .is_err()
    {
        conn.execute_batch(include_str!("./init_db.sql"))?;
        Ok(DatabaseStatus::Created)
    } else {
        Ok(DatabaseStatus::AlreadyExistent)
    }
}

pub enum SearchRegistrationsBy {
    /// Searches the registrations by date. All persons are included, regardless of whether they
    /// registered or not.
    Date(chrono::NaiveDate),

    /// Searches the registrations by person id.
    PersonId { id: i64, ignore_past: bool },
}

/// Creates a vector of [`Registration`]s filtered by the given criteria.
///
/// Note that all registrations are sorted in ascending order by last name.
pub fn search_registrations(
    conn: &mut rusqlite::Connection,
    by: &SearchRegistrationsBy,
) -> Result<Vec<Registration>, rusqlite::Error> {
    let mut statement = match by {
        SearchRegistrationsBy::Date(_) => conn.prepare(
            "SELECT person.person_id, person.prename, person.name, person.email, person.is_visible,
                registration.registered, :date
            FROM person
            LEFT OUTER JOIN drive ON (drive.drivedate == :date)
            LEFT OUTER JOIN registration ON (
                registration.person_id == person.person_id AND
                registration.drive_id == drive.drive_id
            )
            WHERE person.is_visible
            ORDER BY person.name",
        ),
        SearchRegistrationsBy::PersonId {
            ignore_past: false, ..
        } => conn.prepare(
            "SELECT person.person_id, person.prename, person.name, person.email, person.is_visible,
                registration.registered, drive.drivedate
            FROM drive
            LEFT OUTER JOIN person ON (person.person_id == :id)
            LEFT OUTER JOIN registration ON (
                registration.drive_id == drive.drive_id
                AND registration.person_id == person.person_id
            )
            ORDER BY person.name",
        ),
        SearchRegistrationsBy::PersonId {
            ignore_past: true, ..
        } => conn.prepare(
            "SELECT person.person_id, person.prename, person.name, person.email, person.is_visible,
                registration.registered, drive.drivedate
            FROM drive
            LEFT OUTER JOIN person ON (person.person_id == :id)
            LEFT OUTER JOIN registration ON (
                registration.drive_id == drive.drive_id
                AND registration.person_id == person.person_id
            )
            WHERE drive.drivedate >= :now
            ORDER BY person.name",
        ),
    }?;
    let rows = match by {
        SearchRegistrationsBy::Date(date) => statement.query(named_params! { ":date": date }),
        SearchRegistrationsBy::PersonId {
            id,
            ignore_past: false,
        } => statement.query(named_params! { ":id": id }),
        SearchRegistrationsBy::PersonId {
            id,
            ignore_past: true,
        } => {
            let now = Utc::now().naive_local().date();
            statement.query(named_params! { ":id": id, ":now": now })
        }
    }?;
    Ok(rows
        .mapped(|row| {
            Ok(Registration {
                person: row_to_person(row)?,
                registered: false_if_null(row.get(5))?,
                date: row.get(6)?,
            })
        })
        .map(Result::unwrap)
        .collect())
}

#[derive(Debug, Serialize, Deserialize)]
pub struct CountedRegistrations {
    persons: Vec<PersonWithRegistrations>,
    sum: i64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PersonWithRegistrations {
    person: Person,
    count: i64,
}

/// Counts all registrations between the given dates, [`None`] meaning infinity in either
/// direction.
pub fn list_persons_counted_registrations(
    conn: &mut rusqlite::Connection,
    from: Option<chrono::NaiveDate>,
    to: Option<chrono::NaiveDate>,
) -> Result<CountedRegistrations, rusqlite::Error> {
    let statement = format!(
        "SELECT person.person_id, person.prename, person.name, person.email, person.is_visible,
          COUNT(registration.person_id)
        FROM drive, person
        LEFT OUTER JOIN registration ON (
          registration.drive_id == drive.drive_id
          AND registration.person_id == person.person_id
        )
        WHERE registration.registered == true
            {}
            {}
        GROUP BY registration.person_id
        ORDER BY person.name",
        from.map(|_| "AND :from <= drive.drivedate").unwrap_or(""),
        to.map(|_| "AND drive.drivedate <= :to").unwrap_or(""),
    );
    let mut statement = conn.prepare(&statement)?;
    let rows = match (from, to) {
        (None, None) => statement.query([]),
        (Some(from), None) => statement.query(named_params! { ":from": from }),
        (None, Some(to)) => statement.query(named_params! { ":to": to }),
        (Some(from), Some(to)) => statement.query(named_params! { ":from": from, ":to": to }),
    }?;

    Ok(rows
        .mapped(|row| {
            Ok(PersonWithRegistrations {
                person: row_to_person(row)?,
                count: row.get(5)?,
            })
        })
        .map(Result::unwrap)
        .fold(
            CountedRegistrations {
                persons: Vec::new(),
                sum: 0,
            },
            |mut state, person| {
                let person = person;
                state.sum += person.count;
                state.persons.push(person);
                state
            },
        ))
}

#[derive(Debug, Error)]
pub enum ApplyRegistrationError {
    #[error("Database or query error: {0}")]
    RusqliteError(#[from] rusqlite::Error),
    #[error("Unknown drive date")]
    UnknownDriveDate,
}

/// Creates a registration entry with the given registration and usage, overwriting it if it
/// previously existed.
pub fn update_registration(
    conn: &mut rusqlite::Connection,
    registration: &RegistrationUpdate,
) -> Result<(), ApplyRegistrationError> {
    match_constraint_violation!(
        conn.execute(
            "INSERT INTO registration (person_id, drive_id, registered)
        VALUES (
            :person_id,
            (
                SELECT drive_id
                FROM drive
                WHERE drivedate == :date
            ),
            :registered
        )
        ON CONFLICT(person_id, drive_id)
              DO UPDATE SET registered=:registered",
            named_params! {
                ":person_id": registration.person_id,
                ":date": registration.date,
                ":registered": registration.registered,
            },
        ),
        ApplyRegistrationError::UnknownDriveDate
    )
}

pub enum SearchPersonBy {
    /// Searches by email. Returns only one result because emails are supposed to be unique.
    /// Token, token expiration and superuser state are **not** included.
    Email(String),

    /// Searches by person ID. Note that this is the only search method which includes token, token
    /// expiration and superuser state.
    Id(i64),
}

#[derive(Error, Debug)]
pub enum SearchPersonError {
    #[error("Database or query error: {0}")]
    RusqliteError(#[from] rusqlite::Error),
    #[error("Unable to parse email address: {0}")]
    ParseAddressError(#[from] lettre::address::AddressError),
    #[error("Criteria not found")]
    NotFound,
}

/// Searches for a person in the database by the given criteria. Returns
/// [`SearchPersonError`]`::NotFound` if the search criteria is not present.
pub fn search_person(
    conn: &mut rusqlite::Connection,
    by: &SearchPersonBy,
) -> Result<Person, SearchPersonError> {
    let mut statement = match by {
        SearchPersonBy::Email(_) => conn.prepare(
            "SELECT person_id, prename, name, email, is_visible
            FROM person
            WHERE email == :email",
        ),
        SearchPersonBy::Id(_) => conn.prepare(
            "SELECT person_id, prename, name, email, token, token_expiration, is_superuser, is_visible
            FROM person
            WHERE person_id == :id",
        ),
    }?;
    let rows = match by {
        SearchPersonBy::Email(ref email) => statement.query(named_params! { ":email": email }),
        SearchPersonBy::Id(ref id) => statement.query(named_params! { ":id": id }),
    }?;
    let found_person = rows
        .mapped(|row| {
            if let SearchPersonBy::Id(_) = by {
                row_to_full_person(row)
            } else {
                row_to_person(row)
            }
        })
        .next();
    match found_person {
        Some(Ok(person)) => Ok(person),
        Some(Err(err)) => Err(err.into()),
        None => Err(SearchPersonError::NotFound),
    }
}

pub enum Filter {
    IncludingInvisible,
    #[allow(unused)]
    OnlyVisible,
}

/// Lists all persons, optionally also invisible ones.
///
/// Doesn't include token, token expiration and superuser state (you know it anyways).
pub fn list_all_persons(
    conn: &mut rusqlite::Connection,
    filter: Filter,
) -> rusqlite::Result<Vec<Person>> {
    let mut statement = conn.prepare(&format!(
        "SELECT person_id, prename, name, email, is_visible
        FROM person
        {}
        ORDER BY name",
        match filter {
            Filter::OnlyVisible => "WHERE is_visible",
            Filter::IncludingInvisible => "",
        }
    ))?;
    let persons = statement
        .query_map([], row_to_person)?
        .map(Result::unwrap)
        .collect();
    Ok(persons)
}

/// Updates a token for a person and sets the expiration time to one hour from now if `new_token`
/// is [`Option::Some`], else the token is set to NULL.
pub fn update_token(
    conn: &mut rusqlite::Connection,
    person_id: i64,
    new_token: Option<String>,
) -> Result<(), rusqlite::Error> {
    let (new_token, expiration_timepoint) = new_token.map_or((None, None), |new_token| {
        let expiration_timepoint = relative_to_absolute(Duration::from_secs(60 * 60));
        (Some(new_token), Some(expiration_timepoint))
    });

    conn.execute(
        "UPDATE person
        SET token = :new_token, token_expiration = :expiration_timepoint
        WHERE person_id == :person_id",
        named_params! {
            ":person_id": person_id,
            ":new_token": new_token,
            ":expiration_timepoint": expiration_timepoint,
        },
    )?;
    Ok(())
}

pub fn list_drives(conn: &mut rusqlite::Connection) -> Result<Vec<Drive>, rusqlite::Error> {
    let mut statement = conn.prepare("SELECT drive_id, drivedate FROM drive")?;
    let result = statement.query_map([], |row| {
        Ok(Drive {
            id: row.get(0)?,
            date: row.get(1)?,
        })
    })?;
    Ok(result.map(Result::unwrap).collect())
}

#[derive(Debug, Error)]
pub enum InsertDriveError {
    #[error("Database or query error: {0}")]
    RusqliteError(#[from] rusqlite::Error),
    #[error("The bus drive already exists")]
    AlreadyExists,
}

/// Inserts a new drive entry in the DB. You should check the return result for
/// [`InsertDriveError`]`::AlreadyExists`.
pub fn insert_new_drive(
    conn: &mut rusqlite::Connection,
    date: chrono::NaiveDate,
) -> Result<(), InsertDriveError> {
    match_constraint_violation!(
        conn.execute(
            "INSERT INTO drive (drivedate)
            VALUES (:date)",
            named_params! {
                ":date": date,
            },
        ),
        InsertDriveError::AlreadyExists
    )
}

/// Deletes a drive by ID and all linked registrations. **This action is irreversible.**
pub fn delete_drive(conn: &mut rusqlite::Connection, id: i64) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM drive
        WHERE drive_id == :id",
        named_params! {
            ":id": id,
        },
    )?;
    Ok(())
}

#[derive(Debug, Error)]
pub enum PersonCreationError {
    #[error("Email is already used")]
    EmailAlreadyInUse,
    #[error("Database or query error: {0}")]
    RusqliteError(#[from] rusqlite::Error),
}

#[derive(Clone, Debug)]
pub struct NewPerson {
    pub prename: String,
    pub name: String,
    pub email: Address,
}

/// Inserts a new person into the database. The email is not checked for validity.
pub fn insert_new_person(
    conn: &mut rusqlite::Connection,
    person: &NewPerson,
) -> Result<(), PersonCreationError> {
    match_constraint_violation!(
        conn.execute(
            "INSERT INTO person (prename, name, email, is_superuser, is_visible)
            VALUES (:prename, :name, :email, false, true)",
            named_params! {
                ":prename": person.prename,
                ":name": person.name,
                ":email": person.email.to_string(),
            },
        ),
        PersonCreationError::EmailAlreadyInUse
    )
}

#[derive(Clone, Debug)]
pub struct UpdatePerson {
    pub id: i64,
    pub prename: String,
    pub name: String,
    pub email: Address,
    pub is_visible: bool,
}

/// Updates a person entry by ID. The email is not checked for validity.
pub fn update_person(
    conn: &mut rusqlite::Connection,
    person: &UpdatePerson,
) -> Result<(), rusqlite::Error> {
    // theoretically possible to check if the person ID actually matched an entry, but just
    // omitting here
    conn.execute(
        "UPDATE person
        SET prename = :prename, name = :name, email = :email, is_visible = :is_visible
        WHERE person_id = :id",
        named_params! {
            ":id": person.id,
            ":prename": person.prename,
            ":name": person.name,
            ":email": person.email.to_string(),
            ":is_visible": person.is_visible,
        },
    )?;
    Ok(())
}

/// Deletes a person entry by ID and all linked registrations. **This action is irreversible.**
pub fn delete_person(
    conn: &mut rusqlite::Connection,
    person_id: i64,
) -> Result<(), rusqlite::Error> {
    conn.execute(
        "DELETE FROM person
        WHERE person_id == :id",
        named_params! {
            ":id": person_id,
        },
    )?;
    Ok(())
}

/// Lists _all_ settings currently held, and uses [`stringify_value`] the values.
pub fn all_settings(
    conn: &mut rusqlite::Connection,
) -> Result<BTreeMap<String, String>, rusqlite::Error> {
    let mut statement = conn.prepare(
        "SELECT name, value
        FROM settings",
    )?;

    let settings = statement
        .query_map([], |row| Ok((row.get(0)?, stringify_value(row.get(1)?))))?
        .collect();

    settings
}

/// Retrieves a setting stored in the database.
pub fn get_setting(
    conn: &mut rusqlite::Connection,
    name: impl AsRef<str>,
) -> Result<Value, rusqlite::Error> {
    let mut statement = conn.prepare(
        "SELECT value
        FROM settings
        WHERE name == :name",
    )?;
    let mut query = statement.query(named_params! {
        ":name": name.as_ref(),
    })?;

    // can contain only one row since `name` is the primary key
    let row = query.next()?.unwrap_or_else(|| {
        panic!(
            "expected setting '{:?}' to exist to query, found nothing in database",
            name.as_ref()
        )
    });

    Ok(row.get(0)?)
}

/// Updates a setting stored in the database.
pub fn set_setting(
    conn: &mut rusqlite::Connection,
    name: impl AsRef<str>,
    value: impl ToSql + fmt::Debug,
) -> Result<(), rusqlite::Error> {
    let mut statement = conn.prepare(
        "UPDATE settings
        SET value = :value
        WHERE name == :name
        RETURNING true", // dummy value to retrieve whether the update happened or we were lied to
                         // TODO: .execute returns a usize saying how many rows have been modified,
                         // could use that instead
    )?;
    let mut query = statement.query(named_params! {
        ":name": name.as_ref(),
        ":value": value,
    })?;

    match query.next()? {
        None => panic!(
            "expected '{:?}' to exist to insert '{:?}', found nothing in database",
            name.as_ref(),
            value,
        ),
        Some(_) => Ok(()),
    }
}

pub fn stringify_value(value: Value) -> String {
    match value {
        Value::Null => "".to_string(),
        Value::Integer(number) => number.to_string(),
        Value::Real(number) => number.to_string(),
        Value::Text(text) => text,
        Value::Blob(blob) => String::from_utf8_lossy(&blob).to_string(),
    }
}
