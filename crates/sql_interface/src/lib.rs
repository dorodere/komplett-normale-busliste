#![allow(clippy::cast_possible_truncation)] // Irrelevant in this context

use time::OffsetDateTime as DateTime;

mod sql_struct;
#[cfg(test)]
mod tests;

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
#[derive(Clone, Debug)]
pub struct Drive {
    pub id: i64,
    pub date: DateTime,
    pub deadline: Option<DateTime>,
    pub registration_cap: Option<u32>,
    pub already_registered_count: u32,
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
