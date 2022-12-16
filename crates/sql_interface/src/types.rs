use rusqlite::types::{FromSql, ValueRef};
use time::OffsetDateTime as DateTime;

use crate::sql_struct::{next_converted, ReconstructResult, SqlStruct};

#[derive(Debug, PartialEq, Eq)]
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

/// A drive a user can register for and a registration then refers to.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Drive {
    pub id: i64,
    pub date: DateTime,
    pub deadline: Option<DateTime>,
    pub registration_cap: Option<u32>,
}

impl SqlStruct for Drive {
    fn required_tables() -> Vec<&'static str> {
        vec!["drive"]
    }

    fn select_exprs() -> Vec<&'static str> {
        vec![
            "drive.drive_id",
            "drive.drivedate",
            "drive.deadline",
            "drive.registration_cap",
        ]
    }

    fn from_row<'a>(mut row: impl Iterator<Item = ValueRef<'a>>) -> ReconstructResult<Self> {
        Ok(Self {
            id: next_converted(&mut row)?,
            date: next_converted(&mut row)?,
            deadline: next_converted(&mut row)?,
            registration_cap: next_converted(&mut row)?,
        })
    }
}

/// How a person uses the bus on a specfic date.
#[derive(Debug)]
pub struct Registration {
    /// The person which registered the bususage. `token` and `token_expiration` are set to
    /// [`Option::None`] because they're irrelevant.
    pub person: Person,

    /// The drive this registration is for.
    pub drive: Drive,
}
