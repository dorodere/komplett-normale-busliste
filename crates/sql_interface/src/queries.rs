use sql_interface_macros::Reconstruct;

use crate::types::{Drive, Person, Registration};

/// [`Registration`] but narrowed down to a specific person, listing all drives.
#[derive(Debug, PartialEq, Eq, Reconstruct)]
#[sql(table = "drive")]
pub struct RegistrationPerDrive {
    /// The drive this potential registration is for.
    #[sql(complex = true)]
    pub drive: Drive,

    /// The person which this registration belongs to.
    #[sql(complex = true, condition_in_join = true)]
    pub person: Person,

    #[sql(
        complex = true,
        joined_on = "
            registration.drive_id == drive.drive_id
            AND registration.person_id == person.person_id
        "
    )]
    pub registration: Registration,
}
