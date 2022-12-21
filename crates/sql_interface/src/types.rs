use rusqlite::{
    types::{FromSql, FromSqlError, ValueRef},
    ToSql,
};
use time::OffsetDateTime as DateTime;

use sql_interface_macros::Reconstruct;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Address(pub lettre::Address);

impl FromSql for Address {
    fn column_result(value: ValueRef<'_>) -> rusqlite::types::FromSqlResult<Self> {
        let address = value
            .as_str()?
            .parse()
            .map_err(|err| FromSqlError::Other(Box::new(err)))?;

        Ok(Self(address))
    }
}

impl ToSql for Address {
    fn to_sql(&self) -> rusqlite::Result<rusqlite::types::ToSqlOutput<'_>> {
        let repr: &str = self.0.as_ref();
        repr.to_sql()
    }
}

#[derive(Debug, PartialEq, Eq, Reconstruct)]
pub struct Person {
    #[sql(column = "person_id")]
    pub id: i64,
    pub prename: String,
    pub name: String,
    pub email: Address,

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

/// A drive a user can register for and a registration then refers to.
#[derive(Clone, Debug, PartialEq, Eq, Reconstruct)]
pub struct Drive {
    #[sql(column = "drive_id")]
    pub id: i64,
    #[sql(column = "drivedate")]
    pub date: DateTime,
    pub deadline: Option<DateTime>,
    pub registration_cap: Option<u32>,
}

/// How a person uses the bus on a specfic date.
#[derive(Debug, PartialEq, Eq, Reconstruct)]
pub struct Registration {
    /// The person which this registration belongs to. `token` and `token_expiration` are set
    /// to [`Option::None`] because they're irrelevant.
    #[sql(
        complex = true,
        joined_on = "registration.person_id == person.person_id"
    )]
    pub person: Person,

    /// The drive this potential registration is for.
    #[sql(complex = true, joined_on = "registration.drive_id == drive.drive_id")]
    pub drive: Drive,

    /// Whether or not the person is registered for this bus drive.
    pub registered: bool,
}
